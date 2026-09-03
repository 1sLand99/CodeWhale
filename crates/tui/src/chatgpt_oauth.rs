//! Codewhale-owned ChatGPT / Codex subscription credential storage.
//!
//! The login flow itself lives in [`crate::oauth`] (one access-route flow,
//! parameterized per provider). This module holds what is still per-provider
//! until activation unifies: the owned-generation entry format, activation,
//! refresh-on-read, and best-effort remote revoke. It never reads Codex CLI
//! cookies and never writes `~/.codex/auth.json`.
//!
//! # Terms boundary
//!
//! OpenAI publishes Sign in with ChatGPT for the Codex app, CLI, and IDE
//! ([learn.chatgpt.com/docs/auth](https://learn.chatgpt.com/docs/auth)) and
//! advertises authorization-code + PKCE S256 plus `refresh_token` on the
//! issuer's OIDC discovery document
//! (`https://auth.openai.com/.well-known/openid-configuration`). OpenAI has
//! not published a third-party client-registration path for this public
//! Codex client. The login adapter:
//!
//! - uses the Apache-2.0 Codex CLI public client id with `originator=codewhale`
//!   (never `codex_cli_rs`)
//! - uses the loopback redirect ports that public client registers
//!   (`1455`, fallback `1457`)
//! - uses `/oauth/authorize` and `/oauth/token` on the published issuer, the
//!   paths that public client is registered against
//! - posts best-effort remote revoke to the fixed
//!   `{issuer}/api/accounts/oauth/revoke`, the path that public client is
//!   registered against. This is deliberately not read from the discovery
//!   document: revoke must still clear local credentials when the issuer is
//!   unreachable, so adding a discovery fetch would only add a failure mode to
//!   a path whose contract is to clean up regardless
//! - does **not** call unpublished device-auth paths (`/api/accounts/deviceauth/*`);
//!   the issuer does not advertise `device_authorization_endpoint`
//!
//! External Codex CLI import remains an explicit alternative, not a
//! prerequisite. Token values are never logged.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{ApiProvider, Config};

/// Codex CLI public OAuth client (public PKCE client; no secret).
pub const CHATGPT_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
/// Published issuer.
pub const CHATGPT_OAUTH_ISSUER: &str = "https://auth.openai.com";
/// Honest originator; never impersonate `codex_cli_rs`.
pub const CHATGPT_OAUTH_ORIGINATOR: &str = "codewhale";
pub const CHATGPT_OAUTH_SCOPE: &str = "openid profile email offline_access";
const REFRESH_SKEW_SECS: i64 = 60;

/// The provider row this storage half serves.
fn chatgpt_params() -> &'static crate::oauth::OAuthProviderParams {
    crate::oauth::oauth_provider_params(crate::oauth::OAuthProvider::Chatgpt)
}

#[derive(Clone, Serialize, Deserialize)]
struct ChatgptAuthEntry {
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    oidc_issuer: Option<String>,
    #[serde(default)]
    oidc_client_id: Option<String>,
    #[serde(default)]
    originator: Option<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// Resolved bearer credential ready for the Responses route.
#[derive(Clone)]
pub struct ChatgptOAuthCredentials {
    pub access_token: String,
    pub account_id: Option<String>,
    #[allow(dead_code)]
    pub refresh_token: Option<String>,
    #[allow(dead_code)]
    pub expires_at: Option<String>,
}

/// Successful PKCE exchange that has not yet been made active. No Debug:
/// the token material never prints.
pub struct PendingChatgptPkceLogin {
    issuer: String,
    client_id: String,
    token: crate::oauth::OAuthTokenMaterial,
}

/// Transitional bridge: the unified flow in [`crate::oauth`] performs the
/// login and hands back provider-neutral material; this converts it into the
/// legacy pending shape the (not yet unified) activation path consumes.
/// Deleted with the rest of this module when activation unifies.
pub(crate) fn pending_from_unified(
    pending: crate::oauth::PendingOAuthLogin,
) -> PendingChatgptPkceLogin {
    PendingChatgptPkceLogin {
        issuer: pending.issuer,
        client_id: pending.client_id,
        token: pending.token,
    }
}

/// Receipt for the committed Codewhale-owned ChatGPT OAuth generation.
#[derive(Debug)]
pub struct ChatgptPkceActivation {
    #[allow(dead_code)]
    pub credentials: ChatgptOAuthCredentials,
    pub config_path: PathBuf,
    pub auth_path: PathBuf,
}

fn redacted(present: bool) -> &'static str {
    if present { "<redacted>" } else { "<none>" }
}

impl std::fmt::Debug for ChatgptAuthEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatgptAuthEntry")
            .field("access_token", &redacted(self.access_token.is_some()))
            .field("refresh_token", &redacted(self.refresh_token.is_some()))
            .field("expires_at", &self.expires_at)
            .field("id_token", &redacted(self.id_token.is_some()))
            .field("account_id", &self.account_id)
            .field("oidc_issuer", &self.oidc_issuer)
            .field("oidc_client_id", &self.oidc_client_id)
            .field("originator", &self.originator)
            .finish()
    }
}

impl std::fmt::Debug for ChatgptOAuthCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatgptOAuthCredentials")
            .field("access_token", &redacted(true))
            .field("account_id", &self.account_id)
            .field("refresh_token", &redacted(self.refresh_token.is_some()))
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Commit a pending PKCE login as a uniquely named owned generation and
/// point `[providers.openai_codex]` at it under the shared config lock.
pub fn activate_pkce_login(
    pending: PendingChatgptPkceLogin,
    config_path: Option<&Path>,
    live_config: Option<&mut Config>,
) -> Result<ChatgptPkceActivation> {
    codewhale_config::with_xai_oauth_lifecycle_lock(move |store| {
        activate_pkce_login_locked(pending, config_path, live_config, store)
    })
}

fn activate_pkce_login_locked(
    pending: PendingChatgptPkceLogin,
    config_path: Option<&Path>,
    live_config: Option<&mut Config>,
    store: &codewhale_config::XaiOAuthCredentialStore,
) -> Result<ChatgptPkceActivation> {
    let config_path = crate::config_persistence::config_toml_path(config_path)?;
    let generation = format!(
        "{}{}{}",
        codewhale_config::CHATGPT_OAUTH_GENERATION_PREFIX,
        uuid::Uuid::new_v4().simple(),
        codewhale_config::CHATGPT_OAUTH_GENERATION_SUFFIX
    );
    codewhale_config::validate_chatgpt_oauth_generation(&generation)?;
    let auth_path = store.path_for(&generation)?;
    let key_inside = crate::config::provider_config_key(ApiProvider::OpenaiCodex)
        .context("openai-codex auth mode key")?;
    let mut stage_written = false;

    let activation = codewhale_config::mutate_config_document(&config_path, |document| {
        let previous_generation_item = document
            .get("providers")
            .and_then(toml_edit::Item::as_table_like)
            .and_then(|providers| providers.get(key_inside))
            .and_then(toml_edit::Item::as_table_like)
            .and_then(|provider| provider.get("oauth_credential_generation"));
        let previous_generation = previous_generation_item
            .map(|item| {
                item.as_str()
                    .context(
                        "refusing ChatGPT login because the existing credential generation pointer is not a string",
                    )
                    .map(ToOwned::to_owned)
            })
            .transpose()?;
        if let Some(previous) = previous_generation.as_deref() {
            codewhale_config::validate_chatgpt_oauth_generation(previous).with_context(|| {
                "refusing ChatGPT login because the existing credential generation pointer is invalid"
            })?;
        }
        let previous_owned_name = match previous_generation.as_deref() {
            Some(previous) => Some(previous.to_string()),
            None if store
                .read_to_string(codewhale_config::LEGACY_CHATGPT_OAUTH_FILE_NAME)?
                .is_some() =>
            {
                Some(codewhale_config::LEGACY_CHATGPT_OAUTH_FILE_NAME.to_string())
            }
            None => None,
        };
        let mut file = BTreeMap::new();
        let scope = format!("{}::{}", pending.issuer, pending.client_id);
        let mut entry = ChatgptAuthEntry {
            access_token: None,
            refresh_token: None,
            expires_at: None,
            id_token: None,
            account_id: None,
            oidc_issuer: Some(pending.issuer.clone()),
            oidc_client_id: Some(pending.client_id.clone()),
            originator: Some(CHATGPT_OAUTH_ORIGINATOR.to_string()),
            extra: BTreeMap::new(),
        };
        apply_token_response(
            &mut entry,
            &pending.issuer,
            &pending.client_id,
            &pending.token,
        )?;
        let access = entry
            .access_token
            .clone()
            .filter(|token| !token.trim().is_empty())
            .context("ChatGPT PKCE login returned an empty access token")?;
        file.insert(scope.clone(), entry.clone());
        write_auth_file_to_store(store, &generation, &file, false)?;
        stage_written = true;

        codewhale_config::set_config_document_value(
            document,
            &["providers", key_inside, "auth_mode"],
            "oauth",
        )?;
        codewhale_config::set_config_document_value(
            document,
            &["providers", key_inside, "oauth_credential_generation"],
            generation.clone(),
        )?;
        codewhale_config::unset_config_document_value(
            document,
            &["providers", key_inside, "external_credentials"],
        )?;
        Ok((previous_owned_name, credentials_from_entry(&entry, access)))
    });

    let (previous_owned_name, credentials) = match activation {
        Ok(activation) => activation,
        Err(error) => {
            if stage_written && let Err(cleanup_error) = store.remove(&generation) {
                return Err(error).context(format!(
                    "ChatGPT login was not activated; also failed to remove unreferenced staged credentials at {}: {cleanup_error}",
                    codewhale_config::quote_os_path(&auth_path)
                ));
            }
            return Err(error)
                .context("ChatGPT login was not activated; provider configuration is unchanged");
        }
    };

    if let Some(config) = live_config {
        config.mark_codewhale_owned_chatgpt_oauth(generation.clone());
    }
    if let Some(previous) = previous_owned_name
        && previous != generation
        && let Err(error) = store.remove(&previous)
    {
        tracing::warn!(
            target: "codewhale::chatgpt_oauth",
            error = %error,
            "new ChatGPT OAuth generation committed but superseded generation cleanup failed"
        );
    }
    eprintln!(
        "Signed in with ChatGPT. Codewhale-owned credentials activated at {}.",
        codewhale_config::quote_os_path(&auth_path)
    );
    Ok(ChatgptPkceActivation {
        credentials,
        config_path,
        auth_path,
    })
}

/// Remove Codewhale-owned ChatGPT tokens and the config pointer.
///
/// Remote revoke is best-effort against the fixed
/// `{issuer}/api/accounts/oauth/revoke` (see [`revoke_endpoint`]), not a
/// discovery lookup: a failed or unreachable revoke must never stop the local
/// credentials from being removed. External Codex CLI consent is left
/// untouched.
pub fn revoke_owned_login(
    config_path: Option<&Path>,
    live_config: Option<&mut Config>,
) -> Result<()> {
    codewhale_config::with_xai_oauth_lifecycle_lock(|store| {
        revoke_owned_login_locked(
            config_path,
            live_config,
            store,
            &crate::oauth::ReqwestOAuthFormClient,
        )
    })
}

fn revoke_owned_login_locked(
    config_path: Option<&Path>,
    live_config: Option<&mut Config>,
    store: &codewhale_config::XaiOAuthCredentialStore,
    client: &dyn crate::oauth::OAuthFormClient,
) -> Result<()> {
    let config_path = crate::config_persistence::config_toml_path(config_path)?;
    let key_inside = crate::config::provider_config_key(ApiProvider::OpenaiCodex)
        .context("openai-codex auth mode key")?;
    let previous = codewhale_config::mutate_config_document(&config_path, |document| {
        let previous = document
            .get("providers")
            .and_then(toml_edit::Item::as_table_like)
            .and_then(|providers| providers.get(key_inside))
            .and_then(toml_edit::Item::as_table_like)
            .and_then(|provider| provider.get("oauth_credential_generation"))
            .and_then(toml_edit::Item::as_str)
            .map(ToOwned::to_owned);
        codewhale_config::unset_config_document_value(
            document,
            &["providers", key_inside, "oauth_credential_generation"],
        )?;
        let auth_mode_is_oauth = document
            .get("providers")
            .and_then(toml_edit::Item::as_table_like)
            .and_then(|providers| providers.get(key_inside))
            .and_then(toml_edit::Item::as_table_like)
            .and_then(|provider| provider.get("auth_mode"))
            .and_then(toml_edit::Item::as_str)
            == Some("oauth");
        if auth_mode_is_oauth {
            codewhale_config::unset_config_document_value(
                document,
                &["providers", key_inside, "auth_mode"],
            )?;
        }
        Ok(previous)
    })?;
    if let Some(config) = live_config {
        config.clear_codewhale_owned_chatgpt_oauth();
    }
    let names = match previous.as_deref() {
        Some(generation) if codewhale_config::is_valid_chatgpt_oauth_generation(generation) => {
            vec![generation.to_string()]
        }
        _ => vec![codewhale_config::LEGACY_CHATGPT_OAUTH_FILE_NAME.to_string()],
    };
    for name in names {
        if let Ok(Some(raw)) = store.read_to_string(&name)
            && let Ok(file) = parse_auth_file(&raw, &store.path_for(&name)?)
        {
            for entry in file.values() {
                if let Some(token) = entry
                    .refresh_token
                    .as_deref()
                    .or(entry.access_token.as_deref())
                    .filter(|token| !token.trim().is_empty())
                {
                    let issuer = entry.oidc_issuer.as_deref().unwrap_or(CHATGPT_OAUTH_ISSUER);
                    let client_id = entry
                        .oidc_client_id
                        .as_deref()
                        .unwrap_or(CHATGPT_OAUTH_CLIENT_ID);
                    if let Err(error) = crate::oauth::revoke_remote_token_via(
                        client,
                        chatgpt_params(),
                        issuer,
                        client_id,
                        token,
                    ) {
                        tracing::warn!(
                            target: "codewhale::chatgpt_oauth",
                            error = %error,
                            "ChatGPT OAuth remote revoke failed; local credentials will still be removed"
                        );
                    }
                }
            }
        }
        let _ = store.remove(&name);
    }
    Ok(())
}

#[must_use]
pub fn credentials_present(config: &Config) -> bool {
    credentials_valid(config)
}

#[must_use]
pub fn credentials_valid(config: &Config) -> bool {
    if let Ok(Some(path)) = configured_owned_auth_file_path(config)
        && let Ok(Some(mut file)) = load_owned_auth_file(&path)
        && let Some((_, entry)) = select_entry(&mut file)
        && (entry_access_token_is_fresh(&entry)
            || entry
                .refresh_token
                .as_deref()
                .is_some_and(|token| !token.trim().is_empty()))
    {
        return true;
    }
    false
}

fn configured_owned_auth_file_path(config: &Config) -> Result<Option<PathBuf>> {
    let generation = config
        .provider_config_for(ApiProvider::OpenaiCodex)
        .and_then(|entry| entry.oauth_credential_generation.as_deref());
    match generation {
        Some(generation) => codewhale_config::chatgpt_oauth_generation_path(generation).map(Some),
        None => Ok(None),
    }
}

pub fn get_owned_credentials(config: &Config) -> Result<ChatgptOAuthCredentials> {
    get_owned_credentials_with(config, &crate::oauth::ReqwestOAuthFormClient)
}

fn get_owned_credentials_with(
    config: &Config,
    client: &dyn crate::oauth::OAuthFormClient,
) -> Result<ChatgptOAuthCredentials> {
    let Some(path) = configured_owned_auth_file_path(config)? else {
        bail!("Codewhale-owned ChatGPT OAuth credentials are not configured");
    };
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("Codewhale-owned ChatGPT OAuth path must have a UTF-8 basename")?;
    codewhale_config::with_xai_oauth_lifecycle_lock(|store| {
        get_owned_credentials_locked(store, name, |issuer, client_id, refresh| {
            crate::oauth::refresh_access_token_via(
                client,
                chatgpt_params(),
                issuer,
                client_id,
                refresh,
            )
        })
    })
}

fn get_owned_credentials_locked<F>(
    store: &codewhale_config::XaiOAuthCredentialStore,
    name: &str,
    refresh_access: F,
) -> Result<ChatgptOAuthCredentials>
where
    F: FnOnce(&str, &str, &str) -> Result<crate::oauth::OAuthTokenMaterial>,
{
    let path = store.path_for(name)?;
    let mut file = load_owned_auth_file_from_store(store, name)?.ok_or_else(|| {
        anyhow::anyhow!(
            "Codewhale-owned ChatGPT OAuth credentials were not found at {}. Run `codewhale auth chatgpt` again.",
            codewhale_config::quote_os_path(&path)
        )
    })?;
    let (scope, mut entry) = select_entry(&mut file).ok_or_else(|| {
        anyhow::anyhow!(
            "Codewhale-owned ChatGPT OAuth credentials at {} have no usable entry. Run `codewhale auth chatgpt` again.",
            codewhale_config::quote_os_path(&path)
        )
    })?;

    if entry_access_token_is_fresh(&entry) {
        let token = entry
            .access_token
            .clone()
            .filter(|t| !t.trim().is_empty())
            .context("ChatGPT OAuth access token is empty")?;
        return Ok(credentials_from_entry(&entry, token));
    }

    let refresh = entry
        .refresh_token
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .context(
            "ChatGPT OAuth access token expired and no refresh_token is stored. \
             Run `codewhale auth chatgpt` again.",
        )?;
    let issuer = entry
        .oidc_issuer
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| issuer_from_scope(&scope));
    let client_id = entry
        .oidc_client_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| client_id_from_scope(&scope));

    let refreshed = refresh_access(&issuer, &client_id, refresh)?;
    apply_token_response(&mut entry, &issuer, &client_id, &refreshed)?;
    file.insert(scope.clone(), entry.clone());
    write_auth_file_to_store(store, name, &file, true)?;

    let token = entry
        .access_token
        .clone()
        .filter(|t| !t.trim().is_empty())
        .context("ChatGPT OAuth refresh returned an empty access token")?;
    Ok(credentials_from_entry(&entry, token))
}

#[must_use]
pub fn missing_auth_message() -> String {
    format!(
        "OpenAI Codex OAuth credentials are unavailable.\n\
         \n\
         Sign in with ChatGPT (subscription billing, Codewhale-owned tokens):\n\
         `codewhale auth chatgpt` or /provider setup openai-codex.\n\
         The openai API-key route is a different billing owner.\n\
         \n\
         Alternatives:\n\
         - Process token: OPENAI_CODEX_ACCESS_TOKEN / CODEX_ACCESS_TOKEN\n\
         - Explicit Codex CLI import (not a prerequisite): after `codex login`, run \
         `codewhale auth external-consent --provider openai-codex --mode read-only --path {}`\n\
         Read-only access never refreshes or rewrites the Codex CLI file.\n\
         Revoke Codewhale-owned tokens with `codewhale auth chatgpt-revoke`.",
        codewhale_config::quote_os_path(&crate::oauth::auth_file_path())
    )
}

type AuthFile = BTreeMap<String, ChatgptAuthEntry>;

fn load_owned_auth_file(path: &Path) -> Result<Option<AuthFile>> {
    let Some(raw) = crate::external_credentials::read_codewhale_owned_to_string(path)? else {
        return Ok(None);
    };
    parse_auth_file(&raw, path).map(Some)
}

fn load_owned_auth_file_from_store(
    store: &codewhale_config::XaiOAuthCredentialStore,
    name: &str,
) -> Result<Option<AuthFile>> {
    let Some(raw) = store.read_to_string(name)? else {
        return Ok(None);
    };
    parse_auth_file(&raw, &store.path_for(name)?).map(Some)
}

fn parse_auth_file(raw: &str, path: &Path) -> Result<AuthFile> {
    let value: Value = serde_json::from_str(raw).map_err(|_| {
        anyhow::anyhow!(
            "ChatGPT credential file {} is not valid credential JSON",
            codewhale_config::quote_os_path(path)
        )
    })?;
    let obj = value.as_object().ok_or_else(|| {
        anyhow::anyhow!(
            "ChatGPT credential file {} must be a JSON object of entries",
            codewhale_config::quote_os_path(path)
        )
    })?;
    let mut out = BTreeMap::new();
    for (k, v) in obj {
        match serde_json::from_value::<ChatgptAuthEntry>(v.clone()) {
            Ok(entry) => {
                out.insert(k.clone(), entry);
            }
            Err(_) => {
                tracing::warn!(
                    target: "codewhale::chatgpt_oauth",
                    "skipping unreadable ChatGPT auth entry"
                );
            }
        }
    }
    Ok(out)
}

fn write_auth_file_to_store(
    store: &codewhale_config::XaiOAuthCredentialStore,
    name: &str,
    file: &AuthFile,
    allow_replace: bool,
) -> Result<()> {
    let serialized =
        serde_json::to_vec_pretty(file).context("serializing ChatGPT OAuth credentials")?;
    store
        .write(name, &serialized, allow_replace)
        .with_context(|| {
            format!(
                "writing ChatGPT OAuth credentials to {}",
                codewhale_config::quote_os_path(&store.directory().join(name))
            )
        })?;
    #[cfg(test)]
    crate::external_credentials::record_owned_credential_write();
    Ok(())
}

fn select_entry(file: &mut AuthFile) -> Option<(String, ChatgptAuthEntry)> {
    let preferred_suffix = format!("::{CHATGPT_OAUTH_CLIENT_ID}");
    if let Some((k, v)) = file
        .iter()
        .find(|(k, e)| k.ends_with(&preferred_suffix) && entry_has_usable_secret(e))
    {
        return Some((k.clone(), v.clone()));
    }
    file.iter()
        .find(|(_, e)| entry_has_usable_secret(e))
        .map(|(k, v)| (k.clone(), v.clone()))
}

fn entry_has_usable_secret(entry: &ChatgptAuthEntry) -> bool {
    entry
        .access_token
        .as_deref()
        .is_some_and(|t| !t.trim().is_empty())
        || entry
            .refresh_token
            .as_deref()
            .is_some_and(|t| !t.trim().is_empty())
}

fn entry_access_token_is_fresh(entry: &ChatgptAuthEntry) -> bool {
    let Some(token) = entry
        .access_token
        .as_deref()
        .filter(|t| !t.trim().is_empty())
    else {
        return false;
    };
    if let Some(exp) = entry.expires_at.as_deref().and_then(parse_rfc3339_secs) {
        let now = now_unix_secs().unwrap_or(0);
        return exp - now > REFRESH_SKEW_SECS;
    }
    match jwt_expiry_seconds(token) {
        Some(exp) => {
            let now = now_unix_secs().unwrap_or(0) as u64;
            (exp as i64) - (now as i64) > REFRESH_SKEW_SECS
        }
        None => false,
    }
}

fn credentials_from_entry(
    entry: &ChatgptAuthEntry,
    access_token: String,
) -> ChatgptOAuthCredentials {
    ChatgptOAuthCredentials {
        access_token,
        account_id: entry.account_id.clone(),
        refresh_token: entry.refresh_token.clone(),
        expires_at: entry.expires_at.clone(),
    }
}

fn issuer_from_scope(scope: &str) -> String {
    scope
        .split_once("::")
        .map(|(issuer, _)| issuer.to_string())
        .unwrap_or_else(|| CHATGPT_OAUTH_ISSUER.to_string())
}

fn client_id_from_scope(scope: &str) -> String {
    scope
        .split_once("::")
        .map(|(_, id)| id.to_string())
        .unwrap_or_else(|| CHATGPT_OAUTH_CLIENT_ID.to_string())
}

fn apply_token_response(
    entry: &mut ChatgptAuthEntry,
    issuer: &str,
    client_id: &str,
    token: &crate::oauth::OAuthTokenMaterial,
) -> Result<()> {
    let access = token
        .access_token
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .context("token response missing access_token")?;
    entry.access_token = Some(access.to_string());
    if let Some(rt) = token
        .refresh_token
        .as_deref()
        .filter(|t| !t.trim().is_empty())
    {
        entry.refresh_token = Some(rt.to_string());
    }
    entry.oidc_issuer = Some(issuer.to_string());
    entry.oidc_client_id = Some(client_id.to_string());
    entry.originator = Some(CHATGPT_OAUTH_ORIGINATOR.to_string());
    if let Some(id_token) = token.id_token.clone() {
        if let Some(account_id) = account_id_from_id_token(&id_token) {
            entry.account_id = Some(account_id);
        }
        entry.id_token = Some(id_token);
    }
    if let Some(expires_in) = token.expires_in {
        entry.expires_at = Some(rfc3339_from_now(expires_in));
    } else if let Some(exp) = jwt_expiry_seconds(access) {
        entry.expires_at = Some(rfc3339_from_unix(exp as i64));
    }
    Ok(())
}

fn jwt_expiry_seconds(token: &str) -> Option<u64> {
    jwt_payload(token)?.get("exp")?.as_u64()
}

fn account_id_from_id_token(token: &str) -> Option<String> {
    let payload = jwt_payload(token)?;
    if let Some(id) = payload.get("chatgpt_account_id").and_then(Value::as_str) {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    payload
        .get("https://api.openai.com/auth")
        .and_then(|auth| auth.get("chatgpt_account_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned)
}

fn jwt_payload(token: &str) -> Option<Value> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&decoded).ok()
}

fn now_unix_secs() -> Option<i64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
}

fn parse_rfc3339_secs(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.timestamp())
}

fn rfc3339_from_now(expires_in: u64) -> String {
    let ts = now_unix_secs().unwrap_or(0) + expires_in as i64;
    rfc3339_from_unix(ts)
}

fn rfc3339_from_unix(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_else(|| format!("{ts}"))
}

#[cfg(test)]
pub(crate) fn pending_pkce_login_for_test(
    access_token: &str,
    refresh_token: &str,
    id_token: Option<&str>,
) -> PendingChatgptPkceLogin {
    pending_from_unified(crate::oauth::PendingOAuthLogin {
        provider: crate::oauth::OAuthProvider::Chatgpt,
        issuer: CHATGPT_OAUTH_ISSUER.to_string(),
        client_id: CHATGPT_OAUTH_CLIENT_ID.to_string(),
        token: crate::oauth::OAuthTokenMaterial {
            access_token: Some(access_token.to_string()),
            refresh_token: Some(refresh_token.to_string()),
            expires_in: Some(3600),
            id_token: id_token.map(ToOwned::to_owned),
            interval: None,
            error: None,
            error_description: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    type MockForm = Vec<(String, String)>;
    type MockPost = (String, MockForm);

    struct MockTokenClient {
        responses: Mutex<Vec<(u16, String)>>,
        posts: Mutex<Vec<MockPost>>,
    }

    impl MockTokenClient {
        fn new(responses: Vec<(u16, String)>) -> Self {
            Self {
                responses: Mutex::new(responses),
                posts: Mutex::new(Vec::new()),
            }
        }
    }

    impl crate::oauth::OAuthFormClient for MockTokenClient {
        fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<(u16, String)> {
            self.posts.lock().expect("posts").push((
                url.to_string(),
                form.iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
            ));
            let mut responses = self.responses.lock().expect("responses");
            anyhow::ensure!(
                !responses.is_empty(),
                "mock issuer has no remaining responses"
            );
            Ok(responses.remove(0))
        }
    }

    fn jwt_with_exp(exp: u64) -> String {
        let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{exp}}}"#));
        format!("header.{payload}.sig")
    }

    fn jwt_with_account(account: &str) -> String {
        let payload = URL_SAFE_NO_PAD.encode(format!(
            r#"{{"https://api.openai.com/auth":{{"chatgpt_account_id":"{account}"}}}}"#
        ));
        format!("header.{payload}.sig")
    }

    #[test]
    fn store_persist_refresh_and_revoke_use_mock_issuer() {
        let _lock = crate::test_support::lock_test_env();
        let home = tempfile::tempdir().expect("temp home");
        let root = home.path().canonicalize().expect("canonical home");
        let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", &root);
        let config_path = root.join("config.toml");
        std::fs::write(&config_path, "").expect("empty config");

        let pending =
            pending_pkce_login_for_test("access-1", "refresh-1", Some(&jwt_with_account("acct-7")));
        let activation = activate_pkce_login(pending, Some(&config_path), None).expect("activate");
        assert!(activation.auth_path.exists());
        let persisted = std::fs::read_to_string(root.join("config.toml")).expect("config");
        assert!(persisted.contains("chatgpt-auth-"));
        assert!(persisted.contains("auth_mode = \"oauth\""));
        assert!(!persisted.contains("access-1"), "{persisted}");

        let generation = toml::from_str::<toml::Value>(&persisted)
            .unwrap()["providers"]["openai_codex"]["oauth_credential_generation"]
            .as_str()
            .unwrap()
            .to_string();
        let mut config = Config {
            provider: Some(ApiProvider::OpenaiCodex.as_str().to_string()),
            ..Config::default()
        };
        config.mark_codewhale_owned_chatgpt_oauth(generation.clone());
        assert!(credentials_valid(&config));

        let stale = jwt_with_exp(1_000_000_000);
        let scope = format!("{CHATGPT_OAUTH_ISSUER}::{CHATGPT_OAUTH_CLIENT_ID}");
        let raw = serde_json::json!({
            &scope: {
                "access_token": stale,
                "refresh_token": "refresh-old",
                "expires_at": "2000-01-01T00:00:00Z",
                "oidc_issuer": CHATGPT_OAUTH_ISSUER,
                "oidc_client_id": CHATGPT_OAUTH_CLIENT_ID,
                "originator": CHATGPT_OAUTH_ORIGINATOR
            }
        });
        codewhale_config::with_xai_oauth_lifecycle_lock(|store| {
            store.write(
                &generation,
                serde_json::to_vec_pretty(&raw).unwrap().as_slice(),
                true,
            )
        })
        .unwrap();

        let mock = MockTokenClient::new(vec![(
            200,
            serde_json::json!({
                "access_token": "access-2",
                "refresh_token": "refresh-2",
                "expires_in": 3600
            })
            .to_string(),
        )]);
        let refreshed = get_owned_credentials_with(&config, &mock).expect("refresh");
        assert_eq!(refreshed.access_token, "access-2");
        let stored = codewhale_config::with_xai_oauth_lifecycle_lock(|store| {
            store.read_to_string(&generation)
        })
        .unwrap()
        .unwrap();
        assert!(stored.contains("refresh-2"), "{stored}");
        assert!(!stored.contains("refresh-old"), "{stored}");

        let revoke_mock = MockTokenClient::new(vec![(200, String::new())]);
        codewhale_config::with_xai_oauth_lifecycle_lock(|store| {
            revoke_owned_login_locked(Some(&config_path), None, store, &revoke_mock)
        })
        .expect("revoke");
        let after = std::fs::read_to_string(&config_path).expect("config after revoke");
        assert!(!after.contains("chatgpt-auth-"), "{after}");
        let posts = revoke_mock.posts.lock().unwrap();
        assert!(
            posts.iter().any(|(url, _)| url.contains("/oauth/revoke")),
            "{posts:?}"
        );
    }

    #[test]
    fn debug_impls_redact_secrets() {
        let entry = ChatgptAuthEntry {
            access_token: Some("secret-access".into()),
            refresh_token: Some("secret-refresh".into()),
            expires_at: None,
            id_token: Some("secret-id".into()),
            account_id: None,
            oidc_issuer: None,
            oidc_client_id: None,
            originator: None,
            extra: BTreeMap::new(),
        };
        let rendered = format!("{entry:?}");
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("secret-access"));
        assert!(!rendered.contains("secret-refresh"));
        assert!(!rendered.contains("secret-id"));

        let credentials = ChatgptOAuthCredentials {
            access_token: "secret-access".into(),
            account_id: None,
            refresh_token: Some("secret-refresh".into()),
            expires_at: None,
        };
        let rendered = format!("{credentials:?}");
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("secret-access"));
        assert!(!rendered.contains("secret-refresh"));
    }
}
