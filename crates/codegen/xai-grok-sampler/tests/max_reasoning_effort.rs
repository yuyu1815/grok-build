use std::sync::{Arc, Mutex};

use axum::{Router, extract::State, http::StatusCode, routing::post};
use futures_util::StreamExt;
use indexmap::IndexMap;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use xai_grok_sampler::{ApiBackend, SamplerConfig, SamplingClient};
use xai_grok_sampling_types::{
    ConversationItem, ConversationRequest, CreateResponseWrapper, ReasoningEffort, rs,
};

#[derive(Clone)]
struct Capture(Arc<Mutex<Vec<serde_json::Value>>>);

async fn capture(
    State(capture): State<Capture>,
    body: axum::body::Bytes,
) -> (StatusCode, &'static str) {
    capture
        .0
        .lock()
        .unwrap()
        .push(serde_json::from_slice(&body).unwrap());
    (StatusCode::OK, "{}")
}

async fn spawn() -> (
    String,
    Arc<Mutex<Vec<serde_json::Value>>>,
    oneshot::Sender<()>,
) {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/v1/chat/completions", post(capture))
        .route("/v1/messages", post(capture))
        .route("/v1/responses", post(capture))
        .with_state(Capture(captured.clone()));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = rx.await;
            })
            .await
            .unwrap();
    });
    (format!("http://{addr}/v1"), captured, tx)
}

fn client(base_url: String, api_backend: ApiBackend) -> SamplingClient {
    SamplingClient::new(SamplerConfig {
        api_key: Some("test".into()),
        base_url,
        model: "test".into(),
        max_completion_tokens: None,
        temperature: None,
        top_p: None,
        api_backend,
        auth_scheme: Default::default(),
        extra_headers: IndexMap::new(),
        context_window: 1,
        force_http1: false,
        max_retries: None,
        stream_tool_calls: false,
        idle_timeout_secs: None,
        reasoning_effort: None,
        origin_client: None,
        client_identifier: None,
        deployment_id: None,
        user_id: None,
        client_version: None,
        attribution_callback: None,
        bearer_resolver: None,
        supports_backend_search: false,
        compactions_remaining: None,
        compaction_at_tokens: None,
        doom_loop_recovery: None,
        header_injector: None,
    })
    .unwrap()
}

fn request() -> ConversationRequest {
    ConversationRequest {
        items: vec![ConversationItem::user("hi")],
        reasoning_effort: Some(ReasoningEffort::Max),
        ..Default::default()
    }
}

#[tokio::test]
async fn max_is_sent_by_all_non_streaming_backends() {
    let (base_url, captured, shutdown) = spawn().await;
    let _ = client(base_url.clone(), ApiBackend::ChatCompletions)
        .conversation(request())
        .await;
    let _ = client(base_url.clone(), ApiBackend::Messages)
        .conversation_messages(request())
        .await;
    let _ = client(base_url, ApiBackend::Responses)
        .conversation_responses(request())
        .await;
    shutdown.send(()).unwrap();
    let captured = captured.lock().unwrap();
    assert_eq!(captured.len(), 3);
    assert_eq!(captured[0]["reasoning_effort"], "max");
    assert_eq!(captured[1]["output_config"]["effort"], "max");
    assert_eq!(captured[2]["reasoning"]["effort"], "max");
}

#[tokio::test]
async fn max_is_sent_by_responses_streaming() {
    let (base_url, captured, shutdown) = spawn().await;
    let (stream, _, _) = client(base_url, ApiBackend::Responses)
        .conversation_stream_responses(request())
        .await
        .unwrap();
    let _: Vec<_> = stream.collect().await;
    shutdown.send(()).unwrap();
    let captured = captured.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0]["reasoning"]["effort"], "max");
}

fn response_wrapper_with_effort(effort: ReasoningEffort) -> CreateResponseWrapper {
    let request = ConversationRequest {
        items: vec![ConversationItem::user("hi")],
        reasoning_effort: Some(ReasoningEffort::Xhigh),
        ..Default::default()
    };
    let inner: rs::CreateResponse = (&request).into();
    CreateResponseWrapper::new(inner).with_reasoning_effort(effort)
}

#[tokio::test]
async fn low_level_responses_request_sends_max_without_changing_xhigh() {
    let (base_url, captured, shutdown) = spawn().await;
    let client = client(base_url, ApiBackend::Responses);

    let _ = client
        .create_response(response_wrapper_with_effort(ReasoningEffort::Max))
        .await;
    let _ = client
        .create_response(response_wrapper_with_effort(ReasoningEffort::Xhigh))
        .await;

    shutdown.send(()).unwrap();
    let captured = captured.lock().unwrap();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0]["reasoning"]["effort"], "max");
    assert_eq!(captured[1]["reasoning"]["effort"], "xhigh");
}

#[tokio::test]
async fn low_level_responses_stream_sends_max_without_changing_xhigh() {
    let (base_url, captured, shutdown) = spawn().await;
    let client = client(base_url, ApiBackend::Responses);

    for effort in [ReasoningEffort::Max, ReasoningEffort::Xhigh] {
        let (stream, _, _) = client
            .create_response_stream(response_wrapper_with_effort(effort))
            .await
            .unwrap();
        let _: Vec<_> = stream.collect().await;
    }

    shutdown.send(()).unwrap();
    let captured = captured.lock().unwrap();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0]["reasoning"]["effort"], "max");
    assert_eq!(captured[1]["reasoning"]["effort"], "xhigh");
}
