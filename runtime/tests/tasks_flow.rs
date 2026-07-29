//! Phase 2's end-to-end contract: a task's whole life through the real
//! runtime — core events in, SQLite out, the ViewModel reflecting it — with
//! no Swift and no mocks. These are the tests that would catch a wiring
//! break between the core, the router and the store.

mod common;

use std::sync::Arc;

use common::{NullShell, poll_until};
use runtime::AppRuntime;
use shared::{Bucket, Event, Status};

const TODAY: &str = "2026-07-04";

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ys-tasks-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn started(name: &str) -> (Arc<AppRuntime>, std::path::PathBuf) {
    let dir = temp_dir(name);
    let db = dir.join("daily.db");
    let rt = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup {
        today: TODAY.into(),
    });
    (rt, dir)
}

fn row_ids(rt: &AppRuntime) -> Vec<String> {
    rt.view()
        .list
        .groups
        .iter()
        .flat_map(|g| g.rows.iter().map(|r| r.id.clone()))
        .collect()
}

#[test]
fn capture_lands_in_the_inbox_with_its_source_and_bumps_the_count() {
    let (rt, dir) = started("capture");
    rt.send_event(Event::CaptureTask {
        title: "Finalize vendor contract".into(),
        source: "quick_add".into(),
    });

    poll_until(5, "the captured task to reach the Inbox count", || {
        rt.view()
            .sidebar
            .views
            .iter()
            .any(|v| v.kind == "inbox" && v.count == 1)
    });

    rt.send_event(Event::SelectView {
        kind: "inbox".into(),
    });
    poll_until(5, "the Inbox surface", || rt.view().list.title == "Inbox");
    let list = rt.view().list;
    assert_eq!(list.subtitle, "Captured today · unsorted");
    assert_eq!(list.groups[0].rows[0].title, "Finalize vendor contract");
    assert_eq!(list.groups[0].rows[0].meta, "quick add");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn triage_moves_a_task_out_of_the_inbox_and_into_its_bucket() {
    let (rt, dir) = started("triage");
    rt.send_event(Event::CaptureTask {
        title: "Book dentist".into(),
        source: "quick_add".into(),
    });
    rt.send_event(Event::SelectView {
        kind: "inbox".into(),
    });
    poll_until(5, "the captured row", || row_ids(&rt).len() == 1);
    let id = row_ids(&rt).remove(0);

    rt.send_event(Event::TriageTask {
        id: id.clone(),
        bucket: Bucket::Next,
        priority: 1,
        due: "2026-07-31".into(),
    });
    poll_until(5, "the Inbox to empty", || row_ids(&rt).is_empty());

    rt.send_event(Event::SelectView {
        kind: "next".into(),
    });
    poll_until(5, "the Next surface", || rt.view().list.title == "Next");
    let row = rt.view().list.groups[0].rows[0].clone();
    assert_eq!(row.priority, 1);
    assert_eq!(row.meta, "Fri", "the due weekday, per Journey 1C");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn done_and_undone_round_trip_through_the_database_restoring_the_old_status() {
    let (rt, dir) = started("done");
    rt.send_event(Event::CaptureTask {
        title: "Review PR".into(),
        source: "quick_add".into(),
    });
    rt.send_event(Event::SelectView {
        kind: "inbox".into(),
    });
    poll_until(5, "the captured row", || row_ids(&rt).len() == 1);
    let id = row_ids(&rt).remove(0);

    rt.send_event(Event::SetStatus {
        id: id.clone(),
        status: Status::Blocked,
        reason: "Legal review".into(),
    });
    poll_until(5, "the blocked pill", || {
        rt.view().list.groups[0]
            .rows
            .first()
            .is_some_and(|r| r.status_pill == "Blocked" && r.blocked_reason == "Legal review")
    });

    rt.send_event(Event::ToggleDone { id: id.clone() });
    poll_until(5, "the task to leave the Inbox as done", || {
        row_ids(&rt).is_empty()
    });

    rt.send_event(Event::ToggleDone { id: id.clone() });
    poll_until(5, "the task to come back", || row_ids(&rt).len() == 1);
    let row = rt.view().list.groups[0].rows[0].clone();
    assert_eq!(
        row.status_pill, "Blocked",
        "spec §7: unticking restores the previous status, end to end"
    );
    assert_eq!(row.blocked_reason, "Legal review");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_bulk_update_moves_every_selected_task_and_leaves_the_rest() {
    let (rt, dir) = started("bulk");
    for title in ["one", "two", "three"] {
        rt.send_event(Event::CaptureTask {
            title: title.into(),
            source: "quick_add".into(),
        });
    }
    rt.send_event(Event::SelectView {
        kind: "inbox".into(),
    });
    poll_until(5, "three captured rows", || row_ids(&rt).len() == 3);
    let ids = row_ids(&rt);

    rt.send_event(Event::BulkUpdateTasks {
        ids: vec![ids[0].clone(), ids[2].clone()],
        bucket: Some(Bucket::Later),
        priority: Some(3),
        status: None,
    });
    poll_until(5, "one task left in the Inbox", || row_ids(&rt).len() == 1);
    assert_eq!(row_ids(&rt)[0], ids[1], "the unselected task stayed put");

    rt.send_event(Event::SelectView {
        kind: "later".into(),
    });
    poll_until(5, "two tasks in Later", || row_ids(&rt).len() == 2);
    assert!(
        rt.view().list.groups[0]
            .rows
            .iter()
            .all(|r| r.priority == 3)
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn everything_survives_a_restart_on_the_same_database() {
    let dir = temp_dir("restart");
    let db = dir.join("daily.db");
    let id = {
        let rt = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
        rt.send_event(Event::Startup {
            today: TODAY.into(),
        });
        rt.send_event(Event::CaptureTask {
            title: "Persisted task".into(),
            source: "mcp".into(),
        });
        rt.send_event(Event::SelectView {
            kind: "inbox".into(),
        });
        poll_until(5, "the captured row", || row_ids(&rt).len() == 1);
        let id = row_ids(&rt).remove(0);
        rt.send_event(Event::TriageTask {
            id: id.clone(),
            bucket: Bucket::Now,
            priority: 2,
            due: String::new(),
        });
        poll_until(5, "the Inbox to empty", || row_ids(&rt).is_empty());
        id
    };

    // Fresh runtime, same file — as after a relaunch.
    let rt = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup {
        today: TODAY.into(),
    });
    poll_until(5, "the Now list to reload", || {
        rt.view().list.title == "Now" && !rt.view().list.groups[0].rows.is_empty()
    });
    let row = rt.view().list.groups[0].rows[0].clone();
    assert_eq!(row.id, id);
    assert_eq!(row.title, "Persisted task");
    assert_eq!(row.priority, 2);
    let momentum = rt.view().list.momentum.expect("the Now list has momentum");
    assert_eq!(momentum.label, "0 done · 1 to go");

    std::fs::remove_dir_all(&dir).ok();
}
