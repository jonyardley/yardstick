# Building a Rust MCP Server Embedded in a macOS Desktop App (2026)

Research for Yardstick: a macOS app owning a SQLite knowledge base (notes, tasks, briefs) that external agents (Claude Code, a Claude briefing skill) must read **and** write via MCP tools (`search`, `create_task`, `update_task`, `write_brief`, `get_day`, `list_bucket`).

---

## 1. The official Rust MCP SDK: `rmcp`

**Crate:** `rmcp` (repo: [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk))
**Current version:** **2.1.0** (released 2026-07-02; 2.0.0 on 2026-06-29, 1.8.0 on 2026-06-23) — per [crates.io](https://crates.io/crates/rmcp)
**Maturity:** Past 1.0 and now 2.x; actively maintained with frequent releases. 2.0 was a breaking release that **aligned model types with the MCP 2025-11-25 spec** (migration guide provided). 2.1 fixed cancel-safety in `AsyncRwTransport`, OAuth spoofing/session-leak security issues, and refresh-token preservation. This is now a production-grade SDK — a big step up from the 0.x era. Notably, Warp maintains a fork ([warpdotdev/rmcp](https://github.com/warpdotdev/rmcp)), i.e., real products ship on it.

**Key features/flags:**

```toml
[dependencies]
rmcp = { version = "2", features = [
    "server",                          # ServerHandler, tool router (default-ish for servers)
    "macros",                          # #[tool], #[tool_router], #[tool_handler], #[prompt]
    "schemars",                        # JSON Schema generation for typed tool params
    "transport-io",                    # stdio transport
    "transport-streamable-http-server" # StreamableHttpService (tower/axum-compatible)
] }
# "auth" adds OAuth 2.0 support (mainly relevant for remote servers)
tokio = { version = "1", features = ["full"] }
axum = "0.8"
schemars = "1"
serde = { version = "1", features = ["derive"] }
```

Both **stdio** and **streamable HTTP** transports are first-class: `transport::stdio()` for child-process servers, and `StreamableHttpService` (a `tower` service you nest in an axum `Router`) for HTTP. Legacy SSE also exists but streamable HTTP is the current standard.

### Defining tools with typed params

Tools are methods on a struct; params are plain structs deriving `Deserialize + JsonSchema`, wrapped in `Parameters<T>` — the macro generates the JSON Schema advertised to clients:

```rust
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateTaskParams {
    /// Task title
    pub title: String,
    /// Bucket to file it under, e.g. "inbox", "work"
    pub bucket: Option<String>,
    /// ISO-8601 due date
    pub due: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WriteBriefParams {
    /// ISO date, e.g. "2026-07-02"
    pub date: String,
    /// Brief payload (markdown or structured JSON as string)
    pub payload: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetDayParams { pub date: String }

#[derive(Clone)]
pub struct YardstickMcp {
    db: crate::Db,               // your existing store handle (pool / actor)
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl YardstickMcp {
    pub fn new(db: crate::Db) -> Self {
        Self { db, tool_router: Self::tool_router() }
    }

    #[tool(description = "Full-text search across notes, tasks and briefs")]
    async fn search(&self, Parameters(p): Parameters<SearchParams>)
        -> Result<CallToolResult, McpError>
    {
        let hits = self.db.search(&p.query, p.limit.unwrap_or(20)).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::json(&hits)?]))
    }

    #[tool(description = "Create a task in the knowledge base")]
    async fn create_task(&self, Parameters(p): Parameters<CreateTaskParams>)
        -> Result<CallToolResult, McpError>
    {
        let task = self.db.create_task(p.title, p.bucket, p.due).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::json(&task)?]))
    }

    #[tool(description = "Write (upsert) the daily brief for a date")]
    async fn write_brief(&self, Parameters(p): Parameters<WriteBriefParams>)
        -> Result<CallToolResult, McpError>
    {
        self.db.upsert_brief(&p.date, &p.payload).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text("ok")]))
    }

    #[tool(description = "Get everything for a day: notes, tasks, brief")]
    async fn get_day(&self, Parameters(p): Parameters<GetDayParams>)
        -> Result<CallToolResult, McpError>
    { /* ... same pattern ... */ }
}

#[tool_handler]
impl ServerHandler for YardstickMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some("Yardstick knowledge base: notes, tasks, daily briefs. \
                Use get_day for a day overview; write_brief upserts.".into()),
            ..Default::default()
        }
    }
}
```

(For a tools-only server there's a shortcut: `#[tool_router(server_handler)]` on the impl block generates the `ServerHandler` too. `ServerHandler` also has overridable `list_resources` / `read_resource` methods if you want to expose briefs as `brief://2026-07-02` resources — but for agent read/write, tools are the right primitive; claude.ai/Claude Code exercise tools far more reliably than resources.)

### Serving over stdio

```rust
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = YardstickMcp::new(db).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

### Serving over streamable HTTP (embeddable in the app process)

```rust
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

let ct = tokio_util::sync::CancellationToken::new();
let db2 = db.clone();
let mcp = StreamableHttpService::new(
    move || Ok(YardstickMcp::new(db2.clone())),   // per-session factory
    LocalSessionManager::default().into(),
    StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
);

let router = axum::Router::new()
    .nest_service("/mcp", mcp)
    .layer(axum::middleware::from_fn(require_bearer_token)); // see §4

let listener = tokio::net::TcpListener::bind("127.0.0.1:52111").await?;
axum::serve(listener, router).await?;
```

`StreamableHttpServerConfig` also exposes `allowed_hosts` / `allowed_origins` (Host/Origin validation — the MCP spec requires Origin validation on local HTTP servers to block DNS-rebinding attacks), `stateful_mode`, `sse_keep_alive`, and `json_response` (plain JSON responses instead of SSE for stateless mode).

---

## 2. Deployment topology

### The client-connectivity matrix (this drives everything)

| Client | stdio (spawns binary) | HTTP to `http://127.0.0.1:PORT/mcp` | Remote custom connector |
|---|---|---|---|
| **Claude Code** | ✅ `claude mcp add name -- /path/to/bin` | ✅ `claude mcp add --transport http yardstick http://127.0.0.1:52111/mcp --header "Authorization: Bearer $TOK"` | ✅ |
| **Claude Desktop / Cowork (local config)** | ✅ `claude_desktop_config.json` / `.mcpb` bundles | ❌ rejects plain-`http` URLs even for localhost ([known issue](https://github.com/anthropics/claude-ai-mcp/issues/9)); bridge via `mcp-remote` stdio shim | — |
| **claude.ai web / mobile connectors** | ❌ | ❌ — connectors connect **from Anthropic's cloud**, need public **HTTPS** with CA cert ([docs](https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp)) | ✅ only if you tunnel/expose it |

Takeaway: **Claude Code talks to localhost HTTP natively; everything stdio-only can be bridged with a ~30-line shim or `mcp-remote`.** claude.ai-hosted connectors can never see your laptop's localhost — if the "briefing skill" runs in Claude Code/Desktop locally you're fine; if it must run on claude.ai you'd need a tunnel (not recommended for a personal KB).

### Option A — MCP server embedded in the GUI app process (streamable HTTP on localhost)

- **How:** app owns SQLite; a tokio runtime inside the app runs the axum/`StreamableHttpService` server on `127.0.0.1:<fixed-port>`.
- **Pros:** single process owns the DB → zero multi-process SQLite concerns; MCP writes go through the *same* domain layer as the UI (validation, FTS indexing, undo, live UI refresh via your existing event/update path — with Crux, tool handlers can dispatch the same events the UI does); one migration path; trivially consistent.
- **Cons:** server is down when the app isn't running (agents get connection refused); needs a port + token; stdio-only clients need a bridge.

### Option B — separate stdio binary opening the same SQLite file

- **How:** ship `yardstick-mcp` binary; clients spawn it per session; it opens the DB directly.
- **SQLite reality:** works *if* the DB is in **WAL mode** on a local disk: WAL allows many concurrent readers + one writer across processes; set `PRAGMA busy_timeout` (e.g. 5000ms) on all connections so writers queue instead of erroring `SQLITE_BUSY`. Real costs: **two write paths** must share schema migrations, FTS-index maintenance, and invariants (extract a shared `yardstick-core` crate or you *will* drift); the GUI doesn't learn about external writes without polling `PRAGMA data_version`/file-watching; a mid-migration launch of the other process is a footgun.
- **Pros:** works when the app is closed; zero-config for stdio clients (no port/token).
- **Cons:** dual-writer complexity; no push into a running GUI; each client session spawns its own process.

### Option C — always-on daemon owning the DB; GUI is a client

- **How:** launchd LaunchAgent runs a headless daemon (DB owner + MCP HTTP server); the GUI talks to it via the same HTTP/IPC.
- **Pros:** always available; cleanest single-writer story; survives GUI crashes.
- **Cons:** highest engineering cost (daemon lifecycle, versioned GUI↔daemon protocol, upgrade coordination, launchd plumbing); the GUI loses in-process access to its own data; overkill for a single-user personal KB.

### Recommended topology for Yardstick

**Option A (embedded streamable HTTP server in the app) + a thin stdio→HTTP bridge, with the domain logic factored into a core crate as the escape hatch.**

1. **Embed** the rmcp streamable HTTP server in the app process on `127.0.0.1:52111` (fixed, configurable). This matches your architecture (app = single source of truth) and gives MCP writes full domain semantics and instant UI updates.
2. **Claude Code** connects directly: `claude mcp add --transport http yardstick http://127.0.0.1:52111/mcp --header "Authorization: Bearer $(cat ~/Library/Application\ Support/Yardstick/mcp-token)"`.
3. **stdio-only clients** (Claude Desktop/Cowork local config): ship a tiny `yardstick-mcp-stdio` proxy — an rmcp *client* to localhost HTTP re-exposed over stdio (or just document `npx mcp-remote http://127.0.0.1:52111/mcp --header ...`). Bonus: the shim can `open -g -a Yardstick` and retry if the port is closed, which mostly solves "app not running."
4. **"App not running" story:** (a) menu-bar/login-item mode so the app is effectively always on (the Obsidian-style answer), and/or (b) the shim auto-launches the app. Only if headless access becomes a hard requirement later, promote the core crate into an Option-B stdio binary (WAL + `busy_timeout` makes it safe for occasional dual access) or an Option-C daemon — the code layering (`yardstick-core` = DB + domain; `yardstick-mcp` = rmcp tool layer; shells choose transport) keeps all three reachable.

---

## 3. Auth for localhost HTTP

The MCP OAuth 2.1 authorization flow is designed for remote servers; for a local personal server the accepted pattern (and what Obsidian's plugin does) is:

- **Bind to `127.0.0.1` only** (never `0.0.0.0`).
- **Static bearer token**: generate 32 random bytes on first run, store at `~/Library/Application Support/Yardstick/mcp-token` with `0600` perms (or Keychain); show a "copy Claude Code setup command" button in settings.
- **Validate `Origin`/`Host`** (spec requirement for local servers, anti-DNS-rebinding) — rmcp's `StreamableHttpServerConfig.allowed_hosts/allowed_origins` covers this.

```rust
async fn require_bearer_token(
    req: axum::extract::Request, next: axum::middleware::Next,
) -> axum::response::Response {
    let ok = req.headers().get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|t| constant_time_eq(t.as_bytes(), stored_token().as_bytes()));
    if ok { next.run(req).await }
    else { axum::http::StatusCode::UNAUTHORIZED.into_response() }
}
```

Claude Code stores the header in its MCP config and sends it on every request; HTTP servers in Claude Code also support OAuth natively if you ever outgrow the static token.

---

## 4. Prior art

- **Obsidian** — closest analogue and validates the recommended topology. The [Local REST API plugin](https://github.com/coddingtonbear/obsidian-local-rest-api) runs an HTTPS server **inside the app process** on `127.0.0.1:27124`, authenticated by an API key; it now advertises itself as "a secure REST API **and MCP server** for your vault." Historically a separate stdio bridge ([mcp-obsidian](https://github.com/MarkusPfundstein/mcp-obsidian), or [obsidian-mcp-tools](https://github.com/jacksteamdev/obsidian-mcp-tools) which installs a bridge binary) translated MCP↔REST for stdio-only clients — exactly the "embedded server + stdio shim" pattern. Limitation users hit: nothing works when Obsidian is closed; the community answer is "leave it running."
- **Tana** — ships a *local* MCP server with the desktop app (`tana-local`) in addition to the older cloud Input-API-based [tana-mcp](https://github.com/tim-mcdonnell/tana-mcp) community server; same in-app-server direction.
- **Raycast** — MCP *client*, not server (installs stdio servers, supports HTTP/SSE servers); useful as a target client, not as prior art for serving.
- **Notion** — went the opposite way: a **cloud-hosted** remote MCP with OAuth, viable because their data already lives server-side. Not applicable to a local-first SQLite app.

Pattern across the ecosystem: **local-first apps embed an HTTP(ish) server on localhost with an API key and let bridges adapt to stdio; cloud apps host remote MCP with OAuth.**

## 5. Risks / notes

- rmcp 2.x is young (2.0 landed June 2026); pin the minor version and skim the migration guide when bumping — 2.0 renamed/aligned model types with the MCP 2025-11-25 spec.
- If tool handlers use `rusqlite` (sync), run DB work via `tokio::task::spawn_blocking` or an actor/channel to avoid blocking the server runtime; `sqlx`(sqlite) is async-native.
- Fixed port can collide; on bind failure, pick a free port and write it to a well-known discovery file the shim reads.
- Keep tool results compact JSON (agents pay tokens for output); include ids in every response so `update_task` round-trips work.

**Sources:** [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) · [rmcp on crates.io](https://crates.io/crates/rmcp) · [rmcp README](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/README.md) · [rust-sdk releases](https://github.com/modelcontextprotocol/rust-sdk/releases) · [Claude Code MCP docs](https://code.claude.com/docs/en/mcp) · [claude.ai custom connectors (remote MCP)](https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp) · [Claude Desktop local MCP servers](https://support.claude.com/en/articles/10949351-getting-started-with-local-mcp-servers-on-claude-desktop) · [Claude Desktop rejects http://localhost issue](https://github.com/anthropics/claude-ai-mcp/issues/9) · [obsidian-local-rest-api](https://github.com/coddingtonbear/obsidian-local-rest-api) · [mcp-obsidian](https://github.com/MarkusPfundstein/mcp-obsidian) · [obsidian-mcp-tools](https://github.com/jacksteamdev/obsidian-mcp-tools) · [tana-mcp](https://github.com/tim-mcdonnell/tana-mcp) · [Raycast MCP](https://www.raycast.com/changelog)