use axum::Json;
use axum::extract::State;
use serde::Serialize;

use super::{ApiError, RuntimeApiState};

#[derive(Debug, Serialize)]
pub(super) struct CommandCatalogEntry {
    name: &'static str,
    aliases: Vec<&'static str>,
    usage: &'static str,
    description: String,
    subcommands: Vec<&'static str>,
    discovery: &'static str,
    requires_argument: bool,
    requires_required_argument: bool,
    composer_wants_trailing_space: bool,
    palette_runs_directly: bool,
    show_in_empty_discovery: bool,
    unlisted: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct CommandCatalogResponse {
    commands: Vec<CommandCatalogEntry>,
    locale: String,
}

/// Typed projection of the built-in slash-command registry for native-client
/// composers and palettes. The registry in `crate::commands` stays the single
/// authority: this handler only serializes the same `CommandInfo` the TUI
/// consumes, including the discovery tiers and argument-shape hints the
/// composer needs to reproduce slash completion without re-parsing `usage`.
///
/// Descriptions are localized with the runtime's configured locale; `locale`
/// in the response names the resolved pack so a client can detect fallback.
/// User-registered commands are intentionally absent — they are per-session
/// state, and a session-scoped projection can layer them on later without
/// changing this contract.
pub(super) async fn list_commands(
    State(_state): State<RuntimeApiState>,
) -> Result<Json<CommandCatalogResponse>, ApiError> {
    let settings = crate::settings::Settings::load_persisted().unwrap_or_default();
    let locale = codewhale_localization::resolve_locale(&settings.locale);
    let commands = crate::commands::command_infos()
        .into_iter()
        .map(|info| CommandCatalogEntry {
            name: info.name,
            aliases: info.aliases.to_vec(),
            usage: info.usage,
            description: info.description_for(locale).into_owned(),
            subcommands: info.subcommands(),
            discovery: match info.discovery() {
                crate::commands::traits::CommandDiscovery::Primary => "primary",
                crate::commands::traits::CommandDiscovery::Advanced => "advanced",
                crate::commands::traits::CommandDiscovery::Compatibility => "compatibility",
            },
            requires_argument: info.requires_argument(),
            requires_required_argument: info.requires_required_argument(),
            composer_wants_trailing_space: info.composer_wants_trailing_space(),
            palette_runs_directly: info.palette_runs_directly(),
            show_in_empty_discovery: info.show_in_empty_discovery(),
            unlisted: info.is_unlisted(),
        })
        .collect();
    Ok(Json(CommandCatalogResponse {
        commands,
        locale: locale.tag().to_string(),
    }))
}
