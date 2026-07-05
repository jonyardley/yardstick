use std::sync::Arc;

use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use runtime::AppRuntime;
use shared::Event;

mod common;
use common::NullShell;

/// The full loop the product depends on: MCP tool call -> core event ->
/// storage thread writes the FILE -> the read-only MCP reader sees it,
/// and so does the core view.
#[tokio::test]
async fn mcp_create_task_reaches_the_database_and_the_view() {
    let dir = std::env::temp_dir().join(format!("daily-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("e2e.db");

    let rt = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup {
        today: "2026-07-04".into(),
    });

    let port = runtime::start_mcp(rt.clone(), Some(db.clone()), 0, "sekrit".into()).unwrap();
    let client = mcp::test_support::connect(format!("127.0.0.1:{port}"), "sekrit").await;

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

    // Poll THROUGH THE READER (read-only SQLite conn on the same file):
    let ro = store::open_read_only(&db).unwrap();
    common::poll_until(5, "MCP write to land in the database", || {
        matches!(
            store::execute(&ro, &shared::StorageOperation::ListTasks),
            shared::StorageResult::Tasks(tasks) if tasks.iter().any(|t| t.title == "Via MCP")
        )
    });
    // ...and the core view agrees (same event drove both).
    common::poll_until(5, "MCP write to reach the core view", || {
        rt.view()
            .sidebar
            .views
            .iter()
            .any(|v| v.kind == "inbox" && v.count == 1)
    });

    client.cancel().await.unwrap();
    std::fs::remove_dir_all(&dir).ok();
}

/// Decision #4: an in-memory runtime has no shareable database file, so
/// starting MCP against it is an error — not a silently different reader.
#[test]
fn start_mcp_without_a_db_path_is_an_error() {
    let rt = AppRuntime::new(None, Arc::new(NullShell)).unwrap();
    let result = runtime::start_mcp(rt, None, 0, "sekrit".into());
    assert!(result.is_err());
}

/// Auth holds through the embedding: a client with the wrong token cannot
/// complete the handshake / call tools.
#[tokio::test]
async fn mcp_wrong_token_client_fails() {
    let dir = std::env::temp_dir().join(format!("daily-e2e-badauth-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("badauth.db");

    let rt = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup {
        today: "2026-07-04".into(),
    });

    let port = runtime::start_mcp(rt.clone(), Some(db.clone()), 0, "sekrit".into()).unwrap();

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

    std::fs::remove_dir_all(&dir).ok();
}
