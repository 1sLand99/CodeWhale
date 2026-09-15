use axum::Json;
use axum::extract::{Path, State};
use codewhale_config::ConfigStore;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::ApiProvider;

use super::{ApiError, ProviderCredentialState, RuntimeApiState};

/// Largest accepted credential payload. Provider keys are single-line
/// tokens; anything larger is a mistake, not a longer secret.
const MAX_KEY_BYTES: usize = 4 * 1024;

/// Request body cap for the key route — the key plus JSON framing.
pub(super) const PROVIDER_KEY_BODY_LIMIT_BYTES: usize = MAX_KEY_BYTES + 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SetProviderKeyRequest {
    key: String,
}

/// Write-only credential entry for native clients (APPS-48).
///
/// `PUT /v1/providers/{id}/key` accepts `{ "key": "…" }`, persists it through
/// the same transactional path as `codewhale auth set` (secret backend plus
/// plaintext-free config metadata, rolled back together on failure), and
/// answers with the redacted receipt: which backend holds the secret and the
/// post-write `credential_state` readback. The key itself — and even its
/// length — never appears in the response, in errors, or in logs.
///
/// There is deliberately no GET: a route that can return a secret can leak
/// one. Clients needing assurance re-read `credential_state` here or on
/// `GET /v1/providers`.
pub(super) async fn set_provider_key(
    State(state): State<RuntimeApiState>,
    Path(id): Path<String>,
    Json(request): Json<SetProviderKeyRequest>,
) -> Result<Json<Value>, ApiError> {
    let provider = ApiProvider::parse(&id)
        .ok_or_else(|| ApiError::bad_request(format!("Unknown provider id '{id}'")))?;
    if provider == ApiProvider::DeepseekCN {
        return Err(ApiError::bad_request(
            "provider 'deepseek-cn' is a legacy alias; use 'deepseek' instead",
        ));
    }
    let kind = provider
        .kind()
        .ok_or_else(|| ApiError::bad_request("provider has no credential slot"))?;

    let key = request.key;
    let key = key.trim();
    if key.is_empty() {
        return Err(ApiError::bad_request("key must not be empty"));
    }
    if key.len() > MAX_KEY_BYTES || key.chars().any(char::is_control) {
        return Err(ApiError::bad_request(
            "key must be a single-line credential at most 4 KiB",
        ));
    }

    let secrets = crate::config::credential_secret_store().ok_or_else(|| {
        ApiError::internal("no credential store is available in this environment")
    })?;

    let store_path = state.config_path.clone();
    let kind_owned = kind;
    let key_owned = key.to_string();
    let provider_owned = provider;
    let (backend, saved_config_path) = tokio::task::spawn_blocking(move || {
        let mut store = ConfigStore::load(store_path)
            .map_err(|error| ApiError::internal(format!("config store unavailable: {error}")))?;
        let mut credential_store = codewhale_config::credentials::credential_metadata_store(&store)
            .map_err(|error| ApiError::internal(format!("credential store: {error}")))?;
        let target = credential_store.as_mut().unwrap_or(&mut store);
        let slot = codewhale_config::credentials::provider_slot(kind_owned);
        crate::credentials::store::with_provider_write_lock(slot, || {
            codewhale_config::credentials::set_provider_api_key(
                target, &secrets, kind_owned, &key_owned,
            )
        })
        .map_err(|error| {
            // The credential-write errors name paths and backends only — the
            // key material is never embedded in the message.
            ApiError::internal(format!("credential write failed: {error}"))
        })?;
        Ok::<_, ApiError>((
            secrets.backend_name().to_string(),
            target.path().to_path_buf(),
        ))
    })
    .await
    .map_err(|_| ApiError::internal("credential write task failed"))??;

    // Mirror the persisted credential markers into the live config. The
    // durable write may have landed on the user-global document while this
    // server's ambient config is workspace-scoped, and `credential_state`
    // only probes the secret store for an inactive provider when the
    // `auth_mode` save marker is visible — without this mirror
    // `GET /v1/providers` would keep reporting the provider as missing its
    // credential until the next process start. Only marker fields are
    // mirrored; the key itself never enters the runtime config.
    {
        let mut config = state.config.write();
        config.auth_mode = Some("api_key".to_string());
        {
            let entry = config.provider_config_for_mut(provider_owned);
            entry.auth_mode = Some("api_key".to_string());
            entry.external_credentials = None;
            entry.api_key = None;
            if provider_owned == ApiProvider::Xai {
                entry.oauth_credential_generation = None;
            }
        }
        if provider_owned == ApiProvider::Deepseek {
            config.api_key = None;
            if config.default_text_model.is_none() {
                config.default_text_model = config
                    .provider_config_for(ApiProvider::Deepseek)
                    .and_then(|entry| entry.model.clone())
                    .or_else(|| Some("deepseek-v4-pro".to_string()));
            }
        }
    }

    let credential_state: ProviderCredentialState =
        crate::provider_readiness::credential_state_for_provider(
            &state.config.read(),
            provider_owned,
        )
        .into();

    Ok(Json(json!({
        "provider": provider_owned.as_str(),
        "stored": true,
        "backend": backend,
        "credentialState": credential_state,
        "configPath": saved_config_path,
    })))
}
