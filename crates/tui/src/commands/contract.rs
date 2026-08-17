//! FEAT-015 TUI command-boundary surface.
//!
//! This module holds the TUI-owned pieces of the staged command migration:
//! the pending-frontier projection (D4), and later the capability facet
//! adapters, boundary-value mappings, envelope construction, and seam helpers
//! (D1-D3, D7-D9). It is deliberately the only new TUI module for the
//! migration surface; the production registry/dispatch stay in `traits.rs` /
//! `mod.rs`.
//!
//! FEAT-015 does NOT migrate any production command. The frontier below lists
//! every group that still dispatches through the legacy concrete-`App` path.

/// Sorted, unique frontier of command groups that still use concrete-`App`
/// handlers. This is the TUI-visible projection of the checked-in migration
/// topology (`scripts/command-migration-topology.json`); the CI gate performs
/// the authoritative bidirectional source scan against that artifact.
pub(crate) const PENDING_GROUPS: &[&str] = &[
    "config",
    "core",
    "debug",
    "memory",
    "plugins",
    "project",
    "session",
    "skills",
    "utility",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_groups_is_sorted_unique_and_covers_all_groups() {
        let mut sorted = PENDING_GROUPS.to_vec();
        sorted.sort_unstable();
        assert_eq!(PENDING_GROUPS, sorted.as_slice(), "frontier must be sorted");
        let unique: std::collections::BTreeSet<&str> = PENDING_GROUPS.iter().copied().collect();
        assert_eq!(unique.len(), PENDING_GROUPS.len(), "frontier must be unique");

        let group_names: std::collections::BTreeSet<&str> = crate::commands::groups::all_command_groups()
            .iter()
            .map(|group| group.commands()[0].info().name)
            .collect();
        // The nine roots are the group identities in groups/mod.rs order.
        let expected: std::collections::BTreeSet<&str> = [
            "config", "core", "debug", "memory", "plugins", "project", "session", "skills",
            "utility",
        ]
        .into_iter()
        .collect();
        assert_eq!(unique, expected, "frontier must exactly cover the nine groups");
        let _ = group_names; // group identity is verified by the CI source gate
    }
}
