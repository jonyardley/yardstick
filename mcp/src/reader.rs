//! Read-only SQLite reader for MCP reads (spec §5 consistency rule: reads
//! go straight to `store`; writes dispatch core events).

use std::sync::Mutex;

use rusqlite::Connection;
use shared::{StorageOperation, StorageResult, Task};

use crate::TaskReader;

/// Wraps a read-only [`Connection`] (which is `Send` but not `Sync`) in a
/// `Mutex` so async tool handlers can share it. Contention is negligible:
/// one local agent, point queries.
pub struct StoreReader {
    conn: Mutex<Connection>,
}

impl StoreReader {
    #[must_use]
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }
}

impl TaskReader for StoreReader {
    fn list_tasks(&self) -> Result<Vec<Task>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        match store::execute(&conn, &StorageOperation::ListTasks) {
            StorageResult::Tasks(tasks) => Ok(tasks),
            StorageResult::Error(e) => Err(e),
            other => Err(format!(
                "unexpected storage result for ListTasks: {other:?}"
            )),
        }
    }
}
