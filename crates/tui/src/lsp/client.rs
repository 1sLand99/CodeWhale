//! Thin JSON-RPC over stdio client for LSP servers.
//!
//! We deliberately do **not** depend on `tower-lsp` — it is a server-side
//! framework and dragging it in here would add hundreds of unnecessary
//! transitive dependencies and slow down `cargo build` for every contributor.
//! The LSP wire protocol is small enough that handling it ourselves is a
//! self-contained ~400 LOC and lets us keep total control of the spawn
//! lifecycle, timeouts, and the async surface.
//!
//! Architecture:
//!
//! - [`LspTransport`] is the trait the [`super::LspManager`] talks to. The
//!   real implementation is [`StdioLspTransport`] (forks an LSP server with
//!   `tokio::process::Command`); tests use `super::tests::FakeTransport`.
//! - [`StdioLspTransport`] runs three tokio tasks: a reader, a writer, and
//!   the public API. Communication uses tokio mpsc channels.
//! - We parse `Content-Length`-framed JSON-RPC and route inbound messages
//!   either to a per-request response slot (for replies) or to the
//!   diagnostics queue (for `textDocument/publishDiagnostics` notifications).
//!
//! The transport is one-shot per file in MVP form: the manager spawns a
//! transport on demand for a language and reuses it. We do not implement
//! workspace sync beyond didOpen/didChange because the goal is "post-edit
//! diagnostics," not full IDE smartness.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use super::diagnostics::{Diagnostic, Severity};
use crate::utils::spawn_supervised;

/// A publication retains the server's document version instead of pretending
/// that a matching URI alone proves which text was checked.
#[derive(Debug)]
pub struct DiagnosticPublication {
    pub items: Vec<Diagnostic>,
    pub document_version: Option<i64>,
    pub diagnostic_version: Option<i64>,
}

impl DiagnosticPublication {
    #[must_use]
    pub fn freshness(&self) -> &'static str {
        if self.document_version.is_some() && self.document_version == self.diagnostic_version {
            "verified"
        } else {
            "unverified"
        }
    }
}

// Diagnostic-only transports have no document-version proof. Existing callers
// may still use their results, but must not claim freshness from an empty list.
impl From<Vec<Diagnostic>> for DiagnosticPublication {
    fn from(items: Vec<Diagnostic>) -> Self {
        Self {
            items,
            document_version: None,
            diagnostic_version: None,
        }
    }
}

/// Trait the LSP manager talks to. A real LSP server speaks this via stdio;
/// tests use an in-process fake.
#[async_trait]
pub trait LspTransport: Send + Sync {
    /// Notify the server that a file was opened or its contents updated, then
    /// wait up to `wait` for a `publishDiagnostics` notification for that
    /// file. Returns the diagnostics list (possibly empty). Implementations
    /// must NOT block past `wait`.
    async fn diagnostics_for(
        &self,
        path: &Path,
        text: &str,
        wait: Duration,
    ) -> Result<DiagnosticPublication>;

    /// Send a JSON-RPC request and wait up to `wait` for the reply.
    ///
    /// Default returns "unsupported" so diagnostic-only fakes keep working.
    /// Real transports implement this for go-to-definition, symbols, and
    /// references without spawning a second server lifecycle.
    async fn request(&self, _method: &str, _params: Value, _wait: Duration) -> Result<Value> {
        Err(anyhow!("LSP request not supported by this transport"))
    }

    /// Ensure `path` is open with `text` (didOpen/didChange) so position-based
    /// requests can target it. Default is a no-op; real transports track opens.
    async fn ensure_open(&self, _path: &Path, _text: &str) -> Result<()> {
        Ok(())
    }

    /// Best-effort shutdown. Called via `LspManager::shutdown_all`.
    #[expect(dead_code)]
    async fn shutdown(&self);
}

/// Stdio-backed transport. Spawns the LSP server as a child process and
/// pipes JSON-RPC over stdin/stdout. Stderr is captured into a buffer so
/// callers can include it in error messages without polluting our own stderr.
pub struct StdioLspTransport {
    /// JoinHandle for the running server. Held so the child stays alive for
    /// the transport's lifetime; consumed during `shutdown`.
    #[expect(dead_code)]
    child: AsyncMutex<Option<Child>>,
    /// Outgoing message sender to the writer task.
    tx_outbound: mpsc::Sender<Vec<u8>>,
    /// Inbound diagnostics queue. We push every `publishDiagnostics`
    /// notification into here and the public API drains the relevant entries.
    diagnostics_gate: AsyncMutex<()>,
    diagnostics_rx: AsyncMutex<mpsc::Receiver<(PathBuf, Option<i64>, Vec<Diagnostic>)>>,
    /// Map of in-flight request id -> reply slot for model-facing intelligence
    /// requests (definition, references, symbols).
    pending: Arc<AsyncMutex<HashMap<i64, oneshot::Sender<Value>>>>,
    /// Monotonic request id counter for JSON-RPC request/reply methods.
    next_id: AsyncMutex<i64>,
    /// Language id passed in `textDocument/didOpen` (e.g. "rust").
    language_id: String,
    /// Track which files we have opened so the second touch sends
    /// `didChange` instead of `didOpen`.
    opened: AsyncMutex<HashMap<PathBuf, i64>>,
}

impl StdioLspTransport {
    /// Spawn `command args…` and run the LSP `initialize` handshake. Returns
    /// `Err` immediately if the binary is not on PATH or `initialize` fails.
    pub async fn spawn(
        command: &str,
        args: &[String],
        language_id: &str,
        workspace: PathBuf,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn LSP server `{command}`"))?;

        let stdin = child
            .stdin
            .take()
            .context("LSP child has no stdin handle")?;
        let stdout = child
            .stdout
            .take()
            .context("LSP child has no stdout handle")?;

        let (tx_outbound, rx_outbound) = mpsc::channel::<Vec<u8>>(64);
        let (tx_inbound, rx_inbound) = mpsc::channel::<Value>(64);
        let (tx_diag, rx_diag) = mpsc::channel::<(PathBuf, Option<i64>, Vec<Diagnostic>)>(64);

        // Writer task: drain outbound channel, frame with Content-Length, write to stdin.
        spawn_supervised(
            "lsp-writer",
            std::panic::Location::caller(),
            writer_task(stdin, rx_outbound),
        );
        // Reader task: parse Content-Length frames from stdout, push to inbound queue.
        spawn_supervised(
            "lsp-reader",
            std::panic::Location::caller(),
            reader_task(stdout, tx_inbound),
        );
        // Inbound dispatcher: routes notifications to `tx_diag`, replies to a
        // pending map. We keep the pending map for completeness even though
        // diagnostics polling itself does not reuse it.
        let pending: Arc<AsyncMutex<HashMap<i64, oneshot::Sender<Value>>>> =
            Arc::new(AsyncMutex::new(HashMap::new()));
        spawn_supervised(
            "lsp-dispatcher",
            std::panic::Location::caller(),
            dispatcher_task(rx_inbound, tx_diag, pending.clone()),
        );

        // Send `initialize` and wait for `initialized`. We synthesize id=1.
        let init_payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": uri_from_path(&workspace),
                "capabilities": {
                    "textDocument": {
                        "publishDiagnostics": { "relatedInformation": false, "versionSupport": true }
                    }
                },
                "workspaceFolders": [{
                    "uri": uri_from_path(&workspace),
                    "name": "workspace"
                }]
            }
        });
        send_message(&tx_outbound, &init_payload).await?;

        // We do not actually wait for the initialize response here in MVP —
        // most servers buffer notifications until they are ready, and waiting
        // for `initialize` reply doubles the latency of the first edit. Send
        // `initialized` immediately and let publishDiagnostics arrive on its
        // own clock.
        let initialized = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });
        send_message(&tx_outbound, &initialized).await?;

        Ok(Self {
            child: AsyncMutex::new(Some(child)),
            tx_outbound,
            diagnostics_gate: AsyncMutex::new(()),
            diagnostics_rx: AsyncMutex::new(rx_diag),
            pending,
            next_id: AsyncMutex::new(2),
            language_id: language_id.to_string(),
            opened: AsyncMutex::new(HashMap::new()),
        })
    }
}

impl StdioLspTransport {
    async fn open_or_change(&self, path: &Path, text: &str) -> Result<(String, i64)> {
        let path_buf = path.to_path_buf();
        let uri = uri_from_path(&path_buf);
        let mut opened = self.opened.lock().await;
        let is_new = !opened.contains_key(&path_buf);
        let new_version = opened.get(&path_buf).copied().unwrap_or(0) + 1;

        let payload = if is_new {
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri.clone(),
                        "languageId": self.language_id,
                        "version": new_version,
                        "text": text
                    }
                }
            })
        } else {
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri.clone(),
                        "version": new_version
                    },
                    "contentChanges": [{ "text": text }]
                }
            })
        };
        send_message(&self.tx_outbound, &payload).await?;
        opened.insert(path_buf, new_version);
        Ok((uri, new_version))
    }
}

#[async_trait]
impl LspTransport for StdioLspTransport {
    async fn diagnostics_for(
        &self,
        path: &Path,
        text: &str,
        wait: Duration,
    ) -> Result<DiagnosticPublication> {
        // One receiver cannot serve concurrent polling safely: serialize the
        // open/version/send/wait transaction, including semantic ensure_open.
        let deadline = tokio::time::Instant::now() + wait;
        let _gate = timeout(wait, self.diagnostics_gate.lock())
            .await
            .map_err(|_| anyhow!("LSP diagnostics timed out waiting for another document"))?;
        let path_buf = path.to_path_buf();
        let (_, version) = timeout(
            deadline.saturating_duration_since(tokio::time::Instant::now()),
            self.open_or_change(path, text),
        )
        .await
        .map_err(|_| anyhow!("LSP diagnostics timed out sending document"))??;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(anyhow!(
                    "LSP diagnostics timed out before a current publication"
                ));
            }
            let mut rx = self.diagnostics_rx.lock().await;
            let (file, published_version, items) = match timeout(remaining, rx.recv()).await {
                Ok(Some(item)) => item,
                Ok(None) => {
                    return Err(anyhow!(
                        "LSP diagnostics channel closed before publishDiagnostics"
                    ));
                }
                Err(_) => {
                    return Err(anyhow!(
                        "LSP diagnostics timed out before a current publication"
                    ));
                }
            };
            if file != path_buf || published_version.is_some_and(|published| published != version) {
                continue;
            }
            return Ok(DiagnosticPublication {
                items,
                document_version: Some(version),
                diagnostic_version: published_version,
            });
        }
    }

    async fn ensure_open(&self, path: &Path, text: &str) -> Result<()> {
        let _gate = self.diagnostics_gate.lock().await;
        self.open_or_change(path, text).await?;
        Ok(())
    }

    async fn request(&self, method: &str, params: Value, wait: Duration) -> Result<Value> {
        let id = {
            let mut next = self.next_id.lock().await;
            let id = *next;
            *next = next.saturating_add(1);
            id
        };
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        if let Err(err) = send_message(&self.tx_outbound, &payload).await {
            let mut pending = self.pending.lock().await;
            pending.remove(&id);
            return Err(err);
        }
        match timeout(wait, rx).await {
            Ok(Ok(reply)) => {
                if let Some(error) = reply.get("error") {
                    let message = error
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("LSP request failed");
                    return Err(anyhow!("{message}"));
                }
                Ok(reply.get("result").cloned().unwrap_or(Value::Null))
            }
            Ok(Err(_)) => Err(anyhow!("LSP request channel closed")),
            Err(_) => {
                let mut pending = self.pending.lock().await;
                pending.remove(&id);
                Err(anyhow!("LSP request timed out for {method}"))
            }
        }
    }

    async fn shutdown(&self) {
        let mut child = self.child.lock().await;
        if let Some(mut c) = child.take() {
            let _ = c.start_kill();
            let _ = c.wait().await;
        }
    }
}

/// Send a JSON value as one Content-Length-framed JSON-RPC message.
async fn send_message(tx: &mpsc::Sender<Vec<u8>>, value: &Value) -> Result<()> {
    let body = serde_json::to_vec(value).context("serialize LSP message")?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut frame = Vec::with_capacity(header.len() + body.len());
    frame.extend_from_slice(header.as_bytes());
    frame.extend_from_slice(&body);
    tx.send(frame)
        .await
        .map_err(|_| anyhow!("LSP outbound channel closed"))?;
    Ok(())
}

/// Background task that drains the outbound queue and writes each frame to
/// the LSP server's stdin. Exits cleanly when the channel closes.
async fn writer_task(mut stdin: tokio::process::ChildStdin, mut rx: mpsc::Receiver<Vec<u8>>) {
    while let Some(frame) = rx.recv().await {
        if stdin.write_all(&frame).await.is_err() {
            break;
        }
        if stdin.flush().await.is_err() {
            break;
        }
    }
}

/// Background task that parses `Content-Length`-framed JSON-RPC frames from
/// the LSP server's stdout. Pushes each parsed JSON value to `tx`. Exits
/// when stdout closes or a frame is malformed (we choose to fail closed
/// rather than risk hanging).
async fn reader_task(mut stdout: tokio::process::ChildStdout, tx: mpsc::Sender<Value>) {
    let mut buf: Vec<u8> = Vec::with_capacity(8 * 1024);
    let mut tmp = [0u8; 4096];
    loop {
        let n = match stdout.read(&mut tmp).await {
            Ok(0) => return,
            Ok(n) => n,
            Err(_) => return,
        };
        buf.extend_from_slice(&tmp[..n]);
        // Try to parse as many frames as we can from the accumulated buffer.
        while let Some((header_end, content_length)) = parse_header(&buf) {
            if buf.len() < header_end + content_length {
                break; // need more bytes
            }
            let body = &buf[header_end..header_end + content_length];
            let parsed = serde_json::from_slice::<Value>(body).ok();
            // Drop the consumed bytes regardless of parse result so a bad frame
            // does not stall the loop.
            buf.drain(..header_end + content_length);
            if let Some(value) = parsed
                && tx.send(value).await.is_err()
            {
                return;
            }
        }
    }
}

/// Parse a JSON-RPC header block. Returns `Some((header_end, content_length))`
/// where `header_end` is the byte offset of the first body byte. The header
/// terminator is `\r\n\r\n`. We require a `Content-Length` header.
fn parse_header(buf: &[u8]) -> Option<(usize, usize)> {
    let term = b"\r\n\r\n";
    let pos = buf.windows(term.len()).position(|window| window == term)?;
    let header = std::str::from_utf8(&buf[..pos]).ok()?;
    let mut content_length: Option<usize> = None;
    for line in header.split("\r\n") {
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse::<usize>().ok();
        }
    }
    content_length.map(|cl| (pos + term.len(), cl))
}

/// Background task that consumes inbound JSON values, classifies them as
/// notifications/responses, and routes accordingly.
async fn dispatcher_task(
    mut rx: mpsc::Receiver<Value>,
    tx_diag: mpsc::Sender<(PathBuf, Option<i64>, Vec<Diagnostic>)>,
    pending: Arc<AsyncMutex<HashMap<i64, oneshot::Sender<Value>>>>,
) {
    while let Some(value) = rx.recv().await {
        // Notifications have a `method` and no `id`.
        let method = value.get("method").and_then(|v| v.as_str());
        if method == Some("textDocument/publishDiagnostics") {
            if let Some(publication) = parse_publish_diagnostics(&value) {
                let _ = tx_diag.send(publication).await;
            }
            continue;
        }
        // Replies have an `id` and a `result` or `error`.
        if let Some(id) = value.get("id").and_then(|v| v.as_i64()) {
            let mut map = pending.lock().await;
            if let Some(slot) = map.remove(&id) {
                let _ = slot.send(value);
            }
        }
    }
}

/// Decode a `textDocument/publishDiagnostics` notification.
fn parse_publish_diagnostics(value: &Value) -> Option<(PathBuf, Option<i64>, Vec<Diagnostic>)> {
    let params = value.get("params")?;
    let uri = params.get("uri")?.as_str()?;
    let path = path_from_uri(uri)?;
    let version = match params.get("version") {
        None | Some(Value::Null) => None,
        Some(value) => Some(value.as_i64()?),
    };
    let raw = params.get("diagnostics")?.as_array()?;
    let mut out = Vec::with_capacity(raw.len());
    for d in raw {
        let range = d.get("range")?;
        let start = range.get("start")?;
        let line = start.get("line")?.as_u64()? as u32 + 1;
        let column = start.get("character")?.as_u64()? as u32 + 1;
        let severity = Severity::from_lsp(d.get("severity").and_then(|v| v.as_i64()))
            .unwrap_or(Severity::Error);
        let message = d
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        out.push(Diagnostic {
            line,
            column,
            severity,
            message,
        });
    }
    Some((path, version, out))
}

/// Convert a filesystem path to a `file://` URI. Best-effort — we do not
/// support Windows drive letters perfectly, but the LSP servers in our
/// registry accept percent-encoded paths well enough for the post-edit
/// diagnostics use case.
pub(crate) fn uri_from_path(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let s = canonical.to_string_lossy();
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{}", s.trim_start_matches('/'))
    }
}

/// Inverse of [`uri_from_path`]. Returns `None` when the URI is not a `file://`.
fn path_from_uri(uri: &str) -> Option<PathBuf> {
    let stripped = uri.strip_prefix("file://")?;
    Some(PathBuf::from(stripped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lsp_header() {
        let frame = b"Content-Length: 5\r\n\r\nhello";
        let (end, len) = parse_header(frame).expect("header parses");
        assert_eq!(end, 21);
        assert_eq!(len, 5);
    }

    #[test]
    fn parse_header_returns_none_when_truncated() {
        let frame = b"Content-Length: 5\r\nMissingTerm";
        assert!(parse_header(frame).is_none());
    }

    #[test]
    fn parses_publish_diagnostics_payload() {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///tmp/foo.rs",
                "diagnostics": [
                    {
                        "range": {
                            "start": { "line": 11, "character": 7 },
                            "end":   { "line": 11, "character": 8 }
                        },
                        "severity": 1,
                        "message": "missing semicolon"
                    }
                ]
            }
        });
        let (path, version, diags) = parse_publish_diagnostics(&payload).expect("parses");
        assert_eq!(path, PathBuf::from("/tmp/foo.rs"));
        assert_eq!(version, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 12);
        assert_eq!(diags[0].column, 8);
        assert_eq!(diags[0].severity, Severity::Error);
        assert_eq!(diags[0].message, "missing semicolon");
    }

    #[test]
    fn round_trips_uri_path() {
        let path = PathBuf::from("/tmp/example/foo.rs");
        let uri = format!("file://{}", path.display());
        assert_eq!(path_from_uri(&uri), Some(path));
    }

    #[tokio::test]
    async fn closed_diagnostics_channel_is_an_error_not_an_empty_result() {
        let (tx_outbound, _rx_outbound) = mpsc::channel(1);
        let (tx_diag, rx_diag) = mpsc::channel(1);
        drop(tx_diag);
        let transport = StdioLspTransport {
            child: AsyncMutex::new(None),
            tx_outbound,
            diagnostics_gate: AsyncMutex::new(()),
            diagnostics_rx: AsyncMutex::new(rx_diag),
            pending: Arc::new(AsyncMutex::new(HashMap::new())),
            next_id: AsyncMutex::new(1),
            language_id: "rust".to_string(),
            opened: AsyncMutex::new(HashMap::new()),
        };

        let error = transport
            .diagnostics_for(
                Path::new("/tmp/closed-channel.rs"),
                "fn main() {}\n",
                Duration::from_millis(10),
            )
            .await
            .expect_err("a closed transport must not look like an empty lint result");

        assert!(
            error.to_string().contains("diagnostics channel closed"),
            "unexpected error: {error}"
        );
    }

    fn diagnostic_fixture() -> (
        StdioLspTransport,
        mpsc::Receiver<Vec<u8>>,
        mpsc::Sender<(PathBuf, Option<i64>, Vec<Diagnostic>)>,
    ) {
        let (tx_outbound, rx_outbound) = mpsc::channel(8);
        let (tx_diag, rx_diag) = mpsc::channel(8);
        (
            StdioLspTransport {
                child: AsyncMutex::new(None),
                tx_outbound,
                diagnostics_gate: AsyncMutex::new(()),
                diagnostics_rx: AsyncMutex::new(rx_diag),
                pending: Arc::new(AsyncMutex::new(HashMap::new())),
                next_id: AsyncMutex::new(1),
                language_id: "rust".into(),
                opened: AsyncMutex::new(HashMap::new()),
            },
            rx_outbound,
            tx_diag,
        )
    }

    async fn next_document(rx: &mut mpsc::Receiver<Vec<u8>>) -> Value {
        let frame = rx.recv().await.unwrap();
        let (start, _) = parse_header(&frame).unwrap();
        serde_json::from_slice::<Value>(&frame[start..]).unwrap()
    }

    fn diagnostic(line: u32) -> Diagnostic {
        Diagnostic {
            line,
            column: 1,
            severity: Severity::Error,
            message: "fixture".into(),
        }
    }

    #[tokio::test]
    async fn diagnostic_freshness_rejects_old_version_and_accepts_current_empty_publication() {
        let (transport, mut outbound, diag) = diagnostic_fixture();
        let server = tokio::spawn(async move {
            let first = next_document(&mut outbound).await;
            assert_eq!(first["method"], "textDocument/didOpen");
            assert_eq!(first["params"]["textDocument"]["version"], 1);
            let path =
                path_from_uri(first["params"]["textDocument"]["uri"].as_str().unwrap()).unwrap();
            diag.send((path.clone(), Some(1), vec![diagnostic(1)]))
                .await
                .unwrap();
            let second = next_document(&mut outbound).await;
            assert_eq!(second["method"], "textDocument/didChange");
            assert_eq!(second["params"]["textDocument"]["version"], 2);
            assert_eq!(second["params"]["contentChanges"][0]["text"], "new 🐋 text");
            diag.send((path.clone(), Some(1), vec![diagnostic(99)]))
                .await
                .unwrap();
            diag.send((path, Some(2), vec![])).await.unwrap();
        });
        let path = Path::new("/tmp/freshness.rs");
        let first = transport
            .diagnostics_for(path, "old text", Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(first.freshness(), "verified");
        assert_eq!(first.items[0].line, 1);
        let second = transport
            .diagnostics_for(path, "new 🐋 text", Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(second.freshness(), "verified");
        assert_eq!(second.document_version, Some(2));
        assert_eq!(second.diagnostic_version, Some(2));
        assert!(
            second.items.is_empty(),
            "old error must not apply to the new text"
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn diagnostic_freshness_serializes_concurrent_file_requests() {
        let (transport, mut outbound, diag) = diagnostic_fixture();
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let request = next_document(&mut outbound).await;
                assert!(
                    matches!(outbound.try_recv(), Err(mpsc::error::TryRecvError::Empty)),
                    "second file must not advance before this publication"
                );
                let path =
                    path_from_uri(request["params"]["textDocument"]["uri"].as_str().unwrap())
                        .unwrap();
                let line = if path.ends_with("one.rs") { 1 } else { 2 };
                diag.send((
                    PathBuf::from("/tmp/unrelated.rs"),
                    Some(1),
                    vec![diagnostic(99)],
                ))
                .await
                .unwrap();
                diag.send((path, Some(1), vec![diagnostic(line)]))
                    .await
                    .unwrap();
            }
        });
        let (one, two) = tokio::join!(
            transport.diagnostics_for(Path::new("/tmp/one.rs"), "one", Duration::from_secs(1)),
            transport.diagnostics_for(Path::new("/tmp/two.rs"), "two", Duration::from_secs(1)),
        );
        assert_eq!(one.unwrap().items[0].line, 1);
        assert_eq!(two.unwrap().items[0].line, 2);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn diagnostic_freshness_unversioned_is_unverified_and_silence_is_error() {
        let (transport, mut outbound, diag) = diagnostic_fixture();
        let server = tokio::spawn(async move {
            let request = next_document(&mut outbound).await;
            let path =
                path_from_uri(request["params"]["textDocument"]["uri"].as_str().unwrap()).unwrap();
            diag.send((path, None, vec![])).await.unwrap();
            // Keep the channels alive while the second request times out.
            let _ = next_document(&mut outbound).await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        });
        let response = transport
            .diagnostics_for(
                Path::new("/tmp/unversioned.rs"),
                "text",
                Duration::from_secs(1),
            )
            .await
            .unwrap();
        assert_eq!(response.freshness(), "unverified");
        assert_eq!(response.diagnostic_version, None);
        assert!(
            transport
                .diagnostics_for(
                    Path::new("/tmp/silent.rs"),
                    "text",
                    Duration::from_millis(10)
                )
                .await
                .unwrap_err()
                .to_string()
                .contains("timed out")
        );
        server.abort();
    }

    #[test]
    fn diagnostic_freshness_parser_retains_version_and_rejects_malformed_version() {
        let mut payload =
            json!({"params":{"uri":"file:///tmp/version.rs","version":4,"diagnostics":[]}});
        assert_eq!(parse_publish_diagnostics(&payload).unwrap().1, Some(4));
        payload["params"]["version"] = json!("4");
        assert!(parse_publish_diagnostics(&payload).is_none());
    }
}
