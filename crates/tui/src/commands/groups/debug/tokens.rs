//! Token/cost introspection and context commands.

use crate::compaction::estimate_input_tokens_conservative;
use crate::localization::{Locale, MessageId, tr};
use crate::models::SystemPrompt;
use crate::tui::app::{App, AppAction};

use super::CommandResult;

fn token_count(value: Option<u32>, locale: Locale) -> String {
    value.map_or_else(
        || tr(locale, MessageId::CmdTokensNotReported).to_string(),
        |tokens| tokens.to_string(),
    )
}

fn active_context_summary(app: &App, locale: Locale) -> String {
    let estimated =
        estimate_input_tokens_conservative(&app.api_messages, app.system_prompt.as_ref());
    let window = crate::route_budget::route_context_window_tokens(
        app.api_provider,
        app.effective_model_for_budget(),
        app.active_route_limits,
    );
    let used = estimated.min(window as usize);
    let percent = (used as f64 / f64::from(window) * 100.0).clamp(0.0, 100.0);
    tr(locale, MessageId::CmdTokensContextWithWindow)
        .replace("{used}", &used.to_string())
        .replace("{window}", &window.to_string())
        .replace("{percent}", &format!("{percent:.1}"))
}

fn cache_summary(app: &App, locale: Locale) -> String {
    match (
        app.session.last_prompt_cache_hit_tokens,
        app.session.last_prompt_cache_miss_tokens,
    ) {
        (Some(hit), Some(miss)) => tr(locale, MessageId::CmdTokensCacheBoth)
            .replace("{hit}", &hit.to_string())
            .replace("{miss}", &miss.to_string()),
        (Some(hit), None) => {
            tr(locale, MessageId::CmdTokensCacheHitOnly).replace("{hit}", &hit.to_string())
        }
        (None, Some(miss)) => {
            tr(locale, MessageId::CmdTokensCacheMissOnly).replace("{miss}", &miss.to_string())
        }
        (None, None) => tr(locale, MessageId::CmdTokensNotReported).to_string(),
    }
}

/// Show token usage for session
pub fn tokens(app: &mut App) -> CommandResult {
    let locale = app.ui_locale;
    let message_count = app.api_messages.len();
    let chat_count = app.history.len();

    let mut report = tr(locale, MessageId::CmdTokensReport)
        .replace("{active}", &active_context_summary(app, locale))
        .replace(
            "{input}",
            &token_count(app.session.last_prompt_tokens, locale),
        )
        .replace(
            "{output}",
            &token_count(app.session.last_completion_tokens, locale),
        )
        .replace("{cache}", &cache_summary(app, locale))
        .replace("{total}", &app.session.total_tokens.to_string())
        .replace("{cost}", &cost_report_amount(app, locale))
        .replace("{api_messages}", &message_count.to_string())
        .replace("{chat_messages}", &chat_count.to_string())
        .replace("{model}", &app.model);
    // `/tokens` quotes the same cost figure as `/cost`, so it carries the same
    // estimate disclaimer and the same coverage state. Two surfaces showing one
    // number must not disagree about how complete that number is (#4318).
    report.push_str(&cache_write_summary(app, locale));
    report.push_str(&cost_coverage_report(app, locale));
    CommandResult::message(report)
}

/// Session cache-write total, reported as its own class with a pointer to
/// `/cache` for the per-turn breakdown.
///
/// Cache-write is billed at a premium on the providers that publish one, so it
/// is neither folded into input nor hidden: `/tokens` shows the total and says
/// where the detail lives.
fn cache_write_summary(app: &App, locale: Locale) -> String {
    let write = app.session.total_cache_write_tokens;
    let mut out = String::from("\n");
    out.push_str(&tr(locale, MessageId::CmdTokensCacheWriteTotal).replace(
        "{write}",
        &if write > 0 {
            write.to_string()
        } else {
            tr(locale, MessageId::CmdTokensNotReported).to_string()
        },
    ));
    out
}

/// Show session cost breakdown.
///
/// The figure is an **estimate** computed from provider-reported usage and
/// published rates; it is never an invoice. Turns whose route produced no
/// authoritative price are missing from it entirely, so the coverage of the
/// number is reported alongside it rather than left implicit (#4318).
pub fn cost(app: &mut App) -> CommandResult {
    let locale = app.ui_locale;
    let (priced, unpriced) = cost_coverage_counts(app);
    let has_saved_legacy_subtotal = app.session.cost_coverage_unknown_legacy
        && app.displayed_session_cost_for_currency(app.cost_currency) > 0.0;
    let headline = if priced == 0 && !has_saved_legacy_subtotal {
        MessageId::CmdCostReportUnknown
    } else if app.session.cost_coverage_unknown_legacy || unpriced > 0 {
        MessageId::CmdCostReportSubtotal
    } else {
        MessageId::CmdCostReport
    };
    let mut report = tr(locale, headline).replace("{cost}", &cost_report_amount(app, locale));
    report.push_str(&cost_coverage_report(app, locale));
    CommandResult::message(report)
}

fn cost_report_amount(app: &App, locale: Locale) -> String {
    let (priced, _) = cost_coverage_counts(app);
    let total = app.displayed_session_cost_for_currency(app.cost_currency);
    if priced > 0 || (app.session.cost_coverage_unknown_legacy && total > 0.0) {
        app.format_cost_amount_precise(total)
    } else {
        tr(locale, MessageId::CmdCostUnknownValue).to_string()
    }
}

fn joined(values: &std::collections::BTreeSet<String>) -> String {
    values
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The honesty block appended to `/cost` and `/tokens`: what the estimate covers
/// and what it cannot.
///
/// Both surfaces render the same block from the same session counters, so they
/// cannot disagree about completeness (#4318).
pub(crate) fn cost_coverage_report(app: &App, locale: Locale) -> String {
    let (priced, unpriced) = cost_coverage_counts(app);
    let mut out = String::from("\n\n");
    out.push_str(&tr(locale, MessageId::CmdCostEstimateOnly));
    out.push('\n');
    if app.session.cost_coverage_unknown_legacy {
        // A restored pre-coverage session has real money and no evidence of what
        // it covers. Saying "0 of 0 priced" here would assert the total is
        // complete, so the unknown state is stated instead.
        out.push_str(&tr(locale, MessageId::CmdCostCoverageUnknownLegacy));
    } else {
        out.push_str(
            &tr(locale, MessageId::CmdCostCoverage)
                .replace("{priced}", &priced.to_string())
                .replace("{turns}", &(priced.saturating_add(unpriced)).to_string()),
        );
    }
    if unpriced > 0 {
        let reasons = match app.cost_display_currency(app.cost_currency) {
            crate::pricing::CostCurrency::Usd => &app.session.cost_unpriced_reasons,
            crate::pricing::CostCurrency::Cny => &app.session.cost_cny_unpriced_reasons,
        };
        out.push('\n');
        out.push_str(
            &tr(locale, MessageId::CmdCostUnpricedTurns)
                .replace("{unpriced}", &unpriced.to_string())
                .replace("{reasons}", &joined(reasons)),
        );
    }
    if !app.session.cost_unpriced_classes.is_empty() {
        out.push('\n');
        out.push_str(
            &tr(locale, MessageId::CmdCostUnpricedClasses)
                .replace("{classes}", &joined(&app.session.cost_unpriced_classes)),
        );
    }
    if !app.session.cost_pricing_provenances.is_empty() {
        out.push('\n');
        out.push_str(
            &tr(locale, MessageId::CmdCostPricingProvenance)
                .replace("{sources}", &joined(&app.session.cost_pricing_provenances)),
        );
    }
    if !app.session.cost_live_pricing_defects.is_empty() {
        out.push('\n');
        out.push_str(
            &tr(locale, MessageId::CmdCostLivePricingDowngraded)
                .replace("{defects}", &joined(&app.session.cost_live_pricing_defects)),
        );
    }
    if !app.session.cost_live_pricing_unusable_defects.is_empty() {
        out.push('\n');
        out.push_str(
            &tr(locale, MessageId::CmdCostLivePricingUnavailable).replace(
                "{defects}",
                &joined(&app.session.cost_live_pricing_unusable_defects),
            ),
        );
    }
    if !app.session.cost_route_receipts.is_empty() {
        out.push('\n');
        out.push_str(&tr(locale, MessageId::CmdCostRoutesHeader));
        for receipt in &app.session.cost_route_receipts {
            out.push_str("\n  ");
            out.push_str(receipt);
        }
    }
    out
}

fn cost_coverage_counts(app: &App) -> (u32, u32) {
    match app.cost_display_currency(app.cost_currency) {
        crate::pricing::CostCurrency::Usd => (
            app.session.cost_priced_turns,
            app.session.cost_unpriced_turns,
        ),
        crate::pricing::CostCurrency::Cny => (
            app.session.cost_cny_priced_turns,
            app.session.cost_cny_unpriced_turns,
        ),
    }
}

/// Show current system prompt
pub fn system_prompt(app: &mut App) -> CommandResult {
    let prompt_text = match &app.system_prompt {
        Some(SystemPrompt::Text(text)) => text.clone(),
        Some(SystemPrompt::Blocks(blocks)) => blocks
            .iter()
            .map(|b| b.text.clone())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n"),
        None => "(no system prompt)".to_string(),
    };

    // Truncate if too long
    let display = if prompt_text.len() > 500 {
        // Find a valid UTF-8 char boundary at or before byte 500
        let truncate_at = prompt_text
            .char_indices()
            .take_while(|(i, _)| *i <= 500)
            .last()
            .map_or(0, |(i, _)| i);
        format!(
            "{}...\n\n(truncated, {} chars total)",
            &prompt_text[..truncate_at],
            prompt_text.len()
        )
    } else {
        prompt_text
    };

    CommandResult::message(format!(
        "System Prompt ({} mode):\n─────────────────────────────\n{}",
        app.mode.label(),
        display
    ))
}

/// Show context window usage.
///
/// `/context` keeps opening the interactive inspector. `/context report`,
/// `/context json`, `/context prompt-json`, and `/context summary` expose the diagnostic source map
/// from #3143 without replacing the inspector surface.
pub fn context(app: &mut App, arg: Option<&str>) -> CommandResult {
    let Some(subcommand) = arg.map(str::trim).filter(|arg| !arg.is_empty()) else {
        return CommandResult::action(AppAction::OpenContextInspector);
    };

    match subcommand {
        "prompt-json" | "prompt_json" | "prompt" => {
            let context = crate::context_report::build_prompt_context(app);
            CommandResult::message(crate::context_report::prompt_context_json(&context))
        }
        "report" | "json" | "summary" => {
            let report = crate::context_report::build_context_report(app);
            match subcommand {
                "report" => {
                    CommandResult::message(crate::context_report::format_context_report(&report))
                }
                "json" => {
                    CommandResult::message(crate::context_report::context_report_json(&report))
                }
                "summary" => {
                    CommandResult::message(crate::context_report::format_context_summary(&report))
                }
                _ => unreachable!(),
            }
        }
        other => CommandResult::error(format!(
            "Unknown /context subcommand: {other}. Use report, json, prompt-json, or summary."
        )),
    }
}
