use std::sync::Arc;

use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use runtime::ffi::CoreFFI;
use shared::app::EffectFfi;
use shared::{Event, ViewModel};

mod common;
use common::{NullShell, RecordingShell};

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
    common::poll_until(5, "view to show the created task", || {
        let view_bytes = core.view();
        let view: ViewModel =
            BincodeFfiFormat::deserialize(&view_bytes).expect("view model should decode");
        if view.count == 1 {
            assert_eq!(view.tasks[0].title, "Ship the FFI");
            true
        } else {
            false
        }
    });

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

/// A database that cannot be opened must not panic the constructor (macOS
/// would relaunch-loop the app): the core reports the failure via
/// `init_error()` and every other entry point is inert.
#[test]
fn core_ffi_with_unopenable_db_reports_init_error_and_is_inert() {
    let dir = std::env::temp_dir().join(format!("daily-ffi-baddb-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("corrupt.db");
    std::fs::write(&path, b"this is not a sqlite database").unwrap();

    let core = CoreFFI::new(path.to_string_lossy().into_owned(), Arc::new(NullShell));
    assert!(
        !core.init_error().is_empty(),
        "expected a non-empty init error for a corrupt db"
    );

    // Inert, not panicking (on a healthy core, update(&[]) would panic on
    // decode — here it must return before decoding anything):
    core.update(&[]);
    core.resolve_serialized(0, &[]);
    assert!(core.view().is_empty());
    assert_eq!(core.start_mcp(0, "t".into()), 0);

    std::fs::remove_dir_all(&dir).ok();
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
