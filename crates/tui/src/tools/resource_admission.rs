//! Cross-process admission for expensive local commands.
//!
//! Fleet and Workflow workers execute in separate Codewhale processes, so an
//! in-process semaphore cannot protect the host. Heavy shell commands instead
//! take one of a small number of filesystem-backed permits under
//! `CODEWHALE_HOME`. The default of two permits is deliberately conservative
//! for the 36 GiB laptop class from #4864.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use fd_lock::{RwLock, RwLockWriteGuard};
use tokio_util::sync::CancellationToken;

pub(crate) const DEFAULT_HEAVY_COMMAND_LIMIT: usize = 2;
const MAX_HEAVY_COMMAND_LIMIT: usize = 16;
const ADMISSION_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandExpense {
    Normal,
    Heavy,
}

#[derive(Debug)]
struct HeavyPermitSlot {
    _guard: RwLockWriteGuard<'static, File>,
    // The guard borrows the lock. Keeping the boxed lock here makes that
    // allocation outlive the guard; field drop order is guard, then lock.
    _lock: Box<RwLock<File>>,
}

/// A held cross-process heavy-command permit.
#[derive(Debug)]
pub(crate) struct HeavyCommandPermit {
    _slot: HeavyPermitSlot,
    queued_for: Duration,
    limit: usize,
}

impl HeavyCommandPermit {
    pub(crate) fn queued_for(&self) -> Duration {
        self.queued_for
    }

    pub(crate) fn limit(&self) -> usize {
        self.limit
    }
}

pub(crate) fn infer_command_expense(command: &str) -> CommandExpense {
    let heavy = command
        .split(|character| matches!(character, '\n' | '\r' | ';' | '|' | '&'))
        .any(segment_is_heavy);

    if heavy {
        CommandExpense::Heavy
    } else {
        CommandExpense::Normal
    }
}

fn segment_is_heavy(segment: &str) -> bool {
    let tokens: Vec<String> = segment
        .split_whitespace()
        .map(|token| token.trim_matches(['"', '\'']).to_string())
        .collect();
    let Some(index) = tokens
        .iter()
        .position(|token| !token.contains('=') && token != "env")
    else {
        return false;
    };
    let executable = Path::new(&tokens[index])
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(executable.as_str(), "cargo" | "rustc") {
        return false;
    }
    if executable == "rustc" {
        return true;
    }
    tokens[index + 1..]
        .iter()
        .map(|arg| arg.trim().to_ascii_lowercase())
        .find(|arg| !arg.is_empty() && !arg.starts_with('-') && !arg.contains('='))
        .is_some_and(|subcommand| {
            matches!(
                subcommand.as_str(),
                "build" | "test" | "check" | "clippy" | "rustc"
            )
        })
}

pub(crate) async fn acquire_heavy_command_permit(
    command: &str,
    cancel: Option<&CancellationToken>,
) -> Result<Option<HeavyCommandPermit>> {
    if infer_command_expense(command) == CommandExpense::Normal {
        return Ok(None);
    }

    let limit = configured_heavy_command_limit();
    let root = admission_root();
    acquire_heavy_command_permit_at(&root, limit, cancel).await.map(Some)
}

async fn acquire_heavy_command_permit_at(
    root: &Path,
    limit: usize,
    cancel: Option<&CancellationToken>,
) -> Result<HeavyCommandPermit> {
    std::fs::create_dir_all(root)
        .with_context(|| format!("creating resource admission directory {}", root.display()))?;
    let started = Instant::now();

    loop {
        if cancel.is_some_and(|token| token.is_cancelled()) {
            return Err(anyhow!(
                "heavy command canceled while queued for resource admission"
            ));
        }
        for slot in 0..limit {
            let path = root.join(format!("heavy-{slot}.lock"));
            match try_lock_slot(&path) {
                Ok(Some(slot)) => {
                    return Ok(HeavyCommandPermit {
                        _slot: slot,
                        queued_for: started.elapsed(),
                        limit,
                    });
                }
                Ok(None) => {}
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("acquiring heavy command permit {}", path.display())
                    });
                }
            }
        }
        tokio::time::sleep(ADMISSION_POLL_INTERVAL).await;
    }
}

fn try_lock_slot(path: &Path) -> io::Result<Option<HeavyPermitSlot>> {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)?;
    let lock = Box::new(RwLock::new(file));
    let lock_ptr = Box::into_raw(lock);
    // SAFETY: `lock_ptr` remains allocated in `HeavyPermitSlot::_lock` for the
    // lifetime of `_guard`, and the guard is dropped before that box.
    let guard = match unsafe { (&mut *lock_ptr).try_write() } {
        Ok(guard) => guard,
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
            // SAFETY: no guard was created, so reclaim the allocation now.
            unsafe { drop(Box::from_raw(lock_ptr)) };
            return Ok(None);
        }
        Err(error) => {
            // SAFETY: no guard was created, so reclaim the allocation now.
            unsafe { drop(Box::from_raw(lock_ptr)) };
            return Err(error);
        }
    };
    // SAFETY: the allocation is owned exactly once by this box and is stable on
    // the heap even if `HeavyPermitSlot` moves.
    let lock = unsafe { Box::from_raw(lock_ptr) };
    // SAFETY: the boxed lock remains alive until after `_guard` is dropped.
    let guard = unsafe {
        std::mem::transmute::<RwLockWriteGuard<'_, File>, RwLockWriteGuard<'static, File>>(guard)
    };
    Ok(Some(HeavyPermitSlot {
        _guard: guard,
        _lock: lock,
    }))
}

fn configured_heavy_command_limit() -> usize {
    std::env::var("CODEWHALE_HEAVY_COMMAND_LIMIT")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|limit| *limit > 0)
        .unwrap_or(DEFAULT_HEAVY_COMMAND_LIMIT)
        .min(MAX_HEAVY_COMMAND_LIMIT)
}

fn admission_root() -> PathBuf {
    if let Some(home) = std::env::var_os("CODEWHALE_HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return home.join("resource-admission");
    }
    if let Some(home) = crate::config::effective_home_dir() {
        return home.join(".codewhale").join("resource-admission");
    }
    std::env::temp_dir().join("codewhale-resource-admission")
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    #[test]
    fn infers_only_expensive_rust_compilation_commands() {
        for command in [
            "cargo test -p codewhale-tui shell::tests",
            "env CARGO_BUILD_JOBS=2 cargo build --workspace",
            "cargo check",
            "cargo clippy --all-targets",
            "/usr/bin/rustc src/main.rs",
            "printf ok && cargo rustc -- --emit=metadata",
        ] {
            assert_eq!(
                infer_command_expense(command),
                CommandExpense::Heavy,
                "{command}"
            );
        }
        for command in ["cargo fmt --check", "cargo metadata", "git status", "echo cargo test"] {
            assert_eq!(
                infer_command_expense(command),
                CommandExpense::Normal,
                "{command}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cross_task_heavy_admission_never_exceeds_limit() {
        let temp = tempfile::tempdir().expect("tempdir");
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();

        for _ in 0..12 {
            let root = temp.path().to_path_buf();
            let active = Arc::clone(&active);
            let peak = Arc::clone(&peak);
            tasks.push(tokio::spawn(async move {
                let _permit = acquire_heavy_command_permit_at(&root, 2, None)
                    .await
                    .expect("permit");
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(current, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(30)).await;
                active.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for task in tasks {
            task.await.expect("admission task");
        }

        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn queued_admission_observes_cancellation() {
        let temp = tempfile::tempdir().expect("tempdir");
        let _held = acquire_heavy_command_permit_at(temp.path(), 1, None)
            .await
            .expect("initial permit");
        let cancel = CancellationToken::new();
        let wait_cancel = cancel.clone();
        let root = temp.path().to_path_buf();
        let waiter = tokio::spawn(async move {
            acquire_heavy_command_permit_at(&root, 1, Some(&wait_cancel)).await
        });

        tokio::time::sleep(Duration::from_millis(75)).await;
        cancel.cancel();
        let error = tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("bounded cancellation")
            .expect("waiter task")
            .expect_err("queued command must cancel");
        assert!(error.to_string().contains("canceled while queued"));
    }
}
