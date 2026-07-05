mod common;

use std::sync::Arc;

use common::{NullShell, poll_until};
use runtime::AppRuntime;
use shared::{Event, StorageOperation, StorageResult};

const TODAY: &str = "2026-07-04";
const YESTERDAY: &str = "2026-07-03";

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("daily-notes-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Read a day's text straight off the database file (read-only conn),
/// bypassing the core — the persistence oracle.
fn db_day_text(db: &std::path::Path, date: &str) -> Option<String> {
    let conn = store::open_read_only(db).ok()?;
    match store::execute(&conn, &StorageOperation::GetDay { date: date.into() }) {
        StorageResult::Day(day) => Some(
            day.blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        _ => None,
    }
}

#[test]
fn edited_day_text_survives_a_runtime_restart() {
    let dir = temp_dir("restart");
    let db = dir.join("daily.db");
    {
        let rt = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
        rt.send_event(Event::Startup {
            today: TODAY.into(),
        });
        rt.send_event(Event::EditDay {
            date: TODAY.into(),
            text: "persisted line\n\nsecond".into(),
        });
        poll_until(5, "day text to reach the database", || {
            db_day_text(&db, TODAY).as_deref() == Some("persisted line\n\nsecond")
        });
    } // runtime dropped — same-process stand-in for an app quit; the write was
    // already confirmed committed (WAL) via the read-only poll above, so this
    // proves durability across a fresh runtime, not OS process teardown.
    // True cross-process quit/relaunch is covered by Task 8's manual checklist.

    // "Relaunch": a fresh runtime over the same file shows the text.
    let rt2 = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
    rt2.send_event(Event::Startup {
        today: TODAY.into(),
    });
    poll_until(5, "restarted view to show the persisted text", || {
        rt2.view().day.note_text == "persisted line\n\nsecond"
    });
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn navigating_between_days_round_trips_each_days_text() {
    let dir = temp_dir("navigate");
    let db = dir.join("daily.db");
    let rt = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup {
        today: TODAY.into(),
    });

    // Ordering assumption encoded here: EditDay for the departing day is sent
    // BEFORE NavigateToDay, mirroring Core.swift's flushPendingEdit()-before-
    // navigate contract (apple/Daily/Core.swift). If the shell ever stops
    // flushing before navigation, this proof documents what breaks.
    rt.send_event(Event::EditDay {
        date: TODAY.into(),
        text: "alpha".into(),
    });
    rt.send_event(Event::NavigateToDay {
        date: YESTERDAY.into(),
    });
    rt.send_event(Event::EditDay {
        date: YESTERDAY.into(),
        text: "beta".into(),
    });
    poll_until(5, "both days to reach the database", || {
        db_day_text(&db, TODAY).as_deref() == Some("alpha")
            && db_day_text(&db, YESTERDAY).as_deref() == Some("beta")
    });

    // Back to today: the view reloads today's text, not yesterday's.
    rt.send_event(Event::NavigateToDay { date: TODAY.into() });
    poll_until(5, "view to show today's text after navigating back", || {
        let day = rt.view().day;
        day.date == TODAY && day.note_text == "alpha"
    });
    // Display strings flow from the core's civil module end to end.
    assert_eq!(rt.view().day.title, "Saturday, July 4");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn editor_version_bumps_on_loads_but_not_on_own_edit_echo() {
    let dir = temp_dir("version");
    let db = dir.join("daily.db");
    let rt = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup {
        today: TODAY.into(),
    });
    poll_until(5, "startup day load", || rt.view().day.date == TODAY);
    let after_load = rt.view().day.editor_version;

    rt.send_event(Event::EditDay {
        date: TODAY.into(),
        text: "typed by hand".into(),
    });
    poll_until(5, "edit to reach the database", || {
        db_day_text(&db, TODAY).as_deref() == Some("typed by hand")
    });
    assert_eq!(
        rt.view().day.editor_version,
        after_load,
        "the save round-trip of the user's own edit must not bump the version"
    );

    rt.send_event(Event::NavigateToDay {
        date: YESTERDAY.into(),
    });
    poll_until(5, "navigation to bump the editor version", || {
        rt.view().day.editor_version > after_load
    });
    std::fs::remove_dir_all(&dir).ok();
}
