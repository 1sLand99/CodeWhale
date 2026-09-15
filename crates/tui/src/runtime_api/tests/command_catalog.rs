use super::*;

fn entry<'a>(commands: &'a [CommandCatalogEntry], name: &str) -> &'a CommandCatalogEntry {
    commands
        .iter()
        .find(|command| command.name == name)
        .unwrap_or_else(|| panic!("catalog must contain {name}"))
}

#[test]
fn command_catalog_serves_builtins_with_host_binding() {
    let users = crate::commands::user_registry::UserCommandRegistry::new();
    let commands = command_catalog(&users);

    let model = entry(&commands, "model");
    assert_eq!(model.kind, "builtin");
    assert_eq!(model.binding, "host");
    assert_eq!(model.discovery, Some("primary"));
    assert!(!model.hidden);
    assert_eq!(model.shadowed_by, None);
    assert!(model.summary.is_some());
    assert!(model.usage.is_some());
    assert!(model.takes_arguments);

    // Unlisted builtins run but are not advertised — hidden, not absent.
    assert!(entry(&commands, "lane").hidden);

    // A usage line's literal verbs surface as subcommands.
    let goal = entry(&commands, "goal");
    assert!(
        goal.subcommands.iter().any(|verb| verb == "blocked"),
        "goal usage should declare its verbs: {:?}",
        goal.subcommands
    );
}

#[test]
fn command_catalog_marks_user_shadowing_of_builtin_names_and_aliases() {
    let users = crate::commands::user_registry::UserCommandRegistry::from_loaded(vec![
        ("model".to_string(), "Pick the fast route.".to_string()),
        ("agents".to_string(), "Alias-shaped command.".to_string()),
    ]);
    let commands = command_catalog(&users);

    let model = entry(&commands, "model");
    assert_eq!(model.shadowed_by.as_deref(), Some("model"));

    // The shadowing user command is served as a prompt-bound row.
    let user_model = commands
        .iter()
        .find(|command| command.name == "model" && command.kind == "user")
        .expect("user command row");
    assert_eq!(user_model.binding, "prompt");

    // A user command colliding with a builtin's alias shadows that spelling:
    // `agents` is a `subagents` alias, so the builtin reports it while keeping
    // its canonical name.
    let subagents = entry(&commands, "subagents");
    assert_eq!(subagents.kind, "builtin");
    assert_eq!(subagents.shadowed_by, None);
    assert!(
        subagents.shadowed_aliases.iter().any(|a| a == "agents"),
        "shadowed_aliases must report the taken spelling: {:?}",
        subagents.shadowed_aliases
    );
}
