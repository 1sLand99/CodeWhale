use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use codewhale_agent::ModelRegistry;
use codewhale_config::ConfigToml;
use codewhale_core::Runtime;
use codewhale_execpolicy::{AskForApproval, ExecPolicyEngine};
use codewhale_hooks::{HookDispatcher, HookEvent, HookSink};
use codewhale_mcp::McpManager;
use codewhale_protocol::{ToolKind, ToolOutput, ToolPayload};
use codewhale_state::StateStore;
use codewhale_tools::{
    FunctionCallError, ToolCall, ToolCallSource, ToolDescriptor, ToolHandler, ToolInvocation,
    ToolRegistry,
};
use serde_json::json;
use uuid::Uuid;

struct ApplicationFailureTool;

#[async_trait]
impl ToolHandler for ApplicationFailureTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, _invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        Ok(ToolOutput::Function {
            body: Some(json!({"message": "application failure remains visible"})),
            success: false,
        })
    }
}

#[derive(Default)]
struct RecordingSink(Mutex<Vec<HookEvent>>);

#[async_trait]
impl HookSink for RecordingSink {
    async fn emit(&self, event: &HookEvent) -> anyhow::Result<()> {
        self.0
            .lock()
            .expect("recording hook lock")
            .push(event.clone());
        Ok(())
    }
}

#[tokio::test]
async fn invoke_tool_preserves_application_failure_as_a_tool_result() {
    let mut registry = ToolRegistry::default();
    registry
        .register(
            ToolDescriptor {
                name: "application_failure_tool".into(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
                supports_parallel_tool_calls: true,
                timeout_ms: None,
            },
            Arc::new(ApplicationFailureTool),
        )
        .expect("register application-failure tool");

    let recording = Arc::new(RecordingSink::default());
    let mut hooks = HookDispatcher::default();
    hooks.add_sink(recording.clone());
    let state_path = std::env::temp_dir().join(format!(
        "codewhale-core-tool-success-{}.db",
        Uuid::new_v4().simple()
    ));
    let runtime = Runtime::new(
        ConfigToml::default(),
        ModelRegistry::default(),
        StateStore::open(Some(state_path)).expect("open temporary state"),
        Arc::new(registry),
        Arc::new(McpManager::default()),
        ExecPolicyEngine::new(vec![], vec![]),
        hooks,
    );
    let result = runtime
        .invoke_tool(
            ToolCall {
                name: "application_failure_tool".into(),
                payload: ToolPayload::Function {
                    arguments: "{}".into(),
                },
                source: ToolCallSource::Direct,
                raw_tool_call_id: None,
            },
            AskForApproval::Never,
            Path::new("/tmp/codewhale"),
        )
        .await
        .expect("application failure remains a transport-successful tool result");

    assert_eq!(result["ok"], false);
    assert_eq!(result["status"], "failed");
    assert!(result.get("error").is_none());
    assert_eq!(result["output"]["type"], "function");
    assert_eq!(result["output"]["success"], false);
    assert_eq!(
        result["output"]["body"]["message"],
        "application failure remains visible"
    );
    assert_eq!(result["events"][1]["event"], "tool_call_result");
    assert_eq!(result["events"][1]["output"]["success"], false);

    let events = recording.0.lock().expect("recording hook lock");
    let terminal = events
        .iter()
        .find_map(|event| match event {
            HookEvent::ToolLifecycle {
                tool_name,
                phase,
                payload,
                ..
            } if tool_name == "application_failure_tool" && phase == "failed" => Some(payload),
            _ => None,
        })
        .expect("failed application lifecycle hook");
    assert_eq!(terminal["ok"], false);
}
