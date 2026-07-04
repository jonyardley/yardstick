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
pub struct CoreFFI {
    runtime: Arc<AppRuntime>,
}

#[boltffi::export]
impl CoreFFI {
    /// Build the core. `db_path` is the SQLite database file path; an empty
    /// string opens an in-memory database (used by tests).
    ///
    /// # Panics
    /// If the database cannot be opened/migrated. There is nothing useful
    /// the shell can do without a core, so this is fatal by design.
    #[must_use]
    pub fn new(db_path: String, shell: Arc<dyn CruxShell>) -> Self {
        let path = (!db_path.is_empty()).then(|| std::path::PathBuf::from(db_path));
        let runtime = AppRuntime::new(path.as_deref(), Arc::new(ShellAdapter(shell)))
            .expect("runtime should initialise");
        Self { runtime }
    }

    /// Send a bincode-serialized `shared::Event` to the app. Any resulting
    /// effects arrive asynchronously via [`CruxShell::process_effects`].
    ///
    /// # Panics
    /// If the event cannot be deserialized (a shell/typegen mismatch).
    pub fn update(&self, data: &[u8]) {
        self.runtime
            .router
            .routes
            .serialized
            .update(data)
            .expect("event should deserialize");
    }

    /// Resolve a serialized-lane effect with its bincode-serialized output.
    /// Phase 0 only ever sends Render, which needs no resolution — this is
    /// wired to the real lane for forward-compatibility.
    ///
    /// # Panics
    /// If the id is unknown or the output cannot be deserialized (a shell
    /// bug — mirrors counter-routing).
    pub fn resolve_serialized(&self, effect_id: u32, data: &[u8]) {
        self.runtime
            .router
            .routes
            .serialized
            .resolve(EffectId(effect_id), data)
            .expect("failed to resolve effect");
    }

    /// Get the current view model, bincode-serialized (`shared::ViewModel`).
    ///
    /// # Panics
    /// If the view model cannot be serialized.
    #[must_use]
    pub fn view(&self) -> Vec<u8> {
        self.runtime
            .router
            .routes
            .serialized
            .view()
            .expect("view model should serialize")
    }
}
