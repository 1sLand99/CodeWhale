//! OpenAI Codex / ChatGPT OAuth credential loading.
//!
//! External Codex CLI credentials are read only after an exact, provider-scoped
//! consent grant. Codewhale never refreshes or rewrites that external file.
//!
//! # Security
//!
//! Token values are never logged or printed. All debug representations
//! redact sensitive fields.

use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use codewhale_config::ExternalCredentialReadGrant;
use serde::Deserialize;

/// OAuth token payload stored in `auth.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct AuthTokens {
    access_token: Option<String>,
    account_id: Option<String>,
}

/// Top-level structure of Codex CLI's `auth.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct CodexAuthFile {
    tokens: Option<AuthTokens>,
}

/// Resolved OAuth credentials ready for API use.
#[derive(Debug, Clone)]
pub struct CodexCredentials {
    pub access_token: String,
    pub account_id: Option<String>,
}

/// JWT claims subset for expiry extraction.
#[derive(Debug, Deserialize)]
struct JwtClaims {
    exp: Option<u64>,
}

/// Resolve the path to the Codex auth file.
///
/// Priority:
/// 1. `OPENAI_CODEX_AUTH_FILE` env var
/// 2. `$CODEX_HOME/auth.json`
/// 3. `~/.codex/auth.json`
pub fn auth_file_path() -> PathBuf {
    if let Ok(path) = std::env::var("OPENAI_CODEX_AUTH_FILE") {
        let p = PathBuf::from(&path);
        if !p.as_os_str().is_empty() {
            return codewhale_config::resolve_external_credential_path(&p).unwrap_or(p);
        }
    }
    let codex_home = std::env::var("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            crate::config::effective_home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".codex")
        });
    let path = codex_home.join("auth.json");
    codewhale_config::resolve_external_credential_path(&path).unwrap_or(path)
}

/// Try to extract `exp` (epoch seconds) from a JWT without verifying
/// the signature. Returns `None` on any parse failure.
fn jwt_expiry_seconds(token: &str) -> Option<u64> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload = parts[1];
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let claims: JwtClaims = serde_json::from_slice(&decoded).ok()?;
    claims.exp
}

/// Check whether an access token is expired, with a 60-second safety margin.
fn token_is_expired(access_token: &str) -> bool {
    match jwt_expiry_seconds(access_token) {
        Some(exp) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_secs();
            // 60-second safety margin
            now + 60 >= exp
        }
        // If we can't prove freshness, fail closed. External credentials are
        // never refreshed by Codewhale.
        None => true,
    }
}

/// Load Codex credentials from the auth file.
///
/// Returns `Ok(None)` if the file doesn't exist or has no usable tokens.
/// Returns `Err` only on parse/IO errors that aren't "file not found".
fn load_credentials(grant: &ExternalCredentialReadGrant) -> Result<Option<CodexCredentials>> {
    let Some(contents) = crate::external_credentials::read_to_string(grant)? else {
        return Ok(None);
    };
    let auth: CodexAuthFile = serde_json::from_str(&contents).map_err(|_| {
        anyhow::anyhow!(
            "Codex credential file {} is not valid credential JSON",
            codewhale_config::quote_os_path(grant.path())
        )
    })?;
    let tokens = match auth.tokens {
        Some(t) => t,
        None => return Ok(None),
    };
    let access_token = match tokens.access_token {
        Some(t) if !t.trim().is_empty() => t,
        _ => return Ok(None),
    };
    Ok(Some(CodexCredentials {
        access_token,
        account_id: tokens.account_id,
    }))
}

/// Prompt-free, non-refreshing readiness check for picker/onboarding surfaces.
/// It reads process-level token variables only; no file or network access occurs.
#[must_use]
pub fn credentials_from_env() -> Option<CodexCredentials> {
    ["OPENAI_CODEX_ACCESS_TOKEN", "CODEX_ACCESS_TOKEN"]
        .iter()
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .filter(|token| !token.trim().is_empty())
        })
        .map(|access_token| CodexCredentials {
            access_token,
            account_id: codex_account_id_env(),
        })
}

/// Validate only the stored OAuth file, excluding token environment
/// overrides so config-vs-env provenance remains truthful.
///
/// This consumes a grant, so it can only run for a path the user explicitly
/// consented to. Consent is not a credential (#5772): status surfaces call
/// this to find out whether the consented file *still* holds a usable token,
/// because a record that outlives its token would otherwise read as stored.
#[must_use]
pub fn stored_credentials_present(grant: &ExternalCredentialReadGrant) -> bool {
    load_credentials(grant)
        .ok()
        .flatten()
        .is_some_and(|credentials| !token_is_expired(&credentials.access_token))
}

/// Load read-only credentials from the exact external path authorized by
/// `grant`. Expired tokens fail with guidance; they are never refreshed.
pub fn get_credentials(grant: &ExternalCredentialReadGrant) -> Result<CodexCredentials> {
    let creds = load_credentials(grant)?.with_context(missing_auth_message)?;

    // Check if the access token is still valid.
    if !token_is_expired(&creds.access_token) {
        return Ok(creds);
    }

    bail!(
        "Codex access token in {} is expired. Read-only consent never refreshes or rewrites another CLI's credentials. Sign in with ChatGPT via `codewhale auth chatgpt`, run `codex login` again, or provide OPENAI_CODEX_ACCESS_TOKEN for this process.",
        codewhale_config::quote_os_path(grant.path())
    )
}

#[must_use]
pub fn missing_auth_message() -> String {
    crate::chatgpt_oauth::missing_auth_message()
}

/// Read a ChatGPT account id from env overrides only.
fn codex_account_id_env() -> Option<String> {
    for var in ["OPENAI_CODEX_ACCOUNT_ID", "CODEX_ACCOUNT_ID"] {
        if let Ok(value) = std::env::var(var) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

// ── ONE access-route flow ─────────────────────────────────────────────
// Providers are DATA, not files. Every subscription login — xAI device
// code today, ChatGPT PKCE next — runs through the parameter table below
// and the shared device-code core; per-provider OAuth modules are deleted,
// not repaired.

/// How this run reaches a provider: an OAuth login Codewhale owns, a
/// read-only import from another CLI, a pasted key, or (reserved) an ACP
/// subscription bridge. The bridge arm lands with the ACP work; until then
/// it resolves to a clear error, never a silent fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "3b-ii wires the access-route dispatch that consumes this"
)]
pub enum AccessMethod {
    /// Browser/device OAuth login whose tokens Codewhale stores and refreshes.
    OwnedOAuth(OAuthProvider),
    /// Read-only credentials owned by another CLI, behind a consent grant.
    ExternalImport(ExternalImportSource),
    /// A pasted API key. No login, no refresh, no storage beyond config.
    ApiKey,
    /// Subscription access through an ACP bridge (Antigravity, Copilot).
    /// Reserved: no producer yet.
    AcpBridge,
}

/// Providers with an OAuth login Codewhale can own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthProvider {
    Xai,
    /// No device-code producer yet; the PKCE path (3b-i(b)) constructs this.
    #[allow(
        dead_code,
        reason = "3b-i(b) wires the PKCE login that constructs this"
    )]
    Chatgpt,
}

impl AccessMethod {
    /// Short user-facing name for picker and status surfaces.
    #[must_use]
    #[allow(
        dead_code,
        reason = "3b-ii wires the access-route dispatch that consumes this"
    )]
    pub fn label(&self) -> &'static str {
        match self {
            AccessMethod::OwnedOAuth(OAuthProvider::Xai) => "xAI subscription",
            AccessMethod::OwnedOAuth(OAuthProvider::Chatgpt) => "ChatGPT subscription",
            AccessMethod::ExternalImport(ExternalImportSource::GrokCli) => "Grok CLI import",
            AccessMethod::ExternalImport(ExternalImportSource::CodexCli) => "Codex CLI import",
            AccessMethod::ExternalImport(ExternalImportSource::Antigravity) => "Antigravity import",
            AccessMethod::ExternalImport(ExternalImportSource::Dsh) => "DSH import",
            AccessMethod::ApiKey => "API key",
            AccessMethod::AcpBridge => "ACP bridge",
        }
    }
}

/// Another CLI whose credentials can be imported read-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code, reason = "3b-ii wires the import table that consumes this")]
pub enum ExternalImportSource {
    /// `~/.grok/auth.json` / `XAI_AUTH_PATH`, keyed by issuer::client-id.
    GrokCli,
    /// `~/.codex/auth.json`, `{tokens:{access_token, account_id}}`.
    CodexCli,
    /// Antigravity `state.vscdb` SQLite row, opened read-only.
    Antigravity,
    /// `$DSH_HOME/.credentials.yaml` flat mapping, `DEEPSEEK_API_KEY`.
    Dsh,
}

/// Test/dev knobs a provider reads from the environment. Data, so a new
/// provider adds rows here instead of a new module.
pub struct OAuthEnvOverrides {
    pub issuer_vars: &'static [&'static str],
    pub client_id_vars: &'static [&'static str],
    pub scope_vars: &'static [&'static str],
    pub no_browser_var: &'static str,
}

/// Everything about one provider's OAuth login that is not logic.
pub struct OAuthProviderParams {
    /// Human name for prompts and errors: "xAI", "ChatGPT".
    pub display_name: &'static str,
    pub default_issuer: &'static str,
    pub default_client_id: &'static str,
    pub default_scopes: &'static str,
    pub env: OAuthEnvOverrides,
    /// `Some` device-authorization path under the issuer (xAI); `None`
    /// means the issuer offers no device flow and device login must fail
    /// loudly instead of guessing (ChatGPT).
    pub device_code_path: Option<&'static str>,
    /// Token path under the issuer.
    pub token_path: &'static str,
    /// Whether the issuer was discovered (xAI) or pinned (ChatGPT paths).
    pub discover_endpoints: bool,
    /// Seconds the device-code poll runs past the server's `expires_in`.
    pub device_poll_floor_secs: u64,
}

pub const XAI_OAUTH_PARAMS: OAuthProviderParams = OAuthProviderParams {
    display_name: "xAI",
    // Single source: the legacy module still owns these strings until its
    // activation path unifies and they move here in 3b-iii.
    default_issuer: crate::xai_oauth::XAI_OIDC_ISSUER,
    default_client_id: crate::xai_oauth::GROK_OIDC_CLIENT_ID,
    default_scopes: crate::xai_oauth::DEFAULT_SCOPES,
    env: OAuthEnvOverrides {
        issuer_vars: &["GROK_OIDC_ISSUER", "XAI_OIDC_ISSUER"],
        client_id_vars: &["GROK_OIDC_CLIENT_ID", "XAI_OIDC_CLIENT_ID"],
        scope_vars: &["GROK_OIDC_SCOPES", "XAI_OIDC_SCOPES"],
        no_browser_var: "CODEWHALE_XAI_OAUTH_NO_BROWSER",
    },
    device_code_path: Some("oauth2/device/code"),
    token_path: "oauth2/token",
    discover_endpoints: true,
    device_poll_floor_secs: 30,
};

pub const CHATGPT_OAUTH_PARAMS: OAuthProviderParams = OAuthProviderParams {
    display_name: "ChatGPT",
    // Single source: same arrangement as the xAI row above.
    default_issuer: crate::chatgpt_oauth::CHATGPT_OAUTH_ISSUER,
    default_client_id: crate::chatgpt_oauth::CHATGPT_OAUTH_CLIENT_ID,
    default_scopes: crate::chatgpt_oauth::CHATGPT_OAUTH_SCOPE,
    env: OAuthEnvOverrides {
        issuer_vars: &["CODEWHALE_CHATGPT_OAUTH_ISSUER"],
        client_id_vars: &["CODEWHALE_CHATGPT_OAUTH_CLIENT_ID"],
        scope_vars: &[],
        no_browser_var: "CODEWHALE_CHATGPT_OAUTH_NO_BROWSER",
    },
    device_code_path: None,
    token_path: "oauth/token",
    discover_endpoints: false,
    device_poll_floor_secs: 30,
};

/// The parameter table. A provider login looks its row up here; adding a
/// provider means adding a row, never a module.
#[must_use]
pub fn oauth_provider_params(provider: OAuthProvider) -> &'static OAuthProviderParams {
    match provider {
        OAuthProvider::Xai => &XAI_OAUTH_PARAMS,
        OAuthProvider::Chatgpt => &CHATGPT_OAUTH_PARAMS,
    }
}

/// Resolved login inputs: schema defaults, environment-tested in order.
pub struct ResolvedOAuthInputs {
    pub issuer: String,
    pub client_id: String,
    pub scopes: String,
    pub open_browser: bool,
}

impl OAuthProviderParams {
    /// Resolve issuer/client/scopes from the environment, first var wins.
    #[must_use]
    pub fn resolve_inputs(&self) -> ResolvedOAuthInputs {
        let first_set = |vars: &[&str], fallback: &str| {
            vars.iter()
                .filter_map(|var| std::env::var(var).ok())
                .find(|value| !value.trim().is_empty())
                .unwrap_or_else(|| fallback.to_string())
        };
        ResolvedOAuthInputs {
            issuer: first_set(self.env.issuer_vars, self.default_issuer),
            client_id: first_set(self.env.client_id_vars, self.default_client_id),
            scopes: first_set(self.env.scope_vars, self.default_scopes),
            open_browser: std::env::var_os(self.env.no_browser_var).is_none(),
        }
    }
}

/// Token material from a completed grant. No Debug: the tokens never print.
/// `interval` rides along because a `slow_down` error response may carry
/// the server's new minimum, which the loop prefers over its own tracked
/// value (RFC 8628 §3.5; WSL/VM clock drift).
#[derive(Clone, Deserialize)]
pub struct OAuthTokenMaterial {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    #[serde(default)]
    interval: Option<u64>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

/// A completed login awaiting activation (owned-generation commit).
/// The provider is the table row it resolved through; unified activation
/// (3b-ii) matches on it.
pub struct PendingOAuthLogin {
    #[allow(dead_code, reason = "3b-ii unified activation matches on this")]
    pub provider: OAuthProvider,
    pub issuer: String,
    pub client_id: String,
    pub token: OAuthTokenMaterial,
}

/// Device-authorization response. Error fields ride along so a refused
/// grant classifies instead of failing to parse.
#[derive(Clone, Deserialize)]
struct DeviceGrantResponse {
    device_code: Option<String>,
    user_code: Option<String>,
    verification_uri: Option<String>,
    verification_uri_complete: Option<String>,
    expires_in: Option<u64>,
    interval: Option<u64>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

/// Bounds copied with the behavior: 20 s requests, 64 KiB bodies, 256 B of
/// error detail. A server that will not fit in the budget is an error, and
/// error text is whitespace-collapsed so a hostile endpoint cannot smuggle
/// terminal controls into diagnostics.
const OAUTH_REQUEST_TIMEOUT_SECS: u64 = 20;
const OAUTH_RESPONSE_BODY_LIMIT: u64 = 64 * 1024;
const OAUTH_ERROR_DETAIL_LIMIT: usize = 256;

fn oauth_http_client(purpose: &str) -> Result<reqwest::blocking::Client> {
    crate::tls::reqwest_blocking_client_builder()
        .timeout(Duration::from_secs(OAUTH_REQUEST_TIMEOUT_SECS))
        .build()
        .with_context(|| format!("Failed to build OAuth {purpose} client"))
}

fn parse_oauth_json<T: serde::de::DeserializeOwned>(
    response: reqwest::blocking::Response,
    operation: &str,
) -> Result<(reqwest::StatusCode, T)> {
    let status = response.status();
    // Join every content-type value: some test doubles stack a second one
    // next to the body's implicit type, and the diagnostic must name what
    // the server actually sent, not whichever header won the map lookup.
    let content_type = {
        let joined = response
            .headers()
            .get_all(reqwest::header::CONTENT_TYPE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect::<Vec<_>>()
            .join(", ");
        if joined.is_empty() {
            "missing".to_string()
        } else {
            joined
        }
    };
    let mut reader = response.take(OAUTH_RESPONSE_BODY_LIMIT + 1);
    let mut body = Vec::new();
    reader
        .read_to_end(&mut body)
        .with_context(|| format!("reading {operation} response"))?;
    let truncated = body.len() as u64 > OAUTH_RESPONSE_BODY_LIMIT;
    if truncated {
        body.truncate(OAUTH_RESPONSE_BODY_LIMIT as usize);
    }
    let parsed = serde_json::from_slice(&body).map_err(|_| {
        let limit = if truncated {
            " (body exceeded the 64 KiB diagnostic limit)"
        } else {
            ""
        };
        anyhow::anyhow!(
            "{operation} returned HTTP {status} with content type {content_type}; expected JSON{limit}"
        )
    })?;
    Ok((status, parsed))
}

fn bounded_oauth_error_text(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len().min(OAUTH_ERROR_DETAIL_LIMIT));
    let mut previous_was_space = false;
    let mut written = 0;
    for character in raw.chars() {
        let character = if character.is_whitespace() {
            ' '
        } else if character.is_control() {
            continue;
        } else {
            character
        };
        if character == ' ' && previous_was_space {
            continue;
        }
        if written == OAUTH_ERROR_DETAIL_LIMIT {
            break;
        }
        output.push(character);
        previous_was_space = character == ' ';
        written += 1;
    }
    output.trim().to_string()
}

fn oauth_failure_detail(
    error: Option<&str>,
    description: Option<&str>,
    status: reqwest::StatusCode,
) -> String {
    let mut code = bounded_oauth_error_text(error.unwrap_or("request_failed"));
    if code.is_empty() {
        code = "request_failed".to_string();
    }
    let description = description
        .map(bounded_oauth_error_text)
        .filter(|description| !description.is_empty() && description != &code);
    match description {
        Some(description) => format!("{code}: {description}; HTTP {status}"),
        None => format!("{code}; HTTP {status}"),
    }
}

/// Resolved token + device endpoints for one login.
struct OAuthEndpoints {
    device_authorization_endpoint: Option<String>,
    token_endpoint: String,
}

/// OIDC discovery document. Only the fields a login needs.
#[derive(Clone, Deserialize)]
struct OidcDiscoveryDocument {
    issuer: Option<String>,
    device_authorization_endpoint: Option<String>,
    token_endpoint: Option<String>,
}

/// Resolve endpoints: OIDC discovery when the provider row asks for it,
/// documented-path fallback otherwise — and fallback on ANY discovery
/// failure, loudly logged. A hostile or broken discovery document must
/// never brick login; it only loses the custom endpoints.
fn resolve_oauth_endpoints(params: &OAuthProviderParams, issuer: &str) -> OAuthEndpoints {
    let fallback = || OAuthEndpoints {
        device_authorization_endpoint: params
            .device_code_path
            .map(|path| format!("{}/{}", issuer.trim_end_matches('/'), path)),
        token_endpoint: format!("{}/{}", issuer.trim_end_matches('/'), params.token_path),
    };
    if !params.discover_endpoints {
        return fallback();
    }
    match discover_oauth_endpoints(params, issuer) {
        Ok(endpoints) => endpoints,
        Err(err) => {
            tracing::warn!(
                target: "codewhale::oauth",
                error = %err,
                "{} OIDC discovery failed; using documented endpoint fallback",
                params.display_name
            );
            fallback()
        }
    }
}

fn discover_oauth_endpoints(params: &OAuthProviderParams, issuer: &str) -> Result<OAuthEndpoints> {
    let name = params.display_name;
    let discovery_url = format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    );
    let client = oauth_http_client("OIDC discovery")?;
    let response = client
        .get(&discovery_url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .with_context(|| format!("{name} OIDC discovery request failed"))?;
    let (status, discovery): (_, OidcDiscoveryDocument) =
        parse_oauth_json(response, &format!("{name} OIDC discovery"))?;
    if !status.is_success() {
        bail!("{name} OIDC discovery failed with HTTP {status}");
    }
    let discovered_issuer = discovery
        .issuer
        .as_deref()
        .map(str::trim)
        .filter(|issuer| !issuer.is_empty())
        .with_context(|| format!("{name} OIDC discovery missing issuer"))?;
    if discovered_issuer.trim_end_matches('/') != issuer.trim_end_matches('/') {
        bail!("{name} OIDC discovery issuer does not match the requested issuer");
    }
    let expected_issuer = reqwest::Url::parse(issuer)
        .with_context(|| format!("{name} OIDC issuer is not a valid URL"))?;
    let validate_endpoint = |endpoint: Option<String>, field: &str| -> Result<String> {
        let endpoint = endpoint
            .as_deref()
            .map(str::trim)
            .filter(|endpoint| !endpoint.is_empty())
            .with_context(|| format!("{name} OIDC discovery missing {field}"))?;
        let parsed = reqwest::Url::parse(endpoint)
            .with_context(|| format!("{name} OIDC discovery returned an invalid {field}"))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            bail!("{name} OIDC discovery returned unsupported {field} scheme");
        }
        if expected_issuer.scheme() == "https" && parsed.scheme() != "https" {
            bail!("{name} OIDC discovery attempted to downgrade {field} from HTTPS");
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            bail!("{name} OIDC discovery returned credentials in {field}");
        }
        if parsed.origin() != expected_issuer.origin() {
            bail!("{name} OIDC discovery returned {field} outside the issuer origin");
        }
        Ok(endpoint.to_string())
    };
    Ok(OAuthEndpoints {
        device_authorization_endpoint: params
            .device_code_path
            .map(|_| {
                validate_endpoint(
                    discovery.device_authorization_endpoint,
                    "device_authorization_endpoint",
                )
            })
            .transpose()?,
        token_endpoint: validate_endpoint(discovery.token_endpoint, "token_endpoint")?,
    })
}

/// POST a device-authorization request. Pure transport over an explicit
/// endpoint: discovery (or its absence) is the caller's decision.
fn request_device_grant(
    device_authorization_endpoint: &str,
    client_id: &str,
    scopes: &str,
) -> Result<DeviceGrantResponse> {
    let client = oauth_http_client("device-code")?;
    let params = [("client_id", client_id), ("scope", scopes)];
    let response = client
        .post(device_authorization_endpoint)
        .form(&params)
        .send()
        .context("OAuth device-code request failed")?;
    let (status, body): (_, DeviceGrantResponse) =
        parse_oauth_json(response, "OAuth device-code request")?;
    if !status.is_success() || body.error.is_some() {
        let detail = oauth_failure_detail(
            body.error.as_deref(),
            body.error_description.as_deref(),
            status,
        );
        bail!("OAuth device-code request failed ({detail})");
    }
    if body
        .device_code
        .as_deref()
        .is_some_and(|code| !code.trim().is_empty())
        && body
            .user_code
            .as_deref()
            .is_some_and(|code| !code.trim().is_empty())
    {
        return Ok(body);
    }
    bail!("OAuth device-code request returned success without a device and user code");
}

/// Poll the token endpoint once, classifying the RFC 8628 outcome. Matches
/// the legacy per-provider poll so the ported tests pin identical behavior.
fn poll_device_grant(
    token_endpoint: &str,
    client_id: &str,
    device_code: &str,
) -> Result<codewhale_config::device_code::DevicePollOutcome<OAuthTokenMaterial>> {
    use codewhale_config::device_code::DevicePollOutcome;
    let client = oauth_http_client("device-code poll")?;
    let params = [
        ("client_id", client_id),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", device_code),
    ];
    let response = client
        .post(token_endpoint)
        .form(&params)
        .send()
        .context("OAuth device-code poll failed")?;
    let (status, body): (_, OAuthTokenMaterial) =
        parse_oauth_json(response, "OAuth device-code poll")?;
    if status.is_success() && body.error.is_none() {
        return Ok(DevicePollOutcome::Complete(body));
    }
    match body.error.as_deref().unwrap_or("") {
        "authorization_pending" => Ok(DevicePollOutcome::Pending),
        "slow_down" => Ok(DevicePollOutcome::SlowDown {
            interval_seconds: body.interval,
        }),
        _ => {
            let detail = oauth_failure_detail(
                body.error.as_deref(),
                body.error_description.as_deref(),
                status,
            );
            bail!("OAuth device-code poll failed ({detail})");
        }
    }
}

/// Interactive device-code login for any provider whose row offers it.
/// Prints the verification URL + user code to stderr and polls until
/// approved. A provider with no device flow (ChatGPT) fails here with the
/// reason, instead of deep in transport code.
pub async fn device_code_login(provider: OAuthProvider) -> Result<PendingOAuthLogin> {
    // Endpoint resolution does blocking HTTP (discovery): it must run on the
    // blocking worker, never on the async executor. Providers with no device
    // flow fail here, before any thread spawns and before any network.
    let params = oauth_provider_params(provider);
    if params.device_code_path.is_none() {
        bail!(
            "{} offers no device-code flow; sign in through the browser login instead",
            params.display_name
        );
    }
    let inputs = params.resolve_inputs();
    let display_name = params.display_name;
    tokio::task::spawn_blocking(move || device_code_login_with(provider, &inputs))
        .await
        .with_context(|| format!("{display_name} device-code login worker failed"))?
}

/// Blocking worker body for [`device_code_login`]. `pub(crate)` so the
/// legacy activation tests can drive the unified login end to end until
/// activation unifies in 3b-ii.
pub(crate) fn device_code_login_with(
    provider: OAuthProvider,
    inputs: &ResolvedOAuthInputs,
) -> Result<PendingOAuthLogin> {
    let params = oauth_provider_params(provider);
    let display_name = params.display_name;
    let endpoints = resolve_oauth_endpoints(params, &inputs.issuer);
    let Some(device_endpoint) = endpoints.device_authorization_endpoint else {
        bail!(
            "{display_name} offers no device-code flow; sign in through the browser login instead"
        );
    };
    let token_endpoint = endpoints.token_endpoint;
    let poll_floor_secs = params.device_poll_floor_secs;
    let grant = request_device_grant(&device_endpoint, &inputs.client_id, &inputs.scopes)?;
    let verify = grant
        .verification_uri_complete
        .clone()
        .or(grant.verification_uri.clone())
        .unwrap_or_else(|| format!("{}/device", inputs.issuer.trim_end_matches('/')));
    // Off the wire, headed for `webbrowser::open`: must be a bare
    // navigation, never a scheme or credential smuggle.
    let verify = codewhale_config::device_code::validate_browser_verification_uri(
        &verify,
        &format!("{display_name} device-code request"),
    )?;
    let user_code = grant.user_code.unwrap_or_default();

    eprintln!("{display_name} device-code login");
    eprintln!("  Open:  {verify}");
    eprintln!("  Code:  {user_code}");
    eprintln!("Waiting for approval in the browser… (Ctrl+C to abort)");
    if inputs.open_browser
        && let Err(err) = webbrowser::open(&verify)
    {
        eprintln!("Could not open the browser automatically: {err}");
    }

    let lifetime = Duration::from_secs(
        grant
            .expires_in
            .unwrap_or(crate::xai_oauth::DEVICE_POLL_MAX_SECS)
            .max(poll_floor_secs),
    );
    let token = codewhale_config::device_code::DeviceCodePoll::new(
        lifetime,
        format!(
            "{display_name} device-code authorization timed out. Re-run device login \
             and approve the code before it expires."
        ),
    )
    .interval_seconds(grant.interval)
    .wait_before_first_poll(true)
    .slow_down_timeout_message(format!(
        "{display_name} device-code authorization timed out after one or more slow_down \
         responses. That is usually clock drift in a WSL or VM environment; \
         sync the clock, then re-run device login and approve the code before \
         it expires."
    ))
    .run(std::thread::sleep, || {
        poll_device_grant(
            &token_endpoint,
            &inputs.client_id,
            grant.device_code.as_deref().unwrap_or(""),
        )
    })?;

    Ok(PendingOAuthLogin {
        provider,
        issuer: inputs.issuer.clone(),
        client_id: inputs.client_id.clone(),
        token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(path: &std::path::Path) -> ExternalCredentialReadGrant {
        codewhale_config::ExternalCredentialConsentToml::read_only(
            codewhale_config::ProviderKind::OpenaiCodex,
            codewhale_config::ExternalCredentialSource::CodexCli,
            path.to_path_buf(),
        )
        .read_grant(
            codewhale_config::ProviderKind::OpenaiCodex,
            codewhale_config::ExternalCredentialSource::CodexCli,
            path,
        )
        .expect("test read grant")
    }

    #[test]
    fn jwt_expiry_parses_valid_token() {
        // A minimal JWT with {"exp": 9999999999} as payload.
        let payload = URL_SAFE_NO_PAD.encode(b"{\"exp\":9999999999}");
        let token = format!("header.{payload}.signature");
        assert_eq!(jwt_expiry_seconds(&token), Some(9999999999));
    }

    #[test]
    fn jwt_expiry_returns_none_for_malformed() {
        assert_eq!(jwt_expiry_seconds("not.a.jwt"), None);
        assert_eq!(jwt_expiry_seconds(""), None);
        assert_eq!(jwt_expiry_seconds("x"), None);
    }

    #[test]
    fn token_is_expired_detects_future() {
        // Far future — should not be expired.
        let payload = URL_SAFE_NO_PAD.encode(b"{\"exp\":9999999999}");
        let token = format!("header.{payload}.sig");
        assert!(!token_is_expired(&token));
    }

    #[test]
    fn token_is_expired_detects_past() {
        // Way in the past.
        let payload = URL_SAFE_NO_PAD.encode(b"{\"exp\":1000000000}");
        let token = format!("header.{payload}.sig");
        assert!(token_is_expired(&token));
    }

    #[test]
    fn credential_presence_rejects_empty_and_malformed_files_without_refresh() {
        let _lock = crate::test_support::lock_test_env();
        let home = tempfile::tempdir().expect("temp Codex home");
        let auth_path = home
            .path()
            .canonicalize()
            .expect("canonical temp root")
            .join("auth.json");
        let _auth = crate::test_support::EnvVarGuard::set("OPENAI_CODEX_AUTH_FILE", &auth_path);
        let _access = crate::test_support::EnvVarGuard::remove("OPENAI_CODEX_ACCESS_TOKEN");
        let _legacy_access = crate::test_support::EnvVarGuard::remove("CODEX_ACCESS_TOKEN");
        let grant = grant(&auth_path);

        std::fs::write(&auth_path, "{}").expect("empty auth");
        crate::external_credentials::reset_side_effect_trap();
        assert!(!stored_credentials_present(&grant));
        assert_eq!(
            crate::external_credentials::side_effect_trap_counts(),
            (1, 1)
        );
        assert_eq!(
            crate::external_credentials::complete_side_effect_trap_counts(),
            (1, 1, 0, 0, 0)
        );
        std::fs::write(&auth_path, "{not-json").expect("malformed auth");
        crate::external_credentials::reset_side_effect_trap();
        assert!(!stored_credentials_present(&grant));
        assert_eq!(
            crate::external_credentials::side_effect_trap_counts(),
            (1, 1)
        );

        let payload = URL_SAFE_NO_PAD.encode(b"{\"exp\":9999999999}");
        let access_token = format!("header.{payload}.signature");
        std::fs::write(
            &auth_path,
            serde_json::to_vec(&serde_json::json!({
                "tokens": {"access_token": access_token}
            }))
            .expect("valid auth json"),
        )
        .expect("valid auth");
        crate::external_credentials::reset_side_effect_trap();
        assert!(stored_credentials_present(&grant));
        assert_eq!(
            crate::external_credentials::side_effect_trap_counts(),
            (1, 1)
        );
    }

    #[test]
    fn expired_external_token_fails_without_refresh_or_rewrite() {
        let _lock = crate::test_support::lock_test_env();
        let home = tempfile::tempdir().expect("temp Codex home");
        let auth_path = home
            .path()
            .canonicalize()
            .expect("canonical temp root")
            .join("auth.json");
        let payload = URL_SAFE_NO_PAD.encode(b"{\"exp\":1000000000}");
        let access_token = format!("header.{payload}.signature");
        let raw = serde_json::to_string_pretty(&serde_json::json!({
            "tokens": {
                "access_token": access_token,
                "refresh_token": "must-never-be-used",
                "account_id": "acct-test",
                "future_field": {"preserve": true}
            },
            "future_top_level": [1, 2, 3]
        }))
        .expect("auth fixture");
        std::fs::write(&auth_path, &raw).expect("expired auth fixture");

        crate::external_credentials::reset_side_effect_trap();
        let error = get_credentials(&grant(&auth_path))
            .expect_err("read-only external tokens must not refresh");
        assert!(error.to_string().contains("never refreshes or rewrites"));
        assert_eq!(
            crate::external_credentials::side_effect_trap_counts(),
            (1, 1)
        );
        assert_eq!(
            std::fs::read_to_string(&auth_path).expect("unchanged auth file"),
            raw
        );
    }

    #[test]
    fn auth_file_path_respects_env() {
        // Just verify it returns a path without panicking.
        let path = auth_file_path();
        assert!(path.to_string_lossy().contains("auth.json"));
    }

    #[test]
    fn missing_auth_message_explains_disabled_default_and_explicit_consent() {
        let _lock = crate::test_support::lock_test_env();
        let message = missing_auth_message();

        assert!(message.contains("OpenAI Codex OAuth credentials are unavailable"));
        assert!(message.contains("OPENAI_CODEX_ACCESS_TOKEN"));
        assert!(message.contains("CODEX_ACCESS_TOKEN"));
        assert!(message.contains(&codewhale_config::quote_os_path(&auth_file_path())));
        assert!(message.contains("codewhale auth chatgpt"));
        assert!(message.contains("subscription billing"));
        assert!(message.contains("openai API-key"));
        assert!(message.contains("codex login"));
        assert!(message.contains("external-consent"));
        assert!(message.contains("chatgpt-revoke"));
    }

    #[test]
    fn provider_table_separates_device_flow_from_browser_flow() {
        let xai = oauth_provider_params(OAuthProvider::Xai);
        assert_eq!(xai.device_code_path, Some("oauth2/device/code"));
        assert!(xai.discover_endpoints);
        let chatgpt = oauth_provider_params(OAuthProvider::Chatgpt);
        assert_eq!(chatgpt.device_code_path, None);
        assert!(!chatgpt.discover_endpoints);
        assert_ne!(xai.default_client_id, chatgpt.default_client_id);
    }

    #[test]
    fn provider_inputs_resolve_from_defaults_without_env() {
        let _lock = crate::test_support::lock_test_env();
        let _guards: Vec<_> = [
            "GROK_OIDC_ISSUER",
            "XAI_OIDC_ISSUER",
            "GROK_OIDC_CLIENT_ID",
            "XAI_OIDC_CLIENT_ID",
            "GROK_OIDC_SCOPES",
            "XAI_OIDC_SCOPES",
            "CODEWHALE_XAI_OAUTH_NO_BROWSER",
        ]
        .into_iter()
        .map(crate::test_support::EnvVarGuard::remove)
        .collect();
        let inputs = XAI_OAUTH_PARAMS.resolve_inputs();
        assert_eq!(inputs.issuer, "https://auth.x.ai");
        assert!(inputs.scopes.contains("grok-cli:access"));
        assert!(inputs.open_browser);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn request_device_grant_round_trips_a_mock_grant() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth2/device/code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "device-123",
                "user_code": "USER-456",
                "verification_uri": "https://example.com/device",
                "expires_in": 900
            })))
            .expect(1)
            .mount(&server)
            .await;
        let grant = tokio::task::block_in_place(|| {
            request_device_grant(
                &format!("{}/oauth2/device/code", server.uri()),
                "test-client",
                "openid",
            )
        })
        .expect("mock grant");
        assert_eq!(grant.device_code.as_deref(), Some("device-123"));
        assert_eq!(grant.user_code.as_deref(), Some("USER-456"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn request_device_grant_rejects_success_without_codes() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth2/device/code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .expect(1)
            .mount(&server)
            .await;
        let result = tokio::task::block_in_place(|| {
            request_device_grant(
                &format!("{}/oauth2/device/code", server.uri()),
                "test-client",
                "openid",
            )
        });
        let Err(error) = result else {
            panic!("a grant without codes must fail");
        };
        assert!(error.to_string().contains("without a device and user code"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn poll_device_grant_classifies_rfc8628_states() {
        use codewhale_config::device_code::DevicePollOutcome;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        async fn outcome(
            body: serde_json::Value,
            status: u16,
        ) -> Result<DevicePollOutcome<OAuthTokenMaterial>> {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/oauth2/token"))
                .respond_with(ResponseTemplate::new(status).set_body_json(body))
                .expect(1)
                .mount(&server)
                .await;
            tokio::task::block_in_place(|| {
                poll_device_grant(
                    &format!("{}/oauth2/token", server.uri()),
                    "test-client",
                    "device-token",
                )
            })
        }
        assert!(matches!(
            outcome(serde_json::json!({ "error": "authorization_pending" }), 400).await,
            Ok(DevicePollOutcome::Pending)
        ));
        assert!(matches!(
            outcome(
                serde_json::json!({ "error": "slow_down", "interval": 12 }),
                400
            )
            .await,
            Ok(DevicePollOutcome::SlowDown {
                interval_seconds: Some(12)
            })
        ));
        for error in ["access_denied", "expired_token"] {
            let result = outcome(serde_json::json!({ "error": error }), 400).await;
            let Err(failure) = result else {
                panic!("{error} must stop polling");
            };
            assert!(failure.to_string().contains(error), "{failure}");
        }
        let result = outcome(
            serde_json::json!({ "access_token": "at", "expires_in": 3600 }),
            200,
        )
        .await;
        let Ok(DevicePollOutcome::Complete(material)) = result else {
            panic!("success must complete");
        };
        assert_eq!(material.access_token.as_deref(), Some("at"));
    }

    #[tokio::test]
    async fn device_login_without_a_device_flow_fails_before_network() {
        let result = device_code_login(OAuthProvider::Chatgpt).await;
        let Err(error) = result else {
            panic!("ChatGPT has no device flow");
        };
        assert!(
            error.to_string().contains("no device-code flow"),
            "{error:#}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn discovery_honors_advertised_endpoints() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issuer": server.uri(),
                "device_authorization_endpoint": format!("{}/custom/device", server.uri()),
                "token_endpoint": format!("{}/custom/token", server.uri()),
            })))
            .expect(1)
            .mount(&server)
            .await;
        let endpoints = tokio::task::block_in_place(|| {
            resolve_oauth_endpoints(&XAI_OAUTH_PARAMS, &server.uri())
        });
        assert_eq!(
            endpoints.device_authorization_endpoint.as_deref(),
            Some(format!("{}/custom/device", server.uri()).as_str())
        );
        assert_eq!(
            endpoints.token_endpoint,
            format!("{}/custom/token", server.uri())
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn discovery_issuer_mismatch_falls_back_to_documented_paths() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issuer": "https://someone-else.example",
                "device_authorization_endpoint": "https://someone-else.example/device",
                "token_endpoint": "https://someone-else.example/token",
            })))
            .expect(1)
            .mount(&server)
            .await;
        let endpoints = tokio::task::block_in_place(|| {
            resolve_oauth_endpoints(&XAI_OAUTH_PARAMS, &server.uri())
        });
        assert_eq!(
            endpoints.device_authorization_endpoint.as_deref(),
            Some(format!("{}/oauth2/device/code", server.uri()).as_str())
        );
        assert_eq!(
            endpoints.token_endpoint,
            format!("{}/oauth2/token", server.uri())
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn device_login_aborts_on_untrusted_verification_uri() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issuer": server.uri(),
                "device_authorization_endpoint": format!("{}/oauth2/device-advertised", server.uri()),
                "token_endpoint": format!("{}/oauth2/token", server.uri()),
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth2/device-advertised"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "device-token",
                "user_code": "CW-TEST",
                "verification_uri": "https://auth.x.ai/device",
                "verification_uri_complete": "vscode://attacker/run?code=CW-TEST",
                "expires_in": 60,
                "interval": 1
            })))
            .expect(1)
            .mount(&server)
            .await;
        // No token-endpoint mock: the flow must fail before it ever polls.
        let inputs = ResolvedOAuthInputs {
            issuer: server.uri(),
            client_id: "test-client".to_string(),
            scopes: "openid".to_string(),
            open_browser: false,
        };
        let result =
            tokio::task::block_in_place(|| device_code_login_with(OAuthProvider::Xai, &inputs));
        let Err(error) = result else {
            panic!("a non-web verification URI must abort login");
        };
        assert!(
            format!("{error:#}").contains("untrusted verification URI"),
            "{error:#}"
        );
    }

    /// Discovery + device grant run on the blocking worker: this fails with
    /// the grant refusal, never with a runtime-drop panic.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn device_login_runs_blocking_http_off_the_executor() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let _lock = crate::test_support::lock_test_env();
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issuer": server.uri(),
                "device_authorization_endpoint": format!("{}/oauth2/device-advertised", server.uri()),
                "token_endpoint": format!("{}/oauth2/token-advertised", server.uri())
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth2/device-advertised"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "invalid_scope",
                "error_description": "mock refusal before browser or polling"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let _issuer =
            crate::test_support::EnvVarGuard::set("GROK_OIDC_ISSUER", server.uri().as_str());
        let _no_browser =
            crate::test_support::EnvVarGuard::set("CODEWHALE_XAI_OAUTH_NO_BROWSER", "1");

        let result = device_code_login(OAuthProvider::Xai).await;
        let Err(error) = result else {
            panic!("mock device request must fail without a runtime-drop panic");
        };
        let message = format!("{error:#}");
        assert!(message.contains("invalid_scope"), "{message}");
        assert!(message.contains("HTTP 400"), "{message}");
    }

    /// Full orchestration against mocks: discovery, grant, one poll, done.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn device_login_exchanges_and_returns_token_material() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issuer": server.uri(),
                "device_authorization_endpoint": format!("{}/oauth2/device-advertised", server.uri()),
                "token_endpoint": format!("{}/oauth2/token-advertised", server.uri())
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth2/device-advertised"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "device-token",
                "user_code": "CW-TEST",
                "verification_uri": format!("{}/verify", server.uri()),
                "expires_in": 60,
                "interval": 1
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth2/token-advertised"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "unified-access",
                "refresh_token": "unified-refresh",
                "expires_in": 3600
            })))
            .expect(1)
            .mount(&server)
            .await;
        let inputs = ResolvedOAuthInputs {
            issuer: server.uri(),
            client_id: "test-client".to_string(),
            scopes: "openid".to_string(),
            open_browser: false,
        };
        let pending =
            tokio::task::block_in_place(|| device_code_login_with(OAuthProvider::Xai, &inputs))
                .expect("mock login exchanges");
        assert_eq!(pending.issuer, server.uri());
        assert_eq!(
            pending.token.access_token.as_deref(),
            Some("unified-access")
        );
        assert_eq!(
            pending.token.refresh_token.as_deref(),
            Some("unified-refresh")
        );
    }

    /// The shared poll loop walks pending and slow_down to completion.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn device_login_polls_through_pending_and_slow_down() {
        use codewhale_config::device_code::DeviceCodePoll;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        // wiremock matches mocks in mount order, so mount the one-shot
        // transient-error responses before the terminal success response:
        // poll 1 -> authorization_pending, poll 2 -> slow_down, poll 3 -> ok.
        for (body, status) in [
            (serde_json::json!({ "error": "authorization_pending" }), 400),
            (serde_json::json!({ "error": "slow_down" }), 400),
        ] {
            Mock::given(method("POST"))
                .and(path("/oauth2/token"))
                .respond_with(ResponseTemplate::new(status).set_body_json(body))
                .up_to_n_times(1)
                .expect(1)
                .mount(&server)
                .await;
        }
        Mock::given(method("POST"))
            .and(path("/oauth2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "access_token": "loop-access", "expires_in": 3600 }),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let endpoint = format!("{}/oauth2/token", server.uri());
        let material = tokio::task::block_in_place(|| {
            DeviceCodePoll::new(
                std::time::Duration::from_secs(60),
                "mock poll must complete",
            )
            .run(
                |_| {},
                || poll_device_grant(&endpoint, "test-client", "device-token"),
            )
        })
        .expect("poll loop completes");
        assert!(matches!(
            material.access_token.as_deref(),
            Some("loop-access")
        ));
    }

    /// A denied grant stops the full login with the server's reason.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn device_login_surfaces_user_denial() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issuer": server.uri(),
                "device_authorization_endpoint": format!("{}/oauth2/device-advertised", server.uri()),
                "token_endpoint": format!("{}/oauth2/token-advertised", server.uri())
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth2/device-advertised"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "device-token",
                "user_code": "CW-TEST",
                "verification_uri": format!("{}/verify", server.uri()),
                "expires_in": 60,
                "interval": 1
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth2/token-advertised"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "access_denied",
                "error_description": "The user denied the authorization request"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let inputs = ResolvedOAuthInputs {
            issuer: server.uri(),
            client_id: "test-client".to_string(),
            scopes: "openid".to_string(),
            open_browser: false,
        };
        let result =
            tokio::task::block_in_place(|| device_code_login_with(OAuthProvider::Xai, &inputs));
        let Err(error) = result else {
            panic!("user denial must stop the login");
        };
        let message = format!("{error:#}");
        assert!(message.contains("access_denied"), "{message}");
        assert!(message.contains("HTTP 400"), "{message}");
    }

    /// Non-JSON answers name the content type, never the body.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn device_transport_reports_non_json_without_echoing_body() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        // set_body_bytes carries no implicit content type, so the inserted
        // text/html is the only one on the wire (set_body_string would
        // stack text/plain next to it and the diagnostic would name both).
        Mock::given(method("POST"))
            .and(path("/oauth2/device-code"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes("<html>sentinel-body-bytes</html>".as_bytes())
                    .insert_header("content-type", "text/html"),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth2/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes("<html>sentinel-body-bytes</html>".as_bytes())
                    .insert_header("content-type", "text/html"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let grant = tokio::task::block_in_place(|| {
            request_device_grant(
                &format!("{}/oauth2/device-code", server.uri()),
                "test-client",
                "openid",
            )
        });
        let Err(grant_error) = grant else {
            panic!("non-JSON grant must fail");
        };
        let poll = tokio::task::block_in_place(|| {
            poll_device_grant(
                &format!("{}/oauth2/token", server.uri()),
                "test-client",
                "device-token",
            )
        });
        let Err(poll_error) = poll else {
            panic!("non-JSON poll must fail");
        };
        for message in [format!("{grant_error:#}"), format!("{poll_error:#}")] {
            assert!(message.contains("text/html"), "{message}");
            assert!(!message.contains("sentinel-body-bytes"), "{message}");
        }
    }

    /// The wire format is a contract: form-encoded posts carrying the exact
    /// client, scope, and grant-type parameters the issuers expect.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn device_transport_posts_exact_oauth_form_parameters() {
        use wiremock::matchers::{body_string_contains, header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issuer": server.uri(),
                "device_authorization_endpoint": format!("{}/oauth2/device-advertised", server.uri()),
                "token_endpoint": format!("{}/oauth2/token-advertised", server.uri())
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth2/device-advertised"))
            .and(header("content-type", "application/x-www-form-urlencoded"))
            .and(body_string_contains("client_id=test-client"))
            .and(body_string_contains("scope=openid"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "device-token",
                "user_code": "CW-TEST",
                "verification_uri": format!("{}/verify", server.uri()),
                "expires_in": 60,
                "interval": 1
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth2/token-advertised"))
            .and(header("content-type", "application/x-www-form-urlencoded"))
            .and(body_string_contains(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code",
            ))
            .and(body_string_contains("client_id=test-client"))
            .and(body_string_contains("device_code=device-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "form-access",
                "expires_in": 3600
            })))
            .expect(1)
            .mount(&server)
            .await;
        let inputs = ResolvedOAuthInputs {
            issuer: server.uri(),
            client_id: "test-client".to_string(),
            scopes: "openid".to_string(),
            open_browser: false,
        };
        let pending =
            tokio::task::block_in_place(|| device_code_login_with(OAuthProvider::Xai, &inputs))
                .expect("mock login exchanges");
        assert_eq!(pending.token.access_token.as_deref(), Some("form-access"));
    }

    #[test]
    fn access_method_labels_name_every_route() {
        assert_eq!(
            AccessMethod::OwnedOAuth(OAuthProvider::Xai).label(),
            "xAI subscription"
        );
        assert_eq!(
            AccessMethod::ExternalImport(ExternalImportSource::CodexCli).label(),
            "Codex CLI import"
        );
        assert_eq!(AccessMethod::ApiKey.label(), "API key");
        assert_eq!(AccessMethod::AcpBridge.label(), "ACP bridge");
    }

    #[test]
    fn malformed_codex_credential_errors_never_echo_file_contents() {
        let _lock = crate::test_support::lock_test_env();
        let home = tempfile::tempdir().expect("temp Codex home");
        let path = home.path().canonicalize().unwrap().join("auth.json");
        let sentinel = "must-not-appear-in-diagnostics";
        std::fs::write(
            &path,
            format!(r#"{{"tokens":{{"access_token":{{"secret":"{sentinel}"}}}}}}"#),
        )
        .unwrap();

        let error = load_credentials(&grant(&path)).expect_err("malformed schema");
        let message = format!("{error:#}");
        assert!(message.contains("not valid credential JSON"), "{message}");
        assert!(!message.contains(sentinel), "{message}");
    }
}
