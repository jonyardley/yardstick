use std::sync::{Arc, Mutex};

use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use runtime::ffi::{CoreFFI, CruxShell};
use shared::app::EffectFfi;
use shared::{Event, ViewModel};

#[derive(Default)]
struct RecordingShell {
    batches: Mutex<Vec<Vec<u8>>>,
}

impl CruxShell for RecordingShell {
    fn process_effects(&self, bytes: Vec<u8>) {
        self.batches.lock().unwrap().push(bytes);
    }
}

/// The byte-level round-trip the Swift shell will do: bincode-encode an
/// Event, push it through `CoreFFI::update`, receive a decodable Render
/// batch via the `CruxShell` callback, and read the task back out of a
/// bincode-decoded `ViewModel`.
#[test]
fn ffi_round_trip_create_task_renders_and_updates_view() {
    let shell = Arc::new(RecordingShell::default());
    // Empty db_path = in-memory database.
    let core = CoreFFI::new(String::new(), shell.clone());

    let mut event_bytes = Vec::new();
    BincodeFfiFormat::serialize(
        &mut event_bytes,
        &Event::CreateTask {
            title: "Ship the FFI".into(),
        },
    )
    .expect("event should encode");
    core.update(&event_bytes);

    // Storage handler runs on its own thread; poll until the follow-up
    // event lands (bounded, same idiom as tests/headless.rs).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let view_bytes = core.view();
        let view: ViewModel =
            BincodeFfiFormat::deserialize(&view_bytes).expect("view model should decode");
        if view.count == 1 {
            assert_eq!(view.tasks[0].title, "Ship the FFI");
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "view never updated: {view:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Every batch the shell callback received decodes as a Render batch —
    // exactly what Swift will deserialize.
    let batches = shell.batches.lock().unwrap();
    assert!(!batches.is_empty(), "shell never received an effect batch");
    for batch in batches.iter() {
        let requests: Vec<Request<EffectFfi>> =
            BincodeFfiFormat::deserialize(batch).expect("shell batch should decode");
        assert!(!requests.is_empty());
        for request in &requests {
            assert!(
                matches!(request.effect, EffectFfi::Render(_)),
                "non-Render effect reached the shell"
            );
        }
    }
}

struct NullShell;

impl CruxShell for NullShell {
    fn process_effects(&self, _bytes: Vec<u8>) {}
}

/// Phase 0 sends only Render to the shell, which needs no resolution, so a
/// resolve call is always shell misuse today. It is wired to the real
/// serialized lane and — mirroring counter-routing's ffi.rs — panics on a
/// resolve error rather than corrupting state silently.
#[test]
#[should_panic(expected = "failed to resolve effect")]
fn resolving_unknown_effect_id_panics() {
    let core = CoreFFI::new(String::new(), Arc::new(NullShell));
    core.resolve_serialized(999, &[]);
}

/// `start_mcp` bridges a `Result` to a bare `u16` for the Swift side (0 =
/// failure — see the doc comment on `CoreFFI::start_mcp`). Occupy a port
/// with a plain TCP listener first so the embedded server's bind fails
/// deterministically, then assert the FFI call returns 0 and — bounded by
/// the runtime's 5s `recv_timeout` — does not hang.
#[test]
fn start_mcp_on_a_busy_port_returns_zero_and_does_not_hang() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("should bind free port");
    let port = listener
        .local_addr()
        .expect("should have local addr")
        .port();

    let core = CoreFFI::new(String::new(), Arc::new(NullShell));

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let result = core.start_mcp(port, "sekrit".into());
    assert!(
        std::time::Instant::now() < deadline,
        "start_mcp did not return within the bounded window"
    );
    assert_eq!(result, 0, "start_mcp should return 0 on a bind failure");

    drop(listener);
}
