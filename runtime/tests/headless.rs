use std::sync::{Arc, Mutex};

use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use runtime::{AppRuntime, ShellCallback};
use shared::Event;
use shared::app::EffectFfi;

#[derive(Default)]
struct RecordingShell {
    batches: Mutex<Vec<Vec<u8>>>,
}

impl ShellCallback for RecordingShell {
    fn process_effects(&self, effects_bincode: Vec<u8>) {
        self.batches.lock().unwrap().push(effects_bincode);
    }
}

/// Storage effects are handled inside Rust; the shell only ever sees
/// serialized non-storage effects (Render). The view reflects the DB.
#[test]
fn create_task_flows_through_storage_and_renders() {
    let shell = Arc::new(RecordingShell::default());
    let rt = AppRuntime::new(None, shell.clone()).unwrap();

    rt.send_event(Event::Startup);
    rt.send_event(Event::CreateTask {
        title: "Walk the skeleton".into(),
    });

    // Storage handler runs on its own thread; poll until the follow-up
    // event lands (bounded, deterministic-enough for a skeleton test).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let view = rt.view();
        if view.count == 1 {
            assert_eq!(view.tasks[0].title, "Walk the skeleton");
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "view never updated: {view:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

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
