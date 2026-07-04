use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use mcp::{DailyMcp, EventSink, TaskReader};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, ClientCapabilities, ClientInfo, ErrorCode, Implementation},
    service::ServiceError,
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
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

async fn connect(
    bound: SocketAddr,
) -> rmcp::service::RunningService<rmcp::service::RoleClient, ClientInfo> {
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(format!("http://{bound}/mcp"))
            .auth_header(TOKEN),
    );
    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("mcp-tools-test", "0.0.1"),
    );
    client_info.serve(transport).await.unwrap()
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
async fn create_task_tool_dispatches_core_event() {
    let sink = Arc::new(StubSink::default());
    let reader = Arc::new(StubReader(vec![]));
    let bound = start_server(sink.clone(), reader).await;

    let client = connect(bound).await;
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

    let client = connect(bound).await;

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

    let client = connect(bound).await;

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
