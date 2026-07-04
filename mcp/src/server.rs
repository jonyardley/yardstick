use std::{net::SocketAddr, sync::Arc};

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::{EventSink, TaskReader, auth};

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct CreateTaskParams {
    /// Task title
    pub title: String,
}

#[derive(Clone)]
pub struct DailyMcp {
    reader: Arc<dyn TaskReader>,
    events: Arc<dyn EventSink>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl DailyMcp {
    pub fn new(reader: Arc<dyn TaskReader>, events: Arc<dyn EventSink>) -> Self {
        Self {
            reader,
            events,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Health check — returns 'pong'")]
    async fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text("pong")]))
    }

    #[tool(description = "Create a task in Daily's knowledge base")]
    async fn create_task(
        &self,
        Parameters(p): Parameters<CreateTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        if p.title.trim().is_empty() {
            return Err(McpError::invalid_params("title must be non-empty", None));
        }
        self.events.send_event(shared::Event::CreateTask {
            title: p.title.clone(),
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(format!(
            "created task: {}",
            p.title
        ))]))
    }

    #[tool(description = "List all tasks")]
    async fn list_tasks(&self) -> Result<CallToolResult, McpError> {
        let tasks = self
            .reader
            .list_tasks()
            .map_err(|e| McpError::internal_error(e, None))?;
        Ok(CallToolResult::success(vec![ContentBlock::json(&tasks)?]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DailyMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Daily knowledge base (walking skeleton): create_task and list_tasks.",
        )
    }
}

/// Bind + serve; returns the bound addr and the server future (tests use
/// port 0). `serve_http` is the production wrapper that just awaits it.
///
/// The MCP surface is local-only by design: any non-loopback bind address
/// is refused before a socket is opened.
pub async fn serve_http_on(
    mcp: DailyMcp,
    addr: SocketAddr,
    token: String,
) -> anyhow::Result<(
    SocketAddr,
    impl std::future::Future<Output = anyhow::Result<()>>,
)> {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };

    anyhow::ensure!(
        addr.ip().is_loopback(),
        "MCP server must bind a loopback address, got {addr}"
    );

    let ct = tokio_util::sync::CancellationToken::new();
    // Anti-DNS-rebinding (spec §5): `StreamableHttpServerConfig::default()`
    // already restricts `allowed_hosts` to loopback names/addresses
    // (`localhost`, `127.0.0.1`, `::1`), and — because those entries carry no
    // port — matches them on *any* port, so real clients on an arbitrary
    // loopback port are admitted while a rebound hostile hostname is not.
    // `allowed_origins` defaults to *empty*, which disables Origin validation
    // entirely (rmcp only enforces it when the list is non-empty); set it
    // explicitly to the same loopback origins so a hostile page's
    // browser-borne `Origin` header is rejected too. Local (non-browser) MCP
    // clients don't send an `Origin` header at all, so this list only ever
    // matters for requests that do carry one.
    let config = StreamableHttpServerConfig::default()
        .with_allowed_origins(["http://localhost", "http://127.0.0.1", "http://[::1]"])
        .with_cancellation_token(ct.child_token());
    let service = StreamableHttpService::new(
        move || Ok(mcp.clone()),
        LocalSessionManager::default().into(),
        config,
    );

    let router = axum::Router::new().nest_service("/mcp", service).layer(
        axum::middleware::from_fn_with_state(Arc::new(token), auth::require_bearer_token),
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    let fut = async move {
        axum::serve(listener, router).await?;
        Ok(())
    };
    Ok((bound, fut))
}

/// Production entrypoint: bind `addr` (loopback-only) and serve until the
/// process exits.
pub async fn serve_http(mcp: DailyMcp, addr: SocketAddr, token: String) -> anyhow::Result<()> {
    let (_, fut) = serve_http_on(mcp, addr, token).await?;
    fut.await
}
