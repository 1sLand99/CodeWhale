use super::*;

#[test]
fn foreign_and_unbound_definitions_are_not_repaired_or_paused_by_a_tick() -> Result<()> {
    let root = tempfile::tempdir()?;
    let owner = AutomationManager::open_for_test(root.path().to_path_buf())?;
    let mut foreign = automation_record_with_settings(None, None, None, None);
    foreign.id = "foreign".into();
    foreign.next_run_at = None;
    owner.save_automation(&foreign)?;
    let mut legacy = foreign.clone();
    legacy.id = "legacy".into();
    legacy.execution_scope = None;
    legacy.rrule = "unparseable legacy schedule".into();
    owner.save_automation(&legacy)?;
    let before = [
        fs::read(owner.automation_path(&foreign.id)?)?,
        fs::read(owner.automation_path(&legacy.id)?)?,
    ];
    let mut other = AutomationManager::open(root.path().to_path_buf())?;
    other.execution_scope = Some(crate::task_manager::test_execution_scope("other"));
    assert!(other.collect_due_runs(Utc::now())?.is_empty());
    assert_eq!(fs::read(owner.automation_path(&foreign.id)?)?, before[0]);
    assert_eq!(fs::read(owner.automation_path(&legacy.id)?)?, before[1]);
    Ok(())
}

#[tokio::test]
async fn explicit_run_now_adopts_expired_legacy_once_but_does_not_rebind_old_admissions()
-> Result<()> {
    let root = tempfile::tempdir()?;
    let receipts = root.path().join("executions");
    let tasks = fixture_tasks(&root.path().join("tasks"), &receipts).await?;
    let manager = AutomationManager::open_for_test(root.path().join("automations"))?;
    let mut legacy = automation_record_with_settings(None, None, None, None);
    legacy.execution_scope = None;
    legacy.rrule = format!(
        "FREQ=ONCE;AT={}",
        (Utc::now() - Duration::hours(1)).to_rfc3339()
    );
    legacy.next_run_at = None;
    manager.save_automation(&legacy)?;
    let mut old = queued_run_for(&legacy);
    let mut formerly_bound = legacy.clone();
    formerly_bound.execution_scope = Some(crate::task_manager::test_execution_scope("test"));
    bind_run_dispatch(&mut old, &formerly_bound, &tasks.data_dir(), false)?;
    old.dispatch.as_mut().unwrap().execution_scope = None;
    manager.save_run(&old)?;
    let old_path = manager.run_path(&old)?;
    let before = fs::read(&old_path)?;
    let shared = Arc::new(Mutex::new(manager));
    let explicit = run_now_shared(&shared, &legacy.id, &tasks).await?;
    let id = explicit.task_id.context("explicit task")?;
    let task = crate::task_manager::wait_for_terminal_state(
        &tasks,
        &id,
        std::time::Duration::from_secs(10),
    )
    .await?;
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(
        task.owner_session_id.is_none(),
        "None visibility is valid for a new scoped automation"
    );
    assert_eq!(
        task.execution_scope.as_deref(),
        Some(tasks.execution_scope())
    );
    assert_eq!(
        tasks.get_task_for_active_runtime(&task.id).await?.id,
        task.id
    );
    scheduler_tick_shared(&shared, &tasks).await?;
    assert_eq!(fixture_executions(&receipts), vec![id]);
    assert_eq!(
        fs::read(old_path)?,
        before,
        "definition adoption cannot adopt an old occurrence"
    );
    let adopted = shared.lock().await.get_automation(&legacy.id)?;
    assert_eq!(
        adopted.execution_scope.as_deref(),
        Some(tasks.execution_scope())
    );
    assert_eq!(adopted.status, AutomationStatus::Paused);
    tasks.shutdown_and_wait().await?;
    Ok(())
}

#[tokio::test]
async fn foreign_scoped_automation_and_unbound_trigger_never_dispatch_through_current_service()
-> Result<()> {
    let root = tempfile::tempdir()?;
    let receipts = root.path().join("executions");
    let tasks = fixture_tasks(&root.path().join("tasks"), &receipts).await?;
    let manager = AutomationManager::open_for_test(root.path().join("automations"))?;
    let mut foreign = fixture_due_automation(&manager, "foreign", 1);
    foreign.execution_scope = Some(crate::task_manager::test_execution_scope("foreign"));
    manager.save_automation(&foreign)?;
    let mut trigger = manager.create_trigger(CreateDelayedTriggerRequest {
        fire_at: Utc::now() + Duration::hours(1),
        message: "legacy continuation".into(),
        workspace: Some(root.path().to_path_buf()),
        owner_session_id: None,
        parent_trigger_id: None,
    })?;
    trigger.execution_scope = None;
    trigger.fire_at = Utc::now() - Duration::seconds(1);
    manager.save_trigger(&trigger)?;
    let path = manager.trigger_path(&trigger.trigger_id)?;
    let before = fs::read(&path)?;
    let shared = Arc::new(Mutex::new(manager));
    scheduler_tick_shared(&shared, &tasks).await?;
    fire_due_triggers_shared(&shared, &tasks).await?;
    assert!(run_now_shared(&shared, &foreign.id, &tasks).await.is_err());
    assert!(fixture_executions(&receipts).is_empty());
    assert!(tasks.list_tasks(None).await?.is_empty());
    assert_eq!(fs::read(path)?, before);
    tasks.shutdown_and_wait().await?;
    Ok(())
}
