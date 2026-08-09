//! The first-run telemetry notice, on the interactive startup path.
//!
//! Shown once, before the terminal enters raw mode, on the same TTY the user
//! launched on. It is deliberately *not* hung off the setup wizard's deferral
//! machinery: `defer_update_checkpoint_for_app` persists a completed
//! constitution checkpoint without ever showing the user anything, and a
//! telemetry decision recorded that way would be a decision nobody made.
//!
//! Anonymous usage counting is default-on. This notice explains that default
//! before the terminal enters raw mode and provides an immediate durable
//! opt-out:
//!
//! - `--skip-onboarding` does not suppress this privacy disclosure on a TTY.
//! - non-TTY surfaces use the documented default and the same config/env kill
//!   switches without attempting an interactive prompt.
//! - answered "no": off, and not asked again.

use std::io::{BufRead, IsTerminal, Write};

use codewhale_config::{SetupState, TELEMETRY_NOTICE_VERSION};
use codewhale_telemetry::notice;

/// Show the notice and record the answer, if and only if one is owed and this
/// process is on a terminal that can ask.
///
/// Returns whether the disclosed default remains enabled. Never returns an
/// error; all persistence failures are reported and leave the durable config
/// opt-out authoritative when it was successfully written.
pub fn prompt_if_due(_skip_onboarding: bool, config_path: Option<std::path::PathBuf>) -> bool {
    if !(std::io::stdin().is_terminal() && std::io::stderr().is_terminal()) {
        return false;
    }
    // Read what the config file and the environment already say *before*
    // asking. The notice used to consult neither, so it ran on a machine whose
    // operator had declared `CODEWHALE_TELEMETRY=0` and whose config file
    // already said `telemetry = false`, and a `y` rewrote that `false` to
    // `true`.
    let store = match codewhale_config::ConfigStore::load(config_path.clone()) {
        Ok(store) => store,
        Err(error) => {
            // A config we cannot read is a config we must not write.
            tracing::debug!("telemetry notice skipped; config unreadable: {error}");
            return false;
        }
    };
    let resolved = store
        .config
        .resolve_runtime_options(&codewhale_config::CliRuntimeOverrides::default());

    let mut state = match SetupState::load() {
        Ok(Some(state)) => state,
        // A missing record is a first run, which is exactly when the notice is
        // owed. An *unreadable* record is not: overwriting it would be the one
        // failure mode that costs a user their constitution checkpoint.
        Ok(None) => SetupState::default(),
        Err(error) => {
            tracing::debug!("telemetry notice skipped; setup state unreadable: {error}");
            return false;
        }
    };
    let gate = NoticeGate {
        needs_notice: state.needs_telemetry_notice(TELEMETRY_NOTICE_VERSION),
        persisted_off: resolved.telemetry_explicit_off,
        recorded_opt_out: state.telemetry_opted_out(),
        floor_in_force: codewhale_config::telemetry_floor_in_force(),
    };
    if !gate.may_ask() {
        return false;
    }

    let enabled = ask(&mut std::io::stderr(), &mut std::io::stdin().lock());

    // Continuing with the disclosed default does not rewrite config. An
    // opt-out does: the same `telemetry = false` register applies to every
    // surface and every future run.
    if !enabled && let Err(error) = write_config_opt_out(config_path) {
        tracing::warn!("telemetry opt-out was not saved to config: {error}");
        let _ = writeln!(
            std::io::stderr(),
            "  Could not save the config setting; the setup-state opt-out will still be used.\n"
        );
    }

    state.record_telemetry_notice(TELEMETRY_NOTICE_VERSION, enabled);
    if let Err(error) = state.save() {
        // Nothing was recorded, so the notice remains owed and will be shown
        // again. A successfully persisted config opt-out is still decisive.
        tracing::warn!("telemetry decision was not saved: {error}");
    }
    let _ = writeln!(std::io::stderr(), "{}\n", notice::decision_receipt(enabled));
    enabled
}

/// Everything that decides whether the question may be *put*, as opposed to how
/// it is answered.
///
/// Being asked is not collection, but it is not free either: the answer is
/// written to two durable registers, one of which may already hold the
/// opposite. A question whose "yes" would reverse a decision somebody already
/// made, or would be overridden by this environment anyway, is a question with
/// no honest answer — so it is not asked.
struct NoticeGate {
    /// No decision recorded for the current notice version.
    needs_notice: bool,
    /// `telemetry = false` is in the config file. This is the persistent
    /// opt-out the notice itself advertises; asking again and writing `true`
    /// over it is exactly the reversal the notice promises not to perform.
    persisted_off: bool,
    /// A previous notice recorded a decline. The migration from opt-in to
    /// opt-out must preserve that decision across notice-version bumps.
    recorded_opt_out: bool,
    /// An environment-level kill switch is in force. The operator has already
    /// answered for this machine, and a `y` here could not take effect on this
    /// run — but it would take effect on every later run that does not inherit
    /// the variable.
    floor_in_force: bool,
}

impl NoticeGate {
    fn may_ask(&self) -> bool {
        self.needs_notice && !self.persisted_off && !self.recorded_opt_out && !self.floor_in_force
    }
}

/// Persist the immediate opt-out in the same config this process loaded.
fn write_config_opt_out(config_path: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    let mut store = codewhale_config::ConfigStore::load(config_path)?;
    store.config.set_value("telemetry", "false")?;
    store.save()
}

/// Render the notice to `out` and read one answer from `input`.
///
/// Split out so the wording, the default, and the parsing are testable without
/// a terminal. Enter keeps the disclosed default; explicit negative answers
/// disable it.
fn ask(out: &mut impl Write, input: &mut impl BufRead) -> bool {
    let _ = writeln!(
        out,
        "\n  {}\n\n{}\n\n  [ Keep on ]      [ Disable ]\n\n  Selected: Keep on — press Enter to continue.\n",
        notice::NOTICE_HEADLINE,
        indent(notice::NOTICE_BODY),
    );
    let _ = write!(out, "  {} ", notice::NOTICE_PROMPT);
    let _ = out.flush();

    let mut answer = String::new();
    if input.read_line(&mut answer).is_err() {
        return true;
    }
    notice::answer_keeps_enabled(&answer)
}

fn indent(body: &str) -> String {
    body.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("  {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask_with(answer: &str) -> (bool, String) {
        let mut out: Vec<u8> = Vec::new();
        let mut input = answer.as_bytes();
        let decision = ask(&mut out, &mut input);
        (decision, String::from_utf8(out).expect("utf8"))
    }

    #[test]
    fn enter_keeps_the_disclosed_default_enabled() {
        assert!(ask_with("\n").0);
        assert!(ask_with("").0);
        assert!(ask_with("  \n").0);
    }

    #[test]
    fn explicit_negative_answers_disable() {
        assert!(ask_with("y\n").0);
        assert!(ask_with("Y\n").0);
        assert!(ask_with("yes\n").0);
        assert!(!ask_with("n\n").0);
        assert!(!ask_with("no\n").0);
        assert!(!ask_with("off\n").0);
        assert!(!ask_with("disable\n").0);
        assert!(ask_with("sure\n").0);
    }

    #[test]
    fn the_notice_states_the_red_lines_and_the_way_out() {
        let (_, rendered) = ask_with("\n");
        for claim in [
            "does not collect your conversations, code, prompts, files",
            "per-turn or per-tool timeline",
            "random ID stored on this machine",
            "every 90 days",
            "docs/TELEMETRY.md",
            "codewhale config set telemetry false",
            "CODEWHALE_TELEMETRY=0",
            "press Enter to continue",
        ] {
            assert!(rendered.contains(claim), "notice is missing: {claim}");
        }
        assert!(
            !rendered.contains("anonymized"),
            "the notice must not imply anonymization it does not perform"
        );
    }

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

    #[test]
    fn the_notice_is_not_put_to_someone_who_has_already_answered_it_durably() {
        // Regression: the gate consulted only the setup-state record, so on a
        // machine with `CODEWHALE_TELEMETRY=0` exported and `telemetry = false`
        // in the config file the notice rendered anyway — and `y` rewrote that
        // `false` to `true`, reversing a persistent opt-out with no warning.
        assert!(
            gate(true, false, false, false).may_ask(),
            "an ordinary first run"
        );
        assert!(
            !gate(true, true, false, false).may_ask(),
            "a persisted `telemetry = false` is an answer; do not ask again"
        );
        assert!(
            !gate(true, false, true, false).may_ask(),
            "a historical recorded decline is still an opt-out"
        );
        assert!(
            !gate(true, false, false, true).may_ask(),
            "an environment kill switch is an answer for this machine"
        );
        assert!(!gate(true, true, true, true).may_ask());
        // And the original condition still governs: an answered notice is not
        // re-asked for any reason.
        for persisted_off in [false, true] {
            for floor_in_force in [false, true] {
                assert!(!gate(false, persisted_off, false, floor_in_force).may_ask());
            }
        }
    }

    #[test]
    fn opting_out_writes_the_durable_config_floor() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "").expect("seed config");
        write_config_opt_out(Some(path.clone())).expect("save opt-out");
        assert!(
            std::fs::read_to_string(&path)
                .expect("read back")
                .contains("telemetry = false")
        );
    }

    #[test]
    fn a_non_tty_test_surface_cannot_record_a_notice_decision() {
        assert!(!prompt_if_due(true, None));
    }
}
