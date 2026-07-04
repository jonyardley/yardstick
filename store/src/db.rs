use std::{path::Path, sync::LazyLock, time::Duration};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

pub static MIGRATIONS: LazyLock<Migrations> =
    LazyLock::new(|| Migrations::new(vec![M::up(include_str!("../migrations/001_initial.sql"))]));

pub const DEFAULT_SPACE_ID: &str = "0197f000-0000-7000-8000-000000000001";

fn configure(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(Duration::from_millis(5000))?;
    Ok(())
}

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(path)?;
    configure(&mut conn)?;
    MIGRATIONS.to_latest(&mut conn).expect("migrations failed");
    Ok(conn)
}

pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    // In-memory DBs don't support WAL; skip journal_mode, keep the rest.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    MIGRATIONS.to_latest(&mut conn).expect("migrations failed");
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
}
