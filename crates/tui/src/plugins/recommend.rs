//! Deterministic plugin suggestions for a user task.
//!
//! Ranks installed bundles and locally-added marketplace candidates. A
//! suggestion is never an install, trust, enable, or network side effect.
//! Proactive toasts must use a high `min_score` so description-only matches
//! do not nag; `/plugin suggest` can rank more loosely.

use std::collections::{BTreeMap, BTreeSet};

use crate::skills::install::{RegistryDocument, RegistryEntry};
use crate::skills::recommend::recommend_remote_skills;

use super::marketplace::types::MarketplaceCandidate;
use super::registry::PluginRegistry;
use super::types::LoadedPlugin;

const DEFAULT_LIMIT: usize = 3;
/// Keyword and name matches score 700–900; description fallbacks are ~120.
pub const PROACTIVE_MIN_SCORE: usize = 700;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecommendOptions {
    pub limit: usize,
    pub min_score: usize,
    pub include_active: bool,
}

impl Default for RecommendOptions {
    fn default() -> Self {
        Self {
            limit: DEFAULT_LIMIT,
            min_score: 0,
            include_active: true,
        }
    }
}

impl RecommendOptions {
    #[must_use]
    pub fn proactive() -> Self {
        Self {
            limit: 1,
            min_score: PROACTIVE_MIN_SCORE,
            include_active: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginMatchSource {
    Installed { id: String },
    Marketplace { catalog_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginNextStep {
    AlreadyActive,
    Trust,
    Enable,
    Inspect,
    MarketplaceInstall { catalog_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginTaskRecommendation {
    pub name: String,
    pub source: PluginMatchSource,
    pub matched_terms: Vec<String>,
    pub score: usize,
    pub next_step: PluginNextStep,
}

impl PluginTaskRecommendation {
    #[must_use]
    pub fn command(&self) -> String {
        match &self.next_step {
            PluginNextStep::AlreadyActive | PluginNextStep::Inspect => {
                format!("/plugin show {}", self.name)
            }
            PluginNextStep::Trust => format!("/plugin trust {}", self.name),
            PluginNextStep::Enable => format!("/plugin enable {}", self.name),
            PluginNextStep::MarketplaceInstall { catalog_id } => {
                format!("/plugin marketplace install {catalog_id} {}", self.name)
            }
        }
    }
}

#[must_use]
pub fn recommend_plugins_for_task(
    task: &str,
    registry: &PluginRegistry,
    marketplace: &[MarketplaceCandidate],
    options: RecommendOptions,
) -> Vec<PluginTaskRecommendation> {
    let installed = registry.list();
    let installed_names = installed
        .iter()
        .map(|plugin| plugin.name().to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let mut entries = Vec::new();
    for plugin in &installed {
        entries.push(index_entry_from_installed(plugin));
    }
    for candidate in marketplace {
        if candidate.has_errors() {
            continue;
        }
        if installed_names.contains(&candidate.name.to_ascii_lowercase()) {
            continue;
        }
        entries.push(index_entry_from_marketplace(candidate));
    }
    recommend_from_entries(task, &entries, &installed, options)
}

fn index_entry_from_installed(plugin: &LoadedPlugin) -> (String, RegistryEntry) {
    let mut keywords = plugin.manifest.plugin.keywords.clone();
    keywords.push(plugin.name().to_string());
    let mut description_parts = plugin
        .manifest
        .plugin
        .description
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    for skill in &plugin.skill_snapshots {
        description_parts.push(skill.name.clone());
        description_parts.push(skill.description.clone());
        keywords.push(skill.name.clone());
        keywords.extend(skill.aliases.iter().cloned());
    }
    (
        format!("installed:{}", plugin.name()),
        RegistryEntry {
            source: plugin.id.as_str().to_string(),
            description: (!description_parts.is_empty()).then(|| description_parts.join(" ")),
            keywords,
            domains: plugin.inventory.network_hosts.clone(),
        },
    )
}

fn index_entry_from_marketplace(candidate: &MarketplaceCandidate) -> (String, RegistryEntry) {
    let mut keywords = candidate.keywords.clone();
    keywords.push(candidate.name.clone());
    if let Some(display) = &candidate.display_name {
        keywords.push(display.clone());
    }
    keywords.extend(candidate.categories.iter().cloned());
    (
        format!(
            "marketplace:{}:{}",
            candidate.catalog_id.as_str(),
            candidate.name
        ),
        RegistryEntry {
            source: format!(
                "marketplace:{}:{}",
                candidate.catalog_id.as_str(),
                candidate.name
            ),
            description: candidate.description.clone(),
            keywords,
            domains: Vec::new(),
        },
    )
}

fn recommend_from_entries(
    task: &str,
    entries: &[(String, RegistryEntry)],
    installed: &[&LoadedPlugin],
    options: RecommendOptions,
) -> Vec<PluginTaskRecommendation> {
    if options.limit == 0 {
        return Vec::new();
    }
    let index = RegistryDocument {
        skills: entries.iter().cloned().collect::<BTreeMap<_, _>>(),
    };
    let ranked = recommend_remote_skills(task, &index, options.limit.saturating_mul(2));
    let mut out = Vec::new();
    let mut seen_names = BTreeSet::new();
    for recommendation in ranked {
        if recommendation.score() < options.min_score {
            continue;
        }
        let (source, name, next_step) =
            match recommendation.entry.source.strip_prefix("marketplace:") {
                Some(rest) => {
                    let Some((catalog_id, name)) = rest.split_once(':') else {
                        continue;
                    };
                    (
                        PluginMatchSource::Marketplace {
                            catalog_id: catalog_id.to_string(),
                        },
                        name.to_string(),
                        PluginNextStep::MarketplaceInstall {
                            catalog_id: catalog_id.to_string(),
                        },
                    )
                }
                None => {
                    let Some(plugin) = installed
                        .iter()
                        .find(|plugin| plugin.id.as_str() == recommendation.entry.source)
                    else {
                        continue;
                    };
                    let next_step = if plugin.active() {
                        PluginNextStep::AlreadyActive
                    } else if !plugin.trusted() {
                        PluginNextStep::Trust
                    } else if !plugin.enabled {
                        PluginNextStep::Enable
                    } else {
                        PluginNextStep::Inspect
                    };
                    (
                        PluginMatchSource::Installed {
                            id: plugin.id.as_str().to_string(),
                        },
                        plugin.name().to_string(),
                        next_step,
                    )
                }
            };
        if !options.include_active && next_step == PluginNextStep::AlreadyActive {
            continue;
        }
        let name_key = name.to_ascii_lowercase();
        if !seen_names.insert(name_key) {
            continue;
        }
        out.push(PluginTaskRecommendation {
            name,
            source,
            matched_terms: recommendation.matched_terms.clone(),
            score: recommendation.score(),
            next_step,
        });
        if out.len() >= options.limit {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::marketplace::types::{
        CatalogProvenance, CatalogTier, MarketplaceCandidate, MarketplaceCandidateId,
        MarketplaceCatalogId, MarketplaceInstallPlan, MarketplaceSourceSpec,
    };
    use crate::test_support::{EnvVarGuard, lock_test_env};
    use std::fs;
    use tempfile::TempDir;

    fn write_keyword_bundle(
        root: &std::path::Path,
        name: &str,
        description: &str,
        keywords: &[&str],
    ) {
        let bundle = root.join(".codewhale/plugins").join(name);
        fs::create_dir_all(&bundle).unwrap();
        let keyword_list = keywords
            .iter()
            .map(|keyword| format!("\"{keyword}\""))
            .collect::<Vec<_>>()
            .join(", ");
        fs::write(
            bundle.join("plugin.toml"),
            format!(
                "schema_version = 1\n[plugin]\nname = \"{name}\"\nversion = \"1.0.0\"\ndescription = \"{description}\"\nkeywords = [{keyword_list}]\n"
            ),
        )
        .unwrap();
    }

    fn marketplace_candidate(catalog: &str, name: &str, keywords: &[&str]) -> MarketplaceCandidate {
        MarketplaceCandidate {
            id: MarketplaceCandidateId::new(&MarketplaceCatalogId::new(catalog), name),
            catalog_id: MarketplaceCatalogId::new(catalog),
            name: name.to_string(),
            display_name: Some(format!("{name} plugin")),
            description: Some(format!("{name} integration")),
            version: None,
            author: None,
            homepage: None,
            repository: None,
            license: None,
            keywords: keywords.iter().map(|value| (*value).to_string()).collect(),
            categories: Vec::new(),
            source: MarketplaceSourceSpec::GitHub {
                owner: "example".to_string(),
                repo: name.to_string(),
                git_ref: None,
                sha: None,
            },
            install_plan: MarketplaceInstallPlan::Supported {
                spec: format!("github:example/{name}"),
                source_kind: "github".to_string(),
            },
            declared_components: None,
            compatibility: None,
            provenance: CatalogProvenance {
                tier: CatalogTier::Community,
                publisher: None,
                source_url: None,
            },
            when: None,
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn keyword_match_ranks_installed_supabase_plugin() {
        let _lock = lock_test_env();
        let root = TempDir::new().unwrap();
        let _home = EnvVarGuard::set("CODEWHALE_HOME", root.path().join("home"));
        write_keyword_bundle(
            root.path(),
            "supabase",
            "Hosted Postgres and auth",
            &["supabase", "postgres"],
        );
        let registry = crate::plugins::PluginDiscoveryContext::capture_pre_dotenv()
            .registry_for_workspace(root.path());

        let recs = recommend_plugins_for_task(
            "add supabase auth to this app",
            &registry,
            &[],
            RecommendOptions::proactive(),
        );
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].name, "supabase");
        assert!(recs[0].score >= PROACTIVE_MIN_SCORE);
        assert_eq!(recs[0].next_step, PluginNextStep::Trust);
        assert_eq!(recs[0].command(), "/plugin trust supabase");
    }

    #[test]
    fn marketplace_fills_in_a_missing_plugin() {
        let _lock = lock_test_env();
        let root = TempDir::new().unwrap();
        let _home = EnvVarGuard::set("CODEWHALE_HOME", root.path().join("home"));
        let registry = crate::plugins::PluginDiscoveryContext::capture_pre_dotenv()
            .registry_for_workspace(root.path());
        let catalog = [marketplace_candidate("official", "supabase", &["supabase"])];

        let recs = recommend_plugins_for_task(
            "wire up supabase row level security",
            &registry,
            &catalog,
            RecommendOptions::proactive(),
        );
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].name, "supabase");
        assert_eq!(
            recs[0].next_step,
            PluginNextStep::MarketplaceInstall {
                catalog_id: "official".to_string()
            }
        );
        assert_eq!(
            recs[0].command(),
            "/plugin marketplace install official supabase"
        );
    }

    #[test]
    fn already_active_plugins_are_skipped_for_proactive_toasts() {
        let _lock = lock_test_env();
        let root = TempDir::new().unwrap();
        let _home = EnvVarGuard::set("CODEWHALE_HOME", root.path().join("home"));
        write_keyword_bundle(root.path(), "supabase", "Hosted Postgres", &["supabase"]);
        let mut registry = crate::plugins::PluginDiscoveryContext::capture_pre_dotenv()
            .registry_for_workspace(root.path())
            .as_ref()
            .clone();
        registry.trust("supabase").unwrap();
        registry.enable("supabase").unwrap();

        let recs = recommend_plugins_for_task(
            "add supabase auth",
            &registry,
            &[],
            RecommendOptions::proactive(),
        );
        assert!(recs.is_empty(), "{recs:?}");
    }

    #[test]
    fn generic_prompts_do_not_match_on_description_alone() {
        let _lock = lock_test_env();
        let root = TempDir::new().unwrap();
        let _home = EnvVarGuard::set("CODEWHALE_HOME", root.path().join("home"));
        write_keyword_bundle(
            root.path(),
            "notes",
            "Create and organize spreadsheet notes",
            &[],
        );
        let registry = crate::plugins::PluginDiscoveryContext::capture_pre_dotenv()
            .registry_for_workspace(root.path());

        let recs = recommend_plugins_for_task(
            "fix the failing test",
            &registry,
            &[],
            RecommendOptions::proactive(),
        );
        assert!(recs.is_empty(), "{recs:?}");
    }
}
