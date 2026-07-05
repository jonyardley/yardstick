use std::sync::Arc;

use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use runtime::AppRuntime;
use shared::Event;
use shared::app::EffectFfi;

mod common;
use common::RecordingShell;

/// Storage effects are handled inside Rust; the shell only ever sees
/// serialized non-storage effects (Render). The view reflects the DB.
#[test]
fn create_task_flows_through_storage_and_renders() {
    let shell = Arc::new(RecordingShell::default());
    let rt = AppRuntime::new(None, shell.clone()).unwrap();

    rt.send_event(Event::Startup {
        today: "2026-07-04".into(),
    });
    rt.send_event(Event::CreateTask {
        title: "Walk the skeleton".into(),
    });

    // Storage handler runs on its own thread; poll until the follow-up
    // event lands (bounded, deterministic-enough for a skeleton test).
    common::poll_until(5, "view to show the created task", || {
        rt.view()
            .sidebar
            .views
            .iter()
            .any(|v| v.kind == "inbox" && v.count == 1)
    });

    // The shell received at least one effect batch, and every effect in
    // every batch is Render — Storage effects never reach the shell.
    let batches = shell.batches.lock().unwrap();
    assert!(!batches.is_empty());
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
