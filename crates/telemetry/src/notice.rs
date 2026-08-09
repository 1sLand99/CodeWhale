//! The first-run notice copy.
//!
//! One string, owned by the crate that owns what is collected, so the TUI and
//! the CLI cannot drift into describing two different products. Every claim
//! below is checked against [`crate::event`] by a test: if the schema grows a
//! field this text does not cover, that test fails.
//!
//! Two properties of the wording are deliberate and load-bearing:
//!
//! 1. **The default is stated plainly and the opt-out is immediate.** Enter
//!    continues with anonymous usage counting on; `n`, `no`, `off`, or
//!    `disable` turns it off durably.
//! 2. **The red lines are stated as "not collected", not as "anonymized".**
//!    Sampling and hashing are not the same promise, and a notice that implies
//!    them when neither is true is worse than no notice.

/// Headline shown above [`NOTICE_BODY`].
pub const NOTICE_HEADLINE: &str = "Anonymous usage counting";

/// The notice itself.
///
/// Wrapped at 72 columns so it renders unchanged in a modal, in a pipe, and in
/// an 80-column terminal.
pub const NOTICE_BODY: &str = "\
Codewhale sends anonymous product usage counts by default: which version
you run, OS and CPU family, session duration and outcome, and aggregate
feature and error counters.

Codewhale does not collect your conversations, code, prompts, files,
file, repo, or branch names, model content, or credentials. It does not
send a per-turn or per-tool timeline of agent activity.

You are identified only by a random ID stored on this machine. It is
deleted the moment you turn this off, and it is replaced every 90 days.

Full schema, field by field:  docs/TELEMETRY.md
Disable it any time in settings:
                              codewhale config set telemetry false
                              or CODEWHALE_TELEMETRY=0";

/// The question, with the declining answer capitalised as the default.
pub const NOTICE_PROMPT: &str = "Keep anonymous usage counting on? [Y/n]";

/// The line printed once a decision is recorded, so the user has a receipt.
#[must_use]
pub fn decision_receipt(enabled: bool) -> &'static str {
    if enabled {
        "Anonymous usage counting is on. Disable it any time with `codewhale config set telemetry false`."
    } else {
        "Anonymous usage counting is off. You will not be asked again."
    }
}

/// Whether a typed answer keeps the default enabled.
#[must_use]
pub fn answer_keeps_enabled(input: &str) -> bool {
    !matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "n" | "no" | "off" | "disable" | "disabled"
    )
}
