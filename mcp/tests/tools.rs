use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use mcp::{DailyMcp, EventSink, TaskReader};
use rmcp::model::{CallToolRequestParams, ErrorCode};
use rmcp::service::ServiceError;
use shared::{Event, Task};

#[derive(Default)]
struct StubSink(Mutex<Vec<Event>>);

impl EventSink for StubSink {
    fn send_event(&self, event: Event) {
        self.0.lock().unwrap().push(event);
    }
}

struct StubReader(Vec<Task>);

impl TaskReader for StubReader {
    fn list_tasks(&self) -> Result<Vec<Task>, String> {
        Ok(self.0.clone())
    }
}

const TOKEN: &str = "sekrit";

async fn start_server(sink: Arc<StubSink>, reader: Arc<StubReader>) -> SocketAddr {
    let daily = DailyMcp::new(reader, sink);
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (bound, server) = mcp::serve_http_on(daily, addr, TOKEN.into()).await.unwrap();
    tokio::spawn(server);
    bound
}

#[tokio::test]
async fn request_without_or_with_wrong_token_is_rejected_with_401() {
    let sink = Arc::new(StubSink::default());
    let reader = Arc::new(StubReader(vec![]));
    let bound = start_server(sink.clone(), reader).await;

    let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "ping"});

    // No Authorization header at all.
    let resp = reqwest::Client::new()
        .post(format!("http://{bound}/mcp"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // Wrong bearer token.
    let resp = reqwest::Client::new()
        .post(format!("http://{bound}/mcp"))
        .header("Authorization", "Bearer wrong-token")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // Nothing reached the core.
    assert!(sink.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn request_with_hostile_host_header_is_rejected() {
    let sink = Arc::new(StubSink::default());
    let reader = Arc::new(StubReader(vec![]));
    let bound = start_server(sink.clone(), reader).await;

    let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "ping"});

    // Valid bearer token, but a foreign Host header (DNS-rebinding attempt):
    // the anti-rebinding check must reject this before it ever reaches auth
    // or the core, regardless of a correct token. rmcp's DNS-rebinding guard
    // answers disallowed Host headers with 403 Forbidden specifically (see
    // `forbidden_response` in streamable_http_server/tower.rs) — asserting
    // exactly that status (not just "some 4xx") proves the Host check itself
    // fired, rather than some unrelated downstream rejection.
    let resp = reqwest::Client::new()
        .post(format!("http://{bound}/mcp"))
        .header("Host", "evil.example.com")
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::FORBIDDEN,
        "expected 403 Forbidden from the Host-validation guard, got {}",
        resp.status()
    );

    assert!(sink.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn request_with_hostile_origin_header_is_rejected() {
    let sink = Arc::new(StubSink::default());
    let reader = Arc::new(StubReader(vec![]));
    let bound = start_server(sink.clone(), reader).await;

    let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "ping"});

    // Valid bearer token, legitimate loopback Host, but a foreign Origin —
    // this is the browser-borne DNS-rebinding vector Origin validation guards
    // against (a malicious page in the victim's browser posting to our
    // localhost server). Must be rejected with 403 Forbidden (the same
    // `forbidden_response` the Host guard uses) even though Host is fine —
    // asserting exactly 403 (not just "some 4xx") rules out an unrelated
    // downstream rejection (e.g. a 422 from the session/handshake layer)
    // masquerading as Origin validation having run.
    let resp = reqwest::Client::new()
        .post(format!("http://{bound}/mcp"))
        .header("Origin", "https://evil.example.com")
        .header("Authorization", format!("Bearer {TOKEN}"))
        .header("Accept", "application/json, text/event-stream")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::FORBIDDEN,
        "expected 403 Forbidden from the Origin-validation guard, got {}",
        resp.status()
    );

    assert!(sink.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn request_with_legitimate_localhost_origin_and_arbitrary_port_is_not_blocked() {
    let sink = Arc::new(StubSink::default());
    let reader = Arc::new(StubReader(vec![]));
    let bound = start_server(sink.clone(), reader).await;

    let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "ping"});

    // Real local clients (e.g. a loopback-served page, or an agent tool that
    // sets Origin) run on an arbitrary, unpredictable port — our allowlist
    // entries (`http://127.0.0.1`, no port) must admit any port, not just a
    // fixed one, or legitimate local callers would be locked out.
    let resp = reqwest::Client::new()
        .post(format!("http://{bound}/mcp"))
        .header("Origin", "http://127.0.0.1:54321")
        .header("Authorization", format!("Bearer {TOKEN}"))
        .header("Accept", "application/json, text/event-stream")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_ne!(
        resp.status(),
        reqwest::StatusCode::FORBIDDEN,
        "legitimate localhost Origin with a port must not be blocked by Origin validation, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn create_task_tool_dispatches_core_event() {
    let sink = Arc::new(StubSink::default());
    let reader = Arc::new(StubReader(vec![]));
    let bound = start_server(sink.clone(), reader).await;

    let client = mcp::test_support::connect(bound, TOKEN).await;
    let result = client
        .call_tool(
            CallToolRequestParams::new("create_task").with_arguments(
                serde_json::json!({"title": "From MCP"})
                    .as_object()
                    .cloned()
                    .unwrap(),
            ),
        )
        .await
        .unwrap();
    assert!(!result.is_error.unwrap_or(false));

    {
        let events = sink.0.lock().unwrap();
        assert!(matches!(&events[..], [Event::CreateTask { title }] if title == "From MCP"));
    }

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn list_tasks_returns_reader_tasks_and_ping_returns_pong() {
    let sink = Arc::new(StubSink::default());
    let reader = Arc::new(StubReader(vec![Task {
        id: "t1".into(),
        title: "existing".into(),
    }]));
    let bound = start_server(sink, reader).await;

    let client = mcp::test_support::connect(bound, TOKEN).await;

    let result = client
        .call_tool(CallToolRequestParams::new("ping"))
        .await
        .unwrap();
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].as_text().unwrap().text.clone();
    assert_eq!(text, "pong");

    let result = client
        .call_tool(CallToolRequestParams::new("list_tasks"))
        .await
        .unwrap();
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].as_text().unwrap().text.clone();
    let tasks: Vec<Task> = serde_json::from_str(&text).unwrap();
    assert_eq!(
        tasks,
        vec![Task {
            id: "t1".into(),
            title: "existing".into()
        }]
    );

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn create_task_with_empty_title_is_rejected_and_dispatches_nothing() {
    let sink = Arc::new(StubSink::default());
    let reader = Arc::new(StubReader(vec![]));
    let bound = start_server(sink.clone(), reader).await;

    let client = mcp::test_support::connect(bound, TOKEN).await;

    for bad_title in ["", "   ", "\t\n"] {
        let err = client
            .call_tool(
                CallToolRequestParams::new("create_task").with_arguments(
                    serde_json::json!({"title": bad_title})
                        .as_object()
                        .cloned()
                        .unwrap(),
                ),
            )
            .await
            .unwrap_err();
        match err {
            ServiceError::McpError(data) => assert_eq!(data.code, ErrorCode::INVALID_PARAMS),
            other => panic!("expected invalid-params MCP error, got: {other:?}"),
        }
    }

    assert!(sink.0.lock().unwrap().is_empty());
    client.cancel().await.unwrap();
}
