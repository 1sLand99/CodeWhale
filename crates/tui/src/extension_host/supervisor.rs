//! One extension-host process: spawn, handshake, channel, exit.
//!
//! Phase 1 has no heartbeat and no auto-restart. When the process exits for
//! any reason, every in-flight call fails with a typed error, the owner
//! registry is revoked wholesale, and the host is marked failed with its
//! stderr tail. Nothing respawns until the next session or a newly enabled
//! plugin.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

use super::protocol::{
    self, CoreRequest, HostLimits, HostMessage, HostNotification, HostRequest, InitializeParams,
    RegisterResult, error_code,
};

/// Budget for `host/hello` → `host/initialize` → `host/ready`. The design's
/// 2 s target is kept for warm starts (measured ~40 ms); the hard limit is
/// wider so a cold, loaded CI machine does not fail the handshake.
pub const HANDSHAKE_DEADLINE: Duration = Duration::from_secs(5);
pub const ACTIVATE_DEADLINE: Duration = Duration::from_secs(5);
pub const DISPOSE_DEADLINE: Duration = Duration::from_secs(2);
/// Grace between `$/cancel` and resolving a call as cancelled on this side.
pub const CANCEL_GRACE: Duration = Duration::from_millis(500);
const STDERR_TAIL_BYTES: usize = 8 * 1024;
const OUTBOUND_QUEUE: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HostCallError {
    #[error("{message} (extension error {code})")]
    Rpc { code: i64, message: String },
    #[error("extension host exited: {0}")]
    Exited(String),
    #[error("cancelled: {0}")]
    Cancelled(String),
    #[error("timed out after {0:?}")]
    Timeout(Duration),
    #[error("extension host channel is full")]
    Busy,
}

/// Callbacks from the channel into the manager.
pub(crate) trait HostEvents: Send + Sync + 'static {
    fn register(&self, params: &protocol::RegisterParams) -> RegisterResult;
    fn unregister(&self, params: &protocol::UnregisterParams);
    fn faulted(&self, params: &protocol::FaultedParams);
    fn exited(&self, host_generation: u64, reason: String, stderr_tail: String);
}

/// Receives one request's outcome.
pub(crate) type CallReceiver = oneshot::Receiver<Result<Value, HostCallError>>;

struct PendingCall {
    tx: oneshot::Sender<Result<Value, HostCallError>>,
    /// Plugin whose revocation cancels this call.
    owner: Option<String>,
    revoked: bool,
}

#[derive(Default)]
struct Handshake {
    hello: Option<oneshot::Sender<protocol::HelloParams>>,
    ready: Option<oneshot::Sender<()>>,
}

pub(crate) struct HostProcess {
    pub pid: Option<u32>,
    pub node_version: std::sync::OnceLock<String>,
    outbound: mpsc::Sender<Vec<u8>>,
    pending: Arc<Mutex<HashMap<u64, PendingCall>>>,
    next_id: AtomicU64,
    stderr_tail: Arc<Mutex<VecDeque<u8>>>,
    exited: tokio::sync::watch::Receiver<bool>,
}

fn push_tail(tail: &Mutex<VecDeque<u8>>, bytes: &[u8]) {
    let mut tail = tail.lock().expect("stderr tail lock");
    tail.extend(bytes);
    while tail.len() > STDERR_TAIL_BYTES {
        tail.pop_front();
    }
}

fn tail_string(tail: &Mutex<VecDeque<u8>>) -> String {
    let tail = tail.lock().expect("stderr tail lock");
    let bytes: Vec<u8> = tail.iter().copied().collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

impl HostProcess {
    /// Spawn the host and complete the handshake. `expected_sha256` is the
    /// digest of the bundle this process materialized; the host's
    /// self-reported digest must match (a consistency check, not
    /// anti-substitution: the control is that Rust chooses what to exec).
    pub(crate) async fn spawn(
        generation: u64,
        node: &Path,
        bundle: &Path,
        expected_sha256: &str,
        events: Arc<dyn HostEvents>,
    ) -> Result<Arc<Self>, String> {
        let mut command = tokio::process::Command::new(node);
        crate::utils::suppress_tokio_console_window(&mut command);
        command
            .arg("--max-old-space-size=256")
            .arg("--disable-proto=throw")
            .arg("--no-addons")
            .arg(bundle)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        // Scrubbed environment: no credentials, no ambient proxy URLs.
        command.env_clear();
        let parent_pid = std::process::id().to_string();
        for (key, value) in crate::child_env::sanitized_plugin_mcp_env_from(
            std::env::vars_os(),
            [("CODEWHALE_HOST_PARENT_PID", parent_pid.as_str())],
        ) {
            command.env(key, value);
        }
        #[cfg(unix)]
        command.process_group(0);

        let mut child = command
            .spawn()
            .map_err(|error| format!("failed to start {}: {error}", node.display()))?;
        let pid = child.id();
        let stdin = child.stdin.take().ok_or("host stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("host stdout unavailable")?;
        let stderr = child.stderr.take().ok_or("host stderr unavailable")?;

        let (outbound, mut outbound_rx) = mpsc::channel::<Vec<u8>>(OUTBOUND_QUEUE);
        let pending: Arc<Mutex<HashMap<u64, PendingCall>>> = Arc::default();
        let stderr_tail: Arc<Mutex<VecDeque<u8>>> = Arc::default();
        let (exited_tx, exited_rx) = tokio::sync::watch::channel(false);
        let (hello_tx, hello_rx) = oneshot::channel();
        let (ready_tx, ready_rx) = oneshot::channel();
        let handshake = Arc::new(Mutex::new(Handshake {
            hello: Some(hello_tx),
            ready: Some(ready_tx),
        }));

        // Writer: the only task that touches stdin. Dropping every sender
        // closes stdin, which the host treats as "core is gone".
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(frame) = outbound_rx.recv().await {
                if stdin.write_all(&frame).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        // stderr: tail for diagnostics, lines into tracing.
        {
            let tail = Arc::clone(&stderr_tail);
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    push_tail(&tail, line.as_bytes());
                    push_tail(&tail, b"\n");
                    tracing::debug!(target: "extension_host", "host stderr: {line}");
                }
            });
        }

        // Reader: every frame is validated strictly; a framing or protocol
        // violation kills the host (a plugin wrote to the channel, or the
        // host is not ours).
        let (kill_tx, mut kill_rx) = mpsc::channel::<String>(1);
        {
            let pending = Arc::clone(&pending);
            let outbound = outbound.clone();
            let events = Arc::clone(&events);
            let handshake = Arc::clone(&handshake);
            let kill_tx = kill_tx.clone();
            tokio::spawn(async move {
                let mut stdout = stdout;
                loop {
                    let value = match protocol::read_frame(&mut stdout).await {
                        Ok(Some(value)) => value,
                        Ok(None) => break,
                        Err(error) => {
                            let _ = kill_tx.try_send(format!("channel framing violation: {error}"));
                            break;
                        }
                    };
                    let message = match protocol::parse_host_message(value) {
                        Ok(message) => message,
                        Err(error) => {
                            let _ = kill_tx.try_send(format!("protocol violation: {error}"));
                            break;
                        }
                    };
                    handle_host_message(message, &pending, &outbound, events.as_ref(), &handshake);
                }
            });
        }

        // Exit watcher: owns the child. On exit, fail everything and report.
        {
            let pending = Arc::clone(&pending);
            let tail = Arc::clone(&stderr_tail);
            let events = Arc::clone(&events);
            tokio::spawn(async move {
                let reason = tokio::select! {
                    status = child.wait() => match status {
                        Ok(status) => format!("exited with {status}"),
                        Err(error) => format!("wait failed: {error}"),
                    },
                    Some(reason) = kill_rx.recv() => {
                        kill_process_tree(pid);
                        let _ = child.kill().await;
                        reason
                    }
                };
                let drained: Vec<PendingCall> = pending
                    .lock()
                    .expect("pending lock")
                    .drain()
                    .map(|(_, call)| call)
                    .collect();
                for call in drained {
                    let _ = call.tx.send(Err(HostCallError::Exited(reason.clone())));
                }
                let _ = exited_tx.send(true);
                // Give the stderr task a moment to capture the last lines.
                tokio::time::sleep(Duration::from_millis(50)).await;
                events.exited(generation, reason, tail_string(&tail));
            });
        }

        let host = Arc::new(Self {
            pid,
            node_version: std::sync::OnceLock::new(),
            outbound,
            pending,
            next_id: AtomicU64::new(1),
            stderr_tail,
            exited: exited_rx,
        });

        let handshake_result = tokio::time::timeout(HANDSHAKE_DEADLINE, async {
            let hello = hello_rx
                .await
                .map_err(|_| "host exited before host/hello".to_string())?;
            if hello.protocol.min > protocol::PROTOCOL_VERSION
                || hello.protocol.max < protocol::PROTOCOL_VERSION
            {
                return Err(format!(
                    "host speaks protocol {}..={}, core speaks {}",
                    hello.protocol.min,
                    hello.protocol.max,
                    protocol::PROTOCOL_VERSION
                ));
            }
            if hello.bundle_sha256 != expected_sha256 {
                return Err(format!(
                    "host bundle digest {} does not match the materialized bundle {}",
                    hello.bundle_sha256, expected_sha256
                ));
            }
            let initialize = CoreRequest::Initialize(InitializeParams {
                protocol: protocol::PROTOCOL_VERSION,
                limits: HostLimits {
                    max_frame: protocol::MAX_FRAME as u64,
                    max_inflight: protocol::MAX_INFLIGHT as u64,
                    dispose_deadline_ms: DISPOSE_DEADLINE.as_millis() as u64,
                    activate_deadline_ms: ACTIVATE_DEADLINE.as_millis() as u64,
                },
            });
            host.request(initialize, None)
                .await
                .map_err(|error| format!("host/initialize failed: {error}"))?;
            ready_rx
                .await
                .map_err(|_| "host exited before host/ready".to_string())?;
            Ok(hello.node_version)
        })
        .await;
        match handshake_result {
            Ok(Ok(node_version)) => {
                let _ = host.node_version.set(node_version);
                Ok(host)
            }
            Ok(Err(reason)) => {
                let _ = kill_tx.try_send(reason.clone());
                Err(format!(
                    "{reason}; stderr: {}",
                    tail_string(&host.stderr_tail)
                ))
            }
            Err(_) => {
                let reason = format!("handshake exceeded {HANDSHAKE_DEADLINE:?}");
                let _ = kill_tx.try_send(reason.clone());
                Err(format!(
                    "{reason}; stderr: {}",
                    tail_string(&host.stderr_tail)
                ))
            }
        }
    }

    #[must_use]
    pub fn has_exited(&self) -> bool {
        *self.exited.borrow()
    }

    fn send_frame(&self, value: &Value) -> Result<(), HostCallError> {
        let frame = protocol::encode_frame(value).map_err(|error| HostCallError::Rpc {
            code: error_code::INVALID_PARAMS,
            message: error.to_string(),
        })?;
        self.outbound.try_send(frame).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => HostCallError::Busy,
            mpsc::error::TrySendError::Closed(_) => {
                HostCallError::Exited("channel closed".to_string())
            }
        })
    }

    /// Send a request; the returned id can be cancelled with [`Self::cancel`].
    pub(crate) fn start_request(
        &self,
        request: CoreRequest,
        owner: Option<String>,
    ) -> Result<(u64, CallReceiver), HostCallError> {
        if self.has_exited() {
            return Err(HostCallError::Exited("already exited".to_string()));
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().expect("pending lock");
            if pending.len() >= protocol::MAX_INFLIGHT {
                return Err(HostCallError::Busy);
            }
            pending.insert(
                id,
                PendingCall {
                    tx,
                    owner,
                    revoked: false,
                },
            );
        }
        if let Err(error) = self.send_frame(&request.to_value(id)) {
            self.pending.lock().expect("pending lock").remove(&id);
            return Err(error);
        }
        Ok((id, rx))
    }

    pub(crate) async fn request(
        &self,
        request: CoreRequest,
        owner: Option<String>,
    ) -> Result<Value, HostCallError> {
        let (_, rx) = self.start_request(request, owner)?;
        rx.await
            .unwrap_or_else(|_| Err(HostCallError::Exited("channel closed".to_string())))
    }

    /// Request with a deadline; on expiry the call is cancelled host-side.
    pub(crate) async fn request_with_deadline(
        &self,
        request: CoreRequest,
        owner: Option<String>,
        deadline: Duration,
    ) -> Result<Value, HostCallError> {
        let (id, rx) = self.start_request(request, owner)?;
        match tokio::time::timeout(deadline, rx).await {
            Ok(result) => {
                result.unwrap_or_else(|_| Err(HostCallError::Exited("channel closed".to_string())))
            }
            Err(_) => {
                self.cancel(id);
                self.pending.lock().expect("pending lock").remove(&id);
                Err(HostCallError::Timeout(deadline))
            }
        }
    }

    /// Fire `$/cancel`. Best effort: a full or closed channel is fine, the
    /// caller resolves its side on its own schedule.
    pub(crate) fn cancel(&self, id: u64) {
        let _ = self.send_frame(&protocol::cancel_value(id));
    }

    /// Forget a request without cancelling it (its answer will be dropped).
    pub(crate) fn forget(&self, id: u64) {
        self.pending.lock().expect("pending lock").remove(&id);
    }

    /// Revocation: cancel every in-flight call owned by `plugin_id`; each
    /// resolves as cancelled when the host answers or after `CANCEL_GRACE`,
    /// whichever is first — revocation never waits on the host.
    pub(crate) fn revoke_calls_of(self: &Arc<Self>, plugin_id: &str) {
        let ids: Vec<u64> = {
            let mut pending = self.pending.lock().expect("pending lock");
            pending
                .iter_mut()
                .filter(|(_, call)| call.owner.as_deref() == Some(plugin_id))
                .map(|(id, call)| {
                    call.revoked = true;
                    *id
                })
                .collect()
        };
        for id in ids {
            self.cancel(id);
            let pending = Arc::clone(&self.pending);
            tokio::spawn(async move {
                tokio::time::sleep(CANCEL_GRACE).await;
                if let Some(call) = pending.lock().expect("pending lock").remove(&id) {
                    let _ = call.tx.send(Err(HostCallError::Cancelled(
                        "extension was revoked".to_string(),
                    )));
                }
            });
        }
    }

    /// Bounded shutdown: `host/shutdown` (2 s), close stdin, then kill the
    /// process group at 3 s total.
    #[cfg(test)]
    pub(crate) async fn shutdown(&self) {
        let _ = tokio::time::timeout(
            Duration::from_secs(2),
            self.request(CoreRequest::Shutdown, None),
        )
        .await;
        let mut exited = self.exited.clone();
        let waited = tokio::time::timeout(Duration::from_secs(1), async {
            while !*exited.borrow() {
                if exited.changed().await.is_err() {
                    break;
                }
            }
        })
        .await;
        if waited.is_err() {
            kill_process_tree(self.pid);
        }
    }
}

impl Drop for HostProcess {
    fn drop(&mut self) {
        if !self.has_exited() {
            kill_process_tree(self.pid);
        }
    }
}

/// Kill the host's process group (Unix) or the process (elsewhere).
///
/// Phase-1 deviation from the design: the hooks' `HookProcessTree` /
/// `WindowsHookJob` were not moved into a shared module; the host is its
/// own process group on Unix and `kill_on_drop` covers the immediate child
/// elsewhere. Phase-1 hosts spawn no brokered children.
pub(crate) fn kill_process_tree(pid: Option<u32>) {
    #[cfg(unix)]
    if let Some(pid) = pid {
        // SAFETY: kill(2) dereferences no pointers; a negative pid names the
        // process group created with `process_group(0)` at spawn.
        unsafe {
            let _ = libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    let _ = pid;
}

fn handle_host_message(
    message: HostMessage,
    pending: &Mutex<HashMap<u64, PendingCall>>,
    outbound: &mpsc::Sender<Vec<u8>>,
    events: &dyn HostEvents,
    handshake: &Mutex<Handshake>,
) {
    let send = |value: Value| {
        if let Ok(frame) = protocol::encode_frame(&value) {
            let _ = outbound.try_send(frame);
        }
    };
    match message {
        HostMessage::Response { id, outcome } => {
            let Some(call) = pending.lock().expect("pending lock").remove(&id) else {
                tracing::debug!(target: "extension_host", id, "dropping late host response");
                return;
            };
            let result = if call.revoked {
                Err(HostCallError::Cancelled(
                    "extension was revoked".to_string(),
                ))
            } else {
                match outcome {
                    Ok(value) => Ok(value),
                    Err(error) if error.code == error_code::CANCELLED => {
                        Err(HostCallError::Cancelled(error.message))
                    }
                    Err(error) => Err(HostCallError::Rpc {
                        code: error.code,
                        message: error.message,
                    }),
                }
            };
            let _ = call.tx.send(result);
        }
        HostMessage::Request { id, request } => match request {
            HostRequest::Register(params) => {
                let result = events.register(&params);
                send(protocol::response_ok(
                    id,
                    serde_json::to_value(result).unwrap_or_else(|_| json!({"refused": "internal"})),
                ));
            }
            HostRequest::Unregister(params) => {
                events.unregister(&params);
                send(protocol::response_ok(id, json!({})));
            }
        },
        HostMessage::Notification(notification) => match notification {
            HostNotification::Hello(hello) => {
                if let Some(tx) = handshake.lock().expect("handshake lock").hello.take() {
                    let _ = tx.send(hello);
                }
            }
            HostNotification::Ready => {
                if let Some(tx) = handshake.lock().expect("handshake lock").ready.take() {
                    let _ = tx.send(());
                }
            }
            HostNotification::Faulted(params) => events.faulted(&params),
            HostNotification::Log(log) => {
                let plugin = log.plugin_id.as_deref().unwrap_or("host");
                match log.level.as_str() {
                    "error" => tracing::warn!(target: "extension_host", plugin, "{}", log.msg),
                    "warn" => tracing::info!(target: "extension_host", plugin, "{}", log.msg),
                    _ => tracing::debug!(target: "extension_host", plugin, "{}", log.msg),
                }
            }
            // Phase 1 has no host-originated requests to cancel.
            HostNotification::Cancel(_) => {}
        },
    }
}

/// Where the embedded bundle is written: `<root>/extension-host/<sha256>/`.
#[must_use]
pub fn bundle_dir(root: &Path, sha256: &str) -> PathBuf {
    root.join("extension-host").join(sha256)
}
