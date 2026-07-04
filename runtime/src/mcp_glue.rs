//! Embeds `mcp`'s streamable-HTTP server in a dedicated OS thread hosting its
//! own tokio runtime, wiring it to the real core: tool calls become
//! `shared::Event`s dispatched through [`AppRuntime::send_event`], and reads
//! are served from a read-only SQLite connection over the runtime's
//! database file.

use std::{path::PathBuf, sync::Arc};

use crate::AppRuntime;

struct RuntimeSink(Arc<AppRuntime>);
impl mcp::EventSink for RuntimeSink {
    fn send_event(&self, event: shared::Event) {
        self.0.send_event(event);
    }
}

/// Spawns a dedicated thread running its own tokio runtime that serves the
/// MCP surface, wired to `runtime`. Returns the bound port once the server
/// has actually bound (or an error if it never managed to).
///
/// `port: 0` binds an ephemeral port (used by tests). `db_path` must be
/// `Some` (decision #4): an in-memory runtime has no shareable database
/// file for a second connection to read, so MCP cannot be started against
/// one.
pub fn start_mcp(
    runtime: Arc<AppRuntime>,
    db_path: Option<PathBuf>,
    port: u16,
    token: String,
) -> anyhow::Result<u16> {
    let Some(db_path) = db_path else {
        anyhow::bail!(
            "MCP requires an on-disk database (in-memory runtimes have no shareable file)"
        );
    };
    let reader = mcp::StoreReader::new(store::open_read_only(&db_path)?);
    let daily = mcp::DailyMcp::new(Arc::new(reader), Arc::new(RuntimeSink(runtime)));
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
