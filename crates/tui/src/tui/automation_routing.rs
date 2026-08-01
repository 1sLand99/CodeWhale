//! Durable automation formatting and operator actions.

use crate::automation_manager::{
    AutomationRecord, AutomationRunRecord, AutomationStatus, SharedAutomationManager,
    run_now_shared,
};
use crate::task_manager::SharedTaskManager;
use crate::tui::app::{App, AutomationAction};
use crate::tui::history::HistoryCell;

pub(super) async fn handle_action(
    app: &mut App,
    action: AutomationAction,
    task_manager: &SharedTaskManager,
) {
    let Some(automations) = app.runtime_services.automations.clone() else {
        add_message(
            app,
            "Automation manager is not available in this session.".to_string(),
        );
        return;
    };

    match action {
        AutomationAction::List => list(app, &automations).await,
        AutomationAction::Show(id) => show(app, &automations, &id).await,
        AutomationAction::Pause(id) => mutate(app, &automations, &id, Mutation::Pause).await,
        AutomationAction::Resume(id) => mutate(app, &automations, &id, Mutation::Resume).await,
        AutomationAction::Delete(id) => mutate(app, &automations, &id, Mutation::Delete).await,
        AutomationAction::Run(id) => {
            let content = match run_now_shared(&automations, &id, task_manager).await {
                Ok(run) => format_run_enqueued(&id, &run),
                Err(error) => format!("Failed to run automation {id}: {error}"),
            };
            add_message(app, content);
        }
    }
}

async fn list(app: &mut App, automations: &SharedAutomationManager) {
    let result = automations.lock().await.list_automations();
    let content = match result {
        Ok(records) => format_list(&records),
        Err(error) => format!("Failed to list automations: {error}"),
    };
    add_message(app, content);
}

async fn show(app: &mut App, automations: &SharedAutomationManager, id: &str) {
    let manager = automations.lock().await;
    let content = match manager.get_automation(id) {
        Ok(record) => {
            let runs = manager.list_runs(id, Some(5)).ok();
            format_detail(&record, runs.as_deref())
        }
        Err(error) => format!("Automation {id} not found: {error}"),
    };
    drop(manager);
    add_message(app, content);
}

#[derive(Clone, Copy)]
enum Mutation {
    Pause,
    Resume,
    Delete,
}

impl Mutation {
    const fn label(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Delete => "delete",
        }
    }
}

async fn mutate(
    app: &mut App,
    automations: &SharedAutomationManager,
    id: &str,
    mutation: Mutation,
) {
    let manager = automations.lock().await;
    let result = match mutation {
        Mutation::Pause => manager.pause_automation(id),
        Mutation::Resume => manager.resume_automation(id),
        Mutation::Delete => manager.delete_automation(id),
    };
    drop(manager);

    let action = mutation.label();
    let content = match result {
        Ok(record) => format!(
            "Automation {} {} (status: {:?})",
            record.name, action, record.status
        ),
        Err(error) => format!("Failed to {action} automation {id}: {error}"),
    };
    add_message(app, content);
}

fn format_list(records: &[AutomationRecord]) -> String {
    if records.is_empty() {
        return "No scheduled automations. Use the `automation` tool to create one.".to_string();
    }

    let lines = records
        .iter()
        .map(|record| {
            format!(
                "{}  [{}]  {}  (next: {})",
                record.id,
                status_label(record.status),
                record.name,
                timestamp(record.next_run_at)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("Scheduled automations:\n{lines}")
}

fn format_detail(record: &AutomationRecord, runs: Option<&[AutomationRunRecord]>) -> String {
    let runs = match runs {
        Some(runs) => runs
            .iter()
            .map(|run| {
                format!(
                    "  {:?}  {}  (task {})",
                    run.status,
                    run.scheduled_for.to_rfc3339(),
                    run.task_id.as_deref().unwrap_or("-")
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        None => "  (runs unavailable)".to_string(),
    };
    format!(
        "Automation {} [{}]\n  name: {}\n  rrule: {}\n  next: {}\n  last: {}\nrecent runs:\n{}",
        record.id,
        status_label(record.status),
        record.name,
        record.rrule,
        timestamp(record.next_run_at),
        timestamp(record.last_run_at),
        runs
    )
}

fn format_run_enqueued(id: &str, run: &AutomationRunRecord) -> String {
    format!(
        "Automation {id} run enqueued: {:?} (task {})",
        run.status,
        run.task_id.as_deref().unwrap_or("-")
    )
}

const fn status_label(status: AutomationStatus) -> &'static str {
    match status {
        AutomationStatus::Active => "active",
        AutomationStatus::Paused => "paused",
    }
}

fn timestamp(value: Option<chrono::DateTime<chrono::Utc>>) -> String {
    value
        .map(|timestamp| timestamp.to_rfc3339())
        .unwrap_or_else(|| "-".to_string())
}

fn add_message(app: &mut App, content: String) {
    app.add_message(HistoryCell::System { content });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn record(status: AutomationStatus) -> AutomationRecord {
        let now = Utc::now();
        AutomationRecord {
            schema_version: 1,
            id: "auto_1".to_string(),
            name: "Nightly checks".to_string(),
            prompt: "Run checks".to_string(),
            rrule: "FREQ=DAILY".to_string(),
            cwds: Vec::new(),
            mode: None,
            allow_shell: None,
            trust_mode: None,
            auto_approve: None,
            status,
            created_at: now,
            updated_at: now,
            next_run_at: None,
            last_run_at: None,
        }
    }

    #[test]
    fn list_explains_empty_state_and_operator_controls() {
        assert!(format_list(&[]).contains("`automation` tool to create one"));
        let text = format_list(&[record(AutomationStatus::Paused)]);
        assert!(text.contains("auto_1  [paused]  Nightly checks"));
        assert!(text.contains("next: -"));
    }

    #[test]
    fn detail_keeps_schedule_and_recent_run_shape() {
        let text = format_detail(&record(AutomationStatus::Active), Some(&[]));
        assert!(text.contains("Automation auto_1 [active]"));
        assert!(text.contains("rrule: FREQ=DAILY"));
        assert!(text.contains("recent runs:"));
    }
}
