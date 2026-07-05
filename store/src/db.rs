use std::{path::Path, sync::LazyLock, time::Duration};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

pub static MIGRATIONS: LazyLock<Migrations> = LazyLock::new(|| {
    Migrations::new(vec![
        M::up(include_str!("../migrations/001_initial.sql")),
        M::up(include_str!("../migrations/002_notes.sql")),
    ])
});

pub const DEFAULT_SPACE_ID: &str = "0197f000-0000-7000-8000-000000000001";

/// Everything that can go wrong opening the database: rusqlite errors
/// convert into it (`From<rusqlite::Error>`), so `?` works throughout.
pub use rusqlite_migration::Error as OpenError;

fn configure(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(Duration::from_millis(5000))?;
    Ok(())
}

pub fn open(path: &Path) -> Result<Connection, OpenError> {
    let mut conn = Connection::open(path)?;
    configure(&mut conn)?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(conn)
}

/// Open an existing database read-only. No migrations run here — the
/// writer connection (same process) owns the schema. WAL means this reader
/// never blocks the storage thread and always sees committed writes.
///
/// Opened with `SQLITE_OPEN_NO_MUTEX`, so this connection is NOT internally
/// synchronized. Callers must serialize access themselves — wrap it in a
/// `Mutex` (see `mcp::reader::StoreReader`, which does exactly this).
pub fn open_read_only(path: &Path) -> Result<Connection, OpenError> {
    use rusqlite::OpenFlags;
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(Duration::from_millis(5000))?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection, OpenError> {
    let mut conn = Connection::open_in_memory()?;
    // In-memory DBs don't support WAL; skip journal_mode, keep the rest.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        MIGRATIONS.validate().unwrap();
    }

    #[test]
    fn open_applies_migrations_and_seeds_spaces() {
        let conn = open_in_memory().unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM spaces", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
    }

    #[test]
    fn open_on_disk_uses_wal() {
        let dir = std::env::temp_dir().join(format!("daily-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mode: String = {
            let conn = open(&dir.join("test.db")).unwrap();
            conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))
                .unwrap()
        };
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(mode, "wal");
    }

    #[test]
    fn open_returns_err_not_panic_on_a_corrupt_db_file() {
        let dir = std::env::temp_dir().join(format!("daily-baddb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("corrupt.db");
        std::fs::write(&path, b"this is not a sqlite database").unwrap();
        let result = open(&path);
        std::fs::remove_dir_all(&dir).ok();
        assert!(result.is_err());
    }

    /// The exact scenario the Phase 0 review flagged: a database whose
    /// user_version is ahead of what this binary knows (e.g. the user ran a
    /// newer build, then downgraded). Must be Err, not a panic/crash-loop.
    #[test]
    fn open_returns_err_not_panic_on_a_future_schema_version() {
        let dir = std::env::temp_dir().join(format!("daily-futuredb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("future.db");
        {
            let conn = open(&path).unwrap(); // create a valid, migrated db
            conn.pragma_update(None, "user_version", 999).unwrap();
        }
        let result = open(&path);
        std::fs::remove_dir_all(&dir).ok();
        assert!(result.is_err(), "expected Err on future schema, got Ok");
    }

    #[test]
    fn read_only_connection_sees_committed_writes_and_rejects_writes() {
        let dir = std::env::temp_dir().join(format!("daily-ro-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ro.db");

        let writer = open(&path).unwrap();
        crate::executor::execute(
            &writer,
            &shared::StorageOperation::InsertTask {
                title: "seen by reader".into(),
            },
        );

        let reader = open_read_only(&path).unwrap();
        let n: i64 = reader
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);

        let write_attempt = reader.execute("DELETE FROM tasks", []);
        assert!(write_attempt.is_err(), "read-only conn must reject writes");

        drop((writer, reader));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A real 001-only database (e.g. an early adopter's on-disk file),
    /// reopened through the normal writer path, must land on 002 with its
    /// existing data intact — the exact scenario Task 1's error handling
    /// exists for, now exercised end to end for the first schema growth.
    #[test]
    fn upgrading_a_real_001_database_lands_on_002_with_data_intact() {
        let dir = std::env::temp_dir().join(format!("daily-upgrade-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("upgrade.db");

        let v1_only = Migrations::new(vec![M::up(include_str!("../migrations/001_initial.sql"))]);
        {
            let mut conn = Connection::open(&path).unwrap();
            configure(&mut conn).unwrap();
            v1_only.to_latest(&mut conn).unwrap();
            crate::executor::execute(
                &conn,
                &shared::StorageOperation::InsertTask {
                    title: "pre-upgrade task".into(),
                },
            );
            let version: i64 = conn
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
            assert_eq!(version, 1, "test setup must actually be pinned at 001");
        }

        let conn = open(&path).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 2, "reopening must apply 002");

        let tasks: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tasks, 1, "pre-upgrade data must survive the migration");

        let spaces: i64 = conn
            .query_row("SELECT COUNT(*) FROM spaces", [], |r| r.get(0))
            .unwrap();
        assert_eq!(spaces, 2, "001's seed data must survive too");

        // 002's tables must now exist and be usable.
        for table in ["notes", "blocks", "links", "search"] {
            let n: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table} must exist post-upgrade, empty");
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
