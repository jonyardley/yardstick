# Persistence Strategy for Yardstick (Crux + macOS, local-first)

**TL;DR recommendation:** Use **rusqlite (bundled) + rusqlite_migration** inside a **Rust-side effect handler** wired up with crux_core 0.19's new **`EffectRouter`** (the `counter-routing` example in the crux repo is a near-exact template for this). One background thread owns the `rusqlite::Connection`; the storage effect never crosses FFI into Swift. Model note blocks as rows with JSON content + an extracted `plain_text` column feeding a **single unified FTS5 table**. For the GUI + MCP server question: prefer a **single-owner process** (MCP server talks to the app over IPC), but if both must open the file, WAL + `busy_timeout` + `BEGIN IMMEDIATE` is a well-understood fallback. **Skip Automerge/CRDTs for now** — cheap sync-readiness comes from UUIDv7 ids, `updated_at`, and soft deletes, not from a CRDT engine.

---

## 1. SQLite from Rust in 2026: rusqlite, not sqlx

Current versions (crates.io, checked 2026-07-02): `rusqlite 0.40.1`, `sqlx 0.9.0`, `rusqlite_migration 2.6.0`, `crux_core 0.19.0`, `automerge 0.10.0`.

| | rusqlite | sqlx |
|---|---|---|
| Model | Synchronous, thin wrapper over the C API | Async-first, multi-database |
| SQLite feature access | Full: FTS5, custom scalar functions, virtual tables, hooks, `user_version`, backup API | Limited to what the driver exposes |
| Fit for a Crux effect handler | Perfect — the handler already runs on a dedicated background thread, so async buys nothing | Drags an async runtime into an otherwise synchronous, single-process store |
| Compile-time checked queries | No (write SQL, map rows) | Yes, but requires DATABASE_URL/offline metadata workflow |

For an embedded, single-process, single-writer store driven from a background thread, **rusqlite is the clear choice**. This is the consensus position in current comparisons ([Rust ORMs in 2026](https://aarambhdevhub.medium.com/rust-orms-in-2026-diesel-vs-sqlx-vs-seaorm-vs-rusqlite-which-one-should-you-actually-use-706d0fe912f3)): rusqlite for CLI/desktop apps needing the full SQLite feature set; sqlx when you need async or multi-DB portability. sqlx's SQLite backend is its weakest, and its async model solves a problem (not blocking a server's executor) that Yardstick doesn't have.

**FTS5 works out of the box:** the `bundled` feature compiles the SQLite amalgamation with `SQLITE_ENABLE_FTS5` on by default ([libsqlite3-sys build.rs](https://github.com/rusqlite/rusqlite/blob/master/libsqlite3-sys/build.rs)) — no feature flags, no extension loading, and you control the SQLite version regardless of what macOS ships.

```toml
[dependencies]
rusqlite = { version = "0.40", features = ["bundled", "serde_json"] }
rusqlite_migration = "2.6"
serde_json = "1"
```

**Migrations: `rusqlite_migration` over refinery.** It tracks state in SQLite's native `PRAGMA user_version` (an integer at a fixed offset in the file — no bookkeeping table), takes migrations as plain SQL strings/`include_str!` files, needs no CLI, and has a one-line test that validates all migrations apply cleanly ([docs](https://docs.rs/rusqlite_migration)). refinery's strengths (multi-database, CLI, `V{n}__name` file conventions) are irrelevant for one embedded SQLite file.

```rust
use rusqlite_migration::{Migrations, M};

static MIGRATIONS: LazyLock<Migrations> = LazyLock::new(|| Migrations::new(vec![
    M::up(include_str!("../migrations/001_initial.sql")),
    M::up(include_str!("../migrations/002_fts.sql")),
]));

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;      // persistent, set once
    conn.pragma_update(None, "synchronous", "NORMAL")?;    // recommended pairing with WAL
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(Duration::from_secs(5))?;
    MIGRATIONS.to_latest(&mut conn).expect("migrations failed");
    Ok(conn)
}

#[test]
fn migrations_are_valid() { MIGRATIONS.validate().unwrap(); }
```

---

## 2. Where the store lives in Crux: yes, there is first-class "Rust shell middleware"

This is the best-supported it has ever been, and the timing is good:

- **crux_core 0.16.0** (July 2025) introduced **effect middleware**: the `middleware` module's `Layer` trait with `Layer::handle_effects_using(EffectMiddleware)` lets a middleware stack intercept effects before the shell ([docs.rs middleware module](https://docs.rs/crux_core/latest/crux_core/middleware/index.html)). Still present in 0.19.
- **crux_core 0.19.0** (June 8, 2026) introduced the **`effects` module with `EffectRouter`, which replaces effect middleware** as the recommended pattern ([releases](https://github.com/redbadger/crux/releases), [effects module docs](https://docs.rs/crux_core/latest/crux_core/effects/index.html)). It gives *type-based, per-effect dispatch* without forcing every effect through the serialization bridge:
  - You implement `Routes<App>` to group handlers ("lanes").
  - Built-in lanes: **`Serialized`** (standard FFI bridge to Swift), **`Parked`** (park by `EffectId` for hard-to-serialize payloads), **`Buffer`** (drain-and-handle synchronously; tests and in-process handlers).
  - **`ResolveSink`** lets a core-local Rust handler resolve a `Request` back through the router; follow-up effects automatically re-route through the same closure.
  - Constraint: the app must be `Send + Sync`, and this path needs threads (on wasm the shipped example falls back to a plain `Bridge` — irrelevant for macOS).

**The `counter-routing` example is your template.** In [`examples/counter-routing/shared/src/ffi.rs`](https://github.com/redbadger/crux/tree/master/examples/counter-routing), a `Random` effect is handled entirely in Rust — the Swift shell never sees it — while `Render`/`Http` still go over the serialized bridge:

```rust
// Routes: one serialized lane to Swift + one in-process Rust handler
#[derive(Clone)]
struct EffectRoutes {
    serialized: Arc<Serialized<App, Self, BincodeFfiFormat>>,
    storage: Arc<StorageHandler>,          // <- was RngHandler in the example
}

impl Routes<App> for EffectRoutes {
    fn new(router: Weak<EffectRouter<App, Self>>) -> Self {
        Self {
            serialized: Arc::new(Serialized::new(router.clone())),
            storage: Arc::new(StorageHandler::new(router)),
        }
    }
}

let router = EffectRouter::new(Core::new(), move |routes: EffectRoutes| {
    let shell = shell.clone();
    move |effect| match effect {
        Effect::Storage(req) => routes.storage.process(req),   // stays in Rust
        effect => {
            let bytes = routes.serialized.serialize(FfiEffect::from(effect)).unwrap();
            shell.process_effects(bytes);                       // Render etc. -> Swift
        }
    }
});
```

And the handler (modeled on the example's `RngHandler`) is a **persistent background thread that owns the `Connection`** — which is ideal, because `rusqlite::Connection` is `Send` but not `Sync`, so a single-owner thread with an mpsc queue is exactly the right shape:

```rust
pub struct StorageHandler { jobs_tx: Sender<Request<StorageOperation>> }

impl StorageHandler {
    pub fn new<R>(sink: Weak<R>) -> Self
    where R: ResolveSink<StorageOperation> + Send + Sync + 'static {
        let (jobs_tx, jobs_rx) = channel::<Request<StorageOperation>>();
        spawn(move || {
            let conn = db::open(&db_path()).expect("open db");   // thread owns the Connection
            while let Ok(mut request) = jobs_rx.recv() {
                let output = execute(&conn, &request.operation); // run SQL, map to StorageOutput
                if let Some(sink) = sink.upgrade() {
                    sink.resolve_request(&mut request, output).expect("resolve");
                }
            }
        });
        Self { jobs_tx }
    }
    pub fn process(&self, req: Request<StorageOperation>) { self.jobs_tx.send(req).unwrap(); }
}
```

**Design consequences:**
- The core stays a pure state machine; it emits a `StorageOperation` (an `Operation` request/response pair you define: `UpsertNote`, `QueryTasks`, `Search { query }`, …) and receives typed results. Fully unit-testable with crux's `Buffer` lane / `expect_effect()` assertions — no SQLite needed in core tests.
- Swift never sees storage effects, so there's no Swift persistence code, no serialization of row data across FFI, and the MCP concern stays in Rust.
- One thread = one writer = serialized transactions by construction. No `Mutex<Connection>`, no pool.
- If you stay on crux < 0.19 for a while, the same architecture works via `Layer::handle_effects_using` + `EffectMiddleware` (`HandleEffectLayer`); `EffectRouter` is just the cleaner, non-serializing successor. Prefer 0.19+ if you can (note: 0.19 also swaps UniFFI for BoltFFI, and 0.17 removed the old Capability API — worth landing on the new stack now rather than migrating twice).

---

## 3. Schema sketch

### Rich-text note blocks: rows of JSON, plus extracted plain text

Store **one row per block** rather than one markdown blob per note: partial updates are cheap, blocks can carry structured attrs (checkboxes, embeds, task refs), backlink extraction is per-block, and reordering is an `order_key` update. Keep the block's structured content as JSON (serde-serialized from the core's block model) and *also* store extracted `plain_text` — that column, not the JSON, feeds FTS. (Storing raw JSON in FTS indexes the syntax noise; extracting text at write time in the Rust handler is trivial since the core already has the typed block.)

```sql
CREATE TABLE notes (
  id          TEXT PRIMARY KEY,          -- UUIDv7 / ULID (sync-friendly, sortable)
  date        TEXT NOT NULL UNIQUE,      -- 'YYYY-MM-DD' for daily notes
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER                    -- soft delete / tombstone
) STRICT;

CREATE TABLE blocks (
  id          TEXT PRIMARY KEY,
  note_id     TEXT NOT NULL REFERENCES notes(id),
  order_key   TEXT NOT NULL,             -- fractional index for reordering
  kind        TEXT NOT NULL,             -- 'paragraph' | 'heading' | 'todo' | ...
  content     TEXT NOT NULL,             -- JSON: rich-text spans + attrs
  plain_text  TEXT NOT NULL,             -- extracted at write time; feeds FTS
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER
) STRICT;
CREATE INDEX blocks_by_note ON blocks(note_id, order_key);
```

### Tasks, pages, and backlinks via a polymorphic join table

```sql
CREATE TABLE pages (
  id TEXT PRIMARY KEY, kind TEXT NOT NULL,           -- 'project' | 'person'
  title TEXT NOT NULL, body TEXT,                    -- body: JSON blocks or markdown
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, deleted_at INTEGER
) STRICT;

CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL, notes TEXT,
  bucket TEXT NOT NULL,                              -- 'today' | 'week' | 'backlog' ...
  status TEXT NOT NULL DEFAULT 'open',
  priority INTEGER,
  due_date TEXT,
  origin_block_id TEXT REFERENCES blocks(id),        -- task captured from a note
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
  completed_at INTEGER, deleted_at INTEGER
) STRICT;
CREATE INDEX tasks_by_bucket ON tasks(bucket, status, priority);

-- One edge table for all refs/backlinks (note->page, task->page, block->task, ...)
CREATE TABLE links (
  src_type TEXT NOT NULL, src_id TEXT NOT NULL,
  dst_type TEXT NOT NULL, dst_id TEXT NOT NULL,
  PRIMARY KEY (src_type, src_id, dst_type, dst_id)
) STRICT;
CREATE INDEX links_backlinks ON links(dst_type, dst_id);  -- "what links here"
```

Backlinks = `SELECT src_type, src_id FROM links WHERE dst_type=? AND dst_id=?`. The Rust handler rewrites a block's outgoing links (`DELETE`+`INSERT`) in the same transaction as the block upsert.

### Briefs: JSON payload column, not normalized

Briefs are generated documents: written once, read whole, never queried field-by-field. A JSON payload with a couple of promoted columns is right; normalizing them is pure overhead. If you later need to query inside them, SQLite's `json_extract` / generated columns get you there without a migration of the payload itself.

```sql
CREATE TABLE briefs (
  id TEXT PRIMARY KEY,
  date TEXT NOT NULL, kind TEXT NOT NULL DEFAULT 'daily',
  payload TEXT NOT NULL,        -- structured JSON summary
  rendered_text TEXT NOT NULL,  -- flattened text for FTS
  created_at INTEGER NOT NULL,
  UNIQUE (date, kind)
) STRICT;

CREATE TABLE focus_sessions (
  id TEXT PRIMARY KEY,
  task_id TEXT REFERENCES tasks(id),
  started_at INTEGER NOT NULL, ended_at INTEGER,
  note TEXT
) STRICT;
```

### FTS5: one unified search table, maintained transactionally by the single writer

External-content FTS5 tables + triggers are the classic pattern ([SQLite FTS5 docs](https://www.sqlite.org/fts5.html)), but external content binds one FTS table to one content table — awkward across notes+tasks+briefs. Since **all writes already flow through one Rust handler**, the simpler and more flexible pattern is a standalone FTS5 table updated in the same transaction as each entity write (no triggers, no drift, and delete/update handled explicitly):

```sql
CREATE VIRTUAL TABLE search USING fts5(
  entity_type UNINDEXED,   -- 'block' | 'task' | 'brief' | 'page'
  entity_id   UNINDEXED,
  title,                   -- task title / page title / note date; '' for blocks
  body,                    -- block plain_text / task notes / brief rendered_text
  tokenize = 'porter unicode61 remove_diacritics 2'
);
```

```rust
fn upsert_block(conn: &Connection, b: &Block) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("INSERT INTO blocks (...) VALUES (...) ON CONFLICT(id) DO UPDATE SET ...", ...)?;
    tx.execute("DELETE FROM search WHERE entity_type='block' AND entity_id=?1", [&b.id])?;
    tx.execute("INSERT INTO search (entity_type, entity_id, title, body) VALUES ('block', ?1, '', ?2)",
               (&b.id, &b.plain_text))?;
    // rewrite links for this block here too
    tx.commit()
}

// Query, ranked, with snippets:
// SELECT entity_type, entity_id, snippet(search, 3, '<b>', '</b>', '…', 12) AS snip, rank
// FROM search WHERE search MATCH ?1 ORDER BY rank LIMIT 50;
```

Use `bm25()`/`rank` for ordering, `snippet()`/`highlight()` for result display, and prefix queries (`term*`) for search-as-you-type. If you ever hand write access to a second process (see below), switch to triggers so the index can't be bypassed.

### Two processes (GUI + MCP server): prefer single-owner, WAL as fallback

**Best design: one owner process.** Have the MCP server reach the data through the app (local socket/XPC/HTTP on localhost), or — since both are Rust — ship the MCP server as a mode of the same binary/core so exactly one process ever has the file open for writes. This eliminates the whole class of locking issues and keeps FTS/links maintenance in one code path.

**Acceptable fallback: both processes open the file with WAL.** SQLite handles same-host multi-process access well ([WAL docs](https://sqlite.org/wal.html), [SQLite forum on multi-process WAL](https://sqlite.org/forum/forumpost/c4dbf6ca17)):
- `PRAGMA journal_mode=WAL` (persistent — set once at DB creation). WAL gives **many readers + one writer concurrently**; readers never block the writer and vice versa.
- `PRAGMA busy_timeout` of **5000ms or more** on *every* connection — benchmarks show values below ~5s still produce occasional `SQLITE_BUSY` under concurrent writes ([analysis](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/)).
- Use **`BEGIN IMMEDIATE`** for any transaction that will write. The nasty caveat: a *deferred* read transaction that upgrades to a write can fail with `SQLITE_BUSY` **immediately, ignoring busy_timeout**, if another connection wrote in the meantime ([berthub.eu](https://berthub.eu/articles/posts/a-brief-post-on-sqlite3-database-locked-despite-timeout/)). `BEGIN IMMEDIATE` avoids this class entirely. In rusqlite: `conn.transaction_with_behavior(TransactionBehavior::Immediate)`.
- `PRAGMA synchronous=NORMAL` (the recommended WAL pairing).
- Caveats: WAL uses shared memory, so all processes must be on the same host and the DB must not live on a network filesystem; a read-mostly MCP server under WAL is essentially risk-free.

A good middle path: MCP server opens the file **read-only** (queries, search) and routes any *writes* through the GUI app via IPC. You get zero-contention reads and still a single writer.

---

## 4. Automerge/CRDTs: YAGNI now — buy sync-readiness cheaply instead

**Recommendation: no CRDT now.** For a single-device, single-user app, Automerge (Rust crate at 0.10.0) adds: a document-oriented data model that fights your relational queries and FTS, permanent history growth, a second source of truth to keep consistent with SQLite, and API churn — while solving a concurrent-merge problem you don't have. The local-first ecosystem's own guidance points at table-level CRDT layers over SQLite ([cr-sqlite](https://github.com/vlcn-io/cr-sqlite), [sqlite-sync](https://github.com/sqliteai/sqlite-sync)) precisely because full document CRDTs are heavy for app data ([Automerge data modeling](https://automerge.org/docs/cookbook/modeling-data/)).

**The cheap moves that keep the iOS-sync door open** (all already in the schema above):

1. **Client-generated, globally unique, sortable ids** (UUIDv7/ULID) for every entity — never autoincrement rowids as foreign keys. This is the single biggest painting-into-a-corner risk, and it costs nothing today.
2. **`updated_at` on every table** (and add a `device_id` column later, not now) — enables last-writer-wins sync per row/column.
3. **Soft deletes (`deleted_at`)** instead of hard deletes for syncable entities — tombstones are required by every sync scheme.
4. **All mutations flow through Crux events** — the core is already an event-sourced state machine, so you can later add an oplog/changes table in the storage handler with a one-line hook, giving you a change feed to sync from.
5. **Keep the storage layer behind the `StorageOperation` effect** — if you later adopt cr-sqlite/sqlite-sync (schema-compatible CRDT layers over normal tables) or move *just the rich-text blocks* into Automerge (the one place real merge semantics matter, and [automerge-swift](https://forums.swift.org/t/introducing-automerge-enable-collaborative-asynchronous-syncing-for-your-data-structures/67985) exists for the iOS side), it's an implementation swap inside one Rust module, invisible to the core and the Swift shell.

For single-user cross-device sync, per-row LWW over this schema (or cr-sqlite doing it for you) is almost certainly sufficient; document CRDTs only earn their keep if you ever want real-time collaborative editing of the same note.

---

## Sources

- [Rust ORMs in 2026: Diesel vs SQLx vs SeaORM vs Rusqlite](https://aarambhdevhub.medium.com/rust-orms-in-2026-diesel-vs-sqlx-vs-seaorm-vs-rusqlite-which-one-should-you-actually-use-706d0fe912f3) · [sqlx::sqlite docs](https://docs.rs/sqlx/latest/sqlx/sqlite/index.html) · [rusqlite libsqlite3-sys build.rs (FTS5 in bundled)](https://github.com/rusqlite/rusqlite/blob/master/libsqlite3-sys/build.rs)
- [rusqlite_migration docs](https://docs.rs/rusqlite_migration) · [refinery](https://github.com/rust-db/refinery)
- [crux_core::middleware docs](https://docs.rs/crux_core/latest/crux_core/middleware/index.html) · [crux_core::effects (EffectRouter) docs](https://docs.rs/crux_core/latest/crux_core/effects/index.html) · [crux releases (v0.19.0 Effect Routing, v0.16.0 middleware)](https://github.com/redbadger/crux/releases) · [counter-routing example](https://github.com/redbadger/crux/tree/master/examples/counter-routing) · [Crux overview](https://redbadger.github.io/crux/)
- [SQLite FTS5](https://www.sqlite.org/fts5.html) · [SQLite FTS5 triggers pattern](https://simonh.uk/2021/05/11/sqlite-fts5-triggers/)
- [SQLite WAL](https://sqlite.org/wal.html) · [WAL with multiple processes (SQLite forum)](https://sqlite.org/forum/forumpost/c4dbf6ca17) · [SQLITE_BUSY despite timeout (berthub.eu)](https://berthub.eu/articles/posts/a-brief-post-on-sqlite3-database-locked-despite-timeout/) · [SQLite concurrent writes benchmarks](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/)
- [Automerge](https://github.com/automerge/automerge) · [Automerge data modeling](https://automerge.org/docs/cookbook/modeling-data/) · [automerge-swift announcement](https://forums.swift.org/t/introducing-automerge-enable-collaborative-asynchronous-syncing-for-your-data-structures/67985) · [cr-sqlite](https://github.com/vlcn-io/cr-sqlite) · [sqlite-sync](https://github.com/sqliteai/sqlite-sync)