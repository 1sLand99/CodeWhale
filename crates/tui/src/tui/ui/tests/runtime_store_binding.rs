use super::*;
use crate::automation_manager::{
    AutomationManager, AutomationStatus, CreateAutomationRequest, run_now_shared,
};
use crate::runtime_threads::{RuntimeThreadManager, RuntimeThreadManagerConfig};
use crate::task_manager::{TaskManager, TaskManagerConfig};

fn fixture_config() -> Config {
    let mut config = Config {
        api_key: Some("local-runtime-binding-fixture".into()),
        base_url: Some("http://127.0.0.1:1/v1".into()),
        ..Config::default()
    };
    config.set_feature("mcp", false).unwrap();
    config.set_feature("subagents", false).unwrap();
    config
}

#[tokio::test]
async fn runtime_store_binding_survives_launch_snapshot_and_resume() -> anyhow::Result<()> {
    let _environment = crate::test_support::lock_test_env();
    let root = tempfile::tempdir()?;
    let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", root.path());
    let _runtime = crate::test_support::EnvVarGuard::remove("CODEWHALE_RUNTIME_DIR");
    let _legacy = crate::test_support::EnvVarGuard::remove("DEEPSEEK_RUNTIME_DIR");
    let config = fixture_config();
    let mut app = Box::new(create_test_app());
    app.workspace = root.path().into();
    let initial_id = super::super::event_loop::ensure_runtime_session_id(&mut app);
    let task_config = TaskManagerConfig::from_runtime(&config, root.path().into(), None, Some(1));
    let tasks = TaskManager::start(
        task_config.clone(),
        config.clone(),
        app.plugin_registry.clone(),
        &initial_id,
        None,
    )
    .await?;
    app.runtime_services.task_manager = Some(tasks.clone());
    let sessions = SessionManager::default_location()?;
    // A saved initial conversation may later be deleted while another launch
    // still refers to its Runtime store.
    let initial = build_session_snapshot(&mut app, &sessions).map_err(anyhow::Error::msg)?;
    sessions.save_session(&initial)?;
    let launch = begin_launch_session(&mut app, None);
    assert!(!launch.is_error, "{:?}", launch.message);
    assert_ne!(app.current_session_id.as_deref(), Some(initial_id.as_str()));
    let saved = build_session_snapshot(&mut app, &sessions).map_err(anyhow::Error::msg)?;
    let binding = saved
        .metadata
        .runtime_store
        .clone()
        .expect("attached host binding");
    assert_eq!(binding.execution_scope, tasks.execution_scope());
    sessions.save_session(&saved)?;
    let mut automations = AutomationManager::open(root.path().join("automations"))?;
    automations.bind_task_manager(&tasks)?;
    let automation = automations.create_automation(CreateAutomationRequest {
        name: "resumed ownership fixture".into(),
        prompt: "local fixture only".into(),
        rrule: "FREQ=HOURLY;INTERVAL=1".into(),
        cwds: vec![root.path().into()],
        model: None,
        model_provider: None,
        model_provider_id: None,
        mode: None,
        allow_shell: Some(false),
        trust_mode: Some(false),
        auto_approve: Some(false),
        delivery_mode: None,
        status: Some(AutomationStatus::Paused),
    })?;
    tasks.shutdown_and_wait().await?;
    drop(app);
    drop(tasks);
    drop(automations);
    sessions.delete_session(&initial_id)?;
    assert!(
        binding.data_dir.is_dir(),
        "transcript deletion cannot erase Runtime authority"
    );
    let loaded = sessions.load_session(&saved.metadata.id)?;
    assert_eq!(loaded.metadata.runtime_store.as_ref(), Some(&binding));
    let mut resumed = Box::new(create_test_app());
    let mut resumed_config = config.clone();
    apply_loaded_session_with_goal(&mut resumed, &mut resumed_config, &loaded, None)
        .map_err(anyhow::Error::msg)?;
    let tasks = TaskManager::start(
        task_config.clone(),
        config.clone(),
        resumed.plugin_registry.clone(),
        &loaded.metadata.id,
        loaded.metadata.runtime_store.as_ref(),
    )
    .await?;
    assert_eq!(
        tasks.execution_scope(),
        automation.execution_scope.as_deref().unwrap()
    );
    resumed.runtime_services.task_manager = Some(tasks.clone());
    let automations = Arc::new(tokio::sync::Mutex::new(AutomationManager::open(
        root.path().join("automations"),
    )?));
    // The real Run-now admission must now create its durable receipt. The
    // configured endpoint is closed loopback and no shell/tool is authorized.
    let run = run_now_shared(&automations, &automation.id, &tasks).await?;
    assert!(run.task_id.is_some(), "{run:?}");
    assert_eq!(
        automations
            .lock()
            .await
            .list_runs(&automation.id, None)?
            .len(),
        1
    );
    assert_eq!(
        automations
            .lock()
            .await
            .get_automation(&automation.id)?
            .execution_scope,
        automation.execution_scope
    );
    tasks.shutdown_and_wait().await?;
    drop(resumed);
    drop(tasks);
    // Reproduce the old resume path: deriving a store from the saved
    // conversation id without its binding opens a foreign scope and cannot run.
    let foreign = TaskManager::start(
        task_config,
        config,
        Arc::new(crate::plugins::PluginRegistry::empty(root.path())),
        &loaded.metadata.id,
        None,
    )
    .await?;
    let foreign_automations = Arc::new(tokio::sync::Mutex::new(AutomationManager::open(
        root.path().join("automations"),
    )?));
    let definition_path = root
        .path()
        .join("automations/automations")
        .join(format!("{}.json", automation.id));
    let before_foreign_run = std::fs::read(&definition_path)?;
    let error = run_now_shared(&foreign_automations, &automation.id, &foreign)
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("another Runtime execution scope"),
        "{error:#}"
    );
    assert_eq!(
        foreign_automations
            .lock()
            .await
            .list_runs(&automation.id, None)?
            .len(),
        1
    );
    assert_eq!(std::fs::read(definition_path)?, before_foreign_run);
    let mut other_app = Box::new(create_test_app());
    other_app.runtime_services.task_manager = Some(foreign.clone());
    other_app.input = "preserve pending input".into();
    let old_id = other_app.current_session_id.clone();
    let error = apply_loaded_session_with_goal(&mut other_app, &mut resumed_config, &loaded, None)
        .unwrap_err();
    assert!(error.contains("Resume it in a new Codewhale process"));
    assert_eq!(other_app.current_session_id, old_id);
    assert_eq!(other_app.input, "preserve pending input");
    foreign.shutdown_and_wait().await?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn runtime_store_binding_retention_does_not_follow_session_directory_symlinks() -> anyhow::Result<()>
{
    let _environment = crate::test_support::lock_test_env();
    let root = tempfile::tempdir()?;
    let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", root.path());
    let sessions = SessionManager::default_location()?;
    let saved = crate::session_manager::create_saved_session_with_id_and_mode(
        "linked-session".into(),
        &[],
        "fixture",
        root.path(),
        0,
        None,
        None,
    );
    sessions.save_session(&saved)?;
    let target = root.path().join("unrelated-directory");
    std::fs::create_dir_all(&target)?;
    std::fs::write(target.join("keep.txt"), "preserve user data")?;
    let link = root.path().join("sessions/linked-session");
    std::os::unix::fs::symlink(&target, &link)?;
    sessions.delete_session("linked-session")?;
    assert!(!link.exists());
    assert_eq!(
        std::fs::read_to_string(target.join("keep.txt"))?,
        "preserve user data"
    );
    Ok(())
}

#[test]
fn runtime_store_binding_rejects_foreign_missing_or_overridden_store() -> anyhow::Result<()> {
    let _environment = crate::test_support::lock_test_env();
    let root = tempfile::tempdir()?;
    let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", root.path());
    let _runtime = crate::test_support::EnvVarGuard::remove("CODEWHALE_RUNTIME_DIR");
    let _legacy = crate::test_support::EnvVarGuard::remove("DEEPSEEK_RUNTIME_DIR");
    let config = fixture_config();
    let cfg = RuntimeThreadManagerConfig::for_session(root.path().join("tasks"), "original");
    let runtime = RuntimeThreadManager::open(config.clone(), root.path().into(), cfg.clone())?;
    let binding = runtime.session_store_binding();
    drop(runtime);
    let state_path = binding.data_dir.join("state.json");
    let before = std::fs::read(&state_path)?;
    let mut wrong = binding.clone();
    wrong.execution_scope = "0".repeat(64);
    let open = |binding: &crate::runtime_threads::RuntimeStoreBinding| {
        RuntimeThreadManager::open_for_session(
            config.clone(),
            root.path().into(),
            cfg.clone(),
            Arc::new(crate::plugins::PluginRegistry::empty(root.path())),
            Some(binding),
        )
    };
    assert!(
        open(&wrong)
            .err()
            .unwrap()
            .to_string()
            .contains("ownership does not match")
    );
    assert_eq!(std::fs::read(&state_path)?, before);
    wrong.data_dir = root.path().join("missing-store");
    assert!(open(&wrong).is_err());
    assert!(
        !wrong.data_dir.exists(),
        "saved binding cannot create a replacement authority"
    );
    let _override = crate::test_support::EnvVarGuard::set(
        "CODEWHALE_RUNTIME_DIR",
        root.path().join("foreign-override"),
    );
    assert!(
        open(&binding)
            .err()
            .unwrap()
            .to_string()
            .contains("override conflicts")
    );
    Ok(())
}
