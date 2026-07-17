//! Source-facing `Grep` routing through the session pre-flight boundary.

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

#[tokio::test(flavor = "current_thread")]
async fn source_grep_marker_is_isolated_to_the_uppercase_route() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;

            let marker = r#"__CLAUDE_CODE_GREP__{"pattern":"x"}"#;
            let uppercase = actor
                .execute_tool_calls(vec![tool_call(
                    "uppercase",
                    "Grep",
                    &format!(
                        r#"{{"pattern":{}}}"#,
                        serde_json::to_string(marker).unwrap()
                    ),
                )])
                .await
                .expect("source Grep should stop through the session loop");
            assert!(matches!(uppercase, ToolLoop::Continue));
            assert_eq!(
                tool_result_text(&actor, "uppercase").await,
                UNSUPPORTED_MESSAGE
            );

            let lowercase = prepare(
                &actor,
                tool_call(
                    "lowercase_marker",
                    "grep",
                    &format!(
                        r#"{{"pattern":{}}}"#,
                        serde_json::to_string(marker).unwrap()
                    ),
                ),
            )
            .await
            .expect("lowercase grep must keep accepting the legacy marker as a search pattern");
            assert_eq!(lowercase.registry_tool_name, "grep");
            assert_eq!(lowercase.parsed_args["pattern"], marker);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn lowercase_grep_remains_on_its_existing_route() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;

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
async fn source_grep_returns_the_fixed_unsupported_text_on_the_model_surface() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;
            let result = actor
                .execute_tool_calls(vec![tool_call(
                    "source_unsupported",
                    "Grep",
                    r#"{"pattern":"x"}"#,
                )])
                .await
                .expect("source Grep dispatch should return through the session loop");

            assert!(matches!(result, ToolLoop::Continue));
            assert_eq!(
                tool_result_text(&actor, "source_unsupported").await,
                UNSUPPORTED_MESSAGE
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn source_grep_unknown_fields_fail_before_registry_dispatch() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;
            let mut deferred = Vec::new();
            let result = actor
                .prepare_tool_call(
                    tool_call("unsupported", "Grep", r#"{"pattern":"x","unknown":true}"#),
                    &mut deferred,
                )
                .await
                .expect("unknown Grep should return through the existing session path");

            assert!(matches!(result, Err(ToolLoop::Continue)));
            assert_eq!(
                tool_result_text(&actor, "unsupported").await,
                "unknown source-facing Grep field `unknown`"
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn source_grep_strict_schema_failure_precedes_unsupported_and_resolution() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;
            for (field, value) in [
                ("glob", "false"),
                ("output_mode", r#""not-a-mode""#),
                ("head_limit", "true"),
                ("multiline", r#""TRUE""#),
                ("-i", r#""yes""#),
                ("-n", "1"),
                ("multiline", "null"),
                ("head_limit", r#""999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999""#),
            ] {
                let call_id = format!("invalid_{field}");
                let arguments = format!(r#"{{"pattern":"x","path":"missing","{field}":{value}}}"#);
                let result = prepare(&actor, tool_call(&call_id, "Grep", &arguments)).await;

                assert!(matches!(result, Err(ToolLoop::ToolParsingError)), "{field}");
                let message = tool_result_text(&actor, &call_id).await;
                assert!(message.starts_with("Failed to parse arguments for tool `Grep`:"));
                assert_ne!(message, UNSUPPORTED_MESSAGE);
            }
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn source_grep_rejects_explicit_null_for_each_non_nullable_optional_field() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;
            for field in [
                "path",
                "glob",
                "output_mode",
                "-B",
                "-A",
                "-C",
                "context",
                "type",
                "head_limit",
                "offset",
            ] {
                let call_id = format!("null_{field}");
                let arguments = format!(r#"{{"pattern":"x","{field}":null}}"#);
                let result = prepare(&actor, tool_call(&call_id, "Grep", &arguments)).await;

                assert!(
                    matches!(result, Err(ToolLoop::ToolParsingError)),
                    "explicit null for {field} must fail strict schema validation"
                );
                let message = tool_result_text(&actor, &call_id).await;
                assert!(message.starts_with("Failed to parse arguments for tool `Grep`:"));
                assert_ne!(message, UNSUPPORTED_MESSAGE);
            }
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn source_grep_accepts_omission_for_each_non_nullable_optional_field() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;
            for field in [
                "path",
                "glob",
                "output_mode",
                "-B",
                "-A",
                "-C",
                "context",
                "type",
                "head_limit",
                "offset",
            ] {
                let call_id = format!("omitted_{field}");
                let result =
                    prepare(&actor, tool_call(&call_id, "Grep", r#"{"pattern":"x"}"#)).await;

                assert!(
                    matches!(result, Err(ToolLoop::Continue)),
                    "omitted {field} must be accepted by strict schema validation"
                );
                assert_eq!(
                    tool_result_text(&actor, &call_id).await,
                    UNSUPPORTED_MESSAGE
                );
            }
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn source_grep_coerces_strict_string_booleans_before_unsupported() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;
            let result = actor
                .execute_tool_calls(vec![tool_call(
                    "string_booleans",
                    "Grep",
                    r#"{"pattern":"x","-n":"true","-i":"false","multiline":"true"}"#,
                )])
                .await
                .expect("a source-valid string-boolean Grep should stop through the session loop");

            assert!(matches!(result, ToolLoop::Continue));
            assert_eq!(
                tool_result_text(&actor, "string_booleans").await,
                UNSUPPORTED_MESSAGE
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn source_grep_parses_every_schema_field_before_stopping_as_unsupported() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;
            let result = actor
                .execute_tool_calls(vec![tool_call(
                    "all_fields",
                    "Grep",
                    r#"{"pattern":"x","path":"missing","glob":"*.rs","output_mode":"content","-B":"-5","-A":3.14,"-C":"0","context":-2.5,"-n":true,"-i":false,"type":"rust","head_limit":"1.25","offset":-1,"multiline":true}"#,
                )])
                .await
                .expect("a schema-valid source Grep should stop through the session loop");

            assert!(matches!(result, ToolLoop::Continue));
            assert_eq!(tool_result_text(&actor, "all_fields").await, UNSUPPORTED_MESSAGE);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn source_grep_stops_before_path_resolution_permission_or_dispatch() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let actor = grep_actor().await;
            let mut deferred = Vec::new();
            let result = actor
                .prepare_tool_call(
                    tool_call("source_stop", "Grep", r#"{"pattern":"x","path":"missing"}"#),
                    &mut deferred,
                )
                .await
                .expect("valid source Grep should stop through the existing session path");

            assert!(matches!(result, Err(ToolLoop::Continue)));
            assert_eq!(
                tool_result_text(&actor, "source_stop").await,
                UNSUPPORTED_MESSAGE
            );
        })
        .await;
}
