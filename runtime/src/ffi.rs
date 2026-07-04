//! BoltFFI surface for the Swift shell.
//!
//! Mirrors the crux `examples/counter-routing` `ffi.rs` (the canonical
//! EffectRouter FFI): the shell pushes bincode-encoded events in through
//! [`CoreFFI`], and serialized effect batches are pushed out asynchronously
//! through the [`CruxShell`] callback — `update` and `resolve_serialized`
//! return nothing synchronously.
//!
//! Deviations from counter-routing, both because this crate is native-only
//! (no wasm target, so no need for cfg-independent signatures):
//! - `update`/`resolve_serialized` return `()` instead of an always-empty
//!   `Vec<u8>`.
//! - No narrowed FFI effect enum: `shared::Effect` already implements
//!   `EffectFFI`, and the routing closure consumes `Storage` before the
//!   serialized fall-through (see `crate::router`).

use std::sync::Arc;

use crux_core::bridge::EffectId;

use crate::{AppRuntime, ShellCallback};

/// Implemented by the Swift shell; BoltFFI generates the Swift protocol.
#[boltffi::export]
pub trait CruxShell: Send + Sync {
    /// Called whenever effects need processing by the shell.
    ///
    /// The bytes are a bincode-serialized vector of effect requests
    /// (`Vec<Request<EffectFfi>>` — decode with the generated `App` types).
    fn process_effects(&self, bytes: Vec<u8>);
}

/// Adapts the FFI-facing [`CruxShell`] to the runtime's [`ShellCallback`].
struct ShellAdapter(Arc<dyn CruxShell>);

impl ShellCallback for ShellAdapter {
    fn process_effects(&self, effects_bincode: Vec<u8>) {
        self.0.process_effects(effects_bincode);
    }
}

/// The main interface used by the Swift shell.
///
/// Construction cannot fail across FFI, so a database-open failure parks
/// the core in an inert state: `init_error()` returns the message (the
/// shell must check it right after construction and show alert+quit UX);
/// every other method is a no-op until then.
pub struct CoreFFI {
    runtime: Option<Arc<AppRuntime>>,
    init_error: Option<String>,
}

#[boltffi::export]
impl CoreFFI {
    /// Build the core. `db_path` is the SQLite database file path; an empty
    /// string opens an in-memory database (used by tests).
    #[must_use]
    pub fn new(db_path: String, shell: Arc<dyn CruxShell>) -> Self {
        let path = (!db_path.is_empty()).then(|| std::path::PathBuf::from(db_path));
        match AppRuntime::new(path.as_deref(), Arc::new(ShellAdapter(shell))) {
            Ok(runtime) => Self {
                runtime: Some(runtime),
                init_error: None,
            },
            Err(e) => Self {
                runtime: None,
                init_error: Some(format!("{e:#}")),
            },
        }
    }

    /// Empty string = healthy. Non-empty = fatal init failure (the database
    /// could not be opened or migrated); the shell surfaces it and quits.
    #[must_use]
    pub fn init_error(&self) -> String {
        self.init_error.clone().unwrap_or_default()
    }

    /// Send a bincode-serialized `shared::Event` to the app. Any resulting
    /// effects arrive asynchronously via [`CruxShell::process_effects`].
    ///
    /// # Panics
    /// If the event cannot be deserialized (a shell/typegen mismatch) —
    /// unchanged contract (spec §8). No-op on a failed-init core.
    pub fn update(&self, data: &[u8]) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        runtime
            .router
            .routes
            .serialized
            .update(data)
            .expect("event should deserialize");
    }

    /// Resolve a serialized-lane effect with its bincode-serialized output.
    ///
    /// # Panics
    /// If the id is unknown or the output cannot be deserialized (a shell
    /// bug — mirrors counter-routing). No-op on a failed-init core.
    pub fn resolve_serialized(&self, effect_id: u32, data: &[u8]) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        runtime
            .router
            .routes
            .serialized
            .resolve(EffectId(effect_id), data)
            .expect("failed to resolve effect");
    }

    /// Get the current view model, bincode-serialized (`shared::ViewModel`).
    /// Empty bytes on a failed-init core (the shell never gets here: it
    /// checks `init_error` first and shows the failure screen).
    #[must_use]
    pub fn view(&self) -> Vec<u8> {
        let Some(runtime) = &self.runtime else {
            return Vec::new();
        };
        runtime
            .router
            .routes
            .serialized
            .view()
            .expect("view model should serialize")
    }

    /// Starts the embedded MCP server on `port` (0 = ephemeral), guarded by
    /// `token`. Returns the bound port, or 0 on failure.
    pub fn start_mcp(&self, port: u16, token: String) -> u16 {
        let Some(runtime) = &self.runtime else {
            return 0;
        };
        let db_path = runtime.db_path().map(std::path::Path::to_path_buf);
        crate::start_mcp(runtime.clone(), db_path, port, token)
            .map_err(|e| eprintln!("daily: MCP server failed to start: {e:#}"))
            .unwrap_or(0)
    }
}
