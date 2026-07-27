//! Route-aware billing presentation.
//!
//! Model pricing and the way a user pays for a route are different facts.
//! The same model can be metered through an API key or covered by an OAuth /
//! token-plan subscription.  Keep that decision in one small module so TUI
//! surfaces do not infer dollars from a model id alone.
//!
//! Display rule (TUI-DOG-010):
//! - dollars only for metered routes with a real priced usage basis and
//!   positive accrued spend;
//! - OAuth/token-plan routes show a quota label, or a real used % when one
//!   was supplied by the provider;
//! - unknown stays unknown — never `$0.00` and never an estimate-as-spend.

use crate::config::{ApiProvider, Config, ProviderConfig};
use crate::pricing::{CostCurrency, format_cost_amount};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingPresentation {
    /// Per-token API usage may be rendered as a currency estimate.
    Metered,
    /// Account/subscription quota is the truthful owner; dollar estimates are
    /// intentionally hidden unless the provider later exposes real spend.
    Subscription(&'static str),
    /// The route is local or otherwise has no provider bill.
    Local,
    /// Billing basis is not known; never invent dollars or a fake zero.
    Unknown,
}

/// Truthful chip for session/footer/sidebar usage surfaces.
#[derive(Debug, Clone, PartialEq)]
pub enum UsageChip {
    /// Positive accrued spend on a metered route with real pricing.
    Money(String),
    /// Subscription / OAuth allowance. `used_pct` is only set when the
    /// provider supplied a real percentage.
    Allowance {
        label: &'static str,
        used_pct: Option<f32>,
    },
    Local,
    Unknown,
    /// Metered route with pricing, but nothing spent yet — omit the chip
    /// rather than rendering `$0.00` / `<$0.0001`.
    Hidden,
}

impl BillingPresentation {
    #[must_use]
    pub const fn shows_money(self) -> bool {
        matches!(self, Self::Metered)
    }

    #[must_use]
    #[allow(dead_code)] // label helpers for non-metered chip copy (TUI-DOG-010)
    pub const fn label(self) -> Option<&'static str> {
        match self {
            Self::Metered => None,
            Self::Subscription(label) => Some(label),
            Self::Local => Some("local"),
            Self::Unknown => Some("unknown"),
        }
    }
}

/// Immutable, non-secret receipt of the route a request was dispatched on.
///
/// This is what a child/non-active route must be billed from. Re-reading an
/// ambient `Config` for a non-active provider is unsound: `apply_env_overrides`
/// merges provider endpoint variables (`MOONSHOT_BASE_URL`, `KIMI_BASE_URL`,
/// …) into the **active** provider's table only, so a cross-provider child's
/// config entry does not describe the endpoint its client was built with.
///
/// Test-only: the production dispatch path captures a full
/// [`DispatchedReceipt`] at the client-freeze boundary and classifies with
/// [`for_dispatched_receipt`]. This pair exists so route-resolution tests can
/// assert that the pre-dispatch and receipt answers cannot disagree.
#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub struct DispatchedRoute<'a> {
    /// Provider the dispatched client is bound to.
    pub provider: ApiProvider,
    /// Base URL the dispatched client will call, verbatim.
    pub base_url: &'a str,
}

/// A fully captured, `Config`-free billing receipt.
///
/// This is what [`for_dispatched_receipt`] consumes. Every field is captured
/// at dispatch; nothing here can be re-derived later.
#[derive(Debug, Clone, Copy)]
pub struct DispatchedReceipt<'a> {
    /// Provider the dispatched client was bound to.
    pub provider: ApiProvider,
    /// Non-secret identity key that selected this route's table — the
    /// `[providers.<name>]` key for a named custom route, the provider's own
    /// key otherwise.
    ///
    /// `None` means the identity was not captured. For a named custom route
    /// that is fatal to any product claim: without it there is no way to say
    /// *which* custom vendor ran, and the classifier fails closed rather than
    /// reading whichever custom table happens to be selected now.
    pub identity: Option<&'a str>,
    /// Base URL the dispatched client called, verbatim.
    pub base_url: &'a str,
    /// Product truth captured when this client was built.
    pub product: RouteProduct,
}

/// Immutable, non-secret product truth for one route, captured at the moment
/// its client was built.
///
/// Several providers are *credential-shaped* rather than endpoint-shaped: the
/// same host sells both a metered and a subscription product, and only the
/// credential (or an operator-declared pay mode) separates them. That fact
/// cannot be recovered later from an ambient `Config` — the session may have
/// switched provider, custom table, or key since — so it has to travel with
/// the receipt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RouteProduct {
    /// No product fact was captured. Credential-shaped providers must fail
    /// closed on this: an uncaptured product is not a licence to guess.
    #[default]
    Unproven,
    /// The route's credential/pay mode is subscription-backed, with this
    /// user-facing quota label.
    Subscription(&'static str),
    /// The route bills per token.
    Metered,
}

/// Resolve how a provider route should present usage, from the endpoint that
/// route resolves to right now.
///
/// The endpoint is resolved exactly once, through the same identity-aware
/// [`Config::base_url_for_route`] the client is built from, and is then judged
/// by the same exact-product rules a dispatch receipt gets. There is no
/// separate "ambient" reading of a provider's table: a config entry with no
/// `base_url` still resolves to a real endpoint (an imported Kimi token
/// resolves to the Kimi Code membership host), and classifying from the raw
/// table field would call that route metered and invent dollars against a
/// membership quota.
///
/// This is the pre-dispatch answer — for a turn that already ran, bill from
/// its receipt with [`for_dispatched_route`] instead.
#[must_use]
pub fn for_route(config: &Config, provider: ApiProvider) -> BillingPresentation {
    let base_url = config.base_url_for_route(provider);
    let identity = config.provider_identity_for(provider);
    classify(
        provider,
        Some(identity.as_str()),
        &base_url,
        capture_product(config, provider),
    )
}

/// Capture the immutable product facts for `provider` from the config its
/// client is being built from, **at dispatch time**.
///
/// Call this while the config still describes the route being dispatched. The
/// result is what travels on [`DispatchedRoute::product`]; nothing downstream
/// may re-derive it.
#[must_use]
pub fn capture_product(config: &Config, provider: ApiProvider) -> RouteProduct {
    let provider_config = config.provider_config_for(provider);
    match provider {
        ApiProvider::XiaomiMimo => {
            if xiaomi_is_explicit_pay_as_you_go(provider_config) {
                RouteProduct::Metered
            } else {
                RouteProduct::Subscription("MiMo token plan")
            }
        }
        ApiProvider::Xai => {
            if provider_config.is_some_and(uses_xai_oauth)
                && crate::xai_oauth::credentials_valid(config)
            {
                RouteProduct::Subscription("Grok OAuth quota")
            } else {
                RouteProduct::Metered
            }
        }
        ApiProvider::Anthropic => {
            if provider_config.is_some_and(uses_anthropic_oauth) {
                RouteProduct::Subscription("Claude OAuth quota")
            } else {
                RouteProduct::Metered
            }
        }
        ApiProvider::Custom => match provider_config {
            Some(entry) if !custom_billing_unknown(entry) => RouteProduct::Metered,
            // No table, or a table with no declared pay mode: a custom vendor
            // that has not told us how it bills.
            _ => RouteProduct::Unproven,
        },
        // Endpoint-shaped and flat-rate providers need no credential fact.
        _ => RouteProduct::Unproven,
    }
}

/// Resolve billing for a route from its dispatch-time receipt.
///
/// Deliberately takes no `Config`: after dispatch there is no sound ambient
/// state to consult. The session can have switched provider, custom table, or
/// credential since the request went out, so every fact this needs must
/// already be on the receipt. A receipt that does not name a product fails
/// closed to [`BillingPresentation::Unknown`] rather than inventing one.
/// Classify a receipt with no `Config` in reach at all.
///
/// This is the entry point every post-dispatch caller must use. Because it
/// takes no config, a provider switch, a `/provider` change, or a different
/// custom table being selected after dispatch cannot retro-bill the turn onto
/// another route.
#[must_use]
pub fn for_dispatched_receipt(receipt: DispatchedReceipt<'_>) -> BillingPresentation {
    classify(
        receipt.provider,
        receipt.identity,
        receipt.base_url,
        receipt.product,
    )
}

/// Convenience wrapper for callers that still hold the route's own
/// **dispatch-time** config and have not captured a receipt yet.
///
/// Sound only while `config` still describes the dispatched route. Anything
/// that runs after the turn has already completed must capture a
/// [`DispatchedReceipt`] at dispatch and use [`for_dispatched_receipt`].
#[cfg(test)]
#[must_use]
pub fn for_dispatched_route(config: &Config, route: DispatchedRoute<'_>) -> BillingPresentation {
    let identity = config.provider_identity_for(route.provider);
    for_dispatched_receipt(DispatchedReceipt {
        provider: route.provider,
        identity: Some(identity.as_str()),
        base_url: route.base_url,
        product: capture_product(config, route.provider),
    })
}

/// The one classifier, pure in its inputs.
///
/// `base_url` is the single resolved endpoint for this route and `product` is
/// the captured credential truth. There is no `Config` parameter on purpose:
/// this cannot read a provider table, a custom entry, or an active selection,
/// so a pre-dispatch answer and a receipt answer cannot drift apart and a
/// post-dispatch provider switch cannot retro-bill a turn onto another route.
fn classify(
    provider: ApiProvider,
    identity: Option<&str>,
    base_url: &str,
    product: RouteProduct,
) -> BillingPresentation {
    if matches!(
        provider,
        ApiProvider::Ollama | ApiProvider::Sglang | ApiProvider::Vllm
    ) {
        return BillingPresentation::Local;
    }
    if provider == ApiProvider::OpenaiCodex {
        return BillingPresentation::Subscription("Codex OAuth quota");
    }
    if provider == ApiProvider::OpencodeGo {
        return BillingPresentation::Subscription("OpenCode Go quota");
    }

    match provider {
        // StepFun already reduces an endpoint to a non-secret billing surface
        // and fails closed on anything it does not recognize.
        ApiProvider::Stepfun => stepfun_billing_for_endpoint(Some(base_url)),
        // Z.ai's dedicated Coding endpoint is the GLM Coding Plan route. Its
        // quota is subscription-backed, so a public API price estimate is not
        // truthful spend and must not appear as dollars in the UI. A
        // credentials-only `[providers.zai]` entry still resolves to that
        // endpoint, because it is also CodeWhale's Z.ai default.
        ApiProvider::Zai if base_url.trim().is_empty() => BillingPresentation::Unknown,
        ApiProvider::Zai if is_zai_coding_plan_endpoint(base_url) => {
            BillingPresentation::Subscription("Z.ai Coding Plan quota")
        }
        ApiProvider::Zai => BillingPresentation::Metered,
        ApiProvider::XiaomiMimo => product_billing(product),

        ApiProvider::Xai | ApiProvider::Anthropic => product_billing(product),
        // A named custom route is billed from the identity and endpoint it
        // dispatched on. Without an identity there is no vendor to name, and
        // without an endpoint there is no route at all — either way the honest
        // answer is Unknown rather than whatever the active custom table says.
        ApiProvider::Custom
            if identity.is_none_or(|key| key.trim().is_empty()) || base_url.trim().is_empty() =>
        {
            BillingPresentation::Unknown
        }
        ApiProvider::Custom => product_billing(product),
        _ => BillingPresentation::Metered,
    }
}

/// Credential-shaped providers answer from the captured product and nothing
/// else. An uncaptured product is Unknown: no invented dollars, no invented
/// quota label.
fn product_billing(product: RouteProduct) -> BillingPresentation {
    match product {
        RouteProduct::Subscription(label) => BillingPresentation::Subscription(label),
        RouteProduct::Metered => BillingPresentation::Metered,
        RouteProduct::Unproven => BillingPresentation::Unknown,
    }
}

/// StepFun already reduces an endpoint to a non-secret billing surface and
/// fails closed on anything it does not recognize, so the resolved endpoint
/// and a dispatch receipt use the same reduction unchanged.
fn stepfun_billing_for_endpoint(base_url: Option<&str>) -> BillingPresentation {
    match crate::pricing::billing_surface_for_route(ApiProvider::Stepfun, base_url) {
        Some(crate::pricing::STEPFUN_PAYG_BILLING_SURFACE) => BillingPresentation::Metered,
        Some(crate::pricing::STEPFUN_PLAN_BILLING_SURFACE) => {
            BillingPresentation::Subscription("StepFun Step Plan quota")
        }
        _ => BillingPresentation::Unknown,
    }
}

fn is_zai_coding_plan_endpoint(base_url: &str) -> bool {
    base_url
        .trim()
        .trim_end_matches('/')
        .ends_with("/api/coding/paas/v4")
}

/// Billing for a child route when its full dispatch config is not present in
/// the completion envelope. Never invent metered dollars for providers that
/// support subscription/OAuth routes; the parent route remains authoritative
/// only when the provider is the same.
#[must_use]
pub fn for_child_route(
    parent_provider: ApiProvider,
    parent_billing: BillingPresentation,
    child_provider: ApiProvider,
) -> BillingPresentation {
    if child_provider == parent_provider {
        return parent_billing;
    }
    match child_provider {
        ApiProvider::Ollama | ApiProvider::Sglang | ApiProvider::Vllm => BillingPresentation::Local,
        ApiProvider::OpencodeGo => BillingPresentation::Subscription("OpenCode Go quota"),
        ApiProvider::OpenaiCodex
        | ApiProvider::Xai
        | ApiProvider::Moonshot
        | ApiProvider::Anthropic
        | ApiProvider::XiaomiMimo
        | ApiProvider::Zai => BillingPresentation::Subscription("provider quota"),
        ApiProvider::Stepfun | ApiProvider::Custom => BillingPresentation::Unknown,
        _ => BillingPresentation::Metered,
    }
}

/// Whether this route may show a dollar amount for the given model.
///
/// Requires both a metered billing presentation and an authoritative priced
/// basis for the model. OAuth/token-plan routes always return false even when
/// the same model id is priced on a public API route.
#[must_use]
pub fn has_priced_metered_basis(
    billing: BillingPresentation,
    provider: ApiProvider,
    model: &str,
) -> bool {
    billing.shows_money()
        && if provider == ApiProvider::Stepfun {
            crate::pricing::has_pricing_for_billing_surface(
                provider,
                model,
                Some(crate::pricing::STEPFUN_PAYG_BILLING_SURFACE),
            )
        } else {
            crate::pricing::has_pricing_for_provider(provider, model)
        }
}

/// Build the truthful usage chip for session surfaces.
///
/// `used_pct` is only honored for subscription/OAuth routes and must come from
/// a provider-supplied allowance reading — never from a local estimate.
#[must_use]
pub fn usage_chip(
    billing: BillingPresentation,
    provider: ApiProvider,
    model: &str,
    displayed_cost: f64,
    currency: CostCurrency,
    used_pct: Option<f32>,
) -> UsageChip {
    match billing {
        BillingPresentation::Local => UsageChip::Local,
        BillingPresentation::Unknown => UsageChip::Unknown,
        BillingPresentation::Subscription(label) => UsageChip::Allowance {
            label,
            used_pct: used_pct.filter(|pct| pct.is_finite() && *pct >= 0.0),
        },
        BillingPresentation::Metered => {
            if !has_priced_metered_basis(billing, provider, model) {
                UsageChip::Unknown
            } else if displayed_cost.is_finite() && displayed_cost > 0.0 {
                UsageChip::Money(format_cost_amount(displayed_cost, currency))
            } else {
                UsageChip::Hidden
            }
        }
    }
}

/// Compact footer/header chip text. `None` means omit the chip.
#[must_use]
#[allow(dead_code)] // shared chip formatter for footer/sidebar siblings (TUI-DOG-010)
pub fn format_usage_chip(chip: &UsageChip) -> Option<String> {
    match chip {
        UsageChip::Money(amount) => Some(amount.clone()),
        UsageChip::Allowance { label, used_pct } => Some(match used_pct {
            Some(pct) => format!("usage: {label} · {pct:.0}%"),
            None => format!("usage: {label}"),
        }),
        UsageChip::Local => Some("cost: local".to_string()),
        UsageChip::Unknown => Some("cost: unknown".to_string()),
        UsageChip::Hidden => None,
    }
}

/// Sidebar / detail line. Always returns a string so the panel has an owner.
#[must_use]
pub fn format_usage_line(chip: &UsageChip) -> String {
    match chip {
        UsageChip::Money(amount) => format!("cost: {amount}"),
        UsageChip::Allowance { label, used_pct } => match used_pct {
            Some(pct) => format!("usage: {label} · {pct:.0}% used"),
            None => format!("usage: {label}"),
        },
        UsageChip::Local => "cost: local".to_string(),
        UsageChip::Unknown => "cost: unknown".to_string(),
        UsageChip::Hidden => "cost: —".to_string(),
    }
}

fn custom_billing_unknown(config: &ProviderConfig) -> bool {
    // A custom OpenAI-compatible endpoint with no explicit pay mode and no
    // priced catalog is treated as unknown rather than inventing metered
    // dollars from a borrowed model id.
    let mode = auth_mode(config);
    !mode.as_deref().is_some_and(|mode| {
        matches!(
            mode,
            "api_key"
                | "api"
                | "key"
                | "keyring"
                | "payg"
                | "paygo"
                | "pay_as_you_go"
                | "metered"
                | "standard"
        )
    })
}

fn normalized(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

fn auth_mode(config: &ProviderConfig) -> Option<String> {
    config
        .auth_mode
        .as_deref()
        .or(config.mode.as_deref())
        .map(normalized)
}

fn uses_xai_oauth(config: &ProviderConfig) -> bool {
    auth_mode(config).is_some_and(|mode| crate::xai_oauth::auth_mode_uses_xai_oauth(&mode))
}

fn uses_anthropic_oauth(config: &ProviderConfig) -> bool {
    auth_mode(config).is_some_and(|mode| {
        matches!(
            mode.as_str(),
            "oauth"
                | "anthropic_oauth"
                | "claude_oauth"
                | "claude_cli"
                | "claude_code"
                | "max"
                | "subscription"
        )
    })
}

fn xiaomi_is_explicit_pay_as_you_go(config: Option<&ProviderConfig>) -> bool {
    if let Some(mode) = std::env::var("XIAOMI_MIMO_MODE")
        .ok()
        .filter(|mode| !mode.trim().is_empty())
        .map(|mode| normalized(&mode))
    {
        return matches!(
            mode.as_str(),
            "standard" | "default" | "payg" | "paygo" | "pay_as_you_go" | "pay_as_go"
        );
    }
    if let Some(base_url) = std::env::var("XIAOMI_MIMO_BASE_URL")
        .ok()
        .filter(|base_url| !base_url.trim().is_empty())
    {
        return !base_url.to_ascii_lowercase().contains("token-plan-");
    }
    let token_plan_env = ["XIAOMI_MIMO_TOKEN_PLAN_API_KEY", "MIMO_TOKEN_PLAN_API_KEY"]
        .iter()
        .any(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()));
    let standard_env = ["XIAOMI_MIMO_API_KEY", "XIAOMI_API_KEY", "MIMO_API_KEY"]
        .iter()
        .any(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()));
    if standard_env && !token_plan_env {
        return true;
    }
    let Some(config) = config else {
        // The shipped MiMo default is a token-plan endpoint.
        return false;
    };
    if let Some(mode) = config
        .mode
        .as_deref()
        .filter(|mode| !mode.trim().is_empty())
        .map(normalized)
    {
        return matches!(
            mode.as_str(),
            "pay_as_you_go" | "payg" | "paygo" | "api" | "standard" | "default"
        );
    }
    if let Some(api_key) = config
        .api_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
    {
        return !api_key.trim_start().starts_with("tp-");
    }
    config.base_url.as_deref().is_some_and(|base_url| {
        let lower = base_url.to_ascii_lowercase();
        !lower.contains("token-plan-") && !lower.contains("token_plan_")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricing::CostCurrency;

    fn config_with(provider: ApiProvider, provider_config: ProviderConfig) -> Config {
        let mut config = Config::default();
        *config.provider_config_for_mut(provider) = provider_config;
        config
    }

    #[test]
    fn codex_oauth_never_claims_api_dollars() {
        assert_eq!(
            for_route(&Config::default(), ApiProvider::OpenaiCodex),
            BillingPresentation::Subscription("Codex OAuth quota")
        );
        let chip = usage_chip(
            BillingPresentation::Subscription("Codex OAuth quota"),
            ApiProvider::OpenaiCodex,
            "gpt-5.5",
            12.34,
            CostCurrency::Usd,
            None,
        );
        assert_eq!(
            format_usage_chip(&chip).as_deref(),
            Some("usage: Codex OAuth quota")
        );
        assert!(!format_usage_line(&chip).contains('$'));
    }

    #[test]
    fn unsupported_kimi_cli_mode_never_claims_imported_token_billing() {
        let config = config_with(
            ApiProvider::Moonshot,
            ProviderConfig {
                auth_mode: Some("kimi_oauth".to_string()),
                ..ProviderConfig::default()
            },
        );
        let billing = for_route(&config, ApiProvider::Moonshot);
        assert_eq!(billing, BillingPresentation::Metered);
        let chip = usage_chip(
            billing,
            ApiProvider::Moonshot,
            crate::config::DEFAULT_KIMI_CODE_MODEL,
            12.34,
            CostCurrency::Usd,
            None,
        );
        assert_eq!(format_usage_chip(&chip).as_deref(), Some("cost: unknown"));
        assert!(!format_usage_line(&chip).contains("OAuth"));
        assert!(!format_usage_line(&chip).contains("imported token"));
        assert!(!format_usage_line(&chip).contains('$'));
    }

    #[test]
    fn xai_api_key_fallback_is_metered_when_external_oauth_is_unavailable() {
        let _lock = crate::test_support::lock_test_env();
        let temp = tempfile::tempdir().expect("xAI billing fixture");
        let grok_path = temp.path().join("external-grok-auth.json");
        std::fs::write(&grok_path, "must-never-be-read").expect("external trap");
        let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", temp.path());
        let _grok = crate::test_support::EnvVarGuard::set("GROK_AUTH_PATH", &grok_path);

        let config = config_with(
            ApiProvider::Xai,
            ProviderConfig {
                auth_mode: Some("oauth".to_string()),
                api_key: Some("xai-api-key".to_string()),
                ..ProviderConfig::default()
            },
        );
        crate::external_credentials::reset_side_effect_trap();
        assert_eq!(
            for_route(&config, ApiProvider::Xai),
            BillingPresentation::Metered
        );
        assert_eq!(
            crate::external_credentials::side_effect_trap_counts(),
            (0, 0)
        );
        assert_eq!(
            std::fs::read_to_string(grok_path).expect("external trap unchanged"),
            "must-never-be-read"
        );
    }

    #[test]
    fn opencode_go_quota_never_claims_token_dollars() {
        let billing = for_route(&Config::default(), ApiProvider::OpencodeGo);
        assert_eq!(
            billing,
            BillingPresentation::Subscription("OpenCode Go quota")
        );
        let chip = usage_chip(
            billing,
            ApiProvider::OpencodeGo,
            "deepseek-v4-pro",
            12.34,
            CostCurrency::Usd,
            None,
        );
        assert!(!format_usage_line(&chip).contains('$'));
        assert_eq!(
            for_child_route(
                ApiProvider::Deepseek,
                BillingPresentation::Metered,
                ApiProvider::OpencodeGo,
            ),
            BillingPresentation::Subscription("OpenCode Go quota")
        );
    }

    #[test]
    fn zai_coding_plan_endpoint_never_claims_api_dollars() {
        let config = config_with(
            ApiProvider::Zai,
            ProviderConfig {
                base_url: Some("https://api.z.ai/api/coding/paas/v4".to_string()),
                ..ProviderConfig::default()
            },
        );
        let billing = for_route(&config, ApiProvider::Zai);
        assert_eq!(
            billing,
            BillingPresentation::Subscription("Z.ai Coding Plan quota")
        );
        let chip = usage_chip(
            billing,
            ApiProvider::Zai,
            "glm-5.2",
            0.05,
            CostCurrency::Usd,
            None,
        );
        assert!(!format_usage_line(&chip).contains('$'));
    }

    #[test]
    fn zai_default_coding_endpoint_never_claims_api_dollars() {
        // The route resolves its shipped default, so the ambient generic
        // endpoint override has to be locked out for the assertion to be
        // about the default at all.
        let _lock = crate::test_support::lock_test_env();
        let _generic = crate::test_support::EnvVarGuard::remove("CODEWHALE_BASE_URL");
        let _legacy = crate::test_support::EnvVarGuard::remove("DEEPSEEK_BASE_URL");
        let config = config_with(ApiProvider::Zai, ProviderConfig::default());
        assert_eq!(
            for_route(&config, ApiProvider::Zai),
            BillingPresentation::Subscription("Z.ai Coding Plan quota")
        );
    }

    #[test]
    fn stepfun_payg_shows_money_but_step_plan_stays_subscription_billed() {
        let payg_billing = for_route(&Config::default(), ApiProvider::Stepfun);
        assert_eq!(payg_billing, BillingPresentation::Metered);
        let payg_chip = usage_chip(
            payg_billing,
            ApiProvider::Stepfun,
            crate::config::DEFAULT_STEPFUN_MODEL,
            0.42,
            CostCurrency::Usd,
            None,
        );
        assert_eq!(format_usage_chip(&payg_chip).as_deref(), Some("$0.42"));

        let plan_config = config_with(
            ApiProvider::Stepfun,
            ProviderConfig {
                base_url: Some("https://api.stepfun.ai/step_plan/v1".to_string()),
                ..ProviderConfig::default()
            },
        );
        let plan_billing = for_route(&plan_config, ApiProvider::Stepfun);
        assert_eq!(
            plan_billing,
            BillingPresentation::Subscription("StepFun Step Plan quota")
        );
        let plan_chip = usage_chip(
            plan_billing,
            ApiProvider::Stepfun,
            crate::config::DEFAULT_STEPFUN_MODEL,
            0.42,
            CostCurrency::Usd,
            None,
        );
        assert!(!format_usage_line(&plan_chip).contains('$'));

        assert_eq!(
            for_child_route(
                ApiProvider::Deepseek,
                BillingPresentation::Metered,
                ApiProvider::Stepfun,
            ),
            BillingPresentation::Unknown
        );
    }

    #[test]
    fn routed_zai_child_never_claims_api_dollars_without_full_route_config() {
        assert_eq!(
            for_child_route(
                ApiProvider::Deepseek,
                BillingPresentation::Metered,
                ApiProvider::Zai,
            ),
            BillingPresentation::Subscription("provider quota")
        );
    }

    #[test]
    fn oauth_allowance_percent_is_shown_when_provider_supplies_it() {
        let chip = usage_chip(
            BillingPresentation::Subscription("Grok OAuth quota"),
            ApiProvider::Xai,
            "grok-4",
            0.0,
            CostCurrency::Usd,
            Some(37.0),
        );
        assert_eq!(
            format_usage_chip(&chip).as_deref(),
            Some("usage: Grok OAuth quota · 37%")
        );
    }

    #[test]
    fn api_key_metered_shows_dollars_only_with_priced_positive_spend() {
        let billing = BillingPresentation::Metered;
        assert!(has_priced_metered_basis(
            billing,
            ApiProvider::Deepseek,
            "deepseek-v4-flash"
        ));
        let spent = usage_chip(
            billing,
            ApiProvider::Deepseek,
            "deepseek-v4-flash",
            0.42,
            CostCurrency::Usd,
            None,
        );
        assert_eq!(format_usage_chip(&spent).as_deref(), Some("$0.42"));

        let zero = usage_chip(
            billing,
            ApiProvider::Deepseek,
            "deepseek-v4-flash",
            0.0,
            CostCurrency::Usd,
            None,
        );
        assert_eq!(zero, UsageChip::Hidden);
        assert!(format_usage_chip(&zero).is_none());
        assert!(!format_usage_line(&zero).contains('$'));
    }

    #[test]
    fn local_free_routes_never_show_dollars() {
        assert_eq!(
            for_route(&Config::default(), ApiProvider::Ollama),
            BillingPresentation::Local
        );
        let chip = usage_chip(
            BillingPresentation::Local,
            ApiProvider::Ollama,
            "llama3.2",
            9.99,
            CostCurrency::Usd,
            None,
        );
        assert_eq!(format_usage_chip(&chip).as_deref(), Some("cost: local"));
        assert!(!format_usage_line(&chip).contains('$'));
    }

    #[test]
    fn unknown_is_unknown_not_zero_dollars() {
        let chip = usage_chip(
            BillingPresentation::Metered,
            ApiProvider::NvidiaNim,
            "deepseek-ai/deepseek-v4-pro",
            0.0,
            CostCurrency::Usd,
            None,
        );
        assert_eq!(chip, UsageChip::Unknown);
        assert_eq!(format_usage_chip(&chip).as_deref(), Some("cost: unknown"));
        assert!(!format_usage_line(&chip).contains('$'));

        let unknown_billing = usage_chip(
            BillingPresentation::Unknown,
            ApiProvider::Custom,
            "anything",
            1.23,
            CostCurrency::Usd,
            None,
        );
        assert_eq!(unknown_billing, UsageChip::Unknown);
        assert!(!format_usage_line(&unknown_billing).contains('$'));
    }

    #[test]
    fn xai_oauth_and_api_key_routes_stay_distinct() {
        let _lock = crate::test_support::lock_test_env();
        let temp = tempfile::tempdir().expect("xAI owned credential fixture");
        let owned_home = temp
            .path()
            .canonicalize()
            .expect("canonical xAI owned credential fixture");
        let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", &owned_home);
        let owned_path = owned_home.join("credentials/xai-auth.json");
        std::fs::create_dir_all(owned_path.parent().expect("owned credential parent"))
            .expect("create owned credential directory");
        #[cfg(windows)]
        crate::external_credentials::secure_codewhale_owned_windows_path(
            owned_path.parent().expect("owned credential parent"),
            true,
        )
        .expect("secure owned credential directory");
        let scope = format!(
            "{}::{}",
            crate::xai_oauth::XAI_OIDC_ISSUER,
            crate::xai_oauth::GROK_OIDC_CLIENT_ID
        );
        std::fs::write(
            &owned_path,
            serde_json::json!({
                scope: {
                    "key": crate::test_support::future_test_jwt("billing"),
                    "auth_mode": "oidc"
                }
            })
            .to_string(),
        )
        .expect("write Codewhale-owned xAI credential");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&owned_path, std::fs::Permissions::from_mode(0o600))
                .expect("secure owned credential file");
        }
        #[cfg(windows)]
        crate::external_credentials::secure_codewhale_owned_windows_path(&owned_path, false)
            .expect("secure owned credential file");
        let oauth = config_with(
            ApiProvider::Xai,
            ProviderConfig {
                auth_mode: Some("grok-oauth".to_string()),
                ..ProviderConfig::default()
            },
        );
        let api = config_with(
            ApiProvider::Xai,
            ProviderConfig {
                auth_mode: Some("api-key".to_string()),
                ..ProviderConfig::default()
            },
        );
        assert!(!for_route(&oauth, ApiProvider::Xai).shows_money());
        assert!(for_route(&api, ApiProvider::Xai).shows_money());
    }

    #[test]
    fn future_claude_oauth_does_not_inherit_anthropic_api_prices() {
        let oauth = config_with(
            ApiProvider::Anthropic,
            ProviderConfig {
                auth_mode: Some("claude-code".to_string()),
                ..ProviderConfig::default()
            },
        );
        assert_eq!(
            for_route(&oauth, ApiProvider::Anthropic).label(),
            Some("Claude OAuth quota")
        );
    }

    #[test]
    fn xiaomi_defaults_to_token_plan_but_explicit_payg_is_metered() {
        let _lock = crate::test_support::lock_test_env();
        let _mode = crate::test_support::EnvVarGuard::remove("XIAOMI_MIMO_MODE");
        let _base = crate::test_support::EnvVarGuard::remove("XIAOMI_MIMO_BASE_URL");
        let _token = crate::test_support::EnvVarGuard::remove("XIAOMI_MIMO_TOKEN_PLAN_API_KEY");
        let _token_alias = crate::test_support::EnvVarGuard::remove("MIMO_TOKEN_PLAN_API_KEY");
        let _standard_a = crate::test_support::EnvVarGuard::remove("XIAOMI_MIMO_API_KEY");
        let _standard_b = crate::test_support::EnvVarGuard::remove("XIAOMI_API_KEY");
        let _standard_c = crate::test_support::EnvVarGuard::remove("MIMO_API_KEY");
        assert!(!for_route(&Config::default(), ApiProvider::XiaomiMimo).shows_money());
        let payg = config_with(
            ApiProvider::XiaomiMimo,
            ProviderConfig {
                mode: Some("pay-as-you-go".to_string()),
                ..ProviderConfig::default()
            },
        );
        assert!(for_route(&payg, ApiProvider::XiaomiMimo).shows_money());
        let standard_key = config_with(
            ApiProvider::XiaomiMimo,
            ProviderConfig {
                api_key: Some("sk-standard".to_string()),
                ..ProviderConfig::default()
            },
        );
        assert!(for_route(&standard_key, ApiProvider::XiaomiMimo).shows_money());
    }

    #[test]
    fn unknown_cross_provider_oauth_capable_child_never_invents_dollars() {
        assert!(
            !for_child_route(
                ApiProvider::Deepseek,
                BillingPresentation::Metered,
                ApiProvider::Xai,
            )
            .shows_money()
        );
        assert!(
            for_child_route(
                ApiProvider::Deepseek,
                BillingPresentation::Metered,
                ApiProvider::Openrouter,
            )
            .shows_money()
        );
    }

    #[test]
    fn standard_mimo_env_key_uses_metered_presentation() {
        let _lock = crate::test_support::lock_test_env();
        let _mode = crate::test_support::EnvVarGuard::remove("XIAOMI_MIMO_MODE");
        let _base = crate::test_support::EnvVarGuard::remove("XIAOMI_MIMO_BASE_URL");
        let _token = crate::test_support::EnvVarGuard::remove("XIAOMI_MIMO_TOKEN_PLAN_API_KEY");
        let _token_alias = crate::test_support::EnvVarGuard::remove("MIMO_TOKEN_PLAN_API_KEY");
        let _standard_a = crate::test_support::EnvVarGuard::remove("XIAOMI_MIMO_API_KEY");
        let _standard_b = crate::test_support::EnvVarGuard::remove("XIAOMI_API_KEY");
        let _standard = crate::test_support::EnvVarGuard::set("MIMO_API_KEY", "sk-metered");

        assert!(for_route(&Config::default(), ApiProvider::XiaomiMimo).shows_money());
    }

    #[test]
    fn custom_without_pay_mode_stays_unknown() {
        assert_eq!(
            for_route(&Config::default(), ApiProvider::Custom),
            BillingPresentation::Unknown
        );
        let mut metered_custom = Config {
            provider: Some("acme".to_string()),
            ..Config::default()
        };
        *metered_custom.provider_config_for_mut(ApiProvider::Custom) = ProviderConfig {
            auth_mode: Some("api-key".to_string()),
            ..ProviderConfig::default()
        };
        assert_eq!(
            for_route(&metered_custom, ApiProvider::Custom),
            BillingPresentation::Metered
        );
    }

    /// Cross-provider dispatch receipts for the other endpoint-shaped routes.
    #[test]
    fn dispatched_endpoint_shaped_routes_classify_from_the_receipt() {
        let config = Config::default();
        // StepFun: plan endpoint, PAYG endpoint, unrecognized host.
        assert_eq!(
            for_dispatched_route(
                &config,
                DispatchedRoute {
                    provider: ApiProvider::Stepfun,
                    base_url: "https://api.stepfun.ai/step_plan/v1",
                },
            ),
            BillingPresentation::Subscription("StepFun Step Plan quota")
        );
        assert_eq!(
            for_dispatched_route(
                &config,
                DispatchedRoute {
                    provider: ApiProvider::Stepfun,
                    base_url: crate::config::DEFAULT_STEPFUN_BASE_URL,
                },
            ),
            BillingPresentation::Metered
        );
        assert_eq!(
            for_dispatched_route(
                &config,
                DispatchedRoute {
                    provider: ApiProvider::Stepfun,
                    base_url: "https://gateway.internal.example/v1",
                },
            ),
            BillingPresentation::Unknown
        );
        // Z.ai: the Coding Plan path is quota-billed; a blank receipt is not
        // an excuse to fall back to the plan default.
        assert_eq!(
            for_dispatched_route(
                &config,
                DispatchedRoute {
                    provider: ApiProvider::Zai,
                    base_url: "https://api.z.ai/api/coding/paas/v4",
                },
            ),
            BillingPresentation::Subscription("Z.ai Coding Plan quota")
        );
        assert_eq!(
            for_dispatched_route(
                &config,
                DispatchedRoute {
                    provider: ApiProvider::Zai,
                    base_url: "",
                },
            ),
            BillingPresentation::Unknown
        );
        // Identity-owned routes are unchanged by the receipt.
        assert_eq!(
            for_dispatched_route(
                &config,
                DispatchedRoute {
                    provider: ApiProvider::Ollama,
                    base_url: "http://localhost:11434/v1",
                },
            ),
            BillingPresentation::Local
        );
        assert_eq!(
            for_dispatched_route(
                &config,
                DispatchedRoute {
                    provider: ApiProvider::OpenaiCodex,
                    base_url: "https://chatgpt.com/backend-api/codex",
                },
            ),
            BillingPresentation::Subscription("Codex OAuth quota")
        );
    }
}
