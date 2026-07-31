//! Codewhale account and BYOK credential commands.
//!
//! This module is deliberately separate from the provider-facing `login` and
//! `auth` commands in `lib.rs`: those configure the local runtime, while this
//! surface signs a CLI profile into the managed Codewhale account and stores
//! provider keys in that account's remote vault.

use std::collections::BTreeMap;
use std::io::{self, IsTerminal, Read, Write};
use std::net::IpAddr;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Subcommand, ValueEnum};
use codewhale_config::{ConfigStore, ProviderKind};
use codewhale_secrets::{DefaultKeyringStore, Secrets};
use reqwest::Url;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

const DEFAULT_API_BASE: &str = "https://api.codewhale.net";
const CLOUD_API_BASE_ENV: &str = "CODEWHALE_CLOUD_API_BASE";
const CLOUD_ALLOW_FILE_SESSION_STORE_ENV: &str = "CODEWHALE_CLOUD_ALLOW_FILE_SESSION_STORE";
const MAX_RESPONSE_BYTES: u64 = 256 * 1024;
const MIN_API_KEY_BYTES: usize = 8;
const MAX_API_KEY_BYTES: u64 = 4096;
const MAX_API_KEY_STDIN_BYTES: u64 = MAX_API_KEY_BYTES + 1024;
const MAX_KEY_LABEL_CHARS: usize = 80;
const MAX_TOKEN_BYTES: usize = 64 * 1024;
const DEFAULT_LOGIN_TIMEOUT_SECONDS: u64 = 600;
const MAX_LOGIN_TIMEOUT_SECONDS: u64 = 3600;

#[derive(Debug, Args)]
pub(crate) struct CloudArgs {
    /// Codewhale account API origin. HTTPS is required except for loopback HTTP.
    #[arg(long, global = true, value_name = "URL")]
    api_base: Option<String>,
    #[command(subcommand)]
    command: CloudCommand,
}

#[derive(Debug, Subcommand)]
enum CloudCommand {
    /// Sign this CLI profile in through the browser device flow.
    Login(CloudLoginArgs),
    /// Show the signed-in account for this CLI profile.
    Status,
    /// Remove this profile's local account session and revoke it when reachable.
    Logout,
    /// Manage provider API keys stored in the signed-in Codewhale account.
    Keys(CloudKeysArgs),
}

#[derive(Debug, Args)]
struct CloudLoginArgs {
    /// Print the verification URL without trying to open a browser.
    #[arg(long, default_value_t = false)]
    no_open: bool,
    /// Maximum time to wait for browser authorization.
    #[arg(
        long = "timeout-seconds",
        default_value_t = DEFAULT_LOGIN_TIMEOUT_SECONDS,
        value_parser = clap::value_parser!(u64).range(1..=MAX_LOGIN_TIMEOUT_SECONDS)
    )]
    timeout_seconds: u64,
}

#[derive(Debug, Args)]
struct CloudKeysArgs {
    #[command(subcommand)]
    command: CloudKeysCommand,
}

#[derive(Debug, Subcommand)]
enum CloudKeysCommand {
    /// List configured providers without revealing key values.
    List,
    /// Save a provider key to the signed-in Codewhale account.
    Set(CloudKeySetArgs),
    /// Remove a provider key from the signed-in Codewhale account.
    Remove { provider: CloudProvider },
}

#[derive(Debug, Args)]
struct CloudKeySetArgs {
    provider: CloudProvider,
    /// Read the key from stdin. Useful for pipes and secret-manager commands.
    #[arg(long = "api-key-stdin", conflicts_with = "from_local")]
    api_key_stdin: bool,
    /// Upload the locally resolved key (config, secret store, then environment).
    #[arg(long, conflicts_with = "api_key_stdin")]
    from_local: bool,
    /// Non-secret label shown beside the stored credential.
    #[arg(long, default_value = "Codewhale CLI")]
    label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CloudProvider {
    Deepseek,
    Anthropic,
    Openai,
    Openrouter,
    Zai,
    Moonshot,
    Xai,
    #[value(name = "xiaomi", alias = "xiaomi-mimo")]
    Xiaomi,
}

impl CloudProvider {
    const ALL: [Self; 8] = [
        Self::Deepseek,
        Self::Anthropic,
        Self::Openai,
        Self::Openrouter,
        Self::Zai,
        Self::Moonshot,
        Self::Xai,
        Self::Xiaomi,
    ];

    fn slug(self) -> &'static str {
        match self {
            Self::Deepseek => "deepseek",
            Self::Anthropic => "anthropic",
            Self::Openai => "openai",
            Self::Openrouter => "openrouter",
            Self::Zai => "zai",
            Self::Moonshot => "moonshot",
            Self::Xai => "xai",
            Self::Xiaomi => "xiaomi",
        }
    }

    fn local_kind(self) -> ProviderKind {
        match self {
            Self::Deepseek => ProviderKind::Deepseek,
            Self::Anthropic => ProviderKind::Anthropic,
            Self::Openai => ProviderKind::Openai,
            Self::Openrouter => ProviderKind::Openrouter,
            Self::Zai => ProviderKind::Zai,
            Self::Moonshot => ProviderKind::Moonshot,
            Self::Xai => ProviderKind::Xai,
            Self::Xiaomi => ProviderKind::XiaomiMimo,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

struct CloudRequest {
    method: HttpMethod,
    path: String,
    bearer: Option<String>,
    body: Option<Vec<u8>>,
}

struct CloudResponse {
    status: u16,
    body: Vec<u8>,
}

trait CloudTransport {
    fn execute(&self, request: CloudRequest) -> Result<CloudResponse>;
}

struct ReqwestTransport {
    base: Url,
    client: reqwest::blocking::Client,
}

impl ReqwestTransport {
    fn new(base: Url) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(30))
            // Never replay bearer tokens or provider-key request bodies to a
            // redirect target. The control-plane origin is an explicit trust
            // boundary, so redirects are treated as ordinary non-2xx replies.
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("codewhale/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("failed to initialize the Codewhale account HTTP client")?;
        Ok(Self { base, client })
    }
}

impl CloudTransport for ReqwestTransport {
    fn execute(&self, request: CloudRequest) -> Result<CloudResponse> {
        let url = self
            .base
            .join(request.path.trim_start_matches('/'))
            .context("failed to construct the Codewhale account request URL")?;
        let method = match request.method {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Delete => reqwest::Method::DELETE,
        };
        let mut builder = self
            .client
            .request(method, url)
            .header(reqwest::header::ACCEPT, "application/json");
        if let Some(token) = request.bearer {
            builder = builder.bearer_auth(token);
        }
        if let Some(body) = request.body {
            builder = builder
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body);
        }
        let response = builder
            .send()
            .context("could not reach the Codewhale service")?;
        let status = response.status().as_u16();
        let mut body = Vec::new();
        response
            .take(MAX_RESPONSE_BYTES + 1)
            .read_to_end(&mut body)
            .context("failed to read the Codewhale service response")?;
        if body.len() as u64 > MAX_RESPONSE_BYTES {
            bail!("The Codewhale service returned an unexpectedly large response");
        }
        Ok(CloudResponse { status, body })
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthBundle {
    token_type: String,
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    session: Option<AuthSession>,
    #[serde(default)]
    user: Option<CloudUser>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthSession {
    id: String,
    #[serde(default)]
    provider: String,
    #[serde(default)]
    expires_at: String,
    #[serde(default)]
    refresh_expires_at: String,
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CloudUser {
    #[serde(default)]
    id: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    region: String,
    #[serde(default)]
    plan: String,
    #[serde(default)]
    model_keys: BTreeMap<String, ModelKeyState>,
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelKeyState {
    #[serde(default)]
    configured: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceStart {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct MeResponse {
    user: CloudUser,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceTokenRequest<'a> {
    device_code: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RefreshRequest<'a> {
    refresh_token: &'a str,
}

#[derive(Serialize)]
struct ModelKeyRequest<'a> {
    key: &'a str,
    label: &'a str,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredCloudAuth {
    schema_version: u8,
    api_base: String,
    bundle: AuthBundle,
}

struct CloudClient<'a, T: CloudTransport> {
    transport: &'a T,
    secrets: &'a Secrets,
    auth_slot: String,
    api_base: &'a str,
}

impl<'a, T: CloudTransport> CloudClient<'a, T> {
    fn new(transport: &'a T, secrets: &'a Secrets, profile: &str, api_base: &'a str) -> Self {
        Self {
            transport,
            secrets,
            auth_slot: cloud_auth_slot(profile, api_base),
            api_base,
        }
    }

    fn start_device(&self) -> Result<DeviceStart> {
        let response = self.transport.execute(CloudRequest {
            method: HttpMethod::Post,
            path: "/api/cli/device/start".to_string(),
            bearer: None,
            body: Some(b"{}".to_vec()),
        })?;
        expect_json(response, &[200])
    }

    fn poll_device(
        &self,
        device: &DeviceStart,
        timeout: Duration,
        sleep: &mut dyn FnMut(Duration),
    ) -> Result<AuthBundle> {
        validate_device_code(&device.device_code)?;
        let server_lifetime =
            Duration::from_secs(device.expires_in.clamp(1, MAX_LOGIN_TIMEOUT_SECONDS));
        let timeout = timeout.min(server_lifetime);
        let interval = Duration::from_secs(device.interval.clamp(1, 10));
        let started = Instant::now();

        loop {
            if started.elapsed() >= timeout {
                bail!(
                    "Codewhale account login timed out; run `codewhale account login` to try again"
                );
            }
            let response = self.transport.execute(CloudRequest {
                method: HttpMethod::Post,
                path: "/api/cli/device/token".to_string(),
                bearer: None,
                body: Some(json_body(&DeviceTokenRequest {
                    device_code: &device.device_code,
                })?),
            })?;
            match response.status {
                200 => {
                    let bundle: AuthBundle = parse_json_body(&response.body)?;
                    validate_auth_bundle(&bundle)?;
                    self.save_auth(bundle.clone())?;
                    return Ok(bundle);
                }
                202 => {
                    let remaining = timeout.saturating_sub(started.elapsed());
                    if remaining.is_zero() {
                        bail!(
                            "Codewhale account login timed out; run `codewhale account login` to try again"
                        );
                    }
                    sleep(interval.min(remaining));
                }
                _ => return Err(response_error(&response)),
            }
        }
    }

    fn load_auth(&self) -> Result<Option<StoredCloudAuth>> {
        let Some(raw) = self
            .secrets
            .get(&self.auth_slot)
            .context("failed to read the local Codewhale account session")?
        else {
            return Ok(None);
        };
        let stored: StoredCloudAuth = serde_json::from_str(&raw)
            .context("the local Codewhale account session is unreadable; run `codewhale account logout` and sign in again")?;
        if stored.schema_version != 1 || stored.api_base != self.api_base {
            return Ok(None);
        }
        validate_auth_bundle(&stored.bundle)?;
        Ok(Some(stored))
    }

    fn save_auth(&self, bundle: AuthBundle) -> Result<()> {
        validate_auth_bundle(&bundle)?;
        let stored = StoredCloudAuth {
            schema_version: 1,
            api_base: self.api_base.to_string(),
            bundle,
        };
        let value = serde_json::to_string(&stored)
            .context("failed to encode the local Codewhale account session")?;
        self.secrets
            .set(&self.auth_slot, &value)
            .context("failed to save the Codewhale account session in the local secret store")
    }

    fn clear_auth(&self) -> Result<()> {
        self.secrets
            .delete(&self.auth_slot)
            .context("failed to remove the local Codewhale account session")
    }

    fn me(&self) -> Result<CloudUser> {
        let response = self.execute_authenticated(HttpMethod::Get, "/api/me", None)?;
        let me: MeResponse = expect_json(response, &[200])?;
        if me.user.id.trim().is_empty() {
            bail!("The Codewhale service returned an account without an ID");
        }
        if let Some(mut stored) = self.load_auth()? {
            stored.bundle.user = Some(me.user.clone());
            self.save_auth(stored.bundle)?;
        }
        Ok(me.user)
    }

    fn set_key(&self, provider: CloudProvider, key: &str, label: &str) -> Result<()> {
        let path = format!("/api/model-keys/{}", provider.slug());
        let response = self.execute_authenticated(
            HttpMethod::Put,
            &path,
            Some(json_body(&ModelKeyRequest { key, label })?),
        )?;
        expect_empty(response, &[200, 201])
    }

    fn remove_key(&self, provider: CloudProvider) -> Result<()> {
        let path = format!("/api/model-keys/{}", provider.slug());
        let response = self.execute_authenticated(HttpMethod::Delete, &path, None)?;
        expect_empty(response, &[200, 204])
    }

    fn logout(&self) -> Result<bool> {
        let stored = match self.load_auth() {
            Ok(Some(stored)) => stored,
            Ok(None) => return Ok(false),
            Err(_) => {
                // Logout is also the recovery path for a corrupt or obsolete
                // local record, so it must remain able to remove that record.
                self.clear_auth()?;
                return Ok(false);
            }
        };
        let body = json_body(&RefreshRequest {
            refresh_token: &stored.bundle.refresh_token,
        })?;
        let remote_revoked = self
            .transport
            .execute(CloudRequest {
                method: HttpMethod::Post,
                path: "/api/auth/logout".to_string(),
                bearer: None,
                body: Some(body),
            })
            .is_ok_and(|response| (200..300).contains(&response.status));
        self.clear_auth()?;
        Ok(remote_revoked)
    }

    fn execute_authenticated(
        &self,
        method: HttpMethod,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> Result<CloudResponse> {
        let Some(mut stored) = self.load_auth()? else {
            bail!("Not signed in. Run `codewhale account login` first");
        };
        let first = self.transport.execute(CloudRequest {
            method,
            path: path.to_string(),
            bearer: Some(stored.bundle.access_token.clone()),
            body: body.clone(),
        })?;
        if first.status != 401 {
            return Ok(first);
        }

        let refresh = self.transport.execute(CloudRequest {
            method: HttpMethod::Post,
            path: "/api/auth/refresh".to_string(),
            bearer: None,
            body: Some(json_body(&RefreshRequest {
                refresh_token: &stored.bundle.refresh_token,
            })?),
        })?;
        if refresh.status != 200 {
            self.clear_auth()?;
            bail!("The Codewhale account session expired. Run `codewhale account login` again");
        }
        let mut next: AuthBundle = parse_json_body(&refresh.body)?;
        validate_auth_bundle(&next)?;
        if next.user.is_none() {
            next.user = stored.bundle.user.take();
        }
        self.save_auth(next.clone())?;

        let retried = self.transport.execute(CloudRequest {
            method,
            path: path.to_string(),
            bearer: Some(next.access_token),
            body,
        })?;
        if retried.status == 401 {
            self.clear_auth()?;
            bail!("The Codewhale account session expired. Run `codewhale account login` again");
        }
        Ok(retried)
    }
}

enum KeyReadMode {
    Stdin,
    HiddenPrompt(String),
}

pub(crate) fn run(args: CloudArgs, profile: Option<&str>, config: &ConfigStore) -> Result<()> {
    let requested_base = args
        .api_base
        .or_else(|| std::env::var(CLOUD_API_BASE_ENV).ok())
        .unwrap_or_else(|| DEFAULT_API_BASE.to_string());
    let api_base = validate_api_base(&requested_base)?;
    let transport = ReqwestTransport::new(api_base.url.clone())?;
    // Account refresh tokens require an OS credential manager. The ordinary
    // provider backend remains independently configurable for `--from-local`.
    let cloud_secrets = cloud_session_secrets()?;
    let provider_secrets = Secrets::auto_detect();
    let profile = normalized_profile(profile);
    let mut stdout = io::stdout().lock();
    let mut key_reader = |mode: KeyReadMode| match mode {
        KeyReadMode::Stdin => read_key_from_stdin(),
        KeyReadMode::HiddenPrompt(provider) => read_key_hidden(&provider),
    };
    let mut opener = |url: String| webbrowser::open(&url).is_ok();
    let mut sleeper = |duration| thread::sleep(duration);
    run_with(
        args.command,
        &profile,
        &api_base.display,
        config,
        &cloud_secrets,
        &provider_secrets,
        &transport,
        &mut stdout,
        &mut key_reader,
        &mut opener,
        &mut sleeper,
    )
}

fn cloud_session_secrets() -> Result<Secrets> {
    let keyring = DefaultKeyringStore::new("codewhale-cloud");
    match keyring.probe() {
        Ok(()) => Ok(Secrets::new(Arc::new(keyring))),
        Err(_) if file_session_store_opted_in() => {
            eprintln!(
                "warning: OS credential manager unavailable; {CLOUD_ALLOW_FILE_SESSION_STORE_ENV}=1 explicitly enables the local 0600 Codewhale secrets file for cloud session tokens"
            );
            Ok(Secrets::file_backed())
        }
        Err(_) => bail!(
            "Codewhale account login requires an OS credential manager for session tokens. Configure Keychain, Credential Manager, or Secret Service and try again. Headless users may explicitly opt into the local 0600 secrets file with {CLOUD_ALLOW_FILE_SESSION_STORE_ENV}=1"
        ),
    }
}

fn file_session_store_opted_in() -> bool {
    let value = std::env::var(CLOUD_ALLOW_FILE_SESSION_STORE_ENV).ok();
    file_session_store_opted_in_value(value.as_deref())
}

fn file_session_store_opted_in_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim() == "1")
}

pub(crate) fn reject_inline_api_key(api_key: Option<&str>) -> Result<()> {
    if api_key.is_some() {
        bail!(
            "`codewhale account` does not accept the global `--api-key` flag because command-line values can leak through shell history. Use `account keys set <provider>` for a hidden prompt, `--api-key-stdin`, or `--from-local`"
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_with<T: CloudTransport, W: Write>(
    command: CloudCommand,
    profile: &str,
    api_base: &str,
    config: &ConfigStore,
    cloud_secrets: &Secrets,
    provider_secrets: &Secrets,
    transport: &T,
    out: &mut W,
    key_reader: &mut dyn FnMut(KeyReadMode) -> Result<String>,
    opener: &mut dyn FnMut(String) -> bool,
    sleeper: &mut dyn FnMut(Duration),
) -> Result<()> {
    let client = CloudClient::new(transport, cloud_secrets, profile, api_base);
    match command {
        CloudCommand::Login(login) => {
            let device = client.start_device()?;
            validate_user_code(&device.user_code)?;
            let verification_uri = validate_verification_url(
                &device.verification_uri,
                api_base,
                &device.user_code,
                false,
            )?;
            let verification_uri_complete = validate_verification_url(
                &device.verification_uri_complete,
                api_base,
                &device.user_code,
                true,
            )?;
            writeln!(out, "Codewhale account sign-in")?;
            writeln!(out, "Code: {}", device.user_code)?;
            writeln!(out, "Open: {verification_uri}")?;
            writeln!(out, "Profile: {}", printable(profile))?;
            if !login.no_open && !opener(verification_uri_complete) {
                writeln!(
                    out,
                    "Browser could not be opened; use the URL and code above."
                )?;
            }
            let _ =
                client.poll_device(&device, Duration::from_secs(login.timeout_seconds), sleeper)?;
            let user = client.me()?;
            write_account(out, "Signed in to Codewhale.", profile, api_base, &user)
        }
        CloudCommand::Status => match client.load_auth()? {
            Some(_) => {
                let user = client.me()?;
                write_account(out, "Signed in to Codewhale.", profile, api_base, &user)
            }
            None => {
                writeln!(out, "Not signed in to Codewhale.")?;
                writeln!(out, "Profile: {}", printable(profile))?;
                writeln!(out, "API: {api_base}")?;
                writeln!(out, "Run `codewhale account login` to sign in.")?;
                Ok(())
            }
        },
        CloudCommand::Logout => {
            let remote_revoked = client.logout()?;
            writeln!(out, "Removed the local Codewhale account session.")?;
            writeln!(out, "Profile: {}", printable(profile))?;
            if !remote_revoked {
                writeln!(
                    out,
                    "Remote revocation was not confirmed; the local tokens are gone."
                )?;
            }
            Ok(())
        }
        CloudCommand::Keys(keys) => match keys.command {
            CloudKeysCommand::List => {
                let user = client.me()?;
                write_account(out, "Codewhale account keys.", profile, api_base, &user)?;
                for provider in CloudProvider::ALL {
                    let state = user.model_keys.get(provider.slug());
                    if state.is_some_and(|state| state.configured) {
                        writeln!(out, "{}: set", provider.slug())?;
                    } else {
                        writeln!(out, "{}: not set", provider.slug())?;
                    }
                }
                Ok(())
            }
            CloudKeysCommand::Set(set) => {
                let user = client.me()?;
                let key = if set.from_local {
                    resolve_local_key(config, provider_secrets, set.provider)?.ok_or_else(|| {
                        anyhow!(
                            "No local {} API key was found in config, the secret store, or the environment",
                            set.provider.slug()
                        )
                    })?
                } else if set.api_key_stdin {
                    key_reader(KeyReadMode::Stdin)?
                } else {
                    key_reader(KeyReadMode::HiddenPrompt(set.provider.slug().to_string()))?
                };
                let key = key.trim().to_string();
                validate_api_key(&key)?;
                let label = validate_label(&set.label)?;
                client.set_key(set.provider, &key, &label)?;
                writeln!(
                    out,
                    "Saved {} for Codewhale account {} (profile {}).",
                    set.provider.slug(),
                    printable(&user.id),
                    printable(profile)
                )?;
                Ok(())
            }
            CloudKeysCommand::Remove { provider } => {
                let user = client.me()?;
                client.remove_key(provider)?;
                writeln!(
                    out,
                    "Removed {} from Codewhale account {} (profile {}).",
                    provider.slug(),
                    printable(&user.id),
                    printable(profile)
                )?;
                Ok(())
            }
        },
    }
}

fn write_account<W: Write>(
    out: &mut W,
    heading: &str,
    profile: &str,
    api_base: &str,
    user: &CloudUser,
) -> Result<()> {
    writeln!(out, "{heading}")?;
    writeln!(out, "Account ID: {}", printable(&user.id))?;
    if !user.display_name.trim().is_empty() {
        writeln!(out, "Name: {}", printable(&user.display_name))?;
    }
    if !user.email.trim().is_empty() {
        writeln!(out, "Email: {}", printable(&user.email))?;
    }
    if !user.plan.trim().is_empty() {
        writeln!(out, "Plan: {}", printable(&user.plan))?;
    }
    writeln!(out, "Profile: {}", printable(profile))?;
    writeln!(out, "API: {api_base}")?;
    Ok(())
}

fn normalized_profile(profile: Option<&str>) -> String {
    profile
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("default")
        .to_string()
}

fn cloud_auth_slot(profile: &str, api_base: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(profile.as_bytes());
    digest.update([0]);
    digest.update(api_base.as_bytes());
    let digest = digest.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    format!("codewhale-cloud-auth-v1-{encoded}")
}

struct ValidatedApiBase {
    url: Url,
    display: String,
}

fn validate_api_base(value: &str) -> Result<ValidatedApiBase> {
    let mut url = Url::parse(value.trim()).context("invalid Codewhale account API base URL")?;
    if !url.username().is_empty() || url.password().is_some() {
        bail!("Codewhale account API base URL must not contain credentials");
    }
    if url.query().is_some() || url.fragment().is_some() {
        bail!("Codewhale account API base URL must not contain a query or fragment");
    }
    if !matches!(url.path(), "" | "/") {
        bail!("Codewhale account API base URL must be an origin without a path");
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("Codewhale account API base URL must include a host"))?;
    let allowed = url.scheme() == "https" || (url.scheme() == "http" && is_loopback_host(host));
    if !allowed {
        bail!(
            "Codewhale account API base URL must use HTTPS (loopback HTTP is allowed for testing)"
        );
    }
    url.set_path("/");
    let display = url.as_str().trim_end_matches('/').to_string();
    Ok(ValidatedApiBase { url, display })
}

fn validate_verification_url(
    value: &str,
    api_base: &str,
    user_code: &str,
    complete: bool,
) -> Result<String> {
    let url =
        Url::parse(value).context("The Codewhale service returned an invalid verification URL")?;
    if value != url.as_str() {
        bail!("The Codewhale service returned an unsafe verification URL");
    }
    let host = url.host_str().ok_or_else(|| {
        anyhow!("The Codewhale service returned a verification URL without a host")
    })?;
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        bail!("The Codewhale service returned an unsafe verification URL");
    }
    if url.path() != "/cli/authorize" {
        bail!("The Codewhale service returned an unsafe verification URL");
    }

    let api = Url::parse(api_base).context("invalid Codewhale account API base URL")?;
    let canonical_api = api.scheme() == "https"
        && api.host_str() == Some("api.codewhale.net")
        && api.port_or_known_default() == Some(443);
    let loopback_api = api.host_str().is_some_and(is_loopback_host);
    if canonical_api {
        if url.scheme() != "https"
            || !host.eq_ignore_ascii_case("app.codewhale.net")
            || url.port_or_known_default() != Some(443)
        {
            bail!("The Codewhale service returned an untrusted verification origin");
        }
    } else if loopback_api {
        if !matches!(url.scheme(), "http" | "https") || !is_loopback_host(host) {
            bail!("The Codewhale service returned an untrusted verification origin");
        }
    } else {
        bail!(
            "Browser login is only enabled for the canonical Codewhale account API or a loopback test API"
        );
    }

    let query = url.query_pairs().collect::<Vec<_>>();
    if complete {
        if query.len() != 1 || query[0].0 != "user_code" || query[0].1 != user_code {
            bail!("The Codewhale service returned an unsafe verification URL");
        }
    } else if !query.is_empty() {
        bail!("The Codewhale service returned an unsafe verification URL");
    }
    Ok(url.to_string())
}

fn is_loopback_host(host: &str) -> bool {
    let host = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn validate_user_code(code: &str) -> Result<()> {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let bytes = code.as_bytes();
    if bytes.len() != 14
        || bytes[4] != b'-'
        || bytes[9] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 4 | 9) && !ALPHABET.contains(byte))
    {
        bail!("The Codewhale service returned an invalid user code");
    }
    Ok(())
}

fn validate_device_code(code: &str) -> Result<()> {
    if code.len() != 43
        || !code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        bail!("The Codewhale service returned an invalid device authorization response");
    }
    Ok(())
}

fn validate_auth_bundle(bundle: &AuthBundle) -> Result<()> {
    if !bundle.token_type.eq_ignore_ascii_case("bearer")
        || bundle.access_token.trim().is_empty()
        || bundle.refresh_token.trim().is_empty()
        || bundle.access_token.len() > MAX_TOKEN_BYTES
        || bundle.refresh_token.len() > MAX_TOKEN_BYTES
        || bundle
            .access_token
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        || bundle
            .refresh_token
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        bail!("The Codewhale service returned invalid session credentials");
    }
    Ok(())
}

fn validate_api_key(key: &str) -> Result<()> {
    let bytes = key.len();
    if bytes < MIN_API_KEY_BYTES || bytes as u64 > MAX_API_KEY_BYTES {
        bail!("API key must be {MIN_API_KEY_BYTES}-{MAX_API_KEY_BYTES} UTF-8 bytes");
    }
    if key.chars().any(is_ascii_control) {
        bail!("API key contains invalid control characters");
    }
    Ok(())
}

fn validate_label(label: &str) -> Result<String> {
    let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
    if label.is_empty()
        || label.chars().count() > MAX_KEY_LABEL_CHARS
        || label.chars().any(is_ascii_control)
    {
        bail!("key label must contain 1-{MAX_KEY_LABEL_CHARS} characters");
    }
    Ok(label)
}

fn is_ascii_control(character: char) -> bool {
    character <= '\u{001f}' || character == '\u{007f}'
}

fn resolve_local_key(
    config: &ConfigStore,
    secrets: &Secrets,
    provider: CloudProvider,
) -> Result<Option<String>> {
    let kind = provider.local_kind();
    let provider_config = config.config.providers.for_provider(kind);
    let from_config = provider_config.api_key.clone().or_else(|| {
        (kind == ProviderKind::Deepseek)
            .then(|| config.config.api_key.clone())
            .flatten()
    });
    if let Some(value) = from_config
        .and_then(resolve_config_key_reference)
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(Some(value));
    }
    if let Some(value) = secrets
        .get(kind.as_str())
        .context("failed to read the local provider secret store")?
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(Some(value));
    }
    Ok(kind.provider().env_vars().iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    }))
}

fn resolve_config_key_reference(value: String) -> Option<String> {
    let trimmed = value.trim();
    let Some(variable) = trimmed.strip_prefix('$') else {
        return Some(value);
    };
    if variable.is_empty()
        || !variable
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return None;
    }
    std::env::var(variable)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn read_key_from_stdin() -> Result<String> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(MAX_API_KEY_STDIN_BYTES + 1)
        .read_to_end(&mut bytes)
        .context("failed to read API key from stdin")?;
    parse_key_input(bytes)
}

fn parse_key_input(bytes: Vec<u8>) -> Result<String> {
    if bytes.len() as u64 > MAX_API_KEY_STDIN_BYTES {
        bail!("API key input is unexpectedly large");
    }
    let value = String::from_utf8(bytes).context("API key from stdin is not valid UTF-8")?;
    let value = value.trim().to_string();
    validate_api_key(&value)?;
    Ok(value)
}

fn read_key_hidden(provider: &str) -> Result<String> {
    if !io::stdin().is_terminal() {
        bail!("interactive key entry requires a terminal; use `--api-key-stdin` for piped input");
    }
    let term = console::Term::stderr();
    term.write_str(&format!("Enter {provider} API key: "))
        .context("failed to write API key prompt")?;
    let value = term
        .read_secure_line()
        .context("failed to read API key securely")?;
    term.write_line("").ok();
    let value = value.trim().to_string();
    validate_api_key(&value)?;
    Ok(value)
}

fn json_body(value: &impl Serialize) -> Result<Vec<u8>> {
    serde_json::to_vec(value).context("failed to encode Codewhale account request")
}

fn expect_json<T: DeserializeOwned>(response: CloudResponse, statuses: &[u16]) -> Result<T> {
    if !statuses.contains(&response.status) {
        return Err(response_error(&response));
    }
    parse_json_body(&response.body)
}

fn expect_empty(response: CloudResponse, statuses: &[u16]) -> Result<()> {
    if statuses.contains(&response.status) {
        Ok(())
    } else {
        Err(response_error(&response))
    }
}

fn parse_json_body<T: DeserializeOwned>(body: &[u8]) -> Result<T> {
    serde_json::from_slice(body).context("The Codewhale service returned an invalid JSON response")
}

fn response_error(response: &CloudResponse) -> anyhow::Error {
    let code = serde_json::from_slice::<serde_json::Value>(&response.body)
        .ok()
        .and_then(|body| {
            body.get("code")
                .and_then(serde_json::Value::as_str)
                .or_else(|| {
                    body.get("error")
                        .and_then(|error| error.get("code"))
                        .and_then(serde_json::Value::as_str)
                })
                .and_then(safe_error_code)
        });
    match code {
        Some(code) => anyhow!(
            "Codewhale account request failed (HTTP {}, code {code})",
            response.status
        ),
        None => anyhow!(
            "Codewhale account request failed (HTTP {})",
            response.status
        ),
    }
}

fn safe_error_code(code: &str) -> Option<String> {
    if code.is_empty()
        || code.len() > 80
        || !code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return None;
    }
    Some(code.to_string())
}

fn printable(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(200)
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use clap::Parser;
    use codewhale_secrets::{InMemoryKeyringStore, KeyringStore};
    use serde_json::json;

    use super::*;
    use crate::{Cli, Commands};

    struct FakeTransport {
        responses: Mutex<VecDeque<CloudResponse>>,
        requests: Mutex<Vec<CloudRequest>>,
    }

    impl FakeTransport {
        fn new(responses: Vec<CloudResponse>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn requests(&self) -> std::sync::MutexGuard<'_, Vec<CloudRequest>> {
            self.requests.lock().unwrap()
        }
    }

    impl CloudTransport for FakeTransport {
        fn execute(&self, request: CloudRequest) -> Result<CloudResponse> {
            self.requests.lock().unwrap().push(request);
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| anyhow!("fake transport exhausted"))
        }
    }

    fn response(status: u16, body: serde_json::Value) -> CloudResponse {
        CloudResponse {
            status,
            body: serde_json::to_vec(&body).unwrap(),
        }
    }

    fn account(id: &str) -> serde_json::Value {
        json!({
            "user": {
                "id": id,
                "displayName": "Hunter",
                "email": "hunter@example.test",
                "plan": "free",
                "modelKeys": {}
            }
        })
    }

    fn auth(access: &str, refresh: &str, account_id: &str) -> AuthBundle {
        AuthBundle {
            token_type: "Bearer".to_string(),
            access_token: access.to_string(),
            refresh_token: refresh.to_string(),
            session: Some(AuthSession {
                id: "session-1".to_string(),
                provider: "github".to_string(),
                expires_at: String::new(),
                refresh_expires_at: String::new(),
            }),
            user: Some(CloudUser {
                id: account_id.to_string(),
                display_name: "Hunter".to_string(),
                email: "hunter@example.test".to_string(),
                ..CloudUser::default()
            }),
        }
    }

    fn auth_json(access: &str, refresh: &str, account_id: &str) -> serde_json::Value {
        serde_json::to_value(auth(access, refresh, account_id)).unwrap()
    }

    fn test_secrets() -> (Secrets, Arc<InMemoryKeyringStore>) {
        let store = Arc::new(InMemoryKeyringStore::new());
        (Secrets::new(store.clone()), store)
    }

    fn test_config() -> (tempfile::TempDir, ConfigStore) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let config = ConfigStore::load(Some(path)).unwrap();
        (temp, config)
    }

    fn command(argv: &[&str]) -> CloudCommand {
        let cli = Cli::try_parse_from(argv).unwrap();
        let Some(Commands::Account(args)) = cli.command else {
            panic!("expected account command");
        };
        args.command
    }

    #[test]
    fn parses_cloud_command_matrix_and_rejects_inline_keys() {
        assert!(matches!(
            command(&["codewhale", "account", "status"]),
            CloudCommand::Status
        ));
        assert!(matches!(
            command(&["codewhale", "cloud", "login", "--no-open"]),
            CloudCommand::Login(CloudLoginArgs { no_open: true, .. })
        ));
        assert!(matches!(
            command(&[
                "codewhale",
                "cloud",
                "keys",
                "set",
                "xiaomi-mimo",
                "--from-local"
            ]),
            CloudCommand::Keys(CloudKeysArgs {
                command: CloudKeysCommand::Set(CloudKeySetArgs {
                    provider: CloudProvider::Xiaomi,
                    from_local: true,
                    ..
                })
            })
        ));
        assert!(
            Cli::try_parse_from([
                "codewhale",
                "cloud",
                "keys",
                "set",
                "openai",
                "sk-unsafe-inline"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "codewhale",
                "cloud",
                "keys",
                "set",
                "openai",
                "--from-local",
                "--api-key-stdin"
            ])
            .is_err()
        );
        assert!(reject_inline_api_key(None).is_ok());
        let error = reject_inline_api_key(Some("sk-never-render")).unwrap_err();
        assert!(error.to_string().contains("--api-key-stdin"));
        assert!(!error.to_string().contains("sk-never-render"));
    }

    #[test]
    fn api_base_requires_https_or_literal_loopback_http() {
        assert_eq!(
            validate_api_base("https://api.codewhale.net/")
                .unwrap()
                .display,
            "https://api.codewhale.net"
        );
        assert!(validate_api_base("http://127.0.0.1:8787").is_ok());
        assert!(validate_api_base("http://[::1]:8787").is_ok());
        assert!(validate_api_base("http://api.codewhale.net").is_err());
        assert!(validate_api_base("https://user:secret@example.test").is_err());
        assert!(validate_api_base("https://example.test/prefix").is_err());
    }

    #[test]
    fn verification_urls_are_pinned_to_the_app_or_loopback() {
        const CODE: &str = "ABCD-EFGH-JKLM";
        const API: &str = "https://api.codewhale.net";
        assert!(
            validate_verification_url("https://app.codewhale.net/cli/authorize", API, CODE, false,)
                .is_ok()
        );
        assert!(
            validate_verification_url(
                "https://app.codewhale.net/cli/authorize?user_code=ABCD-EFGH-JKLM",
                API,
                CODE,
                true,
            )
            .is_ok()
        );
        for unsafe_url in [
            "https://attacker.example/cli/authorize",
            "https://user@app.codewhale.net/cli/authorize",
            "https://app.codewhale.net/cli/authorize#continue",
            "https://app.codewhale.net/cli/authorize/extra",
            "https://app.codewhale.net/cli/other/../authorize",
            "https://app.codewhale.net/cli/%61uthorize",
            "https://app.codewhale.net/cli/authorize?next=https%3A%2F%2Fattacker.example",
            "https://app.codewhale.net/cli/authorize?user_code=ABCD-EFGH-JKLM&next=evil",
        ] {
            assert!(
                validate_verification_url(unsafe_url, API, CODE, unsafe_url.contains("user_code"))
                    .is_err(),
                "accepted unsafe URL: {unsafe_url}"
            );
        }
        assert!(
            validate_verification_url(
                "http://localhost:3000/cli/authorize?user_code=ABCD-EFGH-JKLM",
                "http://127.0.0.1:8787",
                CODE,
                true,
            )
            .is_ok()
        );
        assert!(
            validate_verification_url(
                "https://staging-app.example/cli/authorize",
                "https://staging-api.example",
                CODE,
                false,
            )
            .is_err()
        );
    }

    #[test]
    fn user_codes_and_key_inputs_match_the_server_contract() {
        assert!(validate_user_code("ABCD-EFGH-JKLM").is_ok());
        for invalid in [
            "CW-1234",
            "ABCI-EFGH-JKLM",
            "ABCO-EFGH-JKLM",
            "ABC1-EFGH-JKLM",
            "abcd-EFGH-JKLM",
            "ABCD_EFGH_JKLM",
        ] {
            assert!(validate_user_code(invalid).is_err(), "accepted {invalid}");
        }

        assert!(validate_device_code("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").is_ok());
        for invalid in [
            "too-short",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA!",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ] {
            assert!(validate_device_code(invalid).is_err(), "accepted {invalid}");
        }

        assert!(validate_api_key("1234567").is_err());
        assert!(validate_api_key("12345678").is_ok());
        assert!(validate_api_key(&"x".repeat(4096)).is_ok());
        assert!(validate_api_key(&"x".repeat(4097)).is_err());
        assert!(validate_api_key(&"é".repeat(4)).is_ok());
        assert!(validate_api_key("1234567\n8").is_err());
        assert_eq!(
            parse_key_input(format!("{}\n", "x".repeat(4096)).into_bytes()).unwrap(),
            "x".repeat(4096)
        );
        assert!(parse_key_input(vec![b'x'; MAX_API_KEY_STDIN_BYTES as usize + 1]).is_err());
        assert_eq!(
            validate_label("  Codewhale\tCLI  ").unwrap(),
            "Codewhale CLI"
        );
        assert!(validate_label(&"x".repeat(80)).is_ok());
        assert!(validate_label(&"x".repeat(81)).is_err());
    }

    #[test]
    fn file_session_store_requires_explicit_one_value() {
        assert!(!file_session_store_opted_in_value(None));
        assert!(!file_session_store_opted_in_value(Some("")));
        assert!(!file_session_store_opted_in_value(Some("true")));
        assert!(file_session_store_opted_in_value(Some("1")));
        assert!(file_session_store_opted_in_value(Some(" 1 ")));
    }

    #[test]
    fn device_flow_handles_pending_then_authorized_without_printing_tokens() {
        let (temp, config) = test_config();
        let _keep_temp = temp;
        let (secrets, _) = test_secrets();
        let transport = FakeTransport::new(vec![
            response(
                200,
                json!({
                    "deviceCode": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                    "userCode": "ABCD-EFGH-JKLM",
                    "verificationUri": "https://app.codewhale.net/cli/authorize",
                    "verificationUriComplete": "https://app.codewhale.net/cli/authorize?user_code=ABCD-EFGH-JKLM",
                    "expiresIn": 600,
                    "interval": 1
                }),
            ),
            response(202, json!({ "status": "authorization_pending" })),
            response(
                200,
                auth_json("access-never-print", "refresh-never-print", "acct-123"),
            ),
            response(200, account("acct-123")),
        ]);
        let mut output = Vec::new();
        let mut key_reader = |_| bail!("key reader should not be called");
        let mut opened = Vec::new();
        let mut opener = |url: String| {
            opened.push(url);
            true
        };
        let mut sleeper = |_| {};
        run_with(
            command(&["codewhale", "cloud", "login"]),
            "work",
            "https://api.codewhale.net",
            &config,
            &secrets,
            &secrets,
            &transport,
            &mut output,
            &mut key_reader,
            &mut opener,
            &mut sleeper,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("ABCD-EFGH-JKLM"));
        assert!(output.contains("Account ID: acct-123"));
        assert!(output.contains("Profile: work"));
        assert!(!output.contains("access-never-print"));
        assert!(!output.contains("refresh-never-print"));
        assert_eq!(opened.len(), 1);
        let requests = transport.requests();
        assert_eq!(requests[0].path, "/api/cli/device/start");
        assert_eq!(requests[1].path, "/api/cli/device/token");
        assert_eq!(requests[2].path, "/api/cli/device/token");
        assert_eq!(requests[3].path, "/api/me");
    }

    #[test]
    fn cloud_sessions_are_isolated_by_profile_and_api_origin() {
        let (secrets, _) = test_secrets();
        let transport = FakeTransport::new(vec![]);
        let default =
            CloudClient::new(&transport, &secrets, "default", "https://api.codewhale.net");
        let work = CloudClient::new(&transport, &secrets, "work", "https://api.codewhale.net");
        let local = CloudClient::new(&transport, &secrets, "default", "http://127.0.0.1:8787");
        default
            .save_auth(auth("a-default", "r-default", "acct-default"))
            .unwrap();
        work.save_auth(auth("a-work", "r-work", "acct-work"))
            .unwrap();
        local
            .save_auth(auth("a-local", "r-local", "acct-local"))
            .unwrap();

        assert_eq!(
            default
                .load_auth()
                .unwrap()
                .unwrap()
                .bundle
                .user
                .unwrap()
                .id,
            "acct-default"
        );
        assert_eq!(
            work.load_auth().unwrap().unwrap().bundle.user.unwrap().id,
            "acct-work"
        );
        assert_eq!(
            local.load_auth().unwrap().unwrap().bundle.user.unwrap().id,
            "acct-local"
        );
    }

    #[test]
    fn status_refreshes_once_on_unauthorized_and_never_displays_tokens() {
        let (temp, config) = test_config();
        let _keep_temp = temp;
        let (secrets, _) = test_secrets();
        let transport = FakeTransport::new(vec![
            response(401, json!({ "code": "access_token_expired" })),
            response(
                200,
                auth_json("access-new-secret", "refresh-new-secret", "acct-refresh"),
            ),
            response(200, account("acct-refresh")),
        ]);
        CloudClient::new(&transport, &secrets, "default", "https://api.codewhale.net")
            .save_auth(auth(
                "access-old-secret",
                "refresh-old-secret",
                "acct-refresh",
            ))
            .unwrap();
        let mut output = Vec::new();
        let mut key_reader = |_| bail!("unused");
        let mut opener = |_| true;
        let mut sleeper = |_| {};
        run_with(
            CloudCommand::Status,
            "default",
            "https://api.codewhale.net",
            &config,
            &secrets,
            &secrets,
            &transport,
            &mut output,
            &mut key_reader,
            &mut opener,
            &mut sleeper,
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("acct-refresh"));
        for secret in [
            "access-old-secret",
            "refresh-old-secret",
            "access-new-secret",
            "refresh-new-secret",
        ] {
            assert!(!output.contains(secret));
        }
        let requests = transport.requests();
        assert_eq!(requests[0].path, "/api/me");
        assert_eq!(requests[1].path, "/api/auth/refresh");
        assert_eq!(requests[2].path, "/api/me");
    }

    #[test]
    fn set_list_and_remove_use_account_routes_without_secret_output() {
        let (temp, config) = test_config();
        let _keep_temp = temp;
        let (secrets, _) = test_secrets();
        let list_account = json!({
            "user": {
                "id": "acct-keys",
                "displayName": "Hunter",
                "email": "hunter@example.test",
                "modelKeys": {
                    "openai": { "configured": true, "label": "Laptop", "updatedAt": "now" }
                }
            }
        });
        let transport = FakeTransport::new(vec![
            response(200, account("acct-keys")),
            response(200, json!({ "ok": true })),
            response(200, list_account),
            response(200, account("acct-keys")),
            response(204, json!(null)),
        ]);
        CloudClient::new(&transport, &secrets, "default", "https://api.codewhale.net")
            .save_auth(auth("access-secret", "refresh-secret", "acct-keys"))
            .unwrap();
        let mut output = Vec::new();
        let mut key_reader = |_| Ok("sk-provider-never-print".to_string());
        let mut opener = |_| true;
        let mut sleeper = |_| {};
        for cmd in [
            command(&[
                "codewhale",
                "cloud",
                "keys",
                "set",
                "openai",
                "--api-key-stdin",
                "--label",
                "Laptop",
            ]),
            command(&["codewhale", "cloud", "keys", "list"]),
            command(&["codewhale", "cloud", "keys", "remove", "openai"]),
        ] {
            run_with(
                cmd,
                "default",
                "https://api.codewhale.net",
                &config,
                &secrets,
                &secrets,
                &transport,
                &mut output,
                &mut key_reader,
                &mut opener,
                &mut sleeper,
            )
            .unwrap();
        }
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("openai: set"));
        assert!(!output.contains("Laptop"));
        assert!(output.contains("Codewhale account acct-keys"));
        assert!(!output.contains("sk-provider-never-print"));
        assert!(!output.contains("access-secret"));
        assert!(!output.contains("refresh-secret"));

        let requests = transport.requests();
        let put = requests
            .iter()
            .find(|request| request.method == HttpMethod::Put)
            .unwrap();
        assert_eq!(put.path, "/api/model-keys/openai");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(put.body.as_ref().unwrap()).unwrap(),
            json!({ "key": "sk-provider-never-print", "label": "Laptop" })
        );
        assert!(requests.iter().any(|request| {
            request.method == HttpMethod::Delete && request.path == "/api/model-keys/openai"
        }));
    }

    #[test]
    fn from_local_uses_config_without_printing_or_requiring_an_inline_key() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let mut config = ConfigStore::load(Some(path)).unwrap();
        config.config.providers.anthropic.api_key = Some("sk-local-upload-secret".to_string());
        let (secrets, _) = test_secrets();
        let transport = FakeTransport::new(vec![
            response(200, account("acct-local")),
            response(200, json!({ "ok": true })),
        ]);
        CloudClient::new(&transport, &secrets, "work", "https://api.codewhale.net")
            .save_auth(auth("access", "refresh", "acct-local"))
            .unwrap();
        let mut output = Vec::new();
        let mut key_reader = |_| bail!("from-local must not prompt");
        let mut opener = |_| true;
        let mut sleeper = |_| {};
        run_with(
            command(&[
                "codewhale",
                "cloud",
                "keys",
                "set",
                "anthropic",
                "--from-local",
            ]),
            "work",
            "https://api.codewhale.net",
            &config,
            &secrets,
            &secrets,
            &transport,
            &mut output,
            &mut key_reader,
            &mut opener,
            &mut sleeper,
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("acct-local"));
        assert!(!output.contains("sk-local-upload-secret"));
        let requests = transport.requests();
        let put = requests
            .iter()
            .find(|request| request.method == HttpMethod::Put)
            .unwrap();
        assert!(
            String::from_utf8_lossy(put.body.as_ref().unwrap()).contains("sk-local-upload-secret")
        );
    }

    #[test]
    fn from_local_uses_config_before_the_provider_secret_store() {
        let (temp, mut config) = test_config();
        let _keep_temp = temp;
        let (secrets, store) = test_secrets();
        store.set("openai", "sk-secret-store").unwrap();

        assert_eq!(
            resolve_local_key(&config, &secrets, CloudProvider::Openai)
                .unwrap()
                .as_deref(),
            Some("sk-secret-store")
        );
        config.config.providers.openai.api_key = Some("sk-config-first".to_string());
        assert_eq!(
            resolve_local_key(&config, &secrets, CloudProvider::Openai)
                .unwrap()
                .as_deref(),
            Some("sk-config-first")
        );
    }

    #[test]
    fn logout_recovers_from_a_corrupt_local_session_record() {
        let (temp, config) = test_config();
        let _keep_temp = temp;
        let (secrets, store) = test_secrets();
        let slot = cloud_auth_slot("default", "https://api.codewhale.net");
        store.set(&slot, "not-json-and-not-a-token").unwrap();
        let transport = FakeTransport::new(vec![]);
        let mut output = Vec::new();
        let mut key_reader = |_| bail!("unused");
        let mut opener = |_| true;
        let mut sleeper = |_| {};
        run_with(
            CloudCommand::Logout,
            "default",
            "https://api.codewhale.net",
            &config,
            &secrets,
            &secrets,
            &transport,
            &mut output,
            &mut key_reader,
            &mut opener,
            &mut sleeper,
        )
        .unwrap();
        assert!(store.get(&slot).unwrap().is_none());
        assert!(
            !String::from_utf8(output)
                .unwrap()
                .contains("not-json-and-not-a-token")
        );
    }

    #[test]
    fn server_errors_never_echo_response_messages() {
        let error = response_error(&response(
            400,
            json!({
                "error": {
                    "code": "invalid_api_key",
                    "message": "The submitted key was sk-never-echo-this"
                }
            }),
        ))
        .to_string();
        assert!(error.contains("invalid_api_key"));
        assert!(!error.contains("sk-never-echo-this"));
    }

    #[test]
    fn cloud_auth_slot_does_not_embed_profile_or_origin() {
        let slot = cloud_auth_slot("private-profile", "https://api.codewhale.net");
        assert!(!slot.contains("private-profile"));
        assert!(!slot.contains("api.codewhale.net"));
        assert_ne!(
            slot,
            cloud_auth_slot("other-profile", "https://api.codewhale.net")
        );
    }

    #[test]
    fn fake_store_is_profile_safe() {
        let (_, store) = test_secrets();
        store.set("unrelated", "keep-me").unwrap();
        store.delete("missing").unwrap();
        assert_eq!(store.get("unrelated").unwrap().as_deref(), Some("keep-me"));
    }
}
