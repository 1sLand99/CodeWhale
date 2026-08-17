//! FEAT-015 TUI command-boundary surface.
//!
//! This module holds the TUI-owned pieces of the staged command migration:
//! the pending-frontier projection (D4), the seven capability facet adapters
//! (D1), boundary-value and localization-key mappings (D3/D8), the envelope
//! construction helper (D1), and the seam helpers (D7-D9). It is deliberately
//! the only new TUI module for the migration surface; the production
//! registry/dispatch stay in `traits.rs` / `mod.rs`.
//!
//! FEAT-015 does NOT migrate any production command. The adapters below wrap
//! App-owned state behind the FEAT-014 contract shapes so later FEATs
//! (FEAT-018+) can adopt them one group at a time. Handlers only ever see
//! `&mut dyn` facets — concrete `App` is never exposed through an envelope.
//!
//! ## Disjoint-borrow design (D1)
//!
//! `CommandContexts` holds all seven `&mut dyn` facet slots at once, so the
//! adapters must borrow *disjoint* App fields. Each adapter newtype below
//! holds references to exactly the fields its facet needs; `App::command_contexts()`
//! builds the envelope from those disjoint field reborrows. This mirrors the
//! contract crate's test pattern (small per-facet structs).
//!
//! ## Dead-code note (Phase 6 seam)
//!
//! Phase 3 ships the adapters, mappings, and envelope constructor ahead of the
//! dual-path registry/dispatch seam that consumes them (Phase 6), so in
//! production builds these items are unreferenced until then. The module-level
//! allow mirrors the FEAT-014 `#[allow(unused_imports)]` precedent and is
//! removed once the seam references them.
#![allow(dead_code)]

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use codewhale_command_contract::facets::{
    CommandCostContext, CommandModePolicyContext, CommandModelContext, CommandSessionContext,
    CommandSkillsContext, CommandSystemPromptContext, CommandWorkspaceContext,
};
use codewhale_command_contract::handler::{CommandContexts, ContextParts};
use codewhale_command_contract::types::{
    CommandApprovalMode, CommandCurrency, CommandMode, CommandProviderId, CommandReasoningEffort,
};
use codewhale_config::AppMode;
use codewhale_core::request::{Message, SystemPrompt};
use codewhale_execpolicy::ApprovalMode;

use crate::localization::MessageId;
use crate::plugins::types::PluginAuthority;
use crate::pricing::CostCurrency;
use crate::tui::app::{App, QueuedMessage, ReasoningEffort};

// ---------------------------------------------------------------------------
// Pending frontier projection (D4)
// ---------------------------------------------------------------------------

/// Sorted, unique frontier of command groups that still use concrete-`App`
/// handlers. This is the TUI-visible projection of the checked-in migration
/// topology (`scripts/command-migration-topology.json`); the CI gate performs
/// the authoritative bidirectional source scan against that artifact.
pub(crate) const PENDING_GROUPS: &[&str] = &[
    "config", "core", "debug", "memory", "plugins", "project", "session", "skills", "utility",
];

// ---------------------------------------------------------------------------
// Boundary-value mappings (D8)
// ---------------------------------------------------------------------------

/// Map the TUI operating mode onto the portable command boundary value.
pub(crate) fn to_command_mode(mode: AppMode) -> CommandMode {
    match mode {
        AppMode::Agent => CommandMode::Agent,
        AppMode::Auto => CommandMode::Auto,
        AppMode::Yolo => CommandMode::Yolo,
        AppMode::Plan => CommandMode::Plan,
        AppMode::Operate => CommandMode::Operate,
    }
}

/// Map the TUI approval posture onto the portable command boundary value.
pub(crate) fn to_command_approval(mode: ApprovalMode) -> CommandApprovalMode {
    match mode {
        ApprovalMode::Auto => CommandApprovalMode::Auto,
        ApprovalMode::Bypass => CommandApprovalMode::Bypass,
        ApprovalMode::Suggest => CommandApprovalMode::Suggest,
        ApprovalMode::Never => CommandApprovalMode::Never,
    }
}

/// Map the TUI reasoning-effort tier onto the portable command boundary value.
pub(crate) fn to_command_effort(effort: ReasoningEffort) -> CommandReasoningEffort {
    match effort {
        ReasoningEffort::Off => CommandReasoningEffort::Off,
        ReasoningEffort::Minimal => CommandReasoningEffort::Minimal,
        ReasoningEffort::Low => CommandReasoningEffort::Low,
        ReasoningEffort::Medium => CommandReasoningEffort::Medium,
        ReasoningEffort::High => CommandReasoningEffort::High,
        ReasoningEffort::XHigh => CommandReasoningEffort::XHigh,
        ReasoningEffort::Ultra => CommandReasoningEffort::Ultra,
        ReasoningEffort::Auto => CommandReasoningEffort::Auto,
        ReasoningEffort::Max => CommandReasoningEffort::Max,
    }
}

/// Map the TUI cost-display currency onto the portable command boundary value.
pub(crate) fn to_command_currency(currency: CostCurrency) -> CommandCurrency {
    match currency {
        CostCurrency::Usd => CommandCurrency::Usd,
        CostCurrency::Cny => CommandCurrency::Cny,
    }
}

/// Stable provider identity text at the command boundary.
///
/// The TUI persists either the canonical `ApiProvider::as_str()` spelling or —
/// for named custom providers — the exact configured identity text. This
/// function never leaks URLs, credentials, or filesystem paths.
pub(crate) fn to_provider_id(identity: &str) -> CommandProviderId {
    CommandProviderId(identity.to_string())
}

/// Bridge a portable metadata description key onto the TUI localization id.
///
/// The key convention (D3) is mechanical: the contract key equals the
/// snake_case of the [`MessageId`] variant name. The match table is the
/// authoritative bridge; unknown keys fail deterministically.
pub(crate) fn key_to_message_id(key: &'static str) -> Option<MessageId> {
    Some(match key {
        "cmd_advisor_description" => MessageId::CmdAdvisorDescription,
        "cmd_agent_description" => MessageId::CmdAgentDescription,
        "cmd_anchor_description" => MessageId::CmdAnchorDescription,
        "cmd_attach_description" => MessageId::CmdAttachDescription,
        "cmd_auth_description" => MessageId::CmdAuthDescription,
        "cmd_automation_description" => MessageId::CmdAutomationDescription,
        "cmd_balance_description" => MessageId::CmdBalanceDescription,
        "cmd_branch_description" => MessageId::CmdBranchDescription,
        "cmd_cache_description" => MessageId::CmdCacheDescription,
        "cmd_change_description" => MessageId::CmdChangeDescription,
        "cmd_clear_description" => MessageId::CmdClearDescription,
        "cmd_compact_description" => MessageId::CmdCompactDescription,
        "cmd_config_description" => MessageId::CmdConfigDescription,
        "cmd_constitution_description" => MessageId::CmdConstitutionDescription,
        "cmd_context_description" => MessageId::CmdContextDescription,
        "cmd_cost_description" => MessageId::CmdCostDescription,
        "cmd_diff_description" => MessageId::CmdDiffDescription,
        "cmd_edit_description" => MessageId::CmdEditDescription,
        "cmd_effort_description" => MessageId::CmdEffortDescription,
        "cmd_exit_description" => MessageId::CmdExitDescription,
        "cmd_export_description" => MessageId::CmdExportDescription,
        "cmd_feedback_description" => MessageId::CmdFeedbackDescription,
        "cmd_fleet_description" => MessageId::CmdFleetDescription,
        "cmd_fork_description" => MessageId::CmdForkDescription,
        "cmd_goal_description" => MessageId::CmdGoalDescription,
        "cmd_help_description" => MessageId::CmdHelpDescription,
        "cmd_hf_description" => MessageId::CmdHfDescription,
        "cmd_home_description" => MessageId::CmdHomeDescription,
        "cmd_hooks_description" => MessageId::CmdHooksDescription,
        "cmd_hotbar_description" => MessageId::CmdHotbarDescription,
        "cmd_init_description" => MessageId::CmdInitDescription,
        "cmd_jobs_description" => MessageId::CmdJobsDescription,
        "cmd_lane_description" => MessageId::CmdLaneDescription,
        "cmd_links_description" => MessageId::CmdLinksDescription,
        "cmd_load_description" => MessageId::CmdLoadDescription,
        "cmd_logout_description" => MessageId::CmdLogoutDescription,
        "cmd_lsp_description" => MessageId::CmdLspDescription,
        "cmd_mcp_description" => MessageId::CmdMcpDescription,
        "cmd_memory_description" => MessageId::CmdMemoryDescription,
        "cmd_mode_description" => MessageId::CmdModeDescription,
        "cmd_model_db_description" => MessageId::CmdModelDbDescription,
        "cmd_model_description" => MessageId::CmdModelDescription,
        "cmd_models_description" => MessageId::CmdModelsDescription,
        "cmd_network_description" => MessageId::CmdNetworkDescription,
        "cmd_new_description" => MessageId::CmdNewDescription,
        "cmd_note_description" => MessageId::CmdNoteDescription,
        "cmd_permissions_description" => MessageId::CmdPermissionsDescription,
        "cmd_pin_description" => MessageId::CmdPinDescription,
        "cmd_plugin_description" => MessageId::CmdPluginDescription,
        "cmd_plugin_detail_description" => MessageId::CmdPluginDetailDescription,
        "cmd_preview_request_description" => MessageId::CmdPreviewRequestDescription,
        "cmd_profile_description" => MessageId::CmdProfileDescription,
        "cmd_provider_description" => MessageId::CmdProviderDescription,
        "cmd_purge_description" => MessageId::CmdPurgeDescription,
        "cmd_queue_description" => MessageId::CmdQueueDescription,
        "cmd_relay_description" => MessageId::CmdRelayDescription,
        "cmd_remote_control_description" => MessageId::CmdRemoteControlDescription,
        "cmd_remote_env_description" => MessageId::CmdRemoteEnvDescription,
        "cmd_rename_description" => MessageId::CmdRenameDescription,
        "cmd_restore_description" => MessageId::CmdRestoreDescription,
        "cmd_resume_description" => MessageId::CmdResumeDescription,
        "cmd_retry_description" => MessageId::CmdRetryDescription,
        "cmd_review_description" => MessageId::CmdReviewDescription,
        "cmd_rlm_description" => MessageId::CmdRlmDescription,
        "cmd_save_description" => MessageId::CmdSaveDescription,
        "cmd_sessions_description" => MessageId::CmdSessionsDescription,
        "cmd_settings_description" => MessageId::CmdSettingsDescription,
        "cmd_setup_description" => MessageId::CmdSetupDescription,
        "cmd_share_description" => MessageId::CmdShareDescription,
        "cmd_sidebar_description" => MessageId::CmdSidebarDescription,
        "cmd_skill_description" => MessageId::CmdSkillDescription,
        "cmd_skills_description" => MessageId::CmdSkillsDescription,
        "cmd_stash_description" => MessageId::CmdStashDescription,
        "cmd_status_description" => MessageId::CmdStatusDescription,
        "cmd_statusline_description" => MessageId::CmdStatuslineDescription,
        "cmd_structcopy_description" => MessageId::CmdStructcopyDescription,
        "cmd_subagents_description" => MessageId::CmdSubagentsDescription,
        "cmd_system_description" => MessageId::CmdSystemDescription,
        "cmd_task_description" => MessageId::CmdTaskDescription,
        "cmd_theme_description" => MessageId::CmdThemeDescription,
        "cmd_title_description" => MessageId::CmdTitleDescription,
        "cmd_tokens_description" => MessageId::CmdTokensDescription,
        "cmd_tools_description" => MessageId::CmdToolsDescription,
        "cmd_translate_description" => MessageId::CmdTranslateDescription,
        "cmd_tree_description" => MessageId::CmdTreeDescription,
        "cmd_trust_description" => MessageId::CmdTrustDescription,
        "cmd_turn_inspect_description" => MessageId::CmdTurnInspectDescription,
        "cmd_undo_description" => MessageId::CmdUndoDescription,
        "cmd_update_description" => MessageId::CmdUpdateDescription,
        "cmd_verbose_description" => MessageId::CmdVerboseDescription,
        "cmd_voice_control_description" => MessageId::CmdVoiceControlDescription,
        "cmd_voice_description" => MessageId::CmdVoiceDescription,
        "cmd_voice_send_description" => MessageId::CmdVoiceSendDescription,
        "cmd_workflow_description" => MessageId::CmdWorkflowDescription,
        "cmd_workspace_description" => MessageId::CmdWorkspaceDescription,
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Capability facet adapters (D1)
// ---------------------------------------------------------------------------

/// Session capability adapter: session identity, message list, queue, tokens.
///
/// Holds disjoint borrows of `App.current_session_id`, `App.api_messages`,
/// `App.queued_messages`, and `App.session.total_tokens`.
pub(crate) struct SessionAdapter<'a> {
    session_id: &'a Option<String>,
    api_messages: &'a mut Vec<Message>,
    queued_messages: &'a mut VecDeque<QueuedMessage>,
    total_tokens: &'a u32,
}

impl CommandSessionContext for SessionAdapter<'_> {
    fn session_id(&self) -> Option<String> {
        self.session_id.clone()
    }
    fn api_messages(&self) -> Vec<Message> {
        self.api_messages.clone()
    }
    fn add_message(&mut self, message: Message) {
        self.api_messages.push(message);
    }
    fn queued_message_count(&self) -> usize {
        self.queued_messages.len()
    }
    fn remove_queued_message(&mut self, index: usize) -> Result<(), String> {
        if index < self.queued_messages.len() {
            self.queued_messages.remove(index);
            Ok(())
        } else {
            Err(format!("queued message index {index} out of bounds"))
        }
    }
    fn total_tokens(&self) -> u64 {
        u64::from(*self.total_tokens)
    }
}

/// Model capability adapter: current selection, effort, provider identity,
/// and fallback chain.
///
/// Holds disjoint borrows of `App.model`, `App.auto_model`,
/// `App.reasoning_effort`, `App.last_effective_provider_identity`, and
/// `App.provider_chain`.
pub(crate) struct ModelAdapter<'a> {
    model: &'a mut String,
    auto_model: &'a mut bool,
    reasoning_effort: &'a ReasoningEffort,
    provider_identity: &'a mut Option<String>,
    fallback_chain: Vec<CommandProviderId>,
}

impl CommandModelContext for ModelAdapter<'_> {
    fn current_model(&self) -> String {
        self.model.clone()
    }
    fn auto_model(&self) -> bool {
        *self.auto_model
    }
    fn set_model_selection(&mut self, model: String, provider: Option<CommandProviderId>) {
        let auto = model.trim().eq_ignore_ascii_case("auto");
        *self.model = if auto { "auto".to_string() } else { model };
        *self.auto_model = auto;
        if let Some(provider) = provider {
            let identity = provider.0;
            // Mirror App::set_provider_identity semantics: record the stable
            // identity text; never a URL/credential/path.
            *self.provider_identity = Some(identity);
        }
    }
    fn reasoning_effort(&self) -> CommandReasoningEffort {
        to_command_effort(*self.reasoning_effort)
    }
    fn provider_identity(&self) -> Option<CommandProviderId> {
        self.provider_identity.as_ref().map(|id| to_provider_id(id))
    }
    fn fallback_chain(&self) -> Vec<CommandProviderId> {
        self.fallback_chain.clone()
    }
}
/// Cost capability adapter: display currency and session/subagent cost totals.
///
/// Holds disjoint borrows of `App.cost_currency` and the four session cost
/// fields on `App.session`.
pub(crate) struct CostAdapter<'a> {
    currency: &'a CostCurrency,
    session_cost: &'a mut f64,
    session_cost_cny: &'a mut f64,
    subagent_cost: &'a mut f64,
    subagent_cost_cny: &'a mut f64,
}

impl CommandCostContext for CostAdapter<'_> {
    fn display_currency(&self) -> CommandCurrency {
        to_command_currency(*self.currency)
    }
    fn session_cost_for_currency(&self, currency: CommandCurrency) -> f64 {
        match currency {
            CommandCurrency::Usd => *self.session_cost,
            CommandCurrency::Cny => *self.session_cost_cny,
        }
    }
    fn subagent_cost_for_currency(&self, currency: CommandCurrency) -> f64 {
        match currency {
            CommandCurrency::Usd => *self.subagent_cost,
            CommandCurrency::Cny => *self.subagent_cost_cny,
        }
    }
    fn accrue_cost_estimate(&mut self, amount: f64, currency: CommandCurrency) {
        match currency {
            CommandCurrency::Usd => *self.session_cost += amount,
            CommandCurrency::Cny => *self.session_cost_cny += amount,
        }
    }
    fn record_turn_cost(&mut self, amount: f64, currency: CommandCurrency, _receipt: bool) {
        // The full audit path (priced/unpriced counters, provenance receipts)
        // stays TUI-owned in App::record_turn_cost_audit; the adapter applies
        // the same currency-field delta so a migrated handler can record a
        // simple cost without duplicating the audit authority.
        match currency {
            CommandCurrency::Usd => *self.session_cost += amount,
            CommandCurrency::Cny => *self.session_cost_cny += amount,
        }
    }
}

/// Mode/policy capability adapter: operating mode, approval posture, shell
/// access, and policy lock.
///
/// Holds disjoint borrows of `App.mode`, `App.approval_mode`,
/// `App.allow_shell`, plus a snapshot of `App::approval_policy_locked()`.
pub(crate) struct ModePolicyAdapter<'a> {
    mode: &'a mut AppMode,
    approval_mode: &'a mut ApprovalMode,
    allow_shell: &'a mut bool,
    policy_locked: bool,
}

impl CommandModePolicyContext for ModePolicyAdapter<'_> {
    fn mode(&self) -> CommandMode {
        to_command_mode(*self.mode)
    }
    fn set_mode(&mut self, mode: CommandMode) {
        *self.mode = match mode {
            CommandMode::Agent => AppMode::Agent,
            CommandMode::Auto => AppMode::Auto,
            CommandMode::Yolo => AppMode::Yolo,
            CommandMode::Plan => AppMode::Plan,
            CommandMode::Operate => AppMode::Operate,
        };
    }
    fn approval_mode(&self) -> CommandApprovalMode {
        to_command_approval(*self.approval_mode)
    }
    fn allow_shell(&self) -> bool {
        *self.allow_shell
    }
    fn set_shell_access(&mut self, allow: bool) {
        *self.allow_shell = allow;
    }
    fn policy_locked(&self) -> bool {
        self.policy_locked
    }
}

/// System-prompt capability adapter (read-only).
///
/// Holds a disjoint borrow of `App.system_prompt`.
pub(crate) struct SystemPromptAdapter<'a> {
    system_prompt: &'a Option<SystemPrompt>,
}

impl CommandSystemPromptContext for SystemPromptAdapter<'_> {
    fn system_prompt(&self) -> Option<SystemPrompt> {
        self.system_prompt.clone()
    }
}

/// Skills capability adapter: active skill identity and cache refresh.
///
/// Holds disjoint borrows of `App.active_skill`, `App.active_skill_provenance`,
/// `App.cached_skills`, `App.hotbar_actions`, plus the read-only discovery
/// inputs `App.workspace`, `App.skills_dir`, `App.skills_scan_codewhale_only`,
/// and `App.plugin_registry`.
pub(crate) struct SkillsAdapter<'a> {
    active_skill: &'a Option<String>,
    active_skill_provenance: &'a Option<PluginAuthority>,
    cached_skills: &'a mut Vec<(String, String)>,
    hotbar_actions: &'a mut crate::tui::hotbar::HotbarActionRegistry,
    workspace: &'a PathBuf,
    skills_dir: &'a PathBuf,
    skills_scan_codewhale_only: &'a bool,
    plugin_registry: &'a Arc<crate::plugins::PluginRegistry>,
}

impl CommandSkillsContext for SkillsAdapter<'_> {
    fn active_skill(&self) -> Option<String> {
        self.active_skill.clone()
    }
    fn active_skill_provenance(&self) -> Option<String> {
        self.active_skill_provenance
            .as_ref()
            .map(|authority| authority.plugin_name.clone())
    }
    fn refresh_skill_cache(&mut self) {
        crate::skills::clear_skill_discovery_cache();
        let skills = crate::skills::discover_for_workspace_and_dir_with_mode_and_plugins(
            self.workspace,
            self.skills_dir,
            crate::skills::SkillDiscoveryMode::from_codewhale_only(
                *self.skills_scan_codewhale_only,
            ),
            Some(self.plugin_registry.as_ref()),
        )
        .into_enabled()
        .list()
        .iter()
        .map(|skill| (skill.name.clone(), skill.description.clone()))
        .collect::<Vec<_>>();
        self.hotbar_actions.replace_skills(&skills);
        *self.cached_skills = skills;
    }
}

/// Workspace capability adapter: workspace path and bounded work-state
/// snapshot.
///
/// Holds a disjoint borrow of `App.workspace` plus the precomputed bounded
/// serialized work-state snapshot (captured at envelope construction via
/// `App::work_state_snapshot()` + `todo_snapshot_body`).
pub(crate) struct WorkspaceAdapter<'a> {
    workspace: &'a PathBuf,
    work_state: Result<Option<String>, String>,
}

impl CommandWorkspaceContext for WorkspaceAdapter<'_> {
    fn workspace(&self) -> PathBuf {
        self.workspace.clone()
    }
    fn work_state_snapshot(&self) -> Result<Option<String>, String> {
        self.work_state.clone()
    }
}

// ---------------------------------------------------------------------------
// Envelope construction (D1)
// ---------------------------------------------------------------------------

/// Owns the seven disjoint-borrow adapters for one dispatch, so the envelope
/// can borrow them without touching concrete `App`.
///
/// `App::command_contexts()` builds this bundle from disjoint field reborrows;
/// [`Self::contexts()`] then assembles the `CommandContexts` envelope from the
/// owned adapters. The bundle is the safe-Rust vehicle for D1's envelope
/// construction: the envelope itself cannot outlive function-local adapters,
/// so the adapters live here for the duration of one dispatch.
pub(crate) struct CommandContextBundle<'a> {
    session: SessionAdapter<'a>,
    model: ModelAdapter<'a>,
    cost: CostAdapter<'a>,
    mode_policy: ModePolicyAdapter<'a>,
    system_prompt: SystemPromptAdapter<'a>,
    skills: SkillsAdapter<'a>,
    workspace: WorkspaceAdapter<'a>,
}

impl<'a> CommandContextBundle<'a> {
    /// Build the full seven-slot envelope from the owned adapters.
    pub(crate) fn contexts(&mut self) -> CommandContexts<'_> {
        CommandContexts::empty()
            .with_session(&mut self.session)
            .with_model(&mut self.model)
            .with_cost(&mut self.cost)
            .with_mode_policy(&mut self.mode_policy)
            .with_system_prompt(&mut self.system_prompt)
            .with_skills(&mut self.skills)
            .with_workspace(&mut self.workspace)
    }

    /// Split into the consumed [`ContextParts`] for handlers needing several
    /// independent facets.
    pub(crate) fn parts(&mut self) -> ContextParts<'_> {
        self.contexts().into_parts()
    }
}

impl App {
    /// Build the full capability envelope from disjoint field reborrows.
    /// Handlers only ever see `&mut dyn` facets; concrete `App` is never
    /// exposed through an envelope.
    pub(crate) fn command_contexts(&mut self) -> CommandContextBundle<'_> {
        let work_state = self.work_state_snapshot().map(|state| {
            state.and_then(|state| crate::todo_snapshot::todo_snapshot_body(&state.todos))
        });
        let fallback_chain = self
            .fallback_chain_entries()
            .into_iter()
            .map(|(_, provider, _)| to_provider_id(provider.as_str()))
            .collect();
        let policy_locked = self.approval_policy_locked();

        CommandContextBundle {
            session: SessionAdapter {
                session_id: &self.current_session_id,
                api_messages: &mut self.api_messages,
                queued_messages: &mut self.queued_messages,
                total_tokens: &self.session.total_tokens,
            },
            model: ModelAdapter {
                model: &mut self.model,
                auto_model: &mut self.auto_model,
                reasoning_effort: &self.reasoning_effort,
                provider_identity: &mut self.last_effective_provider_identity,
                fallback_chain,
            },
            cost: CostAdapter {
                currency: &self.cost_currency,
                session_cost: &mut self.session.session_cost,
                session_cost_cny: &mut self.session.session_cost_cny,
                subagent_cost: &mut self.session.subagent_cost,
                subagent_cost_cny: &mut self.session.subagent_cost_cny,
            },
            mode_policy: ModePolicyAdapter {
                mode: &mut self.mode,
                approval_mode: &mut self.approval_mode,
                allow_shell: &mut self.allow_shell,
                policy_locked,
            },
            system_prompt: SystemPromptAdapter {
                system_prompt: &self.system_prompt,
            },
            skills: SkillsAdapter {
                active_skill: &self.active_skill,
                active_skill_provenance: &self.active_skill_provenance,
                cached_skills: &mut self.cached_skills,
                hotbar_actions: &mut self.hotbar_actions,
                workspace: &self.workspace,
                skills_dir: &self.skills_dir,
                skills_scan_codewhale_only: &self.skills_scan_codewhale_only,
                plugin_registry: &self.plugin_registry,
            },
            workspace: WorkspaceAdapter {
                workspace: &self.workspace,
                work_state,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_groups_is_sorted_unique_and_covers_all_groups() {
        let mut sorted = PENDING_GROUPS.to_vec();
        sorted.sort_unstable();
        assert_eq!(PENDING_GROUPS, sorted.as_slice(), "frontier must be sorted");
        let unique: std::collections::BTreeSet<&str> = PENDING_GROUPS.iter().copied().collect();
        assert_eq!(
            unique.len(),
            PENDING_GROUPS.len(),
            "frontier must be unique"
        );

        // The nine roots are the group identities in groups/mod.rs order.
        let expected: std::collections::BTreeSet<&str> = [
            "config", "core", "debug", "memory", "plugins", "project", "session", "skills",
            "utility",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            unique, expected,
            "frontier must exactly cover the nine groups"
        );
    }

    #[test]
    fn boundary_mappings_are_exhaustive_and_invertible() {
        for mode in [
            AppMode::Agent,
            AppMode::Auto,
            AppMode::Yolo,
            AppMode::Plan,
            AppMode::Operate,
        ] {
            let _ = to_command_mode(mode);
        }
        for approval in [
            ApprovalMode::Auto,
            ApprovalMode::Bypass,
            ApprovalMode::Suggest,
            ApprovalMode::Never,
        ] {
            let _ = to_command_approval(approval);
        }
        for effort in [
            ReasoningEffort::Off,
            ReasoningEffort::Minimal,
            ReasoningEffort::Low,
            ReasoningEffort::Medium,
            ReasoningEffort::High,
            ReasoningEffort::XHigh,
            ReasoningEffort::Ultra,
            ReasoningEffort::Auto,
            ReasoningEffort::Max,
        ] {
            let _ = to_command_effort(effort);
        }
        for currency in [CostCurrency::Usd, CostCurrency::Cny] {
            let _ = to_command_currency(currency);
        }
    }

    #[test]
    fn key_to_message_id_resolves_convention_keys_and_rejects_unknown() {
        assert_eq!(
            key_to_message_id("cmd_balance_description"),
            Some(MessageId::CmdBalanceDescription)
        );
        assert_eq!(
            key_to_message_id("cmd_voice_control_description"),
            Some(MessageId::CmdVoiceControlDescription)
        );
        assert_eq!(key_to_message_id("cmd_nonexistent_description"), None);
        assert_eq!(key_to_message_id(""), None);
    }

    #[test]
    fn cost_adapter_accrues_per_currency_and_reports_totals() {
        let mut usd = 1.0f64;
        let mut cny = 2.0f64;
        let mut sub_usd = 0.5f64;
        let mut sub_cny = 1.5f64;
        let currency = CostCurrency::Usd;
        let mut adapter = CostAdapter {
            currency: &currency,
            session_cost: &mut usd,
            session_cost_cny: &mut cny,
            subagent_cost: &mut sub_usd,
            subagent_cost_cny: &mut sub_cny,
        };
        adapter.accrue_cost_estimate(3.0, CommandCurrency::Usd);
        adapter.accrue_cost_estimate(4.0, CommandCurrency::Cny);
        assert_eq!(adapter.session_cost_for_currency(CommandCurrency::Usd), 4.0);
        assert_eq!(adapter.session_cost_for_currency(CommandCurrency::Cny), 6.0);
        assert_eq!(adapter.display_currency(), CommandCurrency::Usd);
        assert_eq!(
            adapter.subagent_cost_for_currency(CommandCurrency::Usd),
            0.5
        );
        assert_eq!(
            adapter.subagent_cost_for_currency(CommandCurrency::Cny),
            1.5
        );
    }

    #[test]
    fn session_adapter_manages_messages_queue_and_tokens() {
        let session_id = Some("s1".to_string());
        let mut api_messages = Vec::new();
        let mut queued = VecDeque::new();
        let total_tokens = 42u32;
        let mut adapter = SessionAdapter {
            session_id: &session_id,
            api_messages: &mut api_messages,
            queued_messages: &mut queued,
            total_tokens: &total_tokens,
        };
        assert_eq!(adapter.session_id().as_deref(), Some("s1"));
        adapter.add_message(Message {
            role: "user".to_string(),
            content: vec![],
        });
        assert_eq!(adapter.api_messages().len(), 1);
        adapter.queued_messages.push_back(QueuedMessage {
            display: "q".to_string(),
            skill_instruction: None,
            skill_provenance: None,
        });
        assert_eq!(adapter.queued_message_count(), 1);
        assert!(adapter.remove_queued_message(0).is_ok());
        assert_eq!(adapter.queued_message_count(), 0);
        assert!(adapter.remove_queued_message(5).is_err());
        assert_eq!(adapter.total_tokens(), 42);
        let _ = &session_id;
    }

    #[test]
    fn model_adapter_reports_and_sets_selection() {
        let mut model = "deepseek-v4".to_string();
        let mut auto_model = false;
        let effort = ReasoningEffort::High;
        let mut provider_identity = None;
        let fallback_chain = vec![to_provider_id("deepseek")];
        let mut adapter = ModelAdapter {
            model: &mut model,
            auto_model: &mut auto_model,
            reasoning_effort: &effort,
            provider_identity: &mut provider_identity,
            fallback_chain,
        };
        assert_eq!(adapter.current_model(), "deepseek-v4");
        assert!(!adapter.auto_model());
        assert_eq!(adapter.reasoning_effort(), CommandReasoningEffort::High);
        adapter.set_model_selection("auto".to_string(), Some(to_provider_id("deepseek")));
        assert!(adapter.auto_model());
        assert_eq!(adapter.current_model(), "auto");
        assert_eq!(
            adapter.provider_identity().map(|id| id.0).as_deref(),
            Some("deepseek")
        );
        assert_eq!(adapter.fallback_chain().len(), 1);
    }

    #[test]
    fn mode_policy_adapter_maps_mode_approval_shell_lock() {
        let mut mode = AppMode::Plan;
        let mut approval = ApprovalMode::Suggest;
        let mut allow_shell = false;
        let mut adapter = ModePolicyAdapter {
            mode: &mut mode,
            approval_mode: &mut approval,
            allow_shell: &mut allow_shell,
            policy_locked: true,
        };
        assert_eq!(adapter.mode(), CommandMode::Plan);
        assert_eq!(adapter.approval_mode(), CommandApprovalMode::Suggest);
        assert!(!adapter.allow_shell());
        assert!(adapter.policy_locked());
        adapter.set_mode(CommandMode::Agent);
        adapter.set_shell_access(true);
        assert_eq!(mode, AppMode::Agent);
        assert!(allow_shell);
    }

    #[test]
    fn system_prompt_adapter_returns_owned_prompt() {
        let prompt = SystemPrompt::Text("system".to_string());
        let adapter = SystemPromptAdapter {
            system_prompt: &Some(prompt),
        };
        assert!(adapter.system_prompt().is_some());
    }

    #[test]
    fn workspace_adapter_returns_path_and_snapshot() {
        let workspace = PathBuf::from("/tmp/ws");
        let adapter = WorkspaceAdapter {
            workspace: &workspace,
            work_state: Ok(None),
        };
        assert_eq!(adapter.workspace(), PathBuf::from("/tmp/ws"));
        assert_eq!(adapter.work_state_snapshot().ok().flatten(), None);
    }

    #[test]
    fn envelope_round_trips_through_selected_facets() {
        let mut app = crate::test_support::test_app_with_options(
            crate::test_support::test_tui_options(std::path::PathBuf::from(".")),
        );
        let mut bundle = app.command_contexts();
        let parts: ContextParts<'_> = bundle.parts();
        let _ = parts.workspace.expect("workspace facet present");
        let _ = parts.mode_policy.expect("mode-policy facet present");
        let _ = parts.cost.expect("cost facet present");
    }
}
