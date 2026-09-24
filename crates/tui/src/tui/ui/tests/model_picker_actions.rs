//! #6500: the model picker's ⇧P / ⇧F / ⇧D after a search, driven through the
//! open view stack, the event handler, disk, and the rendered picker.

use super::*;
use crate::tui::model_picker::ModelPickerView;
use crate::tui::ui::handlers::{toggle_model_picker_fleet, toggle_model_picker_pin};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, layout::Rect};

/// An app on a keyed DeepSeek route with `/model` open, under a temp home.
fn app_with_open_picker(root: &std::path::Path) -> (App, Config) {
    let workspace = root.join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let mut config = Config::default();
    config.set_provider_api_key_override(ApiProvider::Deepseek, Some("fixture-key".into()));
    let mut app = App::new(crate::test_support::test_tui_options(&workspace), &config);
    app.workspace = workspace;
    let picker = ModelPickerView::new(&app, &config);
    app.view_stack.push(picker);
    (app, config)
}

fn type_query(app: &mut App, query: &str) {
    for ch in query.chars() {
        let events = app
            .view_stack
            .handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        assert!(events.is_empty(), "typing {ch:?} emitted {events:?}");
    }
}

fn shifted(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
}

fn render_picker(app: &mut App) -> String {
    assert_eq!(app.view_stack.top_kind(), Some(ModalKind::ModelPicker));
    let view = app.view_stack.pop().unwrap();
    let area = Rect::new(0, 0, 140, 40);
    let mut buf = Buffer::empty(area);
    view.render(area, &mut buf);
    app.view_stack.push_boxed(view);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The picker line that lists `model` as a row (not the query title or the
/// receipt line).
fn row_line<'a>(screen: &'a str, model: &str) -> &'a str {
    screen
        .lines()
        .find(|line| {
            line.contains(model) && !line.contains("Model:") && !line.contains(&format!("/{model}"))
        })
        .unwrap_or_else(|| panic!("no row for {model}:\n{screen}"))
}

#[test]
fn shift_p_after_a_search_pins_the_highlighted_row_and_says_so_in_the_picker() {
    let _lock = crate::test_support::lock_test_env();
    let root = tempfile::tempdir().unwrap();
    let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", root.path().join("home"));
    let (mut app, config) = app_with_open_picker(root.path());
    let model = app.model.clone();
    type_query(&mut app, &model);

    let events = app.view_stack.handle_key(shifted('P'));
    let [
        ViewEvent::ModelPickerTogglePin {
            provider,
            provider_id,
            model: picked,
        },
    ] = events.as_slice()
    else {
        panic!("⇧P while searching must pin the highlighted row, got {events:?}");
    };
    assert_eq!(picked, &model);
    let provider_key = provider_id
        .clone()
        .unwrap_or_else(|| provider.as_str().to_string());
    toggle_model_picker_pin(&mut app, &config, &provider_key, picked);

    let saved = crate::settings::Settings::load_persisted().unwrap();
    assert!(
        saved
            .pinned_models
            .iter()
            .any(|pin| pin.provider == provider_key && pin.model == model),
        "pin not persisted: {:?}",
        saved.pinned_models
    );
    let screen = render_picker(&mut app);
    assert!(
        screen.contains(&format!("Pinned {provider_key}/{model}")),
        "receipt must be visible inside the picker:\n{screen}"
    );
    assert!(
        row_line(&screen, &model).contains("pinned"),
        "row must carry its pin:\n{screen}"
    );
    assert!(
        screen.contains(&format!("Model: {model}")) && !screen.contains(&format!("{model}P")),
        "⇧P must not become query text:\n{screen}"
    );

    // The same chord on the same row unpins it, on disk and on screen.
    let events = app.view_stack.handle_key(shifted('P'));
    assert!(matches!(
        events.as_slice(),
        [ViewEvent::ModelPickerTogglePin { model: again, .. }] if again == &model
    ));
    toggle_model_picker_pin(&mut app, &config, &provider_key, &model);
    let saved = crate::settings::Settings::load_persisted().unwrap();
    assert!(saved.pinned_models.is_empty(), "{:?}", saved.pinned_models);
    let screen = render_picker(&mut app);
    assert!(
        screen.contains(&format!("Unpinned {provider_key}/{model}")),
        "{screen}"
    );
    assert!(!row_line(&screen, &model).contains("pinned"), "{screen}");
}

#[test]
fn shift_f_after_a_search_adds_the_highlighted_row_to_fleet_and_says_so_in_the_picker() {
    let _lock = crate::test_support::lock_test_env();
    let root = tempfile::tempdir().unwrap();
    let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", root.path().join("home"));
    let (mut app, config) = app_with_open_picker(root.path());
    let model = app.model.clone();
    type_query(&mut app, &model);

    let events = app.view_stack.handle_key(shifted('F'));
    let [
        ViewEvent::ModelPickerToggleFleet {
            provider,
            provider_id,
            model: picked,
        },
    ] = events.as_slice()
    else {
        panic!("⇧F while searching must toggle Fleet for the highlighted row, got {events:?}");
    };
    assert_eq!(picked, &model);
    let provider_key = provider_id
        .clone()
        .unwrap_or_else(|| provider.as_str().to_string());
    toggle_model_picker_fleet(&mut app, &config, &provider_key, picked);

    let fleet = crate::fleet::members::fleet_models(&app.workspace).unwrap();
    assert!(
        fleet
            .iter()
            .any(|member| member.matches(&provider_key, &model)),
        "route not in the selected fleet: {fleet:?}"
    );
    assert!(app.fleet_roster_stale, "the engine roster must be resynced");
    let screen = render_picker(&mut app);
    assert!(
        screen.contains(&format!("{provider_key}/{model}")),
        "receipt must be visible inside the picker:\n{screen}"
    );
    assert!(
        row_line(&screen, &model).contains("fleet · "),
        "row must carry its Fleet membership:\n{screen}"
    );

    // Toggling again removes the shortlist row it added.
    let events = app.view_stack.handle_key(shifted('F'));
    assert!(matches!(
        events.as_slice(),
        [ViewEvent::ModelPickerToggleFleet { model: again, .. }] if again == &model
    ));
    toggle_model_picker_fleet(&mut app, &config, &provider_key, &model);
    let fleet = crate::fleet::members::fleet_models(&app.workspace).unwrap();
    assert!(
        !fleet
            .iter()
            .any(|member| member.matches(&provider_key, &model)),
        "{fleet:?}"
    );
    let screen = render_picker(&mut app);
    assert!(!row_line(&screen, &model).contains("fleet · "), "{screen}");
}

#[test]
fn shift_d_while_searching_is_query_text_not_a_provider_auth_detour() {
    let _lock = crate::test_support::lock_test_env();
    let root = tempfile::tempdir().unwrap();
    let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", root.path().join("home"));
    let (mut app, _config) = app_with_open_picker(root.path());
    let model = app.model.clone();
    type_query(&mut app, &model);
    assert!(
        !render_picker(&mut app).contains("⇧D"),
        "⇧D is not advertised while it would be query text"
    );

    let events = app.view_stack.handle_key(shifted('D'));
    assert!(
        events.is_empty(),
        "⇧D mid-search used to emit a lock explanation / auth hand-off: {events:?}"
    );
    assert_eq!(app.view_stack.top_kind(), Some(ModalKind::ModelPicker));
    assert!(render_picker(&mut app).contains(&format!("Model: {model}D")));
}
