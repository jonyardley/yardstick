use rusqlite::Connection;
use shared::{BlockData, DayData, StorageOperation, StorageResult, Task};

use crate::db::DEFAULT_SPACE_ID;

pub fn execute(conn: &Connection, op: &StorageOperation) -> StorageResult {
    match run(conn, op) {
        Ok(result) => result,
        Err(e) => StorageResult::Error(e.to_string()),
    }
}

fn run(conn: &Connection, op: &StorageOperation) -> rusqlite::Result<StorageResult> {
    match op {
        StorageOperation::InsertTask { title } => {
            let id = uuid::Uuid::now_v7().to_string();
            conn.execute(
                "INSERT INTO tasks (id, space_id, title, created_at, updated_at)
                 VALUES (?1, ?2, ?3, unixepoch(), unixepoch())",
                (&id, DEFAULT_SPACE_ID, title),
            )?;
            Ok(StorageResult::Task(Task {
                id,
                title: title.clone(),
            }))
        }
        StorageOperation::ListTasks => {
            let mut stmt = conn.prepare(
                "SELECT id, title FROM tasks
                 WHERE deleted_at IS NULL AND space_id = ?1
                 ORDER BY id", // UUIDv7 is time-sortable → oldest first
            )?;
            let tasks = stmt
                .query_map([DEFAULT_SPACE_ID], |row| {
                    Ok(Task {
                        id: row.get(0)?,
                        title: row.get(1)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(StorageResult::Tasks(tasks))
        }
        StorageOperation::GetDay { date } => get_day(conn, date),
        StorageOperation::ReplaceDayBlocks { date, paragraphs } => {
            replace_day_blocks(conn, date, paragraphs)
        }
    }
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
    let tx = conn.unchecked_transaction()?;

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
    use shared::{StorageOperation, StorageResult};

    #[test]
    fn insert_then_list_round_trips() {
        let conn = open_in_memory().unwrap();

        let inserted = execute(
            &conn,
            &StorageOperation::InsertTask {
                title: "Buy milk".into(),
            },
        );
        let StorageResult::Task(task) = inserted else {
            panic!("expected Task, got {inserted:?}");
        };
        assert_eq!(task.title, "Buy milk");
        assert!(!task.id.is_empty());

        let listed = execute(&conn, &StorageOperation::ListTasks);
        let StorageResult::Tasks(tasks) = listed else {
            panic!("expected Tasks, got {listed:?}");
        };
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0], task);
    }

    #[test]
    fn list_is_oldest_first_and_ignores_soft_deleted() {
        let conn = open_in_memory().unwrap();
        execute(
            &conn,
            &StorageOperation::InsertTask {
                title: "first".into(),
            },
        );
        execute(
            &conn,
            &StorageOperation::InsertTask {
                title: "second".into(),
            },
        );
        conn.execute(
            "UPDATE tasks SET deleted_at = unixepoch() WHERE title = 'first'",
            [],
        )
        .unwrap();

        let StorageResult::Tasks(tasks) = execute(&conn, &StorageOperation::ListTasks) else {
            panic!()
        };
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "second");
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
