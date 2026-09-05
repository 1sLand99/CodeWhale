//! Model-bound redaction opt-out (`[redaction] model_bound`).
//!
//! Codewhale masks credential-looking values in tool output before it is sent
//! to an upstream model (the "model boundary"). That masking is a security
//! backstop: a file read by a tool can contain a configured API key, a bare
//! provider token, or a credential-shaped opaque string, and the model must
//! never see those bytes.
//!
//! This module adds a deliberate, documented way to turn that masking off for
//! users who must edit files that contain real credentials. Because it lowers
//! a security boundary, it is not a plain boolean:
//!
//! * Setting `[redaction] model_bound = "disabled"` in `config.toml` only
//!   records a *request*.
//! * The request takes effect only after a restart of the interactive TUI and
//!   an explicit confirmation on the startup gate screen, which persists a
//!   receipt in `redaction-state.json` next to `config.toml`.
//! * Non-interactive entry points (`codewhale exec`, hooks, automation) never
//!   confirm anything; as long as no confirmation receipt exists they resolve
//!   to the safe default (`Enabled`), whatever the config file says.
//! * Dismissing the gate (choosing "keep masking on") leaves the config field
//!   and the receipt untouched, so the next launch asks again until the user
//!   confirms or edits the field back to `"enabled"`.
//!
//! A confirmation receipt is bound to the `config.toml` it was made against:
//! it is honored only while (a) the config still requests `"disabled"` and
//! (b) the config file has not been rewritten since the confirmation.
//! Editing the field back to `"enabled"` - or changing `config.toml` in any
//! way - and later re-requesting `"disabled"` always asks for a fresh
//! confirmation, even when no process ran in between.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

/// Name of the confirmation-receipt file, stored next to `config.toml` in the
/// Codewhale home directory.
pub const MODEL_BOUND_STATE_FILE_NAME: &str = "redaction-state.json";

/// Whether credential-shaped values are masked at the model boundary.
///
/// Parsing is deliberately forgiving on the way in — the config value is a
/// security switch and users reach for boolean spellings — so `true`/`false`,
/// `"on"`/`"off"`, and `"enabled"`/`"disabled"` (any casing) all resolve to
/// the same two states. Serialization always writes the canonical
/// `"enabled"` / `"disabled"` words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelBoundMasking {
    /// Mask credential-shaped tool output before it reaches the model (default).
    #[default]
    Enabled,
    /// Let the model see the raw bytes of tool output, credentials included.
    /// Only effective after an explicit startup confirmation (see the module
    /// docs); until then it resolves to [`ModelBoundMasking::Enabled`].
    Disabled,
}

impl ModelBoundMasking {
    pub fn is_disabled(self) -> bool {
        self == Self::Disabled
    }
}

impl<'de> serde::Deserialize<'de> for ModelBoundMasking {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Flag(bool),
            Word(String),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Flag(true) => Ok(ModelBoundMasking::Enabled),
            Raw::Flag(false) => Ok(ModelBoundMasking::Disabled),
            Raw::Word(word) => match word.to_ascii_lowercase().as_str() {
                "enabled" | "on" | "true" => Ok(ModelBoundMasking::Enabled),
                "disabled" | "off" | "false" => Ok(ModelBoundMasking::Disabled),
                other => Err(serde::de::Error::unknown_variant(
                    other,
                    &["enabled", "disabled", "on", "off", "true", "false"],
                )),
            },
        }
    }
}

/// The `[redaction]` table of `config.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RedactionToml {
    /// Model-bound masking policy: `"enabled"` (default) or `"disabled"`.
    /// Boolean spellings are also accepted: `false` / `"off"` mean the same
    /// as `"disabled"`, and `true` / `"on"` mean `"enabled"`.
    ///
    /// A `"disabled"` request is honored only after a TUI restart and a one-time
    /// confirmation on the startup gate; see the module documentation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_bound: Option<ModelBoundMasking>,
}

impl RedactionToml {
    /// The requested masking mode, defaulting to [`ModelBoundMasking::Enabled`].
    pub fn model_bound_masking(&self) -> ModelBoundMasking {
        self.model_bound.unwrap_or_default()
    }
}

/// Default location of the confirmation-receipt file:
/// `<codewhale-home>/redaction-state.json`.
pub fn default_model_bound_state_path() -> Option<PathBuf> {
    crate::codewhale_home()
        .ok()
        .map(|home| home.join(MODEL_BOUND_STATE_FILE_NAME))
}

/// Whether a one-time confirmation has already been recorded for disabling
/// model-bound masking. Absent or unreadable state reads as `false`, which is
/// the safe answer for every caller.
pub fn model_bound_disabled_confirmed() -> bool {
    default_model_bound_state_path()
        .is_some_and(|path| read_state(&path).model_bound_disabled_confirmed)
}

/// Clear any recorded confirmation. Used when the config no longer requests
/// `"disabled"`: the receipt is only meaningful while the request exists, so
/// an `"enabled"` period must force a fresh confirmation on the next
/// `"disabled"` request.
///
/// The authoritative mechanism is overwriting the receipt with
/// `confirmed = false` - the same write path that records it, so it cannot be
/// blocked by the transient file locks that plague deletion on Windows. The
/// file is then removed when possible; a leftover file whose content is
/// `false` is harmless and reads as unconfirmed everywhere.
pub fn clear_model_bound_disabled_confirmation() -> io::Result<()> {
    let Some(path) = default_model_bound_state_path() else {
        // No resolvable home means there is no receipt to clear.
        return Ok(());
    };
    // On Windows, real-time AV scanning can briefly hold an exclusive lock on
    // a file we just wrote, making the immediate overwrite/delete fail. Retry
    // with backoff; the product flow (clear happens on a later launch) never
    // needs this, but the confirm->reenable test path does it back-to-back.
    let mut last_error: Option<io::Error> = None;
    for attempt in 0..6 {
        match write_state(&path, false) {
            Ok(()) => {
                // Content is now unconfirmed, which is the contract. Removing
                // the file is best-effort cleanup only.
                let _ = fs::remove_file(&path);
                return Ok(());
            }
            Err(err) if attempt < 5 => {
                last_error = Some(err);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(err) => return Err(err),
        }
    }
    Err(last_error
        .unwrap_or_else(|| io::Error::other("failed to clear model-bound redaction confirmation")))
}

/// Persist a confirmation that the user has accepted disabling model-bound
/// masking. Returns the written path on success.
pub fn record_model_bound_disabled_confirmation() -> io::Result<PathBuf> {
    let path = default_model_bound_state_path().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Codewhale home directory not found",
        )
    })?;
    write_state(&path, true)?;
    Ok(path)
}

/// Whether the startup gate must ask before a `"disabled"` request can take
/// effect: the user asked to disable masking and no confirmation exists yet.
///
/// A stale receipt (recorded for an earlier `"disabled"` period) is swept
/// here, so re-enabling and then re-disabling always asks again.
pub fn confirmation_required(desired: ModelBoundMasking) -> bool {
    desired.is_disabled() && !confirmed_for_current_request(desired)
}

/// The masking mode that must actually be applied: a disabled request counts
/// only once it has been confirmed. Every unconfirmed or absent request, in
/// every process, resolves to [`ModelBoundMasking::Enabled`].
///
/// Also the sweep point for a stale receipt: when the desired mode is
/// `Enabled` but a confirmation file exists, that file is removed so the next
/// `Disabled` request cannot ride on an old confirmation.
pub fn effective_masking(desired: ModelBoundMasking) -> ModelBoundMasking {
    if desired.is_disabled() && confirmed_for_current_request(desired) {
        ModelBoundMasking::Disabled
    } else {
        ModelBoundMasking::Enabled
    }
}

/// Read the confirmation receipt, sweeping it when it no longer matches the
/// current config. Every decision entry point goes through here so no caller
/// can accidentally honor a receipt from a previous `"disabled"` era.
///
/// A receipt is stale when either:
/// * the desired mode is `Enabled` (the user turned masking back on), or
/// * `config.toml` was rewritten *after* the receipt was recorded. A rewrite
///   could have flipped the field to `"enabled"` and back to `"disabled"`
///   between two launches with no process running in between, so an old
///   receipt cannot prove the current `"disabled"` request was ever
///   confirmed against this config.
fn confirmed_for_current_request(desired: ModelBoundMasking) -> bool {
    let Some(receipt_path) = default_model_bound_state_path() else {
        return false;
    };
    if !read_state(&receipt_path).model_bound_disabled_confirmed {
        return false;
    }
    let config_rewritten_after_receipt = crate::default_config_path()
        .ok()
        .and_then(|config_path| {
            let config_mtime = fs::metadata(&config_path).and_then(|m| m.modified()).ok();
            let receipt_mtime = fs::metadata(&receipt_path).and_then(|m| m.modified()).ok();
            match (config_mtime, receipt_mtime) {
                (Some(config), Some(receipt)) => Some(config > receipt),
                _ => None,
            }
        })
        .unwrap_or(false);
    let stale = !desired.is_disabled() || config_rewritten_after_receipt;
    if stale {
        // Best-effort sweep; the decision below never honors the stale
        // receipt even if the sweep itself is blocked.
        let _ = clear_model_bound_disabled_confirmation();
        return false;
    }
    true
}

// === State-file plumbing (path-parameterized so tests stay hermetic) ===

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct StateFile {
    model_bound_disabled_confirmed: bool,
}

fn read_state(path: &std::path::Path) -> StateFile {
    fs::read_to_string(path)
        .ok()
        .and_then(|body| serde_json::from_str(&body).ok())
        .unwrap_or_default()
}

fn write_state(path: &std::path::Path, confirmed: bool) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(&StateFile {
        model_bound_disabled_confirmed: confirmed,
    })
    .map_err(io::Error::other)?;
    fs::write(path, body)
}

/// Restore the previous value of an environment variable on drop. Kept to a
/// single test that touches `CODEWHALE_HOME` so parallel unit tests in this
/// crate cannot fight over the ambient home.
#[cfg(test)]
struct EnvGuard(String, Option<std::ffi::OsString>);

#[cfg(test)]
impl EnvGuard {
    fn set(key: &str, value: &std::path::Path) -> Self {
        let previous = std::env::var_os(key);
        // `std::env::set_var` is unsafe on Rust 2024; the whole point of this
        // guard is test isolation, and the value is a fresh tempdir.
        unsafe { std::env::set_var(key, value) };
        Self(key.to_string(), previous)
    }
}

#[cfg(test)]
impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.1 {
            Some(value) => unsafe { std::env::set_var(&self.0, value) },
            None => unsafe { std::env::remove_var(&self.0) },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Serializes the single test that mutates the process-wide
    /// `CODEWHALE_HOME`: other tests in this crate parse or touch their own
    /// temp paths, but the env switch is process-global and parallel test
    /// threads would race each other through it.
    fn home_env_lock() -> &'static std::sync::Mutex<()> {
        static HOME_ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> =
            std::sync::OnceLock::new();
        HOME_ENV_LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    fn state_path(tmp: &Path) -> PathBuf {
        tmp.join(MODEL_BOUND_STATE_FILE_NAME)
    }

    #[test]
    fn absent_state_is_not_confirmed() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(!read_state(&state_path(tmp.path())).model_bound_disabled_confirmed);
    }

    #[test]
    fn confirmation_round_trips_through_the_state_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = state_path(tmp.path());
        write_state(&path, true).expect("write state");
        assert!(read_state(&path).model_bound_disabled_confirmed);
    }

    /// The whole disable-and-confirm lifecycle through the default-path APIs,
    /// under one explicit `CODEWHALE_HOME` so no parallel test shares it.
    #[test]
    fn default_path_lifecycle_requires_confirmation_before_disabling() {
        let _env_lock = home_env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("tempdir");
        let _guard = EnvGuard::set("CODEWHALE_HOME", tmp.path());
        assert!(!model_bound_disabled_confirmed());

        let desired = ModelBoundMasking::Disabled;
        assert!(confirmation_required(desired));
        assert_eq!(effective_masking(desired), ModelBoundMasking::Enabled);

        let written = record_model_bound_disabled_confirmation().expect("record");
        assert_eq!(written, tmp.path().join(MODEL_BOUND_STATE_FILE_NAME));
        assert!(model_bound_disabled_confirmed());
        assert!(!confirmation_required(desired));
        assert_eq!(effective_masking(desired), ModelBoundMasking::Disabled);

        // Windows real-time AV scanning can hold a short exclusive lock on a
        // file we just wrote; the confirm -> re-enable sweep below rewrites
        // that same file back-to-back, which is exactly the lock window. The
        // product flow never does this (record and sweep happen on different
        // launches), so back off briefly here to keep the test deterministic
        // on Defender-equipped machines.
        std::thread::sleep(std::time::Duration::from_millis(300));

        // An enabled request never disables, even with a receipt on disk -
        // and going back to enabled invalidates the receipt, so the next
        // disabled request must be confirmed again.
        let enabled = ModelBoundMasking::Enabled;
        assert!(!confirmation_required(enabled));
        assert_eq!(effective_masking(enabled), ModelBoundMasking::Enabled);
        clear_model_bound_disabled_confirmation()
            .expect("explicit clear must succeed after re-enabling");
        assert!(
            !model_bound_disabled_confirmed(),
            "returning to enabled must clear the confirmation receipt"
        );

        // Re-disabling after an enabled period asks again from scratch.
        assert!(confirmation_required(desired));
        assert_eq!(effective_masking(desired), ModelBoundMasking::Enabled);

        // The receipt is bound to the config it was made against: rewriting
        // config.toml after a fresh confirmation (an enabled -> disabled
        // round trip with zero processes in between) must invalidate it too.
        record_model_bound_disabled_confirmation().expect("record again");
        assert!(!confirmation_required(desired));
        // Ensure config.toml is strictly newer than the receipt before the
        // rewrite check runs.
        std::thread::sleep(std::time::Duration::from_millis(30));
        std::fs::write(
            tmp.path().join(crate::CONFIG_FILE_NAME),
            "[redaction]\nmodel_bound = \"disabled\"\n",
        )
        .expect("touch config after receipt");
        assert!(
            confirmation_required(desired),
            "a config rewritten after the receipt must force a fresh confirmation"
        );
        assert_eq!(effective_masking(desired), ModelBoundMasking::Enabled);
    }

    #[test]
    fn corrupt_state_reads_as_unconfirmed() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = state_path(tmp.path());
        std::fs::write(&path, "not json at all").expect("write corrupt state");
        assert!(!read_state(&path).model_bound_disabled_confirmed);
    }

    #[test]
    fn toml_table_parses_and_round_trips() {
        let parsed: crate::ConfigToml =
            toml::from_str("[redaction]\nmodel_bound = \"disabled\"\n").expect("parse");
        assert_eq!(
            parsed
                .redaction
                .as_ref()
                .expect("redaction table")
                .model_bound_masking(),
            ModelBoundMasking::Disabled
        );

        let absent: crate::ConfigToml = toml::from_str("").expect("parse empty");
        assert_eq!(
            absent.redaction_model_bound_masking(),
            ModelBoundMasking::Enabled
        );

        let serialized = toml::to_string(&parsed).expect("serialize");
        assert!(
            serialized.contains("model_bound = \"disabled\""),
            "{serialized}"
        );
    }

    /// The switch reads like a boolean to most people (`model_bound = false`
    /// is the natural way to ask "don't mask"). Accept boolean and on/off
    /// spellings so a plain `false` cannot hard-fail config parsing.
    #[test]
    fn boolean_and_on_off_spellings_parse_to_the_same_states() {
        for (body, expected) in [
            ("model_bound = false", ModelBoundMasking::Disabled),
            ("model_bound = true", ModelBoundMasking::Enabled),
            ("model_bound = \"false\"", ModelBoundMasking::Disabled),
            ("model_bound = \"off\"", ModelBoundMasking::Disabled),
            ("model_bound = \"OFF\"", ModelBoundMasking::Disabled),
            ("model_bound = \"on\"", ModelBoundMasking::Enabled),
            ("model_bound = \"disabled\"", ModelBoundMasking::Disabled),
            ("model_bound = \"ENABLED\"", ModelBoundMasking::Enabled),
        ] {
            let parsed: crate::ConfigToml =
                toml::from_str(&format!("[redaction]\n{body}\n")).expect("parse");
            assert_eq!(parsed.redaction_model_bound_masking(), expected, "{body}");
        }

        // Garbage stays a hard error with a useful message, not a silent default.
        let err = toml::from_str::<crate::ConfigToml>("[redaction]\nmodel_bound = \"maybe\"\n")
            .expect_err("unknown variant must fail");
        assert!(err.to_string().contains("enabled"), "{err}");
    }
}
