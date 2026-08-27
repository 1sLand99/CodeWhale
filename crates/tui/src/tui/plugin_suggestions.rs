//! In-context plugin reminders: prompt matching and idle catalog polling.

use std::time::{Duration, Instant};

use crate::localization::{MessageId, tr};
use crate::plugins::recommend::{PluginNextStep, RecommendOptions, recommend_plugins_for_task};
use crate::tui::app::{App, StatusToastLevel};

const MAX_PROMPT_SUGGESTS_PER_SESSION: u8 = 2;
const CATALOG_POLL_INTERVAL: Duration = Duration::from_secs(2);

impl App {
    /// When the user sends a task that matches an installed-but-idle plugin
    /// or a locally added marketplace candidate, toast the next review step
    /// once. Never installs, trusts, or enables anything.
    pub fn maybe_nudge_plugin_for_prompt(&mut self, input: &str) -> bool {
        if self.plugin_prompt_suggest_count >= MAX_PROMPT_SUGGESTS_PER_SESSION {
            return false;
        }
        let marketplace = load_marketplace_candidates(self);
        let recommendations = recommend_plugins_for_task(
            input,
            self.plugin_registry.as_ref(),
            &marketplace,
            RecommendOptions::proactive(),
        );
        let Some(recommendation) = recommendations.into_iter().next() else {
            return false;
        };
        if self
            .plugin_prompt_suggest_names
            .contains(&recommendation.name)
        {
            return false;
        }
        let message_id = match recommendation.next_step {
            PluginNextStep::Trust => MessageId::PluginPromptSuggestTrust,
            PluginNextStep::Enable => MessageId::PluginPromptSuggestEnable,
            PluginNextStep::MarketplaceInstall { .. } => MessageId::PluginPromptSuggestMarketplace,
            PluginNextStep::AlreadyActive | PluginNextStep::Inspect => return false,
        };
        let mut message = tr(self.ui_locale, message_id).replace("{name}", &recommendation.name);
        if let PluginNextStep::MarketplaceInstall { catalog_id } = &recommendation.next_step {
            message = message.replace("{catalog}", catalog_id);
        }
        self.plugin_prompt_suggest_names.insert(recommendation.name);
        self.plugin_prompt_suggest_count = self.plugin_prompt_suggest_count.saturating_add(1);
        self.push_status_toast(message, StatusToastLevel::Info, Some(8_000));
        true
    }

    /// Cheap idle poll so on-disk plugin changes can surface between turns,
    /// not only on send. Fingerprints directories; never auto-reloads.
    pub fn maybe_poll_plugin_catalog_idle(&mut self) {
        let now = Instant::now();
        if self
            .last_plugin_catalog_poll
            .is_some_and(|seen| now.duration_since(seen) < CATALOG_POLL_INTERVAL)
        {
            return;
        }
        self.last_plugin_catalog_poll = Some(now);
        if let Some(message) = crate::plugins::plugin_reload_nudge(
            self.plugin_registry.as_ref(),
            &mut self.plugin_reload_nudge_stamp,
        ) {
            self.push_status_toast(message, StatusToastLevel::Warning, Some(8_000));
            self.needs_redraw = true;
        }
    }
}

fn load_marketplace_candidates(
    app: &App,
) -> Vec<crate::plugins::marketplace::types::MarketplaceCandidate> {
    let Some(store) = crate::plugins::marketplace::store::MarketplaceStore::open(
        app.plugin_registry.state_path(),
    ) else {
        return Vec::new();
    };
    let Ok(state) = store.load() else {
        return Vec::new();
    };
    state
        .catalogs()
        .values()
        .flat_map(|catalog| catalog.catalog.candidates.iter().cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::localization::Locale;
    use crate::tui::app::TuiOptions;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn sending_a_supabase_prompt_toasts_trust_for_an_installed_idle_plugin() {
        let _lock = crate::test_support::lock_test_env();
        let root = TempDir::new().unwrap();
        let _home =
            crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", root.path().join("home"));
        let bundle = root.path().join(".codewhale/plugins/supabase");
        fs::create_dir_all(&bundle).unwrap();
        fs::write(
            bundle.join("plugin.toml"),
            "schema_version = 1\n[plugin]\nname = \"supabase\"\nversion = \"1.0.0\"\ndescription = \"Hosted Postgres and auth\"\nkeywords = [\"supabase\"]\n",
        )
        .unwrap();
        let temp = TempDir::new().unwrap();
        let options = TuiOptions {
            config_path: Some(temp.path().join("config.toml")),
            skills_dir: temp.path().join("skills"),
            memory_path: temp.path().join("memory.md"),
            notes_path: temp.path().join("notes.txt"),
            mcp_config_path: temp.path().join("mcp.json"),
            ..crate::test_support::test_tui_options(root.path())
        };
        let discovery = crate::plugins::PluginDiscoveryContext::capture_pre_dotenv();
        let registry = discovery.registry_for_workspace(root.path());
        let mut app = App::new_with_plugin_registry(options, &Config::default(), registry);
        app.ui_locale = Locale::En;

        assert!(app.maybe_nudge_plugin_for_prompt("add supabase auth to login"));
        assert_eq!(app.status_toasts.len(), 1);
        assert!(
            app.status_toasts[0].text.contains("/plugin trust supabase"),
            "{}",
            app.status_toasts[0].text
        );
        assert!(!app.maybe_nudge_plugin_for_prompt("add supabase auth to login"));
    }
}
