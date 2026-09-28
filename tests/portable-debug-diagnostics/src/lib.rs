//! Independently compile and test the *actual* migrated diagnostics leaves.
//! The only stand-ins are the result and five data-only action shapes allowed
//! by FEAT-029 D8; every executable formatter/handler is pulled by path from
//! its original TUI source file without depending on the TUI crate.

mod standins;

pub mod tui {
    pub mod app {
        pub use crate::standins::AppAction;
    }
}

pub mod commands;

#[path = "../../../crates/runtime/src/elapsed.rs"]
pub mod elapsed;

#[path = "../../../crates/tui/src/diagnostics_reports/mod.rs"]
pub mod diagnostics_reports;
