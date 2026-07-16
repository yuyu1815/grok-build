//! Source-facing `Grep` routing and unsupported-field failures through the
//! session pre-flight boundary. These tests intentionally stop before rg
//! dispatch; they exercise only parser selection, cwd canonicalization, and
//! model-facing failure delivery.

use super::support::*;
use super::*;

const UNSUPPORTED_MESSAGE: &str = "unsupported: Claude Code parity for Grep is not implemented";

async fn grep_actor() -> SessionActor {
    use xai_grok_tools::implementations::opencode::grep::GrepTool;
    use xai_grok_tools::registry::types::ToolConfig;

    let (gateway_tx, _gateway_rx) =
        tokio::sync::mpsc::unbounded_channel::<xai_acp_lib::AcpClientMessage>();
    let (persistence_tx, _persistence_rx) = tokio::sync::mpsc::unbounded_channel();
    let actor = create_test_actor(0, 256_000, 85, gateway_tx, persistence_tx).await;
    *actor.agent.borrow_mut() =
        test_agent_with_tools(vec![ToolConfig::for_tool::<GrepTool>()]).await;
    actor
}

fn tool_call(id: &str, name: &str, arguments: &str) -> crate::sampling::types::ToolCallResponse {
    crate::sampling::types::ToolCallResponse {
        id: id.to_string(),
        kind: "function".to_string(),
        function: crate::sampling::types::ToolCallFunction::new(name, arguments),
    }
}

async fn prepare(
    actor: &SessionActor,
    call: crate::sampling::types::ToolCallResponse,
) -> Result<PreparedToolCall, ToolLoop> {
    let mut deferred = Vec::new();
    actor
        .prepare_tool_call(call, &mut deferred)
        .await
        .expect("prepare_tool_call should not return an ACP error")
}

async fn tool_result_text(actor: &SessionActor, call_id: &str) -> String {
    let conversation = actor.chat_state_handle.get_conversation().await;
    conversation
        .iter()
        .rev()
        .find_map(|item| match item {
            xai_grok_sampling_types::ConversationItem::ToolResult(result)
                if result.tool_call_id == call_id =>
            {
                Some(result.content.to_string())
            }
            _ => None,
        })
        .expect("source-facing failure must produce a model-facing tool_result")
}

#[test]
fn every_unsupported_source_grep_field_is_rejected_in_contract_order() {
    for field in SOURCE_GREP_UNSUPPORTED_FIELDS {
        let mut raw = serde_json::json!({"pattern": "x"});
        raw.as_object_mut()
            .expect("test input is an object")
            .insert(field.to_string(), serde_json::Value::Bool(true));
        let result = prepare_source_grep_input(&mut raw, std::path::Path::new("/tmp"), None);
        assert!(
            matches!(result, Ok(SourceGrepPreparation::Unsupported)),
            "{field} must be rejected before lowercase registry parsing"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn source_grep_routes_to_lowercase_registry_and_canonicalizes_cwd() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;

            let omitted = prepare(
                &actor,
                tool_call("source_omitted", "Grep", r#"{"pattern":"x"}"#),
            )
            .await
            .expect("source Grep without path should prepare");
            assert_eq!(omitted.tool_name, "Grep");
            assert_eq!(omitted.registry_tool_name, "grep");
            assert_eq!(omitted.parsed_args["path"], "/tmp");

            let empty = prepare(
                &actor,
                tool_call("source_empty", "Grep", r#"{"pattern":"x","path":""}"#),
            )
            .await
            .expect("source Grep with empty path should prepare");
            assert_eq!(empty.registry_tool_name, "grep");
            assert_eq!(empty.parsed_args["path"], "/tmp");

            let supplied = prepare(
                &actor,
                tool_call("source_path", "Grep", r#"{"pattern":"x","path":"src"}"#),
            )
            .await
            .expect("source Grep path should be resolved before registry parsing");
            assert_eq!(supplied.registry_tool_name, "grep");
            assert_eq!(supplied.parsed_args["path"], "/tmp/src");

            let lowercase = prepare(&actor, tool_call("lowercase", "grep", r#"{"pattern":"x"}"#))
                .await
                .expect("existing lowercase grep should remain unchanged");
            assert_eq!(lowercase.tool_name, "grep");
            assert_eq!(lowercase.registry_tool_name, "grep");
            assert!(lowercase.parsed_args.get("path").is_none());
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn source_grep_unsupported_fields_fail_before_registry_dispatch() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;
            let mut deferred = Vec::new();
            let result = actor
                .prepare_tool_call(
                    tool_call(
                        "unsupported",
                        "Grep",
                        r#"{"pattern":"x","multiline":true,"glob":"*.rs"}"#,
                    ),
                    &mut deferred,
                )
                .await
                .expect("unsupported Grep should return through the existing session path");

            assert!(matches!(result, Err(ToolLoop::Continue)));
            assert_eq!(
                tool_result_text(&actor, "unsupported").await,
                UNSUPPORTED_MESSAGE
            );
        })
        .await;
}
