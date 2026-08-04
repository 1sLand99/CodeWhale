//! The first-run telemetry notice, on the interactive startup path.
//!
//! Shown once, before the terminal enters raw mode, on the same TTY the user
//! launched on. It is deliberately *not* hung off the setup wizard's deferral
//! machinery: `defer_update_checkpoint_for_app` persists a completed
//! constitution checkpoint without ever showing the user anything, and a
//! telemetry decision recorded that way would be a decision nobody made.
//!
//! Every path that does not render and answer the notice leaves
//! `telemetry_notice_decided_for` as `None`, and `None` means nothing is ever
//! collected. Silence is a supported outcome, not a degraded one:
//!
//! - `--skip-onboarding`: no notice, no decision, no emission.
//! - non-TTY (a pipe, CI, a container): no notice, no decision, no emission.
//! - answered "no": off, and not asked again until the notice content itself
//!   changes.

use std::io::{BufRead, IsTerminal, Write};

use codewhale_config::{SetupState, TELEMETRY_NOTICE_VERSION};
use codewhale_telemetry::notice;

/// Show the notice and record the answer, if and only if one is owed and this
/// process is on a terminal that can ask.
///
/// Returns `true` when a decision was recorded. Never returns an error: a
/// notice that cannot be shown is a notice that was not answered, which is the
/// off state, which is the default.
pub fn prompt_if_due(skip_onboarding: bool, config_path: Option<std::path::PathBuf>) -> bool {
    if skip_onboarding {
        return false;
    }
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
            // A config we cannot read is a config we must not write. No notice
            // means no decision, which is the off state, which is the default.
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
        floor_in_force: codewhale_config::telemetry_floor_in_force(),
    };
    if !gate.may_ask() {
        return false;
    }

    let opt_in = ask(&mut std::io::stderr(), &mut std::io::stdin().lock());

    // Enabling writes *both* halves. They are independent AND conditions at
    // emit time, so neither alone does anything, and that is what makes a
    // stale pre-existing `telemetry = true` — a key that has been settable and
    // inert for a long time — stay inert.
    //
    // Config first, decision second. Either order fails closed: a config write
    // without a decision is `ForcedOff` for want of consent, and a decision
    // without the config value is `ForcedOff` for want of the switch.
    if opt_in && let Err(error) = write_config_opt_in(config_path) {
        tracing::warn!("telemetry opt-in was not saved to config: {error}");
        let _ = writeln!(
            std::io::stderr(),
            "  Could not save that setting; telemetry stays off.\n"
        );
        return false;
    }

    state.record_telemetry_notice(TELEMETRY_NOTICE_VERSION, opt_in);
    if let Err(error) = state.save() {
        // Nothing was recorded, so the notice is still owed and will be asked
        // again. Emitting on the strength of an answer we failed to store
        // would be collection without a record of consent.
        tracing::warn!("telemetry decision was not saved: {error}");
        return false;
    }
    let _ = writeln!(std::io::stderr(), "{}\n", notice::decision_receipt(opt_in));
    opt_in
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
    /// An environment-level kill switch is in force. The operator has already
    /// answered for this machine, and a `y` here could not take effect on this
    /// run — but it would take effect on every later run that does not inherit
    /// the variable.
    floor_in_force: bool,
}

impl NoticeGate {
    fn may_ask(&self) -> bool {
        self.needs_notice && !self.persisted_off && !self.floor_in_force
    }
}

/// Set `telemetry = true` in the same config file this process was launched
/// with.
///
/// Re-reads the file rather than reusing the copy the gate was computed from:
/// this is the only write in the feature that can turn collection *on*, so it
/// re-establishes the invariant against the bytes on disk at the moment of the
/// write, not against a snapshot taken before the user was even asked.
fn write_config_opt_in(config_path: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    let mut store = codewhale_config::ConfigStore::load(config_path)?;
    let resolved = store
        .config
        .resolve_runtime_options(&codewhale_config::CliRuntimeOverrides::default());
    anyhow::ensure!(
        !resolved.telemetry_explicit_off,
        "the config file says telemetry = false; the first-run notice never reverses a persistent opt-out"
    );
    store.config.set_value("telemetry", "true")?;
    store.save()
}

/// Render the notice to `out` and read one answer from `input`.
///
/// Split out so the wording, the default, and the parsing are testable without
/// a terminal. Enter — an empty line — declines, and so does EOF.
fn ask(out: &mut impl Write, input: &mut impl BufRead) -> bool {
    let _ = writeln!(
        out,
        "\n  {}\n\n{}\n\n  [ Enable ]      [ No thanks ]\n\n  Selected: No thanks — press Enter to keep telemetry off.\n",
        notice::NOTICE_HEADLINE,
        indent(notice::NOTICE_BODY),
    );
    let _ = write!(out, "  {} ", notice::NOTICE_PROMPT);
    let _ = out.flush();

    let mut answer = String::new();
    if input.read_line(&mut answer).is_err() {
        return false;
    }
    notice::answer_is_yes(&answer)
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
    fn enter_declines() {
        // The declining option is pre-selected and Enter takes it. Enabling
        // costs a deliberate keystroke; declining costs none.
        assert!(!ask_with("\n").0);
        assert!(!ask_with("").0);
        assert!(!ask_with("  \n").0);
    }

    #[test]
    fn only_an_affirmative_answer_enables() {
        assert!(ask_with("y\n").0);
        assert!(ask_with("Y\n").0);
        assert!(ask_with("yes\n").0);
        assert!(!ask_with("n\n").0);
        assert!(!ask_with("no\n").0);
        assert!(!ask_with("sure\n").0);
        assert!(!ask_with("1\n").0);
    }

    #[test]
    fn the_notice_states_the_red_lines_and_the_way_out() {
        let (_, rendered) = ask_with("\n");
        for claim in [
            "never sends prompts",
            "Not sampled, not hashed",
            "random ID stored on this machine",
            "every 90 days",
            "docs/TELEMETRY.md",
            "codewhale config set telemetry false",
            "CODEWHALE_TELEMETRY=0",
            "press Enter to keep telemetry off",
        ] {
            assert!(rendered.contains(claim), "notice is missing: {claim}");
        }
        assert!(
            !rendered.contains("anonymized"),
            "the notice must not imply anonymization it does not perform"
        );
    }

    fn gate(needs_notice: bool, persisted_off: bool, floor_in_force: bool) -> NoticeGate {
        NoticeGate {
            needs_notice,
            persisted_off,
            floor_in_force,
        }
    }

    #[test]
    fn the_notice_is_not_put_to_someone_who_has_already_answered_it_durably() {
        // Regression: the gate consulted only the setup-state record, so on a
        // machine with `CODEWHALE_TELEMETRY=0` exported and `telemetry = false`
        // in the config file the notice rendered anyway — and `y` rewrote that
        // `false` to `true`, reversing a persistent opt-out with no warning.
        assert!(gate(true, false, false).may_ask(), "an ordinary first run");
        assert!(
            !gate(true, true, false).may_ask(),
            "a persisted `telemetry = false` is an answer; do not ask again"
        );
        assert!(
            !gate(true, false, true).may_ask(),
            "an environment kill switch is an answer for this machine"
        );
        assert!(!gate(true, true, true).may_ask());
        // And the original condition still governs: an answered notice is not
        // re-asked for any reason.
        for persisted_off in [false, true] {
            for floor_in_force in [false, true] {
                assert!(!gate(false, persisted_off, floor_in_force).may_ask());
            }
        }
    }

    #[test]
    fn opting_in_never_reverses_a_persisted_opt_out() {
        // Belt and braces behind the gate: this is the one write in the whole
        // feature that can turn collection on, so it re-establishes the
        // invariant against the bytes on disk at the moment of the write.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "telemetry = false\n").expect("seed config");

        let error = write_config_opt_in(Some(path.clone()))
            .expect_err("a persisted opt-out must not be overwritten");
        assert!(
            error.to_string().contains("never reverses"),
            "unexpected error: {error}"
        );
        let after = std::fs::read_to_string(&path).expect("read back");
        assert!(
            after.contains("telemetry = false"),
            "the file was rewritten: {after}"
        );

        // A file that has never said anything is writable, which is the
        // ordinary opt-in path.
        let fresh = dir.path().join("fresh.toml");
        std::fs::write(&fresh, "").expect("seed fresh config");
        write_config_opt_in(Some(fresh.clone())).expect("a fresh config accepts the opt-in");
        assert!(
            std::fs::read_to_string(&fresh)
                .expect("read back")
                .contains("telemetry = true")
        );
    }

    #[test]
    fn skip_onboarding_records_no_decision() {
        // Not a decision, and not a deferral that pretends to be one. The
        // constitution checkpoint records `Deferred` on this path; telemetry
        // deliberately does not mirror it.
        assert!(!prompt_if_due(true, None));
    }
}
