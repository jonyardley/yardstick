use std::sync::Arc;

use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use runtime::{AppRuntime, ShellCallback};
use shared::Event;

struct NullShell;
impl ShellCallback for NullShell {
    fn process_effects(&self, _: Vec<u8>) {}
}

async fn connect(
    port: u16,
    token: &str,
) -> rmcp::service::RunningService<rmcp::service::RoleClient, ClientInfo> {
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(format!("http://127.0.0.1:{port}/mcp"))
            .auth_header(token),
    );
    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("runtime-e2e-test", "0.0.1"),
    );
    client_info.serve(transport).await.unwrap()
}

/// The full loop the product depends on: MCP tool call -> core event ->
/// storage -> view reflects it (what the GUI renders).
#[tokio::test]
async fn mcp_create_task_updates_core_view() {
    let rt = AppRuntime::new(None, Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup);

    let port = runtime::start_mcp(rt.clone(), None, 0, "sekrit".into()).unwrap();

    let client = connect(port, "sekrit").await;
    client
        .call_tool(
            CallToolRequestParams::new("create_task").with_arguments(
                serde_json::json!({"title": "Via MCP"})
                    .as_object()
                    .cloned()
                    .unwrap(),
            ),
        )
        .await
        .unwrap();
    client.cancel().await.unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if rt.view().tasks.iter().any(|t| t.title == "Via MCP") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "MCP write never reached the view"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

/// Proves the reader path end-to-end: list_tasks through MCP reflects the
/// core view (the Phase 0 `ViewReader` seam), not a second store connection.
#[tokio::test]
async fn mcp_list_tasks_returns_created_task() {
    let rt = AppRuntime::new(None, Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup);

    let port = runtime::start_mcp(rt.clone(), None, 0, "sekrit".into()).unwrap();

    let client = connect(port, "sekrit").await;
    client
        .call_tool(
            CallToolRequestParams::new("create_task").with_arguments(
                serde_json::json!({"title": "Listed via MCP"})
                    .as_object()
                    .cloned()
                    .unwrap(),
            ),
        )
        .await
        .unwrap();

    // Wait for the core view to catch up before asserting the reader
    // reflects it (the reader wraps the same view).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if rt.view().tasks.iter().any(|t| t.title == "Listed via MCP") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "MCP write never reached the view"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    let result = client
        .call_tool(CallToolRequestParams::new("list_tasks"))
        .await
        .unwrap();
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].as_text().unwrap().text.clone();
    let tasks: Vec<shared::Task> = serde_json::from_str(&text).unwrap();
    assert!(tasks.iter().any(|t| t.title == "Listed via MCP"));

    client.cancel().await.unwrap();
}

/// Auth holds through the embedding: a client with the wrong token cannot
/// complete the handshake / call tools.
#[tokio::test]
async fn mcp_wrong_token_client_fails() {
    let rt = AppRuntime::new(None, Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup);

    let port = runtime::start_mcp(rt.clone(), None, 0, "sekrit".into()).unwrap();

    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(format!("http://127.0.0.1:{port}/mcp"))
            .auth_header("wrong-token"),
    );
    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("runtime-e2e-test-badauth", "0.0.1"),
    );
    // The bearer-auth middleware rejects the request before the MCP
    // handshake completes, so `serve` itself should fail.
    let result = client_info.serve(transport).await;
    assert!(
        result.is_err(),
        "client with wrong token should not complete the MCP handshake"
    );
}
