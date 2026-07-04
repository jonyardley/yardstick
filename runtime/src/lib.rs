pub mod ffi;
mod router;
mod storage_handler;

pub use router::AppRuntime;

/// Implemented by the shell (Swift via BoltFFI in Task 5; a recording stub in
/// tests). Receives serialized batches of non-storage effect requests.
pub trait ShellCallback: Send + Sync {
    fn process_effects(&self, effects_bincode: Vec<u8>);
}
