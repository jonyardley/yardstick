//! Shared helpers for runtime integration tests. Each tests/*.rs binary
//! that needs them declares `mod common;` (cargo compiles tests/common/
//! as a module, not a test binary).
#![allow(dead_code)] // not every test binary uses every helper

use std::sync::Mutex;

/// Records every effect batch pushed to the shell.
#[derive(Default)]
pub struct RecordingShell {
    pub batches: Mutex<Vec<Vec<u8>>>,
}

impl runtime::ShellCallback for RecordingShell {
    fn process_effects(&self, effects_bincode: Vec<u8>) {
        self.batches.lock().unwrap().push(effects_bincode);
    }
}

impl runtime::ffi::CruxShell for RecordingShell {
    fn process_effects(&self, bytes: Vec<u8>) {
        self.batches.lock().unwrap().push(bytes);
    }
}

/// Discards effects.
pub struct NullShell;

impl runtime::ShellCallback for NullShell {
    fn process_effects(&self, _: Vec<u8>) {}
}

impl runtime::ffi::CruxShell for NullShell {
    fn process_effects(&self, _: Vec<u8>) {}
}

/// Poll `check` every 10 ms until it returns true, panicking with `what`
/// after `secs` seconds. Blocking by design: callers in async tests use it
/// only after all client I/O has completed (the runtime's storage and MCP
/// work happen on their own OS threads, so blocking here starves nothing).
pub fn poll_until(secs: u64, what: &str, mut check: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    while !check() {
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for {what}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
