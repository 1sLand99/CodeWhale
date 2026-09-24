//! Host-owned diagnostics projections for FEAT-029.
//!
//! Only this adapter reads App, pricing, request inspection and prepared-tool
//! evidence. No completed command message or rendered report crosses the facet.
//! The cache observation and final store are separate synchronous operations;
//! in particular observation never mutates the remembered inspection.

use std::collections::BTreeMap;
use std::time::Instant;

use codewhale_command_contract::facets::*;
use codewhale_models::{MessageRequest, SystemPrompt, Usage};

use super::SharedCommandHost;
use crate::client::{
    CacheWarmupKey, PromptInspection, PromptLayerStability, inspect_prompt_for_request,
};
use crate::config::provider_has_balance_api;
use crate::pricing::{CostCurrency, token_usage_for_pricing};
use crate::tui::app::App;

pub(super) struct DebugDiagnosticsAdapter<'a> {
    pub(super) host: SharedCommandHost<'a>,
}

impl CommandDebugDiagnosticsContext for DebugDiagnosticsAdapter<'_> {
    fn balance_projection(&self) -> DebugBalanceProjection {
        let app = self.host.app.borrow();
        DebugBalanceProjection {
            provider_display_name: app.api_provider.display_name().to_string(),
            supports_balance_api: provider_has_balance_api(app.api_provider),
        }
    }

    fn system_projection(&self) -> DebugSystemProjection {
        let app = self.host.app.borrow();
        let prompt = match app.system_prompt.as_ref() {
            None => DebugSystemPrompt::None,
            Some(SystemPrompt::Text(text)) => DebugSystemPrompt::Text(text.clone()),
            Some(SystemPrompt::Blocks(blocks)) => {
                DebugSystemPrompt::Blocks(blocks.iter().map(|block| block.text.clone()).collect())
            }
        };
        DebugSystemProjection {
            mode_label: app.mode.label().to_string(),
            prompt,
        }
    }

    fn token_projection(&self) -> DebugTokenProjection {
        let app = self.host.app.borrow();
        let window = crate::route_budget::route_context_window_tokens(
            app.api_provider,
            app.effective_model_for_budget(),
            app.active_route_limits,
        );
        let estimated = crate::compaction::estimate_input_tokens_conservative(
            &app.api_messages,
            app.system_prompt.as_ref(),
        );
        DebugTokenProjection {
            active_context_used: estimated.min(window as usize),
            context_window: window,
            last_input: app.session.last_prompt_tokens,
            last_output: app.session.last_completion_tokens,
            cache_hit: app.session.last_prompt_cache_hit_tokens,
            cache_miss: app.session.last_prompt_cache_miss_tokens,
            total_tokens: u64::from(app.session.displayed_total_tokens()),
            cache_write_tokens: u64::from(app.session.displayed_total_cache_write_tokens()),
            api_message_count: app.api_messages.len(),
            chat_message_count: app.history.len(),
            model: app.model.clone(),
            cost: cost_projection(&app),
        }
    }

    fn cost_projection(&self) -> DebugCostProjection {
        cost_projection(&self.host.app.borrow())
    }

    fn cache_telemetry(&self) -> DebugCacheTelemetry {
        let app = self.host.app.borrow();
        let currency = app.cost_display_currency(app.cost_currency);
        let now = Instant::now();
        let history = app
            .session
            .turn_cache_history
            .iter()
            .map(|rec| {
                let classes = token_usage_for_pricing(&Usage {
                    input_tokens: rec.input_tokens,
                    output_tokens: rec.output_tokens,
                    prompt_cache_hit_tokens: rec.cache_hit_tokens,
                    prompt_cache_miss_tokens: rec.cache_miss_tokens,
                    prompt_cache_write_tokens: rec.cache_write_tokens,
                    reasoning_tokens: rec.reasoning_tokens,
                    reasoning_replay_tokens: rec.reasoning_replay_tokens,
                    server_tool_use: None,
                });
                DebugCacheTurn {
                    provider: rec.provider.map(|provider| provider.as_str().to_string()),
                    provider_identity: rec.provider_identity.clone(),
                    model: rec.model.clone(),
                    auto_model: rec.auto_model,
                    input_tokens: rec.input_tokens,
                    output_tokens: rec.output_tokens,
                    cache_hit_tokens: rec.cache_hit_tokens,
                    cache_miss_tokens: rec.cache_miss_tokens,
                    cache_write_tokens: rec.cache_write_tokens,
                    reasoning_tokens: rec.reasoning_tokens,
                    reasoning_replay_tokens: rec.reasoning_replay_tokens,
                    priced_amount: rec.cost_audit.as_ref().and_then(|audit| {
                        audit
                            .is_priced_in(currency)
                            .then(|| audit.estimate.map(|value| value.amount(currency)))
                            .flatten()
                    }),
                    unpriced_reason_key: rec.cost_audit.as_ref().and_then(|audit| {
                        audit
                            .unpriced_reason
                            .map(|reason| reason.label().to_string())
                    }),
                    unpriced_classes: rec.cost_audit.as_ref().map_or_else(Vec::new, |audit| {
                        audit
                            .unpriced_classes
                            .iter()
                            .map(|class| class.label().to_string())
                            .collect()
                    }),
                    priced_cache_read: classes.cache_read,
                    priced_cache_miss: classes.input,
                    priced_cache_write: classes.cache_write,
                    age_seconds: now.saturating_duration_since(rec.recorded_at).as_secs(),
                }
            })
            .collect();
        DebugCacheTelemetry {
            model: app.model.clone(),
            history,
            history_capacity: App::TURN_CACHE_HISTORY_CAP,
            prefix_stability_pct: app.prefix_stability_pct,
            prefix_checks_total: app.prefix_checks_total,
            prefix_change_count: app.prefix_change_count,
            prefix_drift_count: app.prefix_drift_count,
            prefix_context_updates: app.prefix_context_updates,
            prefix_pin_reason: app.prefix_pin_reason.clone(),
            prefix_last_miss_reason: app.prefix_last_miss_reason.clone(),
            last_prefix_change_desc: app.last_prefix_change_desc.clone(),
            last_pinned_prefix_hash: app.last_pinned_prefix_hash.clone(),
            api_message_count: app.api_messages.len(),
            non_system_message_count: app
                .api_messages
                .iter()
                .filter(|m| m.role != "system")
                .count(),
        }
    }

    fn context_source_map(&self) -> DebugPromptSourceMap {
        source_map(crate::context_report::build_context_report(
            &self.host.app.borrow(),
        ))
    }

    fn prompt_context(&self) -> DebugPromptContext {
        prompt_context(crate::context_report::build_prompt_context(
            &self.host.app.borrow(),
        ))
    }

    fn tool_snapshot(&self) -> Option<DebugToolSnapshot> {
        self.host
            .app
            .borrow()
            .session
            .last_tool_request_snapshot
            .as_ref()
            .map(tool_snapshot)
    }

    fn inspect_cache(
        &self,
    ) -> Result<DebugCacheInspectionObservation, DebugCacheInspectionUnavailable> {
        let app = self.host.app.borrow();
        let (inspection, key) = observe_cache_for_app(&app)?;
        let current_warmup_hash_short = key.hash_short();
        let last_warmup_hash_short = app
            .session
            .last_warmup_key
            .as_ref()
            .map(CacheWarmupKey::hash_short);
        Ok(DebugCacheInspectionObservation {
            current: prompt_inspection(inspection),
            previous: app
                .session
                .last_cache_inspection
                .clone()
                .map(prompt_inspection),
            current_warmup_key: warmup_key(key),
            last_warmup_key: app.session.last_warmup_key.clone().map(warmup_key),
            current_warmup_hash_short,
            last_warmup_hash_short,
        })
    }

    fn remember_cache_inspection(&mut self, inspection: DebugPromptInspection) {
        self.host.app.borrow_mut().session.last_cache_inspection =
            Some(host_inspection(inspection));
    }
}

/// Resolve the route and build the inspection exactly once using the existing
/// client authority. Both the original handler (until Phase 4 adoption) and
/// the diagnostics facet use this host-owned builder. This function is read-only;
/// the caller explicitly stores the inspected snapshot only after rendering.
pub(crate) fn observe_cache_for_app(
    app: &App,
) -> Result<(PromptInspection, CacheWarmupKey), DebugCacheInspectionUnavailable> {
    let target = app
        .cache_replay_target()
        .ok_or(DebugCacheInspectionUnavailable::NoConcreteRoute)?;
    let replay_base_url = target
        .base_url
        .as_deref()
        .ok_or(DebugCacheInspectionUnavailable::MissingCapturedEndpoint)?;
    let request = MessageRequest {
        model: target.model.clone(),
        messages: app.api_messages.as_ref().clone(),
        max_tokens: 0,
        system: app.system_prompt.clone(),
        tools: app.session.last_tool_catalog.clone(),
        tool_choice: None,
        metadata: None,
        thinking: None,
        reasoning_effort: app
            .reasoning_effort_api_value_for_replay(target.provider, replay_base_url, &target.model)
            .map(str::to_string),
        stream: Some(true),
        temperature: None,
        top_p: None,
    };
    let inspection = inspect_prompt_for_request(&request);
    let key = CacheWarmupKey::from_inspection(
        &target.provider_identity,
        &target.model,
        replay_base_url,
        &inspection,
    );
    Ok((inspection, key))
}

/// Authoritative host cost components shared with the original `/cost`
/// command while its presentation moves to portable code in Phase 4.
/// The #244 high-water floor is not a separate billable amount.
pub(crate) struct CostComponents {
    pub(crate) parent_turns: f64,
    pub(crate) subagents: f64,
    pub(crate) display_floor: f64,
}

impl CostComponents {
    pub(crate) fn compute(app: &App) -> Self {
        fn sanitize(amount: f64) -> f64 {
            if amount.is_finite() && amount >= 0.0 {
                amount
            } else {
                0.0
            }
        }
        let currency = app.cost_display_currency(app.cost_currency);
        let parent_turns = sanitize(app.session_cost_for_currency(currency));
        let subagents = sanitize(app.subagent_cost_for_currency(currency));
        let sum = parent_turns + subagents;
        let current = if sum.is_finite() { sum } else { f64::MAX };
        let total = app.displayed_session_cost_for_currency(app.cost_currency);
        Self {
            parent_turns,
            subagents,
            display_floor: (total - current).max(0.0),
        }
    }

    #[cfg(test)]
    pub(crate) fn sum(&self) -> f64 {
        self.parent_turns + self.subagents + self.display_floor
    }
}

fn cost_projection(app: &App) -> DebugCostProjection {
    let currency = app.cost_display_currency(app.cost_currency);
    let total = app.displayed_session_cost_for_currency(app.cost_currency);
    let components = CostComponents::compute(app);
    let (priced_turns, unpriced_turns, reasons) = match currency {
        CostCurrency::Usd => (
            app.session.cost_priced_turns,
            app.session.cost_unpriced_turns,
            &app.session.cost_unpriced_reasons,
        ),
        CostCurrency::Cny => (
            app.session.cost_cny_priced_turns,
            app.session.cost_cny_unpriced_turns,
            &app.session.cost_cny_unpriced_reasons,
        ),
    };
    let mut by_route = BTreeMap::<String, f64>::new();
    let mut itemized_turns = 0u32;
    for rec in &app.session.turn_cache_history {
        let Some(audit) = rec.cost_audit.as_ref() else {
            continue;
        };
        if !audit.is_priced_in(currency) {
            continue;
        }
        let Some(estimate) = audit.estimate else {
            continue;
        };
        let provider = rec.provider_identity.clone().unwrap_or_else(|| {
            rec.provider.map_or_else(
                || "unknown-provider".to_string(),
                |provider| provider.as_str().to_string(),
            )
        });
        let route = format!(
            "{provider}/{}",
            rec.model.as_deref().unwrap_or("unknown-model")
        );
        *by_route.entry(route).or_default() += estimate.amount(currency);
        itemized_turns = itemized_turns.saturating_add(1);
    }
    DebugCostProjection {
        currency: super::to_command_currency(currency),
        total,
        parent_turns: components.parent_turns,
        subagents: components.subagents,
        display_floor: components.display_floor,
        priced_turns,
        unpriced_turns,
        legacy_coverage_unknown: app.session.cost_coverage_unknown_legacy,
        user_declared_estimates: app
            .session
            .cost_pricing_provenances
            .contains("user_override"),
        itemized_turns,
        route_amounts: by_route
            .into_iter()
            .map(|(route, amount)| DebugRouteCost { route, amount })
            .collect(),
        turn_history_capacity: App::TURN_CACHE_HISTORY_CAP,
        unpriced_reason_labels: reasons.iter().cloned().collect(),
        unpriced_classes: app.session.cost_unpriced_classes.iter().cloned().collect(),
        pricing_provenances: app
            .session
            .cost_pricing_provenances
            .iter()
            .cloned()
            .collect(),
        live_pricing_defects: app
            .session
            .cost_live_pricing_defects
            .iter()
            .cloned()
            .collect(),
        unusable_pricing_defects: app
            .session
            .cost_live_pricing_unusable_defects
            .iter()
            .cloned()
            .collect(),
        route_receipts: app.session.cost_route_receipts.iter().cloned().collect(),
    }
}

pub(crate) fn prompt_context(context: crate::context_report::PromptContext) -> DebugPromptContext {
    DebugPromptContext {
        schema_version: context.schema_version,
        provider: context.provider,
        model: context.model,
        system_prompt_state: context.system_prompt_state.to_string(),
        tool_catalog_state: context.tool_catalog_state.to_string(),
        sections: context
            .sections
            .into_iter()
            .map(|section| DebugPromptContextSection {
                index: section.index,
                block_type: section.block_type,
                cache_control: section.cache_control.map(|control| DebugCacheControl {
                    cache_type: control.cache_type,
                }),
                estimated_tokens: section.estimated_tokens,
                text: section.text,
            })
            .collect(),
        tools: context
            .tools
            .into_iter()
            .map(|tool| DebugPromptTool {
                tool_type: tool.tool_type,
                name: tool.name,
                description: tool.description,
                input_schema: tool.input_schema,
                allowed_callers: tool.allowed_callers,
                defer_loading: tool.defer_loading,
                input_examples: tool.input_examples,
                strict: tool.strict,
                cache_control: tool.cache_control.map(|control| DebugCacheControl {
                    cache_type: control.cache_type,
                }),
            })
            .collect(),
        source_map: source_map(context.source_map),
    }
}

pub(crate) fn source_map(report: crate::context_report::PromptSourceMap) -> DebugPromptSourceMap {
    use crate::context_budget::PressureLevel;
    use crate::context_report::{ActivationReason as A, CountingConfidence as C, SourceKind as S};
    use crate::route_runtime::ContextWindowSource;
    let pressure_label = report
        .budget_used_percent
        .map(|percent| PressureLevel::from_usage_percent(percent).label())
        .unwrap_or("unknown")
        .to_string();
    let source_label = report
        .context_window_source
        .as_deref()
        .unwrap_or_else(|| ContextWindowSource::Fallback.label());
    let context_window_verified =
        ContextWindowSource::from_label(source_label).is_some_and(ContextWindowSource::is_verified);
    let kind = |kind| match kind {
        S::Constitution => DebugSourceKind::Constitution,
        S::UserConstitution => DebugSourceKind::UserConstitution,
        S::RepoConstitution => DebugSourceKind::RepoConstitution,
        S::ProjectContext => DebugSourceKind::ProjectContext,
        S::ProjectContextWarning => DebugSourceKind::ProjectContextWarning,
        S::ProjectContextPack => DebugSourceKind::ProjectContextPack,
        S::SkillsBlock => DebugSourceKind::SkillsBlock,
        S::ContextManagement => DebugSourceKind::ContextManagement,
        S::CompactionRelayTemplate => DebugSourceKind::CompactionRelayTemplate,
        S::RuntimePolicy => DebugSourceKind::RuntimePolicy,
        S::AuthorityRecap => DebugSourceKind::AuthorityRecap,
        S::EnvironmentBlock => DebugSourceKind::EnvironmentBlock,
        S::UserMemory => DebugSourceKind::UserMemory,
        S::SessionGoal => DebugSourceKind::SessionGoal,
        S::HandoffRelay => DebugSourceKind::HandoffRelay,
        S::ToolSchemas => DebugSourceKind::ToolSchemas,
        S::UserRequest => DebugSourceKind::UserRequest,
        S::ConversationHistory => DebugSourceKind::ConversationHistory,
        S::ToolResult => DebugSourceKind::ToolResult,
        S::ModelProviderFact => DebugSourceKind::ModelProviderFact,
    };
    DebugPromptSourceMap {
        entries: report
            .entries
            .into_iter()
            .map(|entry| DebugSourceEntry {
                source_kind: kind(entry.source_kind),
                label: entry.label,
                source_path: entry.source_path,
                activation_reason: match entry.activation_reason {
                    A::AlwaysOn => DebugActivationReason::AlwaysOn,
                    A::FilePresent => DebugActivationReason::FilePresent,
                    A::ConfigEnabled => DebugActivationReason::ConfigEnabled,
                    A::RuntimeState => DebugActivationReason::RuntimeState,
                    A::PerRequest => DebugActivationReason::PerRequest,
                    A::Omitted => DebugActivationReason::Omitted,
                },
                estimated_tokens: entry.estimated_tokens,
                counting_confidence: match entry.counting_confidence {
                    C::High => DebugCountingConfidence::High,
                    C::Approximate => DebugCountingConfidence::Approximate,
                },
                authority_tier: entry.authority_tier,
                truncation_reason: entry.truncation_reason,
            })
            .collect(),
        total_estimated_tokens: report.total_estimated_tokens,
        active_context_estimated_tokens: report.active_context_estimated_tokens,
        overflow_guard_estimated_tokens: report.overflow_guard_estimated_tokens,
        context_window_tokens: report.context_window_tokens,
        context_window_source: report.context_window_source,
        budget_used_percent: report.budget_used_percent,
        pressure_label,
        context_window_verified,
        generated_at: report.generated_at,
        note: report.note,
    }
}

fn prompt_inspection(inspection: PromptInspection) -> DebugPromptInspection {
    DebugPromptInspection {
        base_static_prefix_hash: inspection.base_static_prefix_hash,
        full_request_prefix_hash: inspection.full_request_prefix_hash,
        tool_catalog_hash: inspection.tool_catalog_hash,
        layers: inspection
            .layers
            .into_iter()
            .map(|layer| DebugPromptLayer {
                name: layer.name,
                stability: match layer.stability {
                    PromptLayerStability::Static => DebugPromptLayerStability::Static,
                    PromptLayerStability::History => DebugPromptLayerStability::History,
                    PromptLayerStability::Dynamic => DebugPromptLayerStability::Dynamic,
                },
                char_len: layer.char_len,
                byte_len: layer.byte_len,
                token_estimate: layer.token_estimate,
                sha256: layer.sha256,
                tool_result: layer.tool_result.map(|result| DebugToolResultInspection {
                    original_chars: result.original_chars,
                    sent_chars: result.sent_chars,
                    truncated: result.truncated,
                    deduplicated: result.deduplicated,
                }),
                turn_meta: layer.turn_meta.map(|meta| DebugTurnMetaInspection {
                    original_chars: meta.original_chars,
                    sent_chars: meta.sent_chars,
                    deduplicated: meta.deduplicated,
                    sha256: meta.sha256,
                }),
            })
            .collect(),
    }
}

fn host_inspection(inspection: DebugPromptInspection) -> PromptInspection {
    use crate::client::{PromptLayerInspection, ToolResultInspection, TurnMetaInspection};
    PromptInspection {
        base_static_prefix_hash: inspection.base_static_prefix_hash,
        full_request_prefix_hash: inspection.full_request_prefix_hash,
        tool_catalog_hash: inspection.tool_catalog_hash,
        layers: inspection
            .layers
            .into_iter()
            .map(|layer| PromptLayerInspection {
                name: layer.name,
                stability: match layer.stability {
                    DebugPromptLayerStability::Static => PromptLayerStability::Static,
                    DebugPromptLayerStability::History => PromptLayerStability::History,
                    DebugPromptLayerStability::Dynamic => PromptLayerStability::Dynamic,
                },
                char_len: layer.char_len,
                byte_len: layer.byte_len,
                token_estimate: layer.token_estimate,
                sha256: layer.sha256,
                tool_result: layer.tool_result.map(|result| ToolResultInspection {
                    original_chars: result.original_chars,
                    sent_chars: result.sent_chars,
                    truncated: result.truncated,
                    deduplicated: result.deduplicated,
                }),
                turn_meta: layer.turn_meta.map(|meta| TurnMetaInspection {
                    original_chars: meta.original_chars,
                    sent_chars: meta.sent_chars,
                    deduplicated: meta.deduplicated,
                    sha256: meta.sha256,
                }),
            })
            .collect(),
    }
}

fn warmup_key(key: CacheWarmupKey) -> DebugWarmupKey {
    DebugWarmupKey {
        provider: key.provider,
        model: key.model,
        base_url: key.base_url,
        static_prefix_hash: key.static_prefix_hash,
        tool_catalog_hash: key.tool_catalog_hash,
        project_pack_hash: key.project_pack_hash,
        skills_hash: key.skills_hash,
    }
}

fn bounded(value: &crate::tool_inspection::BoundedString) -> DebugBoundedString {
    DebugBoundedString {
        value: value.value.clone(),
        truncated: value.truncated,
    }
}

fn evidence<T, U>(
    value: &crate::tool_inspection::Evidence<T>,
    convert: impl Fn(&T) -> U,
) -> DebugEvidence<U> {
    match value {
        crate::tool_inspection::Evidence::Known { value } => DebugEvidence::Known {
            value: convert(value),
        },
        crate::tool_inspection::Evidence::Unknown { reason } => DebugEvidence::Unknown {
            reason: reason.clone(),
        },
    }
}

fn bounded_list(value: &crate::tool_inspection::BoundedList) -> DebugBoundedList {
    DebugBoundedList {
        count: value.count,
        rendered: value.rendered.iter().map(bounded).collect(),
        omitted: value.omitted,
    }
}

pub(crate) fn tool_snapshot(
    value: &crate::tool_inspection::ToolInspectionSnapshot,
) -> DebugToolSnapshot {
    use crate::tool_inspection::{
        ProviderAvailability as P, ToolProvenance as R, ToolVisibility as V, TurnStopReason as S,
    };
    DebugToolSnapshot {
        schema_version: value.schema_version,
        capture_source: value.capture_source.to_string(),
        delivery_status: value.delivery_status.to_string(),
        turn_id: bounded(&value.turn_id),
        step: value.step,
        terminal: value
            .terminal
            .as_ref()
            .map(|terminal| DebugTurnStopDiagnostics {
                status: terminal.status.map(|status| match status {
                    crate::core::events::TurnOutcomeStatus::Completed => {
                        DebugTurnOutcomeStatus::Completed
                    }
                    crate::core::events::TurnOutcomeStatus::Interrupted => {
                        DebugTurnOutcomeStatus::Interrupted
                    }
                    crate::core::events::TurnOutcomeStatus::Failed => {
                        DebugTurnOutcomeStatus::Failed
                    }
                }),
                reason: terminal.reason.map(|reason| match reason {
                    S::ProviderNoToolCall => DebugTurnStopReason::ProviderNoToolCall,
                    S::ProviderToolCallMissing => DebugTurnStopReason::ProviderToolCallMissing,
                    S::StepBudgetExhausted => DebugTurnStopReason::StepBudgetExhausted,
                    S::NoProgress => DebugTurnStopReason::NoProgress,
                    S::Interrupted => DebugTurnStopReason::Interrupted,
                    S::Failed => DebugTurnStopReason::Failed,
                }),
                effective_max_steps: terminal.effective_max_steps,
                step_budget_source: terminal.step_budget_source.to_string(),
                model_step_index: terminal.model_step_index,
                model_requests_started: terminal.model_requests_started,
                transparent_stream_retries: terminal.transparent_stream_retries,
                stream_resumes: terminal.stream_resumes,
                reasoning_only_reprompts: terminal.reasoning_only_reprompts,
                empty_stop_retries: terminal.empty_stop_retries,
                soft_landing_sent: terminal.soft_landing_sent,
                final_report_requested: terminal.final_report_requested,
                permission_strategy_switches: terminal.permission_strategy_switches,
                permission_denial_rounds_without_progress: terminal
                    .permission_denial_rounds_without_progress,
                last_provider_finish_reason: terminal
                    .last_provider_finish_reason
                    .as_ref()
                    .map(bounded),
                last_response_tool_calls: terminal.last_response_tool_calls,
                last_response_tool_calls_suppressed: terminal.last_response_tool_calls_suppressed,
                last_reported_input_tokens: terminal.last_reported_input_tokens,
                route_context_window_tokens: terminal.route_context_window_tokens,
                last_prepared_output_limit_tokens: terminal.last_prepared_output_limit_tokens,
                automatic_compaction_attempts: terminal.automatic_compaction_attempts,
                emergency_compaction_attempts: terminal.emergency_compaction_attempts,
            }),
        tools_field_present: value.tools_field_present,
        tool_count: value.tool_count,
        rendered_tool_count: value.rendered_tool_count,
        omitted_tool_count: value.omitted_tool_count,
        payload_json_bytes: value.payload_json_bytes,
        payload_measurement_status: value.payload_measurement_status.clone(),
        active_tool_catalog_sha256: value.active_tool_catalog_sha256.clone(),
        unavailable_for_this_request: value
            .unavailable_for_this_request
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        provider: match &value.provider {
            P::Unknown => DebugProviderAvailability::Unknown,
            P::Available { provider, model } => DebugProviderAvailability::Available {
                provider: provider.clone(),
                model: model.clone(),
            },
            P::Unavailable { reason } => DebugProviderAvailability::Unavailable {
                reason: reason.clone(),
            },
        },
        registry_facts_present: value.registry_facts_present,
        registry_tool_count: evidence(&value.registry_tool_count, |count| *count),
        registry_only_tools: evidence(&value.registry_only_tools, bounded_list),
        tools: value
            .tools
            .iter()
            .map(|tool| DebugToolProjection {
                ordinal: tool.ordinal,
                name: bounded(&tool.name),
                tool_type: evidence(&tool.tool_type, bounded),
                description: bounded(&tool.description),
                input_schema_json: bounded(&tool.input_schema_json),
                allowed_callers: evidence(&tool.allowed_callers, bounded_list),
                defer_loading: evidence(&tool.defer_loading, |flag| *flag),
                input_examples: evidence(&tool.input_examples, |count| DebugCountOnly {
                    count: count.count,
                    values: count.values.to_string(),
                }),
                strict: evidence(&tool.strict, |flag| *flag),
                cache_control_type: evidence(&tool.cache_control_type, bounded),
                provenance: evidence(&tool.provenance, |origin| match origin {
                    R::Builtin => DebugToolProvenance::Builtin,
                    R::Plugin => DebugToolProvenance::Plugin,
                    R::Mcp => DebugToolProvenance::Mcp,
                    R::Synthetic => DebugToolProvenance::Synthetic,
                    R::Unknown => DebugToolProvenance::Unknown,
                }),
                mcp_server: evidence(&tool.mcp_server, bounded),
                capabilities: evidence(&tool.capabilities, bounded_list),
                approval: evidence(&tool.approval, bounded),
                model_visible: evidence(&tool.model_visible, |flag| *flag),
                visibility: match tool.visibility {
                    V::Active => DebugToolVisibility::Active,
                    V::Deferred => DebugToolVisibility::Deferred,
                    V::InRequest => DebugToolVisibility::InRequest,
                    V::RegistryOnly => DebugToolVisibility::RegistryOnly,
                    V::Hidden => DebugToolVisibility::Hidden,
                },
            })
            .collect(),
    }
}
