//! Eligibility and persistence for the interactive telemetry disclosure.
//!
//! Rendering belongs to the native TUI in [`crate::tui::telemetry_notice`].
//! This module owns only the privacy-sensitive state transitions: deciding
//! whether a disclosure is owed, recording the user's choice, and preserving
//! an in-memory decision even when persistence fails.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use codewhale_config::{SetupState, TELEMETRY_NOTICE_VERSION};
use codewhale_telemetry::SessionSource;

use crate::localization::MessageId;

/// Everything the native notice needs to commit the choice against the same
/// files and session source that were resolved before the first TUI frame.
#[derive(Debug, Clone)]
pub(crate) struct PendingTelemetryNotice {
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) setup_state_path: PathBuf,
    pub(crate) session_source: SessionSource,
}

/// Whether an interactive launch owes the native notice, may arm immediately,
/// or must stay unarmed because the durable privacy state could not be read.
#[derive(Debug)]
pub(crate) enum TelemetryNoticePlan {
    Due(PendingTelemetryNotice),
    NotDue,
    SuppressArming,
}

impl TelemetryNoticePlan {
    pub(crate) fn should_arm_before_tui(&self) -> bool {
        matches!(self, Self::NotDue)
    }

    pub(crate) fn into_pending(self) -> Option<PendingTelemetryNotice> {
        match self {
            Self::Due(pending) => Some(pending),
            Self::NotDue | Self::SuppressArming => None,
        }
    }
}

/// The choice after applying it to an in-memory setup state.
///
/// `setup_state` is deliberately returned even if both writes failed. The
/// telemetry predicate consumes this value immediately, so selecting Disable
/// can never arm the current process merely because the filesystem was
/// unwritable.
#[derive(Debug)]
pub(crate) struct AppliedTelemetryDecision {
    pub(crate) setup_state: SetupState,
    pub(crate) status_message_id: MessageId,
}

/// Return a native-notice plan when this interactive launch owes disclosure.
///
/// This is read-only. It never prints, blocks on a line read, creates telemetry
/// state, or records a fictional answer. `--skip-onboarding` intentionally has
/// no bearing on a privacy disclosure.
pub(crate) fn plan_if_due(
    config_path: Option<PathBuf>,
    session_source: SessionSource,
) -> TelemetryNoticePlan {
    if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        return TelemetryNoticePlan::NotDue;
    }

    let store = match codewhale_config::ConfigStore::load(config_path) {
        Ok(store) => store,
        Err(error) => {
            // A config we cannot read is a config we must not write.
            tracing::warn!("telemetry stays unarmed; config unreadable: {error}");
            return TelemetryNoticePlan::SuppressArming;
        }
    };
    let setup_state_path = match SetupState::path() {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!("telemetry stays unarmed; setup-state path unavailable: {error}");
            return TelemetryNoticePlan::SuppressArming;
        }
    };
    plan_for_store_and_state(store, setup_state_path, session_source)
}

fn plan_for_store_and_state(
    store: codewhale_config::ConfigStore,
    setup_state_path: PathBuf,
    session_source: SessionSource,
) -> TelemetryNoticePlan {
    let resolved = store
        .config
        .resolve_runtime_options(&codewhale_config::CliRuntimeOverrides::default());
    let state = match load_notice_state_at(&setup_state_path) {
        Ok(state) => state,
        Err(error) => {
            // Never replace a corrupt constitution/setup sidecar with a fresh
            // telemetry-only record. The next successful setup repair can
            // make this notice eligible again.
            tracing::warn!("telemetry stays unarmed; setup state unreadable: {error}");
            return TelemetryNoticePlan::SuppressArming;
        }
    };
    let gate = NoticeGate {
        needs_notice: state.needs_telemetry_notice(TELEMETRY_NOTICE_VERSION),
        persisted_off: resolved.telemetry_explicit_off,
        recorded_opt_out: state.telemetry_opted_out(),
        floor_in_force: codewhale_config::telemetry_floor_in_force(),
    };
    if gate.may_ask() {
        TelemetryNoticePlan::Due(PendingTelemetryNotice {
            config_path: Some(store.path().to_path_buf()),
            setup_state_path,
            session_source,
        })
    } else {
        TelemetryNoticePlan::NotDue
    }
}

/// Apply the native choice without ever making telemetry a launch blocker.
///
/// Disable is durable when either the root config or setup-state write lands.
/// Keep-on is durable when the notice-version record lands. When no write can
/// land, the choice still governs this process and the notice is shown again
/// next launch.
pub(crate) fn apply_decision(
    pending: &PendingTelemetryNotice,
    enabled: bool,
) -> AppliedTelemetryDecision {
    let (mut state, state_may_be_saved) = match load_notice_state_at(&pending.setup_state_path) {
        Ok(state) => (state, true),
        Err(error) => {
            tracing::warn!("telemetry decision could not reload setup state: {error}");
            (SetupState::default(), false)
        }
    };

    // The modal can remain open while another Codewhale process records an
    // opt-out. A stale Keep-on click must not overwrite that newer privacy
    // decision after we reload the shared setup state.
    if enabled && state.telemetry_opted_out() {
        return AppliedTelemetryDecision {
            setup_state: state,
            status_message_id: MessageId::TelemetryNoticeReceiptDisabled,
        };
    }

    state.record_telemetry_notice(TELEMETRY_NOTICE_VERSION, enabled);

    let config_saved = if enabled {
        false
    } else {
        match write_config_opt_out(pending.config_path.clone()) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!("telemetry opt-out was not saved to config: {error}");
                false
            }
        }
    };
    let state_saved = state_may_be_saved
        && match state.save_to(&pending.setup_state_path) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!("telemetry decision was not saved: {error}");
                false
            }
        };
    let durable = if enabled {
        state_saved
    } else {
        config_saved || state_saved
    };

    let status_message_id = if durable && enabled {
        MessageId::TelemetryNoticeReceiptEnabled
    } else if durable {
        MessageId::TelemetryNoticeReceiptDisabled
    } else if enabled {
        MessageId::TelemetryNoticeReceiptEnabledUnsaved
    } else {
        MessageId::TelemetryNoticeReceiptDisabledUnsaved
    };

    AppliedTelemetryDecision {
        setup_state: state,
        status_message_id,
    }
}

/// Load a missing sidecar as a fresh state, but distinguish it from an
/// existing unreadable/corrupt sidecar so the notice can never overwrite the
/// latter with defaults.
fn load_notice_state_at(path: &Path) -> Result<SetupState> {
    if !path
        .try_exists()
        .map_err(|error| anyhow!("could not inspect {}: {error}", path.display()))?
    {
        return Ok(SetupState::default());
    }
    SetupState::load_from(path)
        .ok_or_else(|| anyhow!("{} could not be read as setup state", path.display()))
}

/// Everything that decides whether the question may be put, as opposed to how
/// it is answered.
struct NoticeGate {
    needs_notice: bool,
    persisted_off: bool,
    recorded_opt_out: bool,
    floor_in_force: bool,
}

impl NoticeGate {
    fn may_ask(&self) -> bool {
        self.needs_notice && !self.persisted_off && !self.recorded_opt_out && !self.floor_in_force
    }
}

/// Persist the immediate opt-out in the exact config this process loaded.
fn write_config_opt_out(config_path: Option<PathBuf>) -> Result<()> {
    let mut store = codewhale_config::ConfigStore::load(config_path)?;
    store.config.set_value("telemetry", "false")?;
    store.save()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate(
        needs_notice: bool,
        persisted_off: bool,
        recorded_opt_out: bool,
        floor_in_force: bool,
    ) -> NoticeGate {
        NoticeGate {
            needs_notice,
            persisted_off,
            recorded_opt_out,
            floor_in_force,
        }
    }

    fn pending_at(config_path: PathBuf, setup_state_path: PathBuf) -> PendingTelemetryNotice {
        PendingTelemetryNotice {
            config_path: Some(config_path),
            setup_state_path,
            session_source: SessionSource::Interactive,
        }
    }

    #[test]
    fn the_notice_is_not_put_to_someone_who_already_answered_durably() {
        assert!(gate(true, false, false, false).may_ask());
        assert!(!gate(true, true, false, false).may_ask());
        assert!(!gate(true, false, true, false).may_ask());
        assert!(!gate(true, false, false, true).may_ask());
        assert!(!gate(false, false, false, false).may_ask());
    }

    #[test]
    fn opting_out_updates_both_durable_registers() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let state_path = dir.path().join("setup_state.json");
        std::fs::write(&config_path, "").expect("seed config");
        let applied = apply_decision(&pending_at(config_path.clone(), state_path.clone()), false);

        assert!(applied.setup_state.telemetry_opted_out());
        assert!(
            std::fs::read_to_string(config_path)
                .expect("read config")
                .contains("telemetry = false")
        );
        assert!(
            SetupState::load_from(&state_path)
                .expect("saved state")
                .telemetry_opted_out()
        );
        assert_eq!(
            applied.status_message_id,
            MessageId::TelemetryNoticeReceiptDisabled
        );
    }

    #[test]
    fn keeping_on_records_the_notice_without_rewriting_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let state_path = dir.path().join("setup_state.json");
        std::fs::write(&config_path, "# keep me\n").expect("seed config");
        let applied = apply_decision(&pending_at(config_path.clone(), state_path.clone()), true);

        assert_eq!(
            std::fs::read_to_string(config_path).expect("read config"),
            "# keep me\n"
        );
        assert!(
            SetupState::load_from(&state_path)
                .expect("saved state")
                .telemetry_accepted(TELEMETRY_NOTICE_VERSION)
        );
        assert_eq!(
            applied.status_message_id,
            MessageId::TelemetryNoticeReceiptEnabled
        );
    }

    #[test]
    fn a_stale_keep_choice_cannot_overwrite_a_newer_external_opt_out() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let state_path = dir.path().join("setup_state.json");
        std::fs::write(&config_path, "# keep me\n").expect("seed config");
        let pending = pending_at(config_path.clone(), state_path.clone());

        // The notice was already open when another process recorded Disable.
        let mut externally_updated = SetupState::default();
        externally_updated.record_telemetry_notice(TELEMETRY_NOTICE_VERSION, false);
        externally_updated
            .save_to(&state_path)
            .expect("save concurrent opt-out");

        let applied = apply_decision(&pending, true);

        assert!(applied.setup_state.telemetry_opted_out());
        assert_eq!(
            applied.status_message_id,
            MessageId::TelemetryNoticeReceiptDisabled
        );
        assert!(
            SetupState::load_from(&state_path)
                .expect("reloaded state")
                .telemetry_opted_out(),
            "the stale modal must preserve the newer on-disk decline"
        );
        assert_eq!(
            std::fs::read_to_string(config_path).expect("read config"),
            "# keep me\n"
        );
    }

    #[test]
    fn corrupt_setup_state_is_never_replaced_with_telemetry_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let state_path = dir.path().join("setup_state.json");
        std::fs::write(&config_path, "").expect("seed config");
        std::fs::write(&state_path, "not-json").expect("seed corrupt state");

        assert!(load_notice_state_at(&state_path).is_err());
        let store = codewhale_config::ConfigStore::load(Some(config_path)).expect("load config");
        let plan = plan_for_store_and_state(store, state_path.clone(), SessionSource::Interactive);
        assert!(matches!(&plan, TelemetryNoticePlan::SuppressArming));
        assert!(
            !plan.should_arm_before_tui(),
            "unreadable privacy state must fail closed instead of arming by default"
        );
        assert_eq!(
            std::fs::read_to_string(&state_path).expect("read corrupt state"),
            "not-json"
        );
    }

    #[test]
    fn an_unpersisted_disable_choice_still_exists_in_memory() {
        let dir = tempfile::tempdir().expect("tempdir");
        // A directory cannot be loaded/saved as a TOML config file, so this
        // deterministically exercises the no-durable-register path even when
        // tests run as a privileged user.
        let unwritable_config = dir.path().to_path_buf();
        let corrupt_state = dir.path().join("setup_state.json");
        std::fs::write(&corrupt_state, "not-json").expect("seed corrupt state");
        let applied = apply_decision(&pending_at(unwritable_config, corrupt_state.clone()), false);

        assert!(applied.setup_state.telemetry_opted_out());
        assert_eq!(
            applied.status_message_id,
            MessageId::TelemetryNoticeReceiptDisabledUnsaved
        );
        assert_eq!(
            std::fs::read_to_string(corrupt_state).expect("read corrupt state"),
            "not-json"
        );
    }

    #[test]
    fn an_unpersisted_keep_choice_is_reported_as_selected_but_unsaved() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let corrupt_state = dir.path().join("setup_state.json");
        std::fs::write(&config_path, "").expect("seed config");
        std::fs::write(&corrupt_state, "not-json").expect("seed corrupt state");

        let applied = apply_decision(&pending_at(config_path, corrupt_state.clone()), true);

        assert!(
            applied
                .setup_state
                .telemetry_accepted(TELEMETRY_NOTICE_VERSION),
            "the explicit choice still governs this process"
        );
        assert_eq!(
            applied.status_message_id,
            MessageId::TelemetryNoticeReceiptEnabledUnsaved
        );
        assert_eq!(
            std::fs::read_to_string(corrupt_state).expect("read corrupt state"),
            "not-json"
        );
    }

    #[test]
    fn a_non_tty_test_surface_cannot_schedule_the_native_notice() {
        assert!(matches!(
            plan_if_due(None, SessionSource::Interactive),
            TelemetryNoticePlan::NotDue
        ));
    }
}
