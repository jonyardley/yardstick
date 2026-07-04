//! Embeds `mcp`'s streamable-HTTP server in a dedicated OS thread hosting its
//! own tokio runtime, wiring it to the real core: tool calls become
//! `shared::Event`s dispatched through [`AppRuntime::send_event`], and reads
//! are served from the core's view.

use std::{path::PathBuf, sync::Arc};

use shared::Task;

use crate::AppRuntime;

struct RuntimeSink(Arc<AppRuntime>);
impl mcp::EventSink for RuntimeSink {
    fn send_event(&self, event: shared::Event) {
        self.0.send_event(event);
    }
}

/// Phase 0 reader: serve reads from the core's view rather than a second
/// SQLite connection. This is a real constraint, not a shortcut: `AppRuntime`
/// may be backed by an in-memory database (tests, and possibly ephemeral
/// dev runs), and a second `rusqlite::Connection` opened against `:memory:`
/// is a *different* database from the runtime's — it would never see writes
/// the storage thread makes. Reading through `runtime.view()` is always
/// consistent with what the GUI renders because it's the same core state.
/// `_db_path` is kept (unused for now) so Phase 1 can swap this for a
/// read-only on-disk connection with richer queries without changing the
/// `start_mcp` signature.
struct ViewReader(Arc<AppRuntime>);
impl mcp::TaskReader for ViewReader {
    fn list_tasks(&self) -> Result<Vec<Task>, String> {
        Ok(self.0.view().tasks)
    }
}

/// Spawns a dedicated thread running its own tokio runtime that serves the
/// MCP surface, wired to `runtime`. Returns the bound port once the server
/// has actually bound (or an error if it never managed to).
///
/// `port: 0` binds an ephemeral port (used by tests); `db_path` is currently
/// unused (see [`ViewReader`]) and kept for the Phase 1 on-disk reader swap.
pub fn start_mcp(
    runtime: Arc<AppRuntime>,
    _db_path: Option<PathBuf>,
    port: u16,
    token: String,
) -> anyhow::Result<u16> {
    let daily = mcp::DailyMcp::new(
        Arc::new(ViewReader(runtime.clone())),
        Arc::new(RuntimeSink(runtime)),
    );
    let (port_tx, port_rx) = std::sync::mpsc::channel();

    std::thread::Builder::new()
        .name("daily-mcp".into())
        .spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    // Nothing to send the port back on failure here except
                    // the tokio-runtime construction error itself; the
                    // caller's `recv_timeout` below covers the (unlikely)
                    // case this send is somehow lost too.
                    let _ = port_tx.send(Err(anyhow::anyhow!(e)));
                    return;
                }
            };
            rt.block_on(async move {
                let addr = match format!("127.0.0.1:{port}").parse::<std::net::SocketAddr>() {
                    Ok(addr) => addr,
                    Err(e) => {
                        let _ = port_tx.send(Err(anyhow::anyhow!(e)));
                        return;
                    }
                };
                match mcp::serve_http_on(daily, addr, token).await {
                    Ok((bound, fut)) => {
                        let _ = port_tx.send(Ok(bound.port()));
                        if let Err(e) = fut.await {
                            eprintln!("mcp server exited: {e}");
                        }
                    }
                    Err(e) => {
                        let _ = port_tx.send(Err(e));
                    }
                }
            });
        })?;

    // The brief's sketch used a plain `recv()`, which hangs forever if the
    // spawned thread panics before sending (e.g. the tokio runtime failing
    // to build) or is killed some other way without a send. Bound the wait
    // so a bind failure surfaces as an `Err` within a few seconds instead of
    // wedging the caller (and, transitively, `CoreFFI::start_mcp`'s caller
    // on the Swift side) indefinitely.
    port_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| anyhow::anyhow!("MCP server did not report its bound port within 5s"))?
}
