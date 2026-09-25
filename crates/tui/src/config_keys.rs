//! Which file owns a `codewhale config set` key, and which config.toml keys
//! nothing reads (#6563).
//!
//! `config set` used to write any key it did not recognise into config.toml,
//! so `config set calm_mode flase` and `config set totally_bogus_key 42` both
//! exited 0 and changed nothing that runs: `calm_mode` lives in settings.toml.
//! This module answers the one question both `config set` and `config doctor`
//! need — is this key read from config.toml, from settings.toml, or by
//! nothing — from the structs that actually read the files, so the answer
//! cannot drift from a hand-kept list.

use std::path::PathBuf;

use anyhow::Result;
use codewhale_config::ConfigToml;
use codewhale_config::settings_schema::SETTINGS_SCHEMA;

use crate::config::Config;
use crate::settings::Settings;

/// Root keys read from config.toml outside both the TUI [`Config`] struct and
/// the dispatcher's [`ConfigToml`] typed fields. Each has a named reader:
/// profile overlays (`ConfigFile::profiles`), per-project trust
/// (`config::project_trust_*`), the route-preference migration
/// (`config_persistence`), the MCP stdio dispatcher's literal JSON key, and
/// the stream-timeout fallbacks in `ConfigToml::stream_chunk_timeout_secs`.
const OTHER_READER_ROOT_KEYS: &[&str] = &[
    "profiles",
    "projects",
    "route_preferences_version",
    "route_preferences_migration",
    "mcp.server_definitions",
    "stream_chunk_timeout_secs",
    "tui.stream_chunk_timeout_secs",
];

/// Where a key a user asked `config set` to write is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigKeyHome {
    /// config.toml: a typed field, a table something deserializes, or a
    /// dotted path whose own validation lives in `ConfigToml::set_value`.
    ConfigToml,
    /// settings.toml: a key `Settings::set` accepts.
    SettingsToml,
    /// Nothing reads it.
    Unknown,
}

/// Classify `key` for `codewhale config set`. config.toml wins when both
/// files know a spelling (`sandbox_mode`, `reasoning_effort`), which keeps
/// every key that already took effect writing where it did before.
#[must_use]
pub fn config_key_home(key: &str) -> ConfigKeyHome {
    let key = key.trim();
    if key.contains('.') || is_config_toml_root_key(key) {
        ConfigKeyHome::ConfigToml
    } else if Settings::canonical_key(key).is_some() {
        ConfigKeyHome::SettingsToml
    } else {
        ConfigKeyHome::Unknown
    }
}

/// Whether some reader consumes `key` at the root of config.toml.
fn is_config_toml_root_key(key: &str) -> bool {
    tui_config_fields().contains(&key)
        || OTHER_READER_ROOT_KEYS.contains(&key)
        || is_config_toml_typed_field(key)
}

/// Root field names of the TUI [`Config`], taken from serde's own field table
/// (renames and aliases included) rather than restated here.
fn tui_config_fields() -> &'static [&'static str] {
    static FIELDS: std::sync::OnceLock<&'static [&'static str]> = std::sync::OnceLock::new();
    FIELDS.get_or_init(struct_fields::<Config>)
}

/// Whether [`ConfigToml`] deserializes `key` into a typed field. Its unknown
/// keys flatten into `extras`, so a probe document either fails to type-check
/// against the field or lands somewhere other than `extras`; both mean a
/// typed field owns the key.
fn is_config_toml_typed_field(key: &str) -> bool {
    let mut probe = toml::Table::new();
    probe.insert(key.to_string(), toml::Value::Integer(0));
    match toml::Value::Table(probe).try_into::<ConfigToml>() {
        Ok(config) => !config.extras.contains_key(key),
        Err(_) => true,
    }
}

/// The field list a derived `Deserialize` hands to `deserialize_struct`.
/// Returns an empty slice for types that deserialize any other way.
fn struct_fields<T: serde::de::DeserializeOwned>() -> &'static [&'static str] {
    struct Probe(Option<&'static [&'static str]>);

    impl<'de> serde::Deserializer<'de> for &mut Probe {
        type Error = serde::de::value::Error;

        fn deserialize_any<V: serde::de::Visitor<'de>>(
            self,
            _visitor: V,
        ) -> Result<V::Value, Self::Error> {
            Err(serde::de::Error::custom("field probe"))
        }

        fn deserialize_struct<V: serde::de::Visitor<'de>>(
            self,
            _name: &'static str,
            fields: &'static [&'static str],
            _visitor: V,
        ) -> Result<V::Value, Self::Error> {
            self.0 = Some(fields);
            Err(serde::de::Error::custom("field probe"))
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map enum identifier ignored_any
        }
    }

    let mut probe = Probe(None);
    let _ = T::deserialize(&mut probe);
    probe.0.unwrap_or(&[])
}

/// Every key `config set` can write, for did-you-mean: the declared settings
/// (`SETTINGS_SCHEMA`) that resolve to a file, plus config.toml's own roots.
fn settable_keys() -> impl Iterator<Item = &'static str> {
    SETTINGS_SCHEMA
        .iter()
        .map(|def| def.key)
        .filter(|key| {
            codewhale_config::notifications::in_namespace(key)
                || (!key.contains('.') && config_key_home(key) != ConfigKeyHome::Unknown)
        })
        // serde's field table carries the camelCase aliases too; suggest the
        // documented snake_case spelling only.
        .chain(
            tui_config_fields()
                .iter()
                .copied()
                .filter(|key| !key.chars().any(|c| c.is_ascii_uppercase())),
        )
        .chain(
            OTHER_READER_ROOT_KEYS
                .iter()
                .copied()
                .filter(|key| !key.contains('.')),
        )
}

/// The closest of `candidates` to `query`, if any is close enough to suggest.
pub(crate) fn nearest_key<'a>(
    query: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    candidates
        .into_iter()
        .filter_map(|candidate| {
            crate::commands::best_suggestion_score(query, [candidate])
                .map(|score| (score, candidate))
        })
        .min_by_key(|(score, _)| *score)
        .map(|(_, candidate)| candidate)
}

/// The refusal `config set` gives for a key nothing reads.
#[must_use]
pub fn unknown_config_key_message(key: &str) -> String {
    let hint = nearest_key(key, settable_keys())
        .map(|candidate| format!(" Did you mean `{candidate}`?"))
        .unwrap_or_default();
    format!(
        "unknown config key `{key}`: nothing reads it, so it was not saved.{hint} \
         Run `codewhale config list` for config.toml keys or `/settings text` for settings."
    )
}

/// Write one settings.toml key through the same validator and locked
/// transaction as `/settings`. Returns the file written.
pub fn set_settings_value(key: &str, value: &str) -> Result<PathBuf> {
    Settings::transact(|settings| settings.set(key, value))?;
    Settings::path()
}

/// The saved settings.toml value for `key`, without terminal or environment
/// overlays, so `config get` reports what `config set` wrote.
pub fn settings_value(key: &str) -> Result<Option<String>> {
    Ok(Settings::load_persisted()?.value(key))
}

/// One line per config.toml root key that nothing reads, for `config doctor`.
/// A settings.toml key is named as misplaced so the fix is obvious.
#[must_use]
pub fn unread_config_keys<'a>(root_keys: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    root_keys
        .into_iter()
        .filter(|key| !is_config_toml_root_key(key))
        .map(|key| {
            if Settings::canonical_key(key).is_some() {
                format!(
                    "`{key}` belongs in settings.toml; config.toml's copy is never read \
                     (move it: `codewhale config set {key} <value>`, then \
                     `codewhale config unset {key}`)"
                )
            } else {
                let hint = nearest_key(key, settable_keys())
                    .map(|candidate| format!("; did you mean `{candidate}`?"))
                    .unwrap_or_default();
                format!("`{key}` is not read by anything{hint}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_key_home_routes_by_reader() {
        // settings.toml keys, including an alias `Settings::set` accepts.
        assert_eq!(config_key_home("calm_mode"), ConfigKeyHome::SettingsToml);
        assert_eq!(config_key_home("calm"), ConfigKeyHome::SettingsToml);
        assert_eq!(
            config_key_home("tool_collapse"),
            ConfigKeyHome::SettingsToml
        );
        // config.toml: dispatcher-typed, TUI-typed, and other named readers.
        for key in [
            "provider",
            "api_key",
            "verbosity",
            "log_level",
            "hook_sinks",
            "allow_shell",
            "max_subagents",
            "skills_dir",
            "tui",
            "features",
            "profiles",
            "projects",
            "stream_chunk_timeout_secs",
        ] {
            assert_eq!(config_key_home(key), ConfigKeyHome::ConfigToml, "{key}");
        }
        // Both files know these spellings; config.toml keeps winning.
        assert_eq!(config_key_home("sandbox_mode"), ConfigKeyHome::ConfigToml);
        assert_eq!(
            config_key_home("reasoning_effort"),
            ConfigKeyHome::ConfigToml
        );
        // Dotted keys keep their validation in `ConfigToml::set_value`.
        assert_eq!(
            config_key_home("providers.deepseek.model"),
            ConfigKeyHome::ConfigToml
        );
        assert_eq!(config_key_home("totally_bogus_key"), ConfigKeyHome::Unknown);
    }

    #[test]
    fn every_shipped_example_root_key_is_read_by_something() {
        let example: toml::Table =
            toml::from_str(include_str!("../../../config.example.toml")).expect("example parses");
        let unread = unread_config_keys(example.keys().map(String::as_str));
        assert!(unread.is_empty(), "{unread:#?}");
    }

    #[test]
    fn unknown_config_key_message_suggests_from_current_keys() {
        let message = unknown_config_key_message("calm_mod");
        assert!(message.contains("Did you mean `calm_mode`?"), "{message}");
        assert!(message.contains("not saved"), "{message}");

        let message = unknown_config_key_message("zzqqxxyy");
        assert!(!message.contains("Did you mean"), "{message}");
    }

    #[test]
    fn unread_config_keys_names_misplaced_settings_and_unknown_keys() {
        let findings = unread_config_keys(["verbosity", "calm_mode", "totally_bogus_key", "tui"]);
        assert_eq!(findings.len(), 2, "{findings:#?}");
        assert!(
            findings[0].contains("`calm_mode` belongs in settings.toml"),
            "{findings:#?}"
        );
        assert!(
            findings[1].contains("`totally_bogus_key` is not read by anything"),
            "{findings:#?}"
        );
    }
}
