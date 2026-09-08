//! Durable CLI route edits use the same Config owner as Runtime and the TUI.
//! `model` and the legacy `default_text_model` address the saved active route;
//! `default_model` addresses DeepSeek. Project root keys retain their scope.

use std::path::Path;

use anyhow::{Context, Result, ensure};

use crate::config::{ApiProvider, Config, ProviderIdentity};
use crate::config_persistence as persistence;

/// Whether a config key names a provider or model selection.
pub fn is_route_key(key: &str) -> bool {
    matches!(
        key,
        "provider" | "model" | "default_model" | "default_text_model"
    ) || provider_model_id(key).is_some()
}

fn provider_model_id(key: &str) -> Option<&str> {
    key.strip_prefix("providers.")?
        .strip_suffix(".model")
        .filter(|id| !id.is_empty())
}

fn parse_config(body: &str) -> Result<Config> {
    toml::from_str(body)
        .map_err(|_| anyhow::anyhow!("Could not parse route configuration; contents omitted"))
}

fn model_identity(config: &Config, key: &str) -> Result<ProviderIdentity> {
    if key == "default_model" {
        config.resolve_provider_pin_identity("deepseek")
    } else if let Some(id) = provider_model_id(key) {
        // Leaf keys name TOML tables, whose canonical spelling can differ
        // from the public provider selector. Exact custom tables still win.
        let selector = if config
            .providers
            .as_ref()
            .and_then(|providers| providers.custom_provider_config(id))
            .is_some()
        {
            id
        } else if id == "deepseek_cn" {
            ApiProvider::DeepseekCN.as_str()
        } else {
            ApiProvider::all()
                .iter()
                .find(|provider| {
                    provider
                        .metadata()
                        .is_some_and(|metadata| metadata.provider_config_key() == id)
                })
                .map_or(id, |provider| provider.as_str())
        };
        config.resolve_provider_pin_identity(selector)
    } else {
        config.active_provider_identity(config.api_provider())
    }
    .map_err(anyhow::Error::msg)
}

fn model_slot(identity: &ProviderIdentity) -> Result<Vec<&str>> {
    if identity.provider == ApiProvider::Custom && identity.persisted_id().is_none() {
        return Ok(vec!["default_text_model"]);
    }
    let key = match identity.provider {
        ApiProvider::Custom => identity.key.as_str(),
        ApiProvider::DeepseekCN => "deepseek_cn",
        ApiProvider::OllamaCloud if identity.migrated_legacy_ollama_cloud_route => "ollama",
        provider => provider
            .metadata()
            .context("provider model table")?
            .provider_config_key(),
    };
    Ok(vec!["providers", key, "model"])
}

/// Locate the canonical model leaf in a saved document without applying
/// device preferences or launch overrides. Export uses this same identity
/// resolution to omit only root aliases shadowed by that leaf.
pub fn model_slot_for_document(body: &str, key: &str) -> Result<Vec<String>> {
    ensure!(
        is_route_key(key) && key != "provider",
        "Not a model preference key: {key}"
    );
    let config = parse_config(body)?;
    let identity = model_identity(&config, key)?;
    Ok(model_slot(&identity)?
        .into_iter()
        .map(str::to_string)
        .collect())
}

fn project_root_key<'a>(path: &Path, key: &'a str) -> Option<&'a str> {
    (codewhale_config::config_path_is_workspace_scoped(path)
        && matches!(key, "model" | "default_text_model"))
    .then_some(key)
}

fn saved_config(store: &codewhale_config::ConfigStore) -> Result<Config> {
    let rendered;
    let body = if let Some(original) = store.original_body() {
        original
    } else {
        rendered = store.rendered_body()?;
        &rendered
    };
    let mut config = parse_config(body)?;
    if config.route_preferences_version.is_none()
        && crate::config::is_home_config_path(store.path())
    {
        config.apply_saved_selection(
            &crate::settings::Settings::load_legacy_route_preferences_read_only()?,
        );
    }
    Ok(config)
}

/// Read the saved route without applying launch overrides or credentials.
pub fn get(path: &Path, key: &str) -> Result<Option<String>> {
    ensure!(is_route_key(key), "Not a route preference key: {key}");
    let store = codewhale_config::ConfigStore::load(Some(path.to_path_buf()))?;
    if let Some(key) = project_root_key(store.path(), key) {
        return Ok(store.config.get_value(key));
    }
    let mut config = saved_config(&store)?;
    if key == "provider" {
        return Ok(Some(
            config.provider.unwrap_or_else(|| "deepseek".to_string()),
        ));
    }
    let identity = model_identity(&config, key)?;
    config.scope_to_provider_identity(&identity);
    Ok(config
        .provider_config_for(identity.provider)
        .and_then(|entry| entry.model.clone())
        .or_else(|| {
            (provider_model_id(key).is_none()
                && (config.default_text_model.is_some() || config.legacy_model.is_some()))
            .then(|| config.default_model())
        }))
}

/// One saved snapshot for CLI route reports, including exact legacy identities.
/// The source distinguishes an explicit model from the provider default.
pub fn selected_route(path: &Path) -> Result<(String, String, codewhale_config::ModelSource)> {
    let store = codewhale_config::ConfigStore::load(Some(path.to_path_buf()))?;
    let config = saved_config(&store)?;
    let identity = config
        .active_provider_identity(config.api_provider())
        .map_err(anyhow::Error::msg)?;
    let provider = identity.key;
    let source = if config
        .provider_config_for(identity.provider)
        .and_then(|entry| entry.model.as_ref())
        .is_some()
    {
        codewhale_config::ModelSource::ProviderConfig
    } else if config.default_text_model.is_some() || config.legacy_model.is_some() {
        if store.config.default_text_model.is_none() && store.config.model.is_some() {
            codewhale_config::ModelSource::RootModel
        } else {
            codewhale_config::ModelSource::RootDefaultTextModel
        }
    } else {
        codewhale_config::ModelSource::ProviderDefault
    };
    Ok((provider, config.default_model(), source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{EnvVarGuard, lock_test_env};

    fn document(path: &Path) -> toml::Value {
        toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn route_edits_adopt_legacy_selection_once_and_preserve_settings() -> Result<()> {
        let _env = lock_test_env();
        let home = tempfile::tempdir()?;
        let _home = EnvVarGuard::set("CODEWHALE_HOME", home.path());
        let _path = EnvVarGuard::remove("CODEWHALE_CONFIG_PATH");
        let _legacy_path = EnvVarGuard::remove("DEEPSEEK_CONFIG_PATH");
        let path = home.path().join("config.toml");
        std::fs::write(
            &path,
            "provider = \"deepseek\"\ndefault_text_model = \"deepseek-v4-pro\"\n[providers.zai]\nmodel = \"GLM-5.2\"\n",
        )?;
        let settings_path = home.path().join("settings.toml");
        let settings = "default_provider = \"zai\"\n[provider_models]\nzai = \"GLM-5.3\"\n";
        std::fs::write(&settings_path, settings)?;
        let before = std::fs::read(&path)?;
        assert_eq!(get(&path, "provider")?.as_deref(), Some("zai"));
        assert_eq!(get(&path, "model")?.as_deref(), Some("GLM-5.3"));
        assert_eq!(std::fs::read(&path)?, before);

        let mut candidate = prepare_document(&path, &std::fs::read_to_string(&path)?)?;
        set_document(&path, &mut candidate, "model", "GLM-5.2")?;
        assert_eq!(std::fs::read(&path)?, before);
        set(&path, "model", "GLM-5.2")?;
        assert_eq!(
            document(&path),
            toml::from_str::<toml::Value>(&candidate.to_string())?
        );
        assert_eq!(
            document(&path)["route_preferences_version"].as_integer(),
            Some(1)
        );
        assert_eq!(
            get(&path, "default_text_model")?.as_deref(),
            Some("GLM-5.2")
        );
        assert_eq!(
            get(&path, "providers.zai.model")?.as_deref(),
            Some("GLM-5.2")
        );
        set(&path, "default_model", "deepseek-v4-flash")?;
        assert_eq!(
            get(&path, "default_model")?.as_deref(),
            Some("deepseek-v4-flash")
        );
        set(&path, "provider", "deepseek")?;
        assert_eq!(get(&path, "model")?.as_deref(), Some("deepseek-v4-flash"));
        unset(&path, "providers.deepseek.model")?;
        assert!(get(&path, "default_model")?.is_none());
        unset(&path, "providers.zai.model")?;
        assert!(get(&path, "providers.zai.model")?.is_none());
        unset(&path, "provider")?;
        assert_eq!(get(&path, "provider")?.as_deref(), Some("deepseek"));
        assert_eq!(std::fs::read_to_string(settings_path)?, settings);
        // An unrelated typed store write must preserve the migration receipt.
        let mut store = codewhale_config::ConfigStore::load(Some(path.clone()))?;
        store.config.set_value("verbosity", "quiet")?;
        store.save()?;
        assert_eq!(
            document(&path)["route_preferences_version"].as_integer(),
            Some(1)
        );
        Ok(())
    }

    #[test]
    fn route_edits_keep_exact_named_provider_keys_and_reject_unknown_routes() -> Result<()> {
        let _env = lock_test_env();
        let home = tempfile::tempdir()?;
        let _home = EnvVarGuard::set("CODEWHALE_HOME", home.path());
        let _path = EnvVarGuard::remove("CODEWHALE_CONFIG_PATH");
        let _legacy_path = EnvVarGuard::remove("DEEPSEEK_CONFIG_PATH");
        let path = home.path().join("config.toml");
        std::fs::write(
            &path,
            r#"provider = "Team.A"
[providers."Team.A"]
kind = "openai-compatible"
base_url = "http://127.0.0.1:9/v1"
model = "Model-X"
[providers."team.a"]
kind = "openai-compatible"
base_url = "http://127.0.0.1:10/v1"
model = "Other-X"
"#,
        )?;
        set(&path, "providers.Team.A.model", "Model-Y")?;
        assert_eq!(get(&path, "model")?.as_deref(), Some("Model-Y"));
        assert_eq!(
            get(&path, "providers.team.a.model")?.as_deref(),
            Some("Other-X")
        );
        let before = std::fs::read(&path)?;
        assert!(set(&path, "providers.TEAM.A.model", "Model-Z").is_err());
        assert!(set(&path, "provider", "unconfigured-route").is_err());
        assert_eq!(std::fs::read(&path)?, before);
        unset(&path, "providers.Team.A.model")?;
        assert!(get(&path, "providers.Team.A.model")?.is_none());
        assert_eq!(
            document(&path)["providers"]["team.a"]["model"].as_str(),
            Some("Other-X")
        );
        Ok(())
    }

    #[test]
    fn project_model_edits_keep_root_fields_and_skip_device_migration() -> Result<()> {
        let _env = lock_test_env();
        let root = tempfile::tempdir()?;
        let home = root.path().join("home");
        std::fs::create_dir_all(&home)?;
        let _home = EnvVarGuard::set("CODEWHALE_HOME", &home);
        let _path = EnvVarGuard::remove("CODEWHALE_CONFIG_PATH");
        let _legacy_path = EnvVarGuard::remove("DEEPSEEK_CONFIG_PATH");
        std::fs::write(home.join("settings.toml"), "default_provider = \"zai\"\n")?;
        let project = root.path().join("project/.codewhale");
        std::fs::create_dir_all(&project)?;
        let path = project.join("config.toml");
        std::fs::write(&path, "model = \"project-old\"\n")?;
        set(&path, "model", "project-new")?;
        assert_eq!(get(&path, "model")?.as_deref(), Some("project-new"));
        assert!(document(&path).get("providers").is_none());
        assert!(document(&path).get("route_preferences_version").is_none());
        unset(&path, "model")?;
        assert!(document(&path).get("model").is_none());
        Ok(())
    }

    #[test]
    fn saved_routes_preserve_regional_and_legacy_table_identity() -> Result<()> {
        let _env = lock_test_env();
        let home = tempfile::tempdir()?;
        let _home = EnvVarGuard::set("CODEWHALE_HOME", home.path());
        let _path = EnvVarGuard::remove("CODEWHALE_CONFIG_PATH");
        let _legacy_path = EnvVarGuard::remove("DEEPSEEK_CONFIG_PATH");
        let _cloud = EnvVarGuard::set("CODEWHALE_DISABLE_CLOUD_FACTS", "1");
        let _overrides: Vec<_> = [
            "CODEWHALE_MODEL",
            "DEEPSEEK_MODEL",
            "DEEPSEEK_DEFAULT_TEXT_MODEL",
            "CODEWHALE_PROVIDER",
            "DEEPSEEK_PROVIDER",
            "CODEWHALE_BASE_URL",
            "DEEPSEEK_BASE_URL",
            "CODEWHALE_PROFILE",
            "DEEPSEEK_PROFILE",
            "ZAI_MODEL",
            "ZAI_BASE_URL",
            "OLLAMA_MODEL",
            "OLLAMA_CLOUD_MODEL",
            "OLLAMA_BASE_URL",
            "OLLAMA_CLOUD_BASE_URL",
        ]
        .into_iter()
        .map(EnvVarGuard::remove)
        .collect();
        let path = home.path().join("config.toml");
        for (provider, table, endpoint, model) in [
            (
                "deepseek-cn",
                "deepseek_cn",
                "https://api.deepseek.cn",
                "deepseek-v4-flash",
            ),
            (
                "ollama",
                "ollama",
                "https://ollama.com/v1",
                "saved-cloud-model",
            ),
        ] {
            std::fs::write(
                &path,
                format!(
                    "route_preferences_version = 1\nprovider = '{provider}'\n[providers.{table}]\nbase_url = '{endpoint}'\n"
                ),
            )?;
            set(&path, "model", model)?;
            assert_eq!(document(&path)["provider"].as_str(), Some(provider));
            assert_eq!(
                document(&path)["providers"][table]["model"].as_str(),
                Some(model)
            );
            assert_eq!(get(&path, "model")?.as_deref(), Some(model));
            assert_eq!(
                selected_route(&path)?,
                (
                    if provider == "ollama" {
                        "ollama-cloud"
                    } else {
                        provider
                    }
                    .to_string(),
                    model.to_string(),
                    codewhale_config::ModelSource::ProviderConfig
                )
            );
            unset(&path, "model")?;
            assert!(document(&path)["providers"][table].get("model").is_none());
            let leaf = format!("providers.{table}.model");
            set(&path, &leaf, model)?;
            assert_eq!(get(&path, &leaf)?.as_deref(), Some(model));
            assert_eq!(
                Config::load(Some(path.clone()), None)?.default_model(),
                model
            );
            assert_eq!(document(&path)["provider"].as_str(), Some(provider));
            unset(&path, &leaf)?;
            assert!(get(&path, &leaf)?.is_none());
            assert!(document(&path)["providers"][table].get("model").is_none());
        }

        std::fs::write(
            &path,
            "route_preferences_version = 1\nprovider = 'zai'\nmodel = 'GLM-5.3'\n",
        )?;
        assert_eq!(get(&path, "model")?.as_deref(), Some("GLM-5.3"));
        assert_eq!(
            Config::load(Some(path.clone()), None)?.default_model(),
            "GLM-5.3"
        );
        assert_eq!(
            selected_route(&path)?.2,
            codewhale_config::ModelSource::RootModel
        );
        // A foreign active-route root must never become the DeepSeek default.
        assert_eq!(
            get(&path, "default_model")?.as_deref(),
            Some(crate::config::DEFAULT_TEXT_MODEL)
        );
        unset(&path, "providers.zai.model")?;
        assert!(document(&path).get("model").is_none());
        assert_eq!(
            Config::load(Some(path.clone()), None)?.default_model(),
            selected_route(&path)?.1
        );

        for (provider, root, leaf) in [
            (
                "deepseek",
                "default_text_model = 'deepseek-v4-flash'\nmodel = 'deepseek-v4-flash-vision-exp'",
                "deepseek-v4-pro",
            ),
            (
                "zai",
                "default_text_model = 'GLM-5.1'\nmodel = 'GLM-5.2'",
                "GLM-5.3",
            ),
        ] {
            std::fs::write(
                &path,
                format!(
                    "route_preferences_version = 1\nprovider = '{provider}'\n{root}\n[providers.{provider}]\nmodel = '{leaf}'\n"
                ),
            )?;
            assert_eq!(
                Config::load(Some(path.clone()), None)?.default_model(),
                leaf
            );
            unset(&path, &format!("providers.{provider}.model"))?;
            let doc = document(&path);
            assert!(doc.get("model").is_none());
            assert!(doc.get("default_text_model").is_none());
            assert!(doc["providers"][provider].get("model").is_none());
            let loaded = Config::load(Some(path.clone()), None)?;
            assert_eq!(loaded.default_model(), selected_route(&path)?.1);
            assert!(loaded.legacy_model.is_none());
        }

        // Clearing Z.ai cannot erase the independent DeepSeek root fallback.
        std::fs::write(
            &path,
            "route_preferences_version = 1\nprovider = 'zai'\ndefault_text_model = 'deepseek-v4-flash'\nmodel = 'GLM-5.1'\n[providers.zai]\nmodel = 'GLM-5.2'\n",
        )?;
        unset(&path, "providers.zai.model")?;
        assert_eq!(
            document(&path)["default_text_model"].as_str(),
            Some("deepseek-v4-flash")
        );
        assert!(document(&path).get("model").is_none());
        let loaded = Config::load(Some(path.clone()), None)?;
        assert_ne!(loaded.default_model(), "GLM-5.1");
        assert_eq!(loaded.default_model(), selected_route(&path)?.1);
        Ok(())
    }
}

/// Save one explicit route preference after atomically adopting legacy choices.
pub fn set(path: &Path, key: &str, value: &str) -> Result<()> {
    persistence::mutate_config_document(path, |doc| set_document(path, doc, key, value))
}

/// Prepare a validated snapshot for a caller's preview and atomic save.
/// Migration changes only this document; this function never writes a file.
pub fn prepare_document(path: &Path, raw: &str) -> Result<toml_edit::DocumentMut> {
    let mut doc = raw
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| anyhow::anyhow!("Could not parse route configuration; contents omitted"))?;
    parse_config(raw)?;
    persistence::migrate_legacy_route_preferences(path, &mut doc)?;
    Ok(doc)
}

/// Edit one route selection in an already-prepared candidate without saving.
/// Callers must prepare migration first and atomically save the final snapshot.
pub fn set_document(
    path: &Path,
    doc: &mut toml_edit::DocumentMut,
    key: &str,
    value: &str,
) -> Result<()> {
    ensure!(is_route_key(key), "Not a route preference key: {key}");
    let value = value.trim();
    ensure!(
        !value.is_empty() && !value.chars().any(char::is_control),
        "Route preference must be nonempty and contain no control characters"
    );
    if let Some(key) = project_root_key(path, key) {
        return persistence::set_document_value(doc, &[key], value);
    }
    let config = parse_config(&doc.to_string())?;
    if key == "provider" {
        let identity = config
            .resolve_provider_pin_identity(value)
            .map_err(anyhow::Error::msg)?;
        return persistence::set_document_value(
            doc,
            &["provider"],
            identity.persisted_id().unwrap_or(&identity.key),
        );
    }
    let identity = model_identity(&config, key)?;
    persistence::set_provider_model_document(
        doc,
        identity.provider,
        identity.persisted_id().unwrap_or(&identity.key),
        value,
    )
}

/// Clear a canonical selection without allowing archived Settings to restore it.
pub fn unset(path: &Path, key: &str) -> Result<()> {
    ensure!(is_route_key(key), "Not a route preference key: {key}");
    persistence::mutate_config_document(path, |doc| {
        if key == "provider" || project_root_key(path, key).is_some() {
            persistence::unset_document_value(doc, &[key])?;
            return Ok(());
        }
        let config = parse_config(&doc.to_string())?;
        let identity = model_identity(&config, key)?;
        persistence::unset_document_value(doc, &model_slot(&identity)?)?;
        // Clear relevant legacy fallbacks as well, or deleting the canonical
        // leaf would restore an older choice on reload. Root fields belong to
        // the active route, with DeepSeek's historical default as an exception.
        if matches!(
            identity.provider,
            ApiProvider::Deepseek | ApiProvider::DeepseekCN
        ) || config
            .active_provider_identity(config.api_provider())
            .is_ok_and(|active| active == identity)
        {
            let mut scoped = config.clone();
            scoped.scope_to_provider_identity(&identity);
            scoped.set_provider_model_override(identity.provider, None);
            scoped.legacy_model = None;
            for root_key in ["default_text_model", "model"] {
                let Some(model) = doc.get(root_key).and_then(toml_edit::Item::as_str) else {
                    continue;
                };
                scoped.default_text_model = Some(model.to_string());
                let wire_model = crate::config::wire_model_for_provider_route(
                    identity.provider,
                    &scoped.deepseek_base_url(),
                    model,
                );
                // Reuse Config's root-model guards: a foreign DeepSeek root
                // ignored by the active vendor remains that provider's fallback.
                if scoped.default_model() == wire_model {
                    persistence::unset_document_value(doc, &[root_key])?;
                }
            }
        }
        Ok(())
    })
}
