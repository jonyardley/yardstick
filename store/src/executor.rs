use rusqlite::Connection;
use shared::{StorageOperation, StorageResult, Task};

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
    }
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
}
