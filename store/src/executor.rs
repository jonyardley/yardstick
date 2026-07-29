use rusqlite::{Connection, Transaction, TransactionBehavior};
use shared::{BlockData, Bucket, DayData, Status, StorageOperation, StorageResult, TaskData};

use crate::db::DEFAULT_SPACE_ID;

pub fn execute(conn: &Connection, op: &StorageOperation) -> StorageResult {
    match run(conn, op) {
        Ok(result) => result,
        Err(e) => StorageResult::Error(e.to_string()),
    }
}

fn run(conn: &Connection, op: &StorageOperation) -> rusqlite::Result<StorageResult> {
    match op {
        StorageOperation::CreateTask { title, source } => create_task(conn, title, source),
        StorageOperation::SaveTask { task } => save_task(conn, task),
        StorageOperation::QueryTasks => query_tasks(conn),
        StorageOperation::GetDay { date } => get_day(conn, date),
        StorageOperation::ReplaceDayBlocks { date, paragraphs } => {
            replace_day_blocks(conn, date, paragraphs)
        }
    }
}

/// Absent-value mapping, one place: "" / 0 across the FFI boundary becomes
/// NULL in the database, and back (plan Task 1 interfaces).
fn none_if_empty(s: &str) -> Option<&str> {
    (!s.is_empty()).then_some(s)
}

fn bucket_sql(b: Bucket) -> &'static str {
    match b {
        Bucket::Inbox => "inbox",
        Bucket::Now => "now",
        Bucket::Next => "next",
        Bucket::Later => "later",
    }
}

fn bucket_from_sql(s: &str) -> Bucket {
    match s {
        "now" => Bucket::Now,
        "next" => Bucket::Next,
        "later" => Bucket::Later,
        _ => Bucket::Inbox,
    }
}

fn status_sql(s: Status) -> &'static str {
    match s {
        Status::Backlog => "backlog",
        Status::InProgress => "in_progress",
        Status::Blocked => "blocked",
        Status::Waiting => "waiting",
        Status::Done => "done",
        Status::Binned => "binned",
    }
}

fn status_from_sql(s: &str) -> Status {
    match s {
        "in_progress" => Status::InProgress,
        "blocked" => Status::Blocked,
        "waiting" => Status::Waiting,
        "done" => Status::Done,
        "binned" => Status::Binned,
        _ => Status::Backlog,
    }
}

const TASK_COLUMNS: &str = "id, title, bucket, status, priority, due, prev_status,
     blocked_reason, source, entered_now_on, done_on, created_at";

fn task_from_row(row: &rusqlite::Row) -> rusqlite::Result<TaskData> {
    let bucket: String = row.get(2)?;
    let status: String = row.get(3)?;
    let prev: Option<String> = row.get(6)?;
    Ok(TaskData {
        id: row.get(0)?,
        title: row.get(1)?,
        bucket: bucket_from_sql(&bucket),
        status: status_from_sql(&status),
        priority: row.get::<_, Option<i64>>(4)?.unwrap_or(0) as u8,
        due: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
        prev_status: prev.as_deref().map(status_from_sql),
        blocked_reason: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
        source: row.get(8)?,
        entered_now_on: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
        done_on: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
        created_at: row.get(11)?,
    })
}

fn create_task(conn: &Connection, title: &str, source: &str) -> rusqlite::Result<StorageResult> {
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO tasks (id, space_id, title, bucket, status, source, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'inbox', 'backlog', ?4, unixepoch(), unixepoch())",
        (&id, DEFAULT_SPACE_ID, title, source),
    )?;
    let task = conn.query_row(
        &format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ?1"),
        [&id],
        task_from_row,
    )?;
    Ok(StorageResult::Task(task))
}

fn save_task(conn: &Connection, task: &TaskData) -> rusqlite::Result<StorageResult> {
    let changed = conn.execute(
        "UPDATE tasks SET
           title = ?2, bucket = ?3, status = ?4, priority = ?5, due = ?6,
           prev_status = ?7, blocked_reason = ?8, entered_now_on = ?9,
           done_on = ?10, updated_at = unixepoch()
         WHERE id = ?1 AND deleted_at IS NULL",
        rusqlite::params![
            &task.id,
            &task.title,
            bucket_sql(task.bucket),
            status_sql(task.status),
            (task.priority > 0).then_some(task.priority),
            none_if_empty(&task.due),
            task.prev_status.map(status_sql),
            none_if_empty(&task.blocked_reason),
            none_if_empty(&task.entered_now_on),
            none_if_empty(&task.done_on),
        ],
    )?;
    if changed == 0 {
        return Ok(StorageResult::Error(format!(
            "no task with id {} to save",
            task.id
        )));
    }
    Ok(StorageResult::TaskSaved {
        id: task.id.clone(),
    })
}

fn query_tasks(conn: &Connection) -> rusqlite::Result<StorageResult> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLUMNS} FROM tasks
         WHERE deleted_at IS NULL AND space_id = ?1
         ORDER BY id" // UUIDv7 is time-sortable → oldest first
    ))?;
    let tasks = stmt
        .query_map([DEFAULT_SPACE_ID], task_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(StorageResult::Tasks(tasks))
}

fn get_day(conn: &Connection, date: &str) -> rusqlite::Result<StorageResult> {
    let mut stmt = conn.prepare(
        "SELECT b.id, b.kind, b.plain_text
         FROM blocks b
         JOIN notes n ON n.id = b.note_id
         WHERE n.space_id = ?1 AND n.date = ?2
           AND n.deleted_at IS NULL AND b.deleted_at IS NULL
         ORDER BY b.order_key",
    )?;
    let blocks = stmt
        .query_map((DEFAULT_SPACE_ID, date), |row| {
            Ok(BlockData {
                id: row.get(0)?,
                kind: row.get(1)?,
                text: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(StorageResult::Day(DayData {
        date: date.to_owned(),
        blocks,
    }))
}

fn replace_day_blocks(
    conn: &Connection,
    date: &str,
    paragraphs: &[String],
) -> rusqlite::Result<StorageResult> {
    // Spec §3: BEGIN IMMEDIATE from day one. A DEFERRED transaction starts as
    // a read and only upgrades to a write on the first write statement; that
    // upgrade can fail with SQLITE_BUSY immediately, ignoring busy_timeout
    // (the timeout only governs waiting to *acquire* a lock, not the
    // read→write upgrade race). IMMEDIATE takes the write lock at BEGIN, so
    // busy_timeout applies from the start.
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;

    // Note: get-or-create matches only non-deleted notes, but `notes` has an
    // unconditional UNIQUE(space_id, date). The first time a date's note is
    // ever soft-deleted and then re-edited, this SELECT will miss the
    // deleted row and the INSERT below will collide with it. Not reachable
    // in Phase 1 (nothing soft-deletes notes yet). Phase 3's note-deletion
    // design must pick one of: (a) a partial unique index
    // `UNIQUE(space_id, date) WHERE deleted_at IS NULL`, or (b) resurrect
    // (un-delete) the existing row on edit instead of inserting a new one.
    let note_id: String = match tx.query_row(
        "SELECT id FROM notes WHERE space_id = ?1 AND date = ?2 AND deleted_at IS NULL",
        (DEFAULT_SPACE_ID, date),
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let id = uuid::Uuid::now_v7().to_string();
            tx.execute(
                "INSERT INTO notes (id, space_id, date, created_at, updated_at)
                 VALUES (?1, ?2, ?3, unixepoch(), unixepoch())",
                (&id, DEFAULT_SPACE_ID, date),
            )?;
            id
        }
        Err(e) => return Err(e),
    };

    // Decision #3: blocks are wholly derived from the day's text in Phase 1
    // — superseded rows (and their FTS entries) are hard-deleted inside
    // this transaction; the note row is the soft-delete unit.
    tx.execute(
        "DELETE FROM search WHERE entity_type = 'block'
           AND entity_id IN (SELECT id FROM blocks WHERE note_id = ?1)",
        [&note_id],
    )?;
    tx.execute("DELETE FROM blocks WHERE note_id = ?1", [&note_id])?;

    for (i, text) in paragraphs.iter().enumerate() {
        let block_id = uuid::Uuid::now_v7().to_string();
        // Positional order keys suffice while every save rewrites the whole
        // day; fractional keys arrive with block-level edits (Phase 3).
        let order_key = format!("{i:08}");
        let content = serde_json::json!({ "text": text }).to_string();
        tx.execute(
            "INSERT INTO blocks
               (id, space_id, note_id, order_key, kind, content, plain_text,
                created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'paragraph', ?5, ?6, unixepoch(), unixepoch())",
            (
                &block_id,
                DEFAULT_SPACE_ID,
                &note_id,
                &order_key,
                &content,
                text,
            ),
        )?;
        if !text.trim().is_empty() {
            tx.execute(
                "INSERT INTO search (entity_type, entity_id, title, body)
                 VALUES ('block', ?1, '', ?2)",
                (&block_id, text),
            )?;
        }
    }

    tx.execute(
        "UPDATE notes SET updated_at = unixepoch() WHERE id = ?1",
        [&note_id],
    )?;
    tx.commit()?;
    Ok(StorageResult::DaySaved {
        date: date.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use shared::{Bucket, Status, StorageOperation, StorageResult, TaskData};

    fn sample(id: &str, title: &str) -> TaskData {
        TaskData {
            id: id.into(),
            title: title.into(),
            bucket: Bucket::Inbox,
            status: Status::Backlog,
            priority: 0,
            due: String::new(),
            prev_status: None,
            blocked_reason: String::new(),
            source: "quick_add".into(),
            entered_now_on: String::new(),
            done_on: String::new(),
            created_at: 0,
        }
    }

    #[test]
    fn create_task_returns_a_row_with_generated_id_and_inbox_defaults() {
        let conn = open_in_memory().unwrap();
        let result = execute(
            &conn,
            &StorageOperation::CreateTask {
                title: "Book dentist".into(),
                source: "quick_add".into(),
            },
        );
        let StorageResult::Task(task) = result else {
            panic!("expected Task, got {result:?}")
        };
        assert!(!task.id.is_empty(), "the store owns id generation");
        assert_eq!(task.title, "Book dentist");
        assert_eq!(task.bucket, Bucket::Inbox, "capture requires no decisions");
        assert_eq!(task.status, Status::Backlog);
        assert_eq!(task.priority, 0);
        assert_eq!(task.source, "quick_add");
        assert!(task.created_at > 0, "created_at is the store's clock");
    }

    #[test]
    fn save_task_round_trips_every_mutable_field() {
        let conn = open_in_memory().unwrap();
        let StorageResult::Task(created) = execute(
            &conn,
            &StorageOperation::CreateTask {
                title: "Finalize vendor contract".into(),
                source: "quick_add".into(),
            },
        ) else {
            panic!("create failed")
        };

        let saved = TaskData {
            title: "Finalise vendor contract".into(),
            bucket: Bucket::Next,
            status: Status::Blocked,
            priority: 1,
            due: "2026-07-31".into(),
            prev_status: Some(Status::InProgress),
            blocked_reason: "Legal review".into(),
            entered_now_on: "2026-07-29".into(),
            ..created.clone()
        };
        let result = execute(
            &conn,
            &StorageOperation::SaveTask {
                task: saved.clone(),
            },
        );
        assert_eq!(
            result,
            StorageResult::TaskSaved {
                id: created.id.clone()
            }
        );

        let StorageResult::Tasks(tasks) = execute(&conn, &StorageOperation::QueryTasks) else {
            panic!("query failed")
        };
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Finalise vendor contract");
        assert_eq!(tasks[0].bucket, Bucket::Next);
        assert_eq!(tasks[0].status, Status::Blocked);
        assert_eq!(tasks[0].priority, 1);
        assert_eq!(tasks[0].due, "2026-07-31");
        assert_eq!(tasks[0].prev_status, Some(Status::InProgress));
        assert_eq!(tasks[0].blocked_reason, "Legal review");
        assert_eq!(tasks[0].entered_now_on, "2026-07-29");
        assert_eq!(
            tasks[0].created_at, created.created_at,
            "a save never rewrites created_at"
        );
    }

    #[test]
    fn save_task_clears_fields_back_to_absent() {
        let conn = open_in_memory().unwrap();
        let StorageResult::Task(created) = execute(
            &conn,
            &StorageOperation::CreateTask {
                title: "Research competitors".into(),
                source: "menu_bar".into(),
            },
        ) else {
            panic!("create failed")
        };
        // Set, then clear: unticking a checkbox and removing a due date must
        // write NULL, not the string "".
        execute(
            &conn,
            &StorageOperation::SaveTask {
                task: TaskData {
                    priority: 2,
                    due: "2026-08-01".into(),
                    done_on: "2026-07-29".into(),
                    ..created.clone()
                },
            },
        );
        execute(
            &conn,
            &StorageOperation::SaveTask {
                task: created.clone(),
            },
        );

        let StorageResult::Tasks(tasks) = execute(&conn, &StorageOperation::QueryTasks) else {
            panic!("query failed")
        };
        assert_eq!(tasks[0].priority, 0);
        assert_eq!(tasks[0].due, "");
        assert_eq!(tasks[0].done_on, "");
        let nulls: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks
                 WHERE priority IS NULL AND due IS NULL AND done_on IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(nulls, 1, "absent must be NULL in the database, never ''");
    }

    #[test]
    fn query_tasks_is_oldest_first_and_ignores_soft_deleted() {
        let conn = open_in_memory().unwrap();
        for title in ["first", "second", "third"] {
            execute(
                &conn,
                &StorageOperation::CreateTask {
                    title: title.into(),
                    source: "quick_add".into(),
                },
            );
        }
        conn.execute(
            "UPDATE tasks SET deleted_at = unixepoch() WHERE title = 'second'",
            [],
        )
        .unwrap();

        let StorageResult::Tasks(tasks) = execute(&conn, &StorageOperation::QueryTasks) else {
            panic!("query failed")
        };
        let titles: Vec<&str> = tasks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, vec!["first", "third"]);
    }

    #[test]
    fn save_task_for_an_unknown_id_is_an_error_not_a_silent_insert() {
        let conn = open_in_memory().unwrap();
        let result = execute(
            &conn,
            &StorageOperation::SaveTask {
                task: sample("no-such-id", "ghost"),
            },
        );
        let StorageResult::Error(message) = result else {
            panic!("expected Error, got {result:?}")
        };
        assert!(
            message.contains("no-such-id"),
            "the message must name the id: {message}"
        );
    }

    fn day_text(conn: &Connection, date: &str) -> String {
        let StorageResult::Day(day) =
            execute(conn, &StorageOperation::GetDay { date: date.into() })
        else {
            panic!("expected Day");
        };
        day.blocks
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn get_day_with_no_note_returns_empty_day_and_creates_nothing() {
        let conn = open_in_memory().unwrap();
        let StorageResult::Day(day) = execute(
            &conn,
            &StorageOperation::GetDay {
                date: "2026-07-04".into(),
            },
        ) else {
            panic!("expected Day");
        };
        assert_eq!(day.date, "2026-07-04");
        assert!(day.blocks.is_empty());
        let notes: i64 = conn
            .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            notes, 0,
            "GetDay must not create a note (lazy creation is on edit)"
        );
    }

    #[test]
    fn replace_then_get_round_trips_paragraphs_in_order_including_empty_lines() {
        let conn = open_in_memory().unwrap();
        let saved = execute(
            &conn,
            &StorageOperation::ReplaceDayBlocks {
                date: "2026-07-04".into(),
                paragraphs: vec!["Release Meeting".into(), "".into(), "Copy changes?".into()],
            },
        );
        assert_eq!(
            saved,
            StorageResult::DaySaved {
                date: "2026-07-04".into()
            }
        );
        assert_eq!(
            day_text(&conn, "2026-07-04"),
            "Release Meeting\n\nCopy changes?"
        );
    }

    #[test]
    fn replacing_again_supersedes_blocks_one_note_row_no_duplicates() {
        let conn = open_in_memory().unwrap();
        for text in ["first draft", "second draft"] {
            execute(
                &conn,
                &StorageOperation::ReplaceDayBlocks {
                    date: "2026-07-04".into(),
                    paragraphs: vec![text.into()],
                },
            );
        }
        assert_eq!(day_text(&conn, "2026-07-04"), "second draft");
        let notes: i64 = conn
            .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(notes, 1);
        let blocks: i64 = conn
            .query_row("SELECT COUNT(*) FROM blocks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            blocks, 1,
            "superseded blocks are hard-deleted (decision #3)"
        );
    }

    #[test]
    fn days_are_isolated_from_each_other() {
        let conn = open_in_memory().unwrap();
        execute(
            &conn,
            &StorageOperation::ReplaceDayBlocks {
                date: "2026-07-03".into(),
                paragraphs: vec!["yesterday".into()],
            },
        );
        execute(
            &conn,
            &StorageOperation::ReplaceDayBlocks {
                date: "2026-07-04".into(),
                paragraphs: vec!["today".into()],
            },
        );
        assert_eq!(day_text(&conn, "2026-07-03"), "yesterday");
        assert_eq!(day_text(&conn, "2026-07-04"), "today");
    }

    #[test]
    fn fts_round_trip_blocks_are_searchable_and_index_follows_rewrites() {
        let conn = open_in_memory().unwrap();
        execute(
            &conn,
            &StorageOperation::ReplaceDayBlocks {
                date: "2026-07-04".into(),
                paragraphs: vec!["Buy oat milk".into(), "".into()],
            },
        );

        let hits = |q: &str| -> i64 {
            conn.query_row(
                "SELECT COUNT(*) FROM search WHERE entity_type = 'block' AND search MATCH ?1",
                [q],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(hits("milk"), 1, "saved block text must be FTS-searchable");

        // Empty paragraphs must not pollute the index.
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM search", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 1);

        // Rewrite without the word: the index follows in the same transaction.
        execute(
            &conn,
            &StorageOperation::ReplaceDayBlocks {
                date: "2026-07-04".into(),
                paragraphs: vec!["Buy nothing".into()],
            },
        );
        assert_eq!(
            hits("milk"),
            0,
            "stale FTS rows must be gone after a rewrite"
        );
        assert_eq!(hits("nothing"), 1);
    }
}
