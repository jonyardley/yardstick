# Daily — Phase 0: Walking Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A running macOS app + embedded MCP server where a trivial real feature (create a task, see the task list) flows through every architectural seam: SwiftUI shell → BoltFFI → Crux core → EffectRouter → Rust SQLite handler, and MCP tool call → core event → live UI update.

**Architecture:** Pure Crux core (`shared`) emits typed `StorageOperation` effects; a `runtime` crate routes storage effects to a Rust thread owning rusqlite (never crossing FFI) and serializes the rest to the Swift shell via a BoltFFI push callback; an rmcp streamable-HTTP server in the same process dispatches the same core events. See `docs/superpowers/specs/2026-07-02-daily-app-design.md` §2.

**Tech Stack:** Rust edition 2024 / crux_core 0.19 / boltffi =0.25.2 / facet =0.44 / rusqlite (bundled) / rusqlite_migration 2.5 / rmcp 2.x / axum 0.8 / tokio 1 / SwiftUI (macOS 15.0) / XcodeGen / just / cargo-nextest.

## Global Constraints

- Pin exactly: `facet = "=0.44"`, `boltffi = "=0.25.2"` (and `boltffi_cli` at `=0.25.2`). crates.io has newer versions; the crux examples pin these — follow the examples, not latest.
- `crux_core = "0.19"`, `rusqlite = { version = "0.39", features = ["bundled"] }`, `rusqlite_migration = "2.5"`, `rmcp = "2"` (pin the minor once resolved, e.g. `"=2.1.x"`). *(Narrowed from 0.40/2.6 in Task 3: rusqlite_migration 2.6.0 requires rustc ≥1.95 but the toolchain is pinned at 1.90 per the crux templates. Alternative — bumping the toolchain to 1.95 — deferred; revisit if a needed dependency forces it again.)*
- Workspace: `resolver = "3"`, `edition = "2024"`, `rust-version = "1.90"`.
- macOS deployment target: **15.0**. App/product name: **Daily** (repo codename Yardstick — spec §12 Q1).
- Crate dependency DAG (never violate): `shared → crux_core` only; `store → shared`; `mcp → shared, store`; `runtime → shared, store, mcp`. `mcp` must NOT depend on `runtime`.
- The core stays pure: no I/O, no clocks, no random, no tokio in `shared`. IDs (UUIDv7) are generated in `store`, never in `shared`.
- All SQL tables STRICT; every entity table has `space_id`, `created_at`, `updated_at`, `deleted_at` (spec §3).
- Canonical upstream references when an API doesn't match this plan: `crux` repo `examples/counter` (Apple shell mechanics, typegen), `examples/counter-routing` (EffectRouter + `CruxShell` push callback), `examples/weather` (Swift `Core`/`CoreBridge` structure). EffectRouter is RFC-stage — if signatures drifted in a patch release, mirror the example and note the deviation in the commit message.
- Commit after every green test cycle. Run Rust tests with `cargo nextest run -p <crate>`.
- **Workflow (docs/SDLC.md):** every task runs on its own branch `p0/t<N>-<slug>` cut from latest `main`. A task's final "Commit" step means: commit on the task branch, push, open a PR (conventional title, template filled in, TDD evidence pasted), then STOP — Jon reviews and squash-merges. Never commit to `main`; never merge your own PR.
- CI: the `guardrails` and `pr-title` jobs exist from the SDLC setup. Task 1 adds the `rust` job; Task 9 adds the `apple` job. The PR that adds a job also adds it to the required status checks (exact command in the task).

---

### Task 1: Repository scaffold and pinned toolchain

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `justfile`
- Modify: `.gitignore` (exists from the SDLC setup — verify it has the entries below), `.github/workflows/ci.yml`
- Test: none (toolchain verification commands stand in for tests)

**Interfaces:**
- Produces: a building (empty) workspace all later tasks add members to; `just` targets other tasks extend.

- [x] **Step 1: Write the workspace files**

`Cargo.toml`:
```toml
[workspace]
resolver = "3"
members = ["shared", "store", "mcp", "runtime"]

[workspace.package]
edition = "2024"
rust-version = "1.90"
version = "0.1.0"

[workspace.dependencies]
crux_core = "0.19"
facet = "=0.44"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v7", "serde"] }
rusqlite = { version = "0.40", features = ["bundled"] }
rusqlite_migration = "2.6"
rmcp = "2"
tokio = { version = "1", features = ["full"] }
axum = "0.8"
schemars = "1"
anyhow = "1"
```

`rust-toolchain.toml`:
```toml
[toolchain]
channel = "1.90"
```

`.gitignore`:
```
/target
apple/generated/
apple/*.xcodeproj
.DS_Store
xcuserdata/
.build/
```

`justfile`:
```make
default: test

test:
    cargo nextest run --workspace
```

- [x] **Step 2: Create the four member crates as stubs**

Run:
```bash
cargo new --lib shared && cargo new --lib store && cargo new --lib mcp && cargo new --lib runtime
```
Then in each generated `Cargo.toml` replace the `[package]` block's edition/version lines with `edition.workspace = true`, `rust-version.workspace = true`, `version.workspace = true`.

- [x] **Step 3: Install and verify the toolchain**

Run (each must succeed; record versions in the commit message):
```bash
rustup show                          # 1.90 from rust-toolchain.toml
cargo install cargo-nextest --locked
cargo install just --locked          # skip if already installed
cargo install boltffi_cli --version '=0.25.2' --locked
brew install xcodegen                # skip if already installed
xcodegen --version && boltffi --version && cargo nextest --version
```

- [x] **Step 4: Verify the empty workspace builds and tests**

Run: `cargo build --workspace && cargo nextest run --workspace --no-tests=pass`
Expected: build succeeds; nextest exits 0 (`--no-tests=pass` because the workspace has no tests yet — keep the flag in the justfile, it is harmless once tests exist).

Update the `justfile` test target accordingly:
```make
test:
    cargo nextest run --workspace --no-tests=pass
```

- [x] **Step 4b: Add the `rust` job to CI**

Append to the `jobs:` map in `.github/workflows/ci.yml` (guardrails/pr-title already live there):
```yaml
  rust:
    runs-on: macos-15
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        # components must be explicit — the action installs a minimal
        # profile; the clippy step failed without them (seen on PR #2)
        with: { toolchain: "1.90", components: "clippy, rustfmt" }
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@v2
        with: { tool: cargo-nextest }
      - run: cargo nextest run --workspace --no-tests=pass
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo fmt --check
```

- [x] **Step 5: Commit, open the PR, add the required check**

```bash
git add -A
git commit -m "chore: workspace scaffold with pinned toolchain (crux 0.19, boltffi =0.25.2, facet =0.44)"
git push -u origin p0/t1-scaffold
gh pr create --fill   # then fill the template sections in the PR body
```
After Jon merges, add `rust` to the required status checks:
```bash
gh api -X POST repos/jonyardley/yardstick/branches/main/protection/required_status_checks/contexts --input - <<< '["rust"]'
```

---

### Task 2: `shared` — pure Crux core with a Task domain and Storage effect

**Files:**
- Create: `shared/src/lib.rs`, `shared/src/app.rs`, `shared/src/effects/mod.rs`, `shared/src/effects/storage.rs`
- Modify: `shared/Cargo.toml`
- Test: inline `#[cfg(test)]` module in `shared/src/app.rs`

**Interfaces:**
- Produces (used by every later task):
  - `shared::Task { pub id: String, pub title: String }`
  - `shared::Event::{Startup, CreateTask { title: String }, TaskSaved(StorageResult), TasksLoaded(StorageResult)}`
  - `shared::effects::storage::StorageOperation::{InsertTask { title: String }, ListTasks}` with `Operation::Output = StorageResult`
  - `shared::effects::storage::StorageResult::{Task(Task), Tasks(Vec<Task>), Error(String)}`
  - `shared::{Daily, Model, ViewModel { pub tasks: Vec<Task>, pub count: u64 }, Effect}` — `Daily: crux_core::App`

- [x] **Step 1: Fill in `shared/Cargo.toml`**

```toml
[package]
name = "shared"
edition.workspace = true
rust-version.workspace = true
version.workspace = true

[lib]
crate-type = ["lib", "staticlib", "cdylib"]

[features]
facet_typegen = ["crux_core/facet_typegen"]
codegen = ["facet_typegen", "dep:anyhow", "dep:clap"]

[dependencies]
crux_core = { workspace = true }
facet = { workspace = true }
serde = { workspace = true }
anyhow = { workspace = true, optional = true }
clap = { version = "4", features = ["derive"], optional = true }

[dev-dependencies]
crux_core = { workspace = true, features = ["testing"] }

[[bin]]
name = "codegen"
required-features = ["codegen"]
```

(The `codegen` bin source arrives in Task 5; `required-features` keeps the workspace building until then — create an empty `shared/src/bin/codegen.rs` containing `fn main() {}` now so cargo doesn't error.)

- [x] **Step 2: Write the storage operation types**

`shared/src/effects/mod.rs`:
```rust
pub mod storage;
```

`shared/src/effects/storage.rs`:
```rust
use crux_core::{capability::Operation, command::RequestBuilder, Command, Request};
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub title: String,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum StorageOperation {
    InsertTask { title: String },
    ListTasks,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum StorageResult {
    Task(Task),
    Tasks(Vec<Task>),
    Error(String),
}

impl Operation for StorageOperation {
    type Output = StorageResult;
}

pub fn insert_task<Effect, Event>(
    title: impl Into<String>,
) -> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::InsertTask { title: title.into() })
}

pub fn list_tasks<Effect, Event>(
) -> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::ListTasks)
}
```

- [x] **Step 3: Write the failing app tests**

`shared/src/app.rs` — start with the test module (the app code in Step 5 goes above it):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::storage::{StorageOperation, StorageResult, Task};

    fn task(id: &str, title: &str) -> Task {
        Task { id: id.into(), title: title.into() }
    }

    #[test]
    fn startup_requests_task_list_then_renders() {
        let app = Daily;
        let mut model = Model::default();

        let mut cmd = app.update(Event::Startup, &mut model);
        // Generated by #[effect]: fluent helpers named after the variant.
        // If the generated name differs, check `#[effect]` docs / cargo expand.
        let request = cmd.expect_one_effect().expect_storage();
        assert_eq!(request.operation, StorageOperation::ListTasks);
    }

    #[test]
    fn tasks_loaded_updates_model_and_renders() {
        let app = Daily;
        let mut model = Model::default();

        let mut cmd = app.update(
            Event::TasksLoaded(StorageResult::Tasks(vec![task("t1", "Buy milk")])),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        let view = app.view(&model);
        assert_eq!(view.count, 1);
        assert_eq!(view.tasks[0].title, "Buy milk");
    }

    #[test]
    fn create_task_requests_insert_then_appends_on_save() {
        let app = Daily;
        let mut model = Model::default();

        let mut cmd = app.update(Event::CreateTask { title: "Ship it".into() }, &mut model);
        let request = cmd.expect_one_effect().expect_storage();
        assert_eq!(
            request.operation,
            StorageOperation::InsertTask { title: "Ship it".into() }
        );

        let mut cmd = app.update(
            Event::TaskSaved(StorageResult::Task(task("t2", "Ship it"))),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        assert_eq!(app.view(&model).tasks.len(), 1);
    }

    #[test]
    fn storage_error_is_surfaced_not_fatal() {
        let app = Daily;
        let mut model = Model::default();

        let mut cmd = app.update(
            Event::TasksLoaded(StorageResult::Error("disk full".into())),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        assert_eq!(app.view(&model).error.as_deref(), Some("disk full"));
    }
}
```

Note on assertion helpers: `crux_core/testing` + the `#[effect]` macro generate per-variant helpers (`EffectTestExt`). The exact spelling in 0.19 for a variant `Storage(...)` is `expect_storage()` / `expect_only_storage_with(...)`-style; confirm against the generated docs (`cargo doc -p shared --open`) or the weather example's tests and adjust the test calls — the *assertions* (operation equality, render emitted, model contents) are the contract.

- [x] **Step 4: Run tests to verify they fail**

Run: `cargo nextest run -p shared`
Expected: FAIL to compile — `Daily`, `Model`, `Event` not defined.

- [x] **Step 5: Write the app**

Top of `shared/src/app.rs`:
```rust
use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    App, Command,
};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::storage::{self, StorageOperation, StorageResult, Task};

#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum Event {
    Startup,
    CreateTask { title: String },
    TaskSaved(StorageResult),
    TasksLoaded(StorageResult),
}

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Storage(StorageOperation),
}

#[derive(Default)]
pub struct Model {
    pub tasks: Vec<Task>,
    pub error: Option<String>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ViewModel {
    pub tasks: Vec<Task>,
    pub count: u64,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct Daily;

impl App for Daily {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Startup => storage::list_tasks().then_send(Event::TasksLoaded),
            Event::CreateTask { title } => {
                storage::insert_task(title).then_send(Event::TaskSaved)
            }
            Event::TaskSaved(StorageResult::Task(task)) => {
                model.error = None;
                model.tasks.push(task);
                render()
            }
            Event::TasksLoaded(StorageResult::Tasks(tasks)) => {
                model.error = None;
                model.tasks = tasks;
                render()
            }
            Event::TaskSaved(StorageResult::Error(e))
            | Event::TasksLoaded(StorageResult::Error(e)) => {
                model.error = Some(e);
                render()
            }
            // Mismatched result shapes are handler bugs; keep state, just re-render.
            Event::TaskSaved(_) | Event::TasksLoaded(_) => render(),
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        ViewModel {
            tasks: model.tasks.clone(),
            count: model.tasks.len() as u64,
            error: model.error.clone(),
        }
    }
}
```

`shared/src/lib.rs`:
```rust
pub mod app;
pub mod effects;

pub use app::{Daily, Effect, Event, Model, ViewModel};
pub use crux_core::Core;
pub use effects::storage::{StorageOperation, StorageResult, Task};
```

- [x] **Step 6: Run tests to verify they pass**

Run: `cargo nextest run -p shared`
Expected: 4 tests PASS.

- [x] **Step 7: Commit**

```bash
git add shared
git commit -m "feat(core): Daily crux app — task list model, storage effect, render loop"
```

---

### Task 3: `store` — SQLite schema, migrations, and operation executor

**Files:**
- Create: `store/migrations/001_initial.sql`, `store/src/lib.rs`, `store/src/db.rs`, `store/src/executor.rs`
- Modify: `store/Cargo.toml`
- Test: inline `#[cfg(test)]` modules

**Interfaces:**
- Consumes: `shared::{StorageOperation, StorageResult, Task}`
- Produces:
  - `store::db::open(path: &std::path::Path) -> rusqlite::Result<rusqlite::Connection>` (WAL, pragmas, migrations applied)
  - `store::db::open_in_memory() -> rusqlite::Result<rusqlite::Connection>` (tests, and MCP read path until Phase 1)
  - `store::executor::execute(conn: &rusqlite::Connection, op: &StorageOperation) -> StorageResult`
  - `store::DEFAULT_SPACE_ID: &str`

- [x] **Step 1: Fill in `store/Cargo.toml`**

```toml
[package]
name = "store"
edition.workspace = true
rust-version.workspace = true
version.workspace = true

[dependencies]
shared = { path = "../shared" }
rusqlite = { workspace = true }
rusqlite_migration = { workspace = true }
uuid = { workspace = true }
```

- [x] **Step 2: Write the initial migration**

`store/migrations/001_initial.sql` (spec §3: `space_id` on every entity from migration 001; STRICT; soft deletes):
```sql
CREATE TABLE spaces (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER
) STRICT;

INSERT INTO spaces (id, name, created_at, updated_at)
VALUES
  ('0197f000-0000-7000-8000-000000000001', 'Red Badger', unixepoch(), unixepoch()),
  ('0197f000-0000-7000-8000-000000000002', 'Yardley',    unixepoch(), unixepoch());

CREATE TABLE tasks (
  id          TEXT PRIMARY KEY,
  space_id    TEXT NOT NULL REFERENCES spaces(id),
  title       TEXT NOT NULL,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER
) STRICT;

CREATE INDEX tasks_by_space ON tasks(space_id, created_at);
```

- [x] **Step 3: Write failing db tests**

`store/src/db.rs` test module:
```rust
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
        let conn = open(&dir.join("test.db")).unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
    }
}
```

- [x] **Step 4: Run to verify failure**

Run: `cargo nextest run -p store`
Expected: FAIL to compile — `MIGRATIONS`, `open`, `open_in_memory` not defined.

- [x] **Step 5: Implement `db.rs`**

```rust
use std::{path::Path, sync::LazyLock, time::Duration};

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

pub static MIGRATIONS: LazyLock<Migrations> = LazyLock::new(|| {
    Migrations::new(vec![M::up(include_str!("../migrations/001_initial.sql"))])
});

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
```

(`open_on_disk_uses_wal` asserts the on-disk path really is WAL — in-memory can't check that.)

- [x] **Step 6: Run db tests to verify they pass**

Run: `cargo nextest run -p store`
Expected: 3 tests PASS.

- [x] **Step 7: Write failing executor tests**

`store/src/executor.rs` test module:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use shared::{StorageOperation, StorageResult};

    #[test]
    fn insert_then_list_round_trips() {
        let conn = open_in_memory().unwrap();

        let inserted = execute(&conn, &StorageOperation::InsertTask { title: "Buy milk".into() });
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
        execute(&conn, &StorageOperation::InsertTask { title: "first".into() });
        execute(&conn, &StorageOperation::InsertTask { title: "second".into() });
        conn.execute("UPDATE tasks SET deleted_at = unixepoch() WHERE title = 'first'", [])
            .unwrap();

        let StorageResult::Tasks(tasks) = execute(&conn, &StorageOperation::ListTasks) else {
            panic!()
        };
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "second");
    }
}
```

- [x] **Step 8: Run to verify failure**

Run: `cargo nextest run -p store`
Expected: FAIL to compile — `execute` not defined.

- [x] **Step 9: Implement `executor.rs`**

```rust
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
            Ok(StorageResult::Task(Task { id, title: title.clone() }))
        }
        StorageOperation::ListTasks => {
            let mut stmt = conn.prepare(
                "SELECT id, title FROM tasks
                 WHERE deleted_at IS NULL AND space_id = ?1
                 ORDER BY id", // UUIDv7 is time-sortable → oldest first
            )?;
            let tasks = stmt
                .query_map([DEFAULT_SPACE_ID], |row| {
                    Ok(Task { id: row.get(0)?, title: row.get(1)? })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(StorageResult::Tasks(tasks))
        }
    }
}
```

`store/src/lib.rs`:
```rust
pub mod db;
pub mod executor;

pub use db::{open, open_in_memory, DEFAULT_SPACE_ID, MIGRATIONS};
pub use executor::execute;
```

- [x] **Step 10: Run all store tests to verify they pass**

Run: `cargo nextest run -p store`
Expected: 5 tests PASS.

- [x] **Step 11: Commit**

```bash
git add store
git commit -m "feat(store): SQLite schema v1, WAL open, storage operation executor"
```

---

### Task 4: `runtime` — EffectRouter wiring with Rust-side storage handling

**Files:**
- Create: `runtime/src/lib.rs`, `runtime/src/router.rs`, `runtime/src/storage_handler.rs`
- Modify: `runtime/Cargo.toml`
- Test: `runtime/tests/headless.rs`

**Interfaces:**
- Consumes: `shared::{Daily, Event, Effect, ViewModel}`, `store::{open, execute}`
- Produces (Tasks 6–8 rely on these exact signatures):
  - `runtime::AppRuntime` with:
    - `AppRuntime::new(db_path: Option<&std::path::Path>, shell: std::sync::Arc<dyn ShellCallback>) -> anyhow::Result<Arc<AppRuntime>>` (None = in-memory DB)
    - `fn send_event(&self, event: shared::Event)`
    - `fn view(&self) -> shared::ViewModel`
  - `runtime::ShellCallback: Send + Sync { fn process_effects(&self, effects_bincode: Vec<u8>); }` — receives the serialized (non-storage) effects for the Swift shell.

- [x] **Step 1: Fill in `runtime/Cargo.toml`**

```toml
[package]
name = "runtime"
edition.workspace = true
rust-version.workspace = true
version.workspace = true

[lib]
crate-type = ["lib", "staticlib"]

[dependencies]
shared = { path = "../shared", features = ["facet_typegen"] }
store = { path = "../store" }
crux_core = { workspace = true }
anyhow = { workspace = true }

[dev-dependencies]
```

(BoltFFI export and the `mcp` dependency are added in Tasks 5 and 7 — keep this task headless.)

- [x] **Step 2: Write the failing headless integration test**

`runtime/tests/headless.rs`:
```rust
use std::sync::{Arc, Mutex};

use runtime::{AppRuntime, ShellCallback};
use shared::Event;

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
    rt.send_event(Event::CreateTask { title: "Walk the skeleton".into() });

    // Storage handler runs on its own thread; poll until the follow-up
    // event lands (bounded, deterministic-enough for a skeleton test).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let view = rt.view();
        if view.count == 1 {
            assert_eq!(view.tasks[0].title, "Walk the skeleton");
            break;
        }
        assert!(std::time::Instant::now() < deadline, "view never updated: {view:?}");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // The shell received at least one effect batch (Render), and the
    // runtime never panicked routing storage internally.
    assert!(!shell.batches.lock().unwrap().is_empty());
}
```

- [x] **Step 3: Run to verify failure**

Run: `cargo nextest run -p runtime`
Expected: FAIL to compile — `AppRuntime`, `ShellCallback` not defined.

- [x] **Step 4: Implement the storage handler thread**

`runtime/src/storage_handler.rs`:
```rust
use std::sync::mpsc::{channel, Sender};
use std::thread;

use crux_core::Request;
use shared::StorageOperation;

/// One background thread owns the rusqlite Connection (Send, not Sync).
/// Requests come in on an mpsc queue; results resolve back through the
/// router via the callback captured at construction.
pub struct StorageHandler {
    jobs: Sender<Request<StorageOperation>>,
}

impl StorageHandler {
    pub fn new<F>(db_path: Option<std::path::PathBuf>, resolve: F) -> anyhow::Result<Self>
    where
        F: Fn(Request<StorageOperation>, shared::StorageResult) + Send + 'static,
    {
        let (jobs, rx) = channel::<Request<StorageOperation>>();
        let conn = match &db_path {
            Some(p) => store::open(p)?,
            None => store::open_in_memory()?,
        };
        thread::Builder::new().name("daily-storage".into()).spawn(move || {
            while let Ok(request) = rx.recv() {
                let output = store::execute(&conn, &request.operation);
                resolve(request, output);
            }
        })?;
        Ok(Self { jobs })
    }

    pub fn process(&self, request: Request<StorageOperation>) {
        // Receiver only drops on app shutdown; a send error then is moot.
        let _ = self.jobs.send(request);
    }
}
```

- [x] **Step 5: Implement the router + runtime**

`runtime/src/router.rs` — this is the EffectRouter seam. Mirror `examples/counter-routing/shared/src/ffi.rs` from the crux repo; the shape below is that example adapted to our names. EffectRouter is RFC-stage: if a 0.19.x signature differs, follow the example and note it in the commit.

```rust
use std::sync::{Arc, Weak};

use crux_core::{
    effects::{EffectRouter, ResolveSink, Routes, Serialized},
    bridge::BincodeFfiFormat,
    Core,
};
use shared::{Daily, Effect, Event, ViewModel};

use crate::storage_handler::StorageHandler;
use crate::ShellCallback;

pub struct AppRuntime {
    router: Arc<EffectRouter<Daily, AppRoutes>>,
}

pub struct AppRoutes {
    pub serialized: Arc<Serialized<Daily, Self, BincodeFfiFormat>>,
    // StorageHandler is constructed in AppRuntime::new (it needs the db
    // path and the router weak ref for resolution).
}

impl AppRuntime {
    pub fn new(
        db_path: Option<&std::path::Path>,
        shell: Arc<dyn ShellCallback>,
    ) -> anyhow::Result<Arc<Self>> {
        let db_path = db_path.map(|p| p.to_path_buf());

        let router = EffectRouter::new(Core::<Daily>::new(), {
            let shell = shell.clone();
            move |routes: &AppRoutes, router: Weak<EffectRouter<Daily, AppRoutes>>, effect: Effect| {
                match effect {
                    Effect::Storage(request) => {
                        // Handled entirely in Rust — never serialized.
                        storage_for(&db_path, router).process(request);
                    }
                    other => {
                        let bytes = routes
                            .serialized
                            .serialize(other)
                            .expect("effect serialization is infallible for Render");
                        shell.process_effects(bytes);
                    }
                }
            }
        });

        Ok(Arc::new(Self { router }))
    }

    pub fn send_event(&self, event: Event) {
        self.router.send_event(event);
    }

    pub fn view(&self) -> ViewModel {
        self.router.view()
    }
}
```

Implementation note (deliberate, resolve while coding against the real API): the closure above sketches intent — in the actual `counter-routing` example, `Routes::new(router: Weak<…>)` constructs handler instances once (each holding the `Weak` router as its `ResolveSink`), and the dispatch closure only pattern-matches and calls `routes.<handler>.process(request)`. Build `StorageHandler` inside `AppRoutes::new` with `resolve = move |req, out| { sink.upgrade().map(|r| r.resolve_request(req, out)); }`. The headless test in Step 2 is the arbiter: storage stays in Rust, Render reaches the shell, `view()` shows the task.

`runtime/src/lib.rs`:
```rust
mod router;
mod storage_handler;

pub use router::AppRuntime;

pub trait ShellCallback: Send + Sync {
    fn process_effects(&self, effects_bincode: Vec<u8>);
}
```

- [x] **Step 6: Run the headless test until green**

Run: `cargo nextest run -p runtime`
Expected: 1 test PASS. This is the highest-risk step of Phase 0 — budget time to read `examples/counter-routing` closely. Do not proceed until green.

- [x] **Step 7: Commit**

```bash
git add runtime
git commit -m "feat(runtime): EffectRouter with Rust-side SQLite storage handling"
```

---

### Task 5: Typegen + BoltFFI export

**Files:**
- Create: `shared/src/bin/codegen.rs` (replace stub), `shared/boltffi.toml`, `runtime/src/ffi.rs`
- Modify: `shared/Cargo.toml` (already has codegen feature), `runtime/Cargo.toml`, `runtime/src/lib.rs`, `justfile`
- Test: command-level verification (generated artifacts exist and compile)

**Interfaces:**
- Consumes: `runtime::{AppRuntime, ShellCallback}`
- Produces:
  - Swift package `apple/generated/App` — value types `Event`, `ViewModel`, `Task`, `Effect` (FFI companion) with `bincodeSerialize/bincodeDeserialize`
  - Swift package `apple/generated/Shared` — BoltFFI bindings exposing `CoreFFI` (constructor taking a `CruxShell` impl, `update(eventBytes:)`, `resolveSerialized(id:outputBytes:)`, `view() -> bytes`) and the `CruxShell` protocol with `processEffects(bytes:)`
  - `just typegen`, `just package` targets

- [x] **Step 1: Write the codegen bin**

`shared/src/bin/codegen.rs`:
```rust
use crux_core::type_generation::facet::{Config, TypeRegistry};
use shared::Daily;

#[derive(clap::Parser)]
struct Args {
    #[arg(long, default_value = "apple/generated")]
    output_dir: std::path::PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = <Args as clap::Parser>::parse();
    let registry = TypeRegistry::new().register_app::<Daily>()?.build()?;
    let config = Config::builder("App", &args.output_dir).build();
    registry.swift(&config)?;
    println!("Swift types written to {}", args.output_dir.display());
    Ok(())
}
```

- [x] **Step 2: Write the BoltFFI export in `runtime`**

Add to `runtime/Cargo.toml` `[dependencies]`: `boltffi = "=0.25.2"`.

`runtime/src/ffi.rs` (mirror `examples/counter-routing` ffi.rs; adapted names):
```rust
use std::sync::Arc;

use crate::{AppRuntime, ShellCallback};

/// Implemented by the Swift shell; BoltFFI generates the Swift protocol.
#[boltffi::export]
pub trait CruxShell: Send + Sync {
    fn process_effects(&self, effects: Vec<u8>);
}

struct ShellAdapter(Arc<dyn CruxShell>);
impl ShellCallback for ShellAdapter {
    fn process_effects(&self, effects_bincode: Vec<u8>) {
        self.0.process_effects(effects_bincode);
    }
}

#[boltffi::export]
pub struct CoreFFI {
    runtime: Arc<AppRuntime>,
}

#[boltffi::export]
impl CoreFFI {
    #[boltffi::constructor]
    pub fn new(db_path: String, shell: Arc<dyn CruxShell>) -> Self {
        let path = if db_path.is_empty() { None } else { Some(std::path::PathBuf::from(db_path)) };
        let runtime = AppRuntime::new(path.as_deref(), Arc::new(ShellAdapter(shell)))
            .expect("runtime init");
        Self { runtime }
    }

    /// event: bincode-serialized shared::Event
    pub fn update(&self, event: Vec<u8>) {
        let event = bincode_deserialize_event(&event);
        self.runtime.send_event(event);
    }

    /// Resolve a serialized-lane effect (e.g. none in Phase 0 beyond Render,
    /// which needs no resolution — present for forward-compat).
    pub fn resolve(&self, id: u32, output: Vec<u8>) {
        self.runtime.resolve_serialized(id, output);
    }

    /// returns bincode-serialized shared::ViewModel
    pub fn view(&self) -> Vec<u8> {
        bincode_serialize_view(&self.runtime.view())
    }
}
```

Add `pub mod ffi;` to `runtime/src/lib.rs`, and add `resolve_serialized(&self, id: u32, output: Vec<u8>)` to `AppRuntime` (delegates to the serialized lane's resolve; for Phase 0 with only Render it can be a logged no-op — keep the FFI surface stable). The bincode helper fns come from the serialized lane's `BincodeFfiFormat` (see how counter-routing's ffi.rs serializes/deserializes at the boundary — reuse its exact calls rather than hand-rolling).

`shared/boltffi.toml`:
```toml
[targets.apple]
include_macos = true

[targets.apple.spm]
layout = "ffi-only"
package_name = "Shared"
out_dir = "../apple/generated/Shared"
```

(BoltFFI packs the crate that contains the `#[boltffi::export]` items. Since exports live in `runtime`, the `boltffi.toml` may need to live in `runtime/` instead — `boltffi pack` will say so; put it where the tool expects and update the `just package` path accordingly.)

- [x] **Step 3: Add just targets**

Append to `justfile`:
```make
typegen:
    cargo run -p shared --bin codegen --features codegen -- --output-dir apple/generated

package:
    cd runtime && boltffi pack apple

generate: typegen package
```

- [x] **Step 4: Run and verify generation**

Run: `just generate && ls apple/generated/App apple/generated/Shared`
Expected: both directories contain a `Package.swift` and Swift sources; `swift build` inside `apple/generated/App` compiles (the `Shared` package needs the app target's link step, so compile-check `App` only).

- [x] **Step 5: Verify the workspace still builds and tests pass**

Run: `cargo build --workspace && cargo nextest run --workspace`
Expected: build OK; all prior tests PASS.

- [x] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(ffi): facet typegen + BoltFFI export (CoreFFI, CruxShell push callback)"
```

---

### Task 6: `mcp` — rmcp server with ping/create_task/list_tasks and bearer auth

**Files:**
- Create: `mcp/src/lib.rs`, `mcp/src/server.rs`, `mcp/src/auth.rs`
- Modify: `mcp/Cargo.toml`
- Test: `mcp/tests/tools.rs`

**Interfaces:**
- Consumes: `shared::Event`, `store::{open_in_memory, execute}`
- Produces (Task 7 relies on these):
  - `mcp::EventSink: Send + Sync { fn send_event(&self, event: shared::Event); }` — implemented by `runtime` in Task 7
  - `mcp::DailyMcp::new(reader: Arc<dyn TaskReader>, events: Arc<dyn EventSink>) -> DailyMcp`
  - `mcp::TaskReader: Send + Sync { fn list_tasks(&self) -> Result<Vec<shared::Task>, String>; }`
  - `mcp::serve_http(mcp: DailyMcp, addr: std::net::SocketAddr, token: String) -> impl Future<Output = anyhow::Result<()>>` — axum server, `/mcp` route, bearer-token middleware

- [x] **Step 1: Fill in `mcp/Cargo.toml`**

```toml
[package]
name = "mcp"
edition.workspace = true
rust-version.workspace = true
version.workspace = true

[dependencies]
shared = { path = "../shared" }
store = { path = "../store" }
rmcp = { workspace = true, features = [
    "server", "macros", "schemars",
    "transport-io", "transport-streamable-http-server",
] }
tokio = { workspace = true }
tokio-util = "0.7"
axum = { workspace = true }
schemars = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
subtle = "2"          # constant-time token comparison

[dev-dependencies]
rmcp = { workspace = true, features = ["client", "transport-streamable-http-client"] }
reqwest = { version = "0.12", features = ["json"] }
```

- [x] **Step 2: Write the failing tool tests**

`mcp/tests/tools.rs` — in-process: start the HTTP server on an ephemeral port with a stub `EventSink`/`TaskReader`, connect with the rmcp client:
```rust
use std::sync::{Arc, Mutex};

use mcp::{DailyMcp, EventSink, TaskReader};
use shared::{Event, Task};

#[derive(Default)]
struct StubSink(Mutex<Vec<Event>>);
impl EventSink for StubSink {
    fn send_event(&self, event: Event) {
        self.0.lock().unwrap().push(event);
    }
}

struct StubReader(Vec<Task>);
impl TaskReader for StubReader {
    fn list_tasks(&self) -> Result<Vec<Task>, String> {
        Ok(self.0.clone())
    }
}

#[tokio::test]
async fn create_task_tool_dispatches_core_event_and_auth_is_enforced() {
    let sink = Arc::new(StubSink::default());
    let reader = Arc::new(StubReader(vec![Task { id: "t1".into(), title: "existing".into() }]));
    let daily = DailyMcp::new(reader, sink.clone());

    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (bound, server) = mcp::serve_http_on(daily, addr, "sekrit".into()).await.unwrap();
    tokio::spawn(server);

    // 1. No/wrong token → 401
    let resp = reqwest::Client::new()
        .post(format!("http://{bound}/mcp"))
        .json(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"ping"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // 2. With token: initialize + call create_task via the rmcp client
    let transport = rmcp::transport::StreamableHttpClientTransport::with_header(
        format!("http://{bound}/mcp"),
        ("Authorization", "Bearer sekrit"),
    );
    let client = ().serve(transport).await.unwrap(); // rmcp client handshake
    let result = client
        .call_tool(rmcp::model::CallToolRequestParam {
            name: "create_task".into(),
            arguments: Some(serde_json::json!({"title": "From MCP"}).as_object().unwrap().clone()),
        })
        .await
        .unwrap();
    assert!(!result.is_error.unwrap_or(false));

    let events = sink.0.lock().unwrap();
    assert!(matches!(&events[..], [Event::CreateTask { title }] if title == "From MCP"));
}
```

(rmcp client construction syntax moves between minors — `examples/clients` in the rust-sdk repo is the canonical reference; keep the three assertions: 401 without token, tool call succeeds with token, `CreateTask` event captured.)

- [x] **Step 3: Run to verify failure**

Run: `cargo nextest run -p mcp`
Expected: FAIL to compile — `DailyMcp`, `EventSink`, `TaskReader`, `serve_http_on` not defined.

- [x] **Step 4: Implement the server**

`mcp/src/lib.rs`:
```rust
mod auth;
mod server;

pub use server::{serve_http, serve_http_on, DailyMcp};

use shared::{Event, Task};

pub trait EventSink: Send + Sync {
    fn send_event(&self, event: Event);
}

pub trait TaskReader: Send + Sync {
    fn list_tasks(&self) -> Result<Vec<Task>, String>;
}
```

`mcp/src/server.rs`:
```rust
use std::{net::SocketAddr, sync::Arc};

use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

use crate::{auth, EventSink, TaskReader};

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct CreateTaskParams {
    /// Task title
    pub title: String,
}

#[derive(Clone)]
pub struct DailyMcp {
    reader: Arc<dyn TaskReader>,
    events: Arc<dyn EventSink>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl DailyMcp {
    pub fn new(reader: Arc<dyn TaskReader>, events: Arc<dyn EventSink>) -> Self {
        Self { reader, events, tool_router: Self::tool_router() }
    }

    #[tool(description = "Health check — returns 'pong'")]
    async fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text("pong")]))
    }

    #[tool(description = "Create a task in Daily's knowledge base")]
    async fn create_task(
        &self,
        Parameters(p): Parameters<CreateTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        if p.title.trim().is_empty() {
            return Err(McpError::invalid_params("title must be non-empty", None));
        }
        self.events.send_event(shared::Event::CreateTask { title: p.title.clone() });
        Ok(CallToolResult::success(vec![Content::text(format!(
            "created task: {}",
            p.title
        ))]))
    }

    #[tool(description = "List all tasks")]
    async fn list_tasks(&self) -> Result<CallToolResult, McpError> {
        let tasks = self
            .reader
            .list_tasks()
            .map_err(|e| McpError::internal_error(e, None))?;
        Ok(CallToolResult::success(vec![Content::json(&tasks)?]))
    }
}

#[tool_handler]
impl ServerHandler for DailyMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Daily knowledge base (walking skeleton): create_task and list_tasks.".into(),
            ),
            ..Default::default()
        }
    }
}

/// Bind + serve; returns the bound addr and the server future (tests use
/// port 0). `serve_http` is the production wrapper that just awaits it.
pub async fn serve_http_on(
    mcp: DailyMcp,
    addr: SocketAddr,
    token: String,
) -> anyhow::Result<(SocketAddr, impl std::future::Future<Output = anyhow::Result<()>>)> {
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    };

    let ct = tokio_util::sync::CancellationToken::new();
    let service = StreamableHttpService::new(
        move || Ok(mcp.clone()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );

    let router = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(token),
            auth::require_bearer_token,
        ));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    let fut = async move {
        axum::serve(listener, router).await?;
        Ok(())
    };
    Ok((bound, fut))
}

pub async fn serve_http(mcp: DailyMcp, addr: SocketAddr, token: String) -> anyhow::Result<()> {
    let (_, fut) = serve_http_on(mcp, addr, token).await?;
    fut.await
}
```

`mcp/src/auth.rs`:
```rust
use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;

pub async fn require_bearer_token(
    State(token): State<Arc<String>>,
    req: Request,
    next: Next,
) -> Response {
    let ok = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|t| t.as_bytes().ct_eq(token.as_bytes()).into());
    if ok {
        next.run(req).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
```

- [x] **Step 5: Run to verify green**

Run: `cargo nextest run -p mcp`
Expected: 1 test PASS (three assertions inside).

- [x] **Step 6: Commit**

```bash
git add mcp
git commit -m "feat(mcp): rmcp streamable-HTTP server — ping/create_task/list_tasks, bearer auth"
```

*(As-built notes: rmcp resolved to and pinned at `=2.1.0`; sketches above kept for history, code is the source of truth. Actual API deltas: client `from_uri`/`from_config` live behind the `transport-streamable-http-client-reqwest` dev-feature; the client sends the token via `StreamableHttpClientTransportConfig::with_uri(...).auth_header(token)` — no `with_header` constructor exists — and connects with `ClientInfo::new(ClientCapabilities::default(), Implementation::new(..)).serve(transport)`; tool calls take `CallToolRequestParams::new(name).with_arguments(obj)`. Server side: `ToolRouter` imports from `handler::server::router::tool`, content type is `ContentBlock` (not `Content`), `ServerInfo`/`InitializeResult` is `#[non_exhaustive]` so `get_info` uses `ServerInfo::new(caps).with_instructions(..)`, and the handler impl is `#[tool_handler(router = self.tool_router)]`. `serve_http_on` also refuses non-loopback bind addresses outright. Tests grew from the sketched one to four: 401 without/with-wrong token, create_task dispatches the event, list_tasks/ping round-trip, empty-title rejection dispatches nothing.)*

---

### Task 7: Embed the MCP server in the runtime

**Files:**
- Create: `runtime/src/mcp_glue.rs`, `runtime/tests/mcp_end_to_end.rs`
- Modify: `runtime/Cargo.toml`, `runtime/src/lib.rs`, `runtime/src/ffi.rs`

**Interfaces:**
- Consumes: `mcp::{DailyMcp, EventSink, TaskReader, serve_http_on}`, `runtime::AppRuntime`
- Produces:
  - `runtime::start_mcp(runtime: Arc<AppRuntime>, db_path: Option<PathBuf>, port: u16, token: String) -> anyhow::Result<u16>` — spawns a tokio runtime thread serving MCP; returns the bound port
  - `CoreFFI::start_mcp(&self, port: u16, token: String) -> u16` added to the FFI surface

- [x] **Step 1: Add deps**

`runtime/Cargo.toml` `[dependencies]` add: `mcp = { path = "../mcp" }`, `tokio = { workspace = true }`. `[dev-dependencies]` add: `reqwest = { version = "0.12", features = ["json"] }`, `rmcp = { workspace = true, features = ["client", "transport-streamable-http-client"] }`.

- [x] **Step 2: Write the failing end-to-end test**

`runtime/tests/mcp_end_to_end.rs`:
```rust
use std::sync::Arc;

use runtime::{AppRuntime, ShellCallback};
use shared::Event;

struct NullShell;
impl ShellCallback for NullShell {
    fn process_effects(&self, _: Vec<u8>) {}
}

/// The full loop the product depends on: MCP tool call → core event →
/// storage → view reflects it (what the GUI renders).
#[tokio::test]
async fn mcp_create_task_updates_core_view() {
    let rt = AppRuntime::new(None, Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup);

    let port = runtime::start_mcp(rt.clone(), None, 0, "sekrit".into()).unwrap();

    let transport = rmcp::transport::StreamableHttpClientTransport::with_header(
        format!("http://127.0.0.1:{port}/mcp"),
        ("Authorization", "Bearer sekrit"),
    );
    let client = ().serve(transport).await.unwrap();
    client
        .call_tool(rmcp::model::CallToolRequestParam {
            name: "create_task".into(),
            arguments: Some(
                serde_json::json!({"title": "Via MCP"}).as_object().unwrap().clone(),
            ),
        })
        .await
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if rt.view().tasks.iter().any(|t| t.title == "Via MCP") {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "MCP write never reached the view");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}
```

**Known limitation to encode here, not hide:** in Phase 0 `TaskReader` for MCP reads uses its own read-only store connection — with `None` (in-memory) DBs the MCP reader and the runtime's storage thread would see *different* databases. The test above therefore goes through `rt.view()` (core state), which is the product-relevant path. `start_mcp`'s reader for the in-memory case reads through the runtime view too (see Step 3). On-disk (production) both paths hit the same file.

- [x] **Step 3: Implement `mcp_glue.rs`**

```rust
use std::{path::PathBuf, sync::Arc};

use crate::AppRuntime;
use shared::Task;

struct RuntimeSink(Arc<AppRuntime>);
impl mcp::EventSink for RuntimeSink {
    fn send_event(&self, event: shared::Event) {
        self.0.send_event(event);
    }
}

/// Phase 0 reader: serve reads from the core's view (always consistent
/// with what the GUI shows). Phase 1 swaps this for a read-only SQLite
/// connection with richer queries.
struct ViewReader(Arc<AppRuntime>);
impl mcp::TaskReader for ViewReader {
    fn list_tasks(&self) -> Result<Vec<Task>, String> {
        Ok(self.0.view().tasks)
    }
}

pub fn start_mcp(
    runtime: Arc<AppRuntime>,
    _db_path: Option<PathBuf>,
    port: u16,
    token: String,
) -> anyhow::Result<u16> {
    let daily = mcp::DailyMcp::new(
        Arc::new(ViewReader(runtime.clone())),
        Arc::new(RuntimeSink(runtime)),
    );
    let (port_tx, port_rx) = std::sync::mpsc::channel();

    std::thread::Builder::new().name("daily-mcp".into()).spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let addr = format!("127.0.0.1:{port}").parse().unwrap();
            match mcp::serve_http_on(daily, addr, token).await {
                Ok((bound, fut)) => {
                    let _ = port_tx.send(Ok(bound.port()));
                    if let Err(e) = fut.await {
                        eprintln!("mcp server exited: {e}");
                    }
                }
                Err(e) => {
                    let _ = port_tx.send(Err(e));
                }
            }
        });
    })?;

    port_rx.recv()?
}
```

Add to `runtime/src/lib.rs`: `mod mcp_glue; pub use mcp_glue::start_mcp;`
Add to `CoreFFI` in `runtime/src/ffi.rs`:
```rust
    /// Returns the bound port (0 in = ephemeral). Token comes from the shell,
    /// which owns the token file (Task 8).
    pub fn start_mcp(&self, port: u16, token: String) -> u16 {
        crate::start_mcp(self.runtime.clone(), None, port, token).unwrap_or(0)
    }
```

- [x] **Step 4: Run to verify green**

Run: `cargo nextest run -p runtime`
Expected: 2 tests PASS (headless + mcp_end_to_end).

- [x] **Step 5: Commit**

```bash
git add runtime
git commit -m "feat(runtime): embedded MCP server — tool calls drive core events"
```

---

### Task 8: Apple shell — SwiftUI app over the FFI

**Files:**
- Create: `apple/project.yml`, `apple/Justfile`, `apple/Daily/DailyApp.swift`, `apple/Daily/Core.swift`, `apple/Daily/ContentView.swift`, `apple/Daily/Info.plist` entries via project.yml
- Modify: root `justfile`
- Test: manual E2E checklist (Step 6) — automated Swift tests arrive with Phase 1's UI work

**Interfaces:**
- Consumes: generated `Shared` (CoreFFI, CruxShell) and `App` (Event/ViewModel/Task/Requests) Swift packages; `CoreFFI.start_mcp(port:token:)`

- [ ] **Step 1: Write `apple/project.yml`**

```yaml
name: Daily
options:
  bundleIdPrefix: com.yardley
packages:
  Shared: { path: ./generated/Shared }
  App:    { path: ./generated/App }
targets:
  Daily:
    type: application
    platform: macOS
    deploymentTarget: "15.0"
    sources: [Daily]
    dependencies:
      - package: Shared
      - package: App
    settings:
      ENABLE_USER_SCRIPT_SANDBOXING: NO
      PRODUCT_NAME: Daily
    info:
      path: Daily/Info.plist
      properties:
        CFBundleDisplayName: Daily
        NSHumanReadableCopyright: ""
```

- [ ] **Step 2: Write the Core wrapper**

`apple/Daily/Core.swift` (push-model shell: Rust calls `processEffects`; mirror the counter-routing/weather Swift shells for the exact deserialization calls):
```swift
import Foundation
import Shared   // BoltFFI: CoreFFI, CruxShell protocol
import App      // typegen: Event, ViewModel, Task, Requests

@Observable @MainActor
final class Core {
    private(set) var view: ViewModel = ViewModel(tasks: [], count: 0, error: nil)
    private var ffi: CoreFFI!
    private(set) var mcpPort: UInt16 = 0

    init() {
        let dbURL = Self.appSupportURL().appendingPathComponent("daily.db")
        ffi = CoreFFI(dbPath: dbURL.path, shell: ShellHandler { [weak self] bytes in
            Task { @MainActor in self?.processEffects(bytes) }
        })
        mcpPort = ffi.startMcp(port: 52111, token: Self.loadOrCreateToken())
        send(.startup)
    }

    func send(_ event: Event) {
        ffi.update(event: Data(try! event.bincodeSerialize()))
    }

    private func processEffects(_ bytes: [UInt8]) {
        let requests = try! Requests.bincodeDeserialize(input: bytes).value
        for request in requests {
            switch request.effect {
            case .render:
                view = try! ViewModel.bincodeDeserialize(input: [UInt8](ffi.view()))
            default:
                assertionFailure("unhandled effect in Phase 0: \(request.effect)")
            }
        }
    }

    private static func appSupportURL() -> URL {
        let url = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Daily")
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    private static func loadOrCreateToken() -> String {
        let url = appSupportURL().appendingPathComponent("mcp-token")
        if let token = try? String(contentsOf: url, encoding: .utf8) { return token }
        let token = (0..<32).map { _ in String(format: "%02x", UInt8.random(in: 0...255)) }.joined()
        try? token.write(to: url, atomically: true, encoding: .utf8)
        try? FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
        return token
    }
}

/// Bridges the BoltFFI CruxShell protocol to a Swift closure.
final class ShellHandler: CruxShell {
    private let onEffects: ([UInt8]) -> Void
    init(_ onEffects: @escaping ([UInt8]) -> Void) { self.onEffects = onEffects }
    func processEffects(effects: [UInt8]) { onEffects(effects) }
}
```

(Names like `Requests`, `bincodeDeserialize`, `startMcp`, the `CruxShell` protocol spelling, and whether `Task` collides with Swift's `Task` — alias the generated one as `import struct App.Task` or rename in typegen config — all come from the generated packages; adjust to what `just generate` actually produced.)

- [ ] **Step 3: Write the UI**

`apple/Daily/ContentView.swift`:
```swift
import SwiftUI

struct ContentView: View {
    @Environment(Core.self) private var core
    @State private var draft = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                TextField("New task", text: $draft)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit(create)
                Button("Add", action: create)
                    .disabled(draft.trimmingCharacters(in: .whitespaces).isEmpty)
            }
            if let error = core.view.error {
                Text(error).foregroundStyle(.secondary)
            }
            List(core.view.tasks, id: \.id) { task in
                Text(task.title)
            }
            Text("\(core.view.count) tasks · MCP on 127.0.0.1:\(core.mcpPort)")
                .font(.caption).foregroundStyle(.secondary)
        }
        .padding(16)
        .frame(minWidth: 420, minHeight: 480)
    }

    private func create() {
        let title = draft.trimmingCharacters(in: .whitespaces)
        guard !title.isEmpty else { return }
        core.send(.createTask(title: title))
        draft = ""
    }
}
```

`apple/Daily/DailyApp.swift`:
```swift
import SwiftUI

@main
struct DailyApp: App {
    @State private var core = Core()

    var body: some Scene {
        WindowGroup("Daily") {
            ContentView().environment(core)
        }
    }
}
```

- [ ] **Step 4: Write `apple/Justfile` and root wiring**

`apple/Justfile`:
```make
generate:
    cd .. && just generate
    xcodegen

build: generate
    xcodebuild -project Daily.xcodeproj -scheme Daily -configuration Debug build

run: build
    open $(xcodebuild -project Daily.xcodeproj -scheme Daily -configuration Debug -showBuildSettings | awk '/BUILT_PRODUCTS_DIR/{d=$3} /FULL_PRODUCT_NAME/{n=$3} END{print d"/"n}')

dev: run
```

Append to root `justfile`:
```make
app:
    cd apple && just build
```

- [ ] **Step 5: Build until green**

Run: `cd apple && just build`
Expected: `BUILD SUCCEEDED`. Iterate on generated-name mismatches here (this step is where BoltFFI/typegen reality meets the plan — the Rust tests stay green throughout; only Swift-side names should need adjustment).

- [ ] **Step 6: Manual E2E acceptance checklist**

Run: `cd apple && just run`, then verify each:
1. Window opens; footer shows `0 tasks · MCP on 127.0.0.1:52111`.
2. Type "hello skeleton" + ⏎ → row appears instantly; footer says `1 tasks`.
3. Quit and relaunch → the task is still there (SQLite persistence).
4. `TOKEN=$(cat ~/Library/Application\ Support/Daily/mcp-token) && claude mcp add --transport http daily http://127.0.0.1:52111/mcp --header "Authorization: Bearer $TOKEN"`, then in a Claude Code session call the `create_task` tool with title "from claude" → **the row appears in the running app without any user action** (this is the money shot of Phase 0).
5. `curl -s -X POST http://127.0.0.1:52111/mcp -d '{}'` (no auth) → HTTP 401.

Record the outcome of each item in the commit message.

- [ ] **Step 7: Commit**

```bash
git add apple justfile
git commit -m "feat(apple): SwiftUI shell over BoltFFI — live task list, embedded MCP verified E2E"
```

---

### Task 9: Apple CI job and developer README

**Files:**
- Modify: `.github/workflows/ci.yml` (guardrails/pr-title exist from the SDLC setup; `rust` added in Task 1)
- Create: `README.md`

**Interfaces:**
- Consumes: `just` targets from Tasks 1/5/8.

- [ ] **Step 1: Add the `apple` job to CI**

Append to the `jobs:` map in `.github/workflows/ci.yml`:
```yaml
  apple:
    runs-on: macos-15
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { toolchain: "1.90" }
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@v2
        with: { tool: just }
      - run: cargo install boltffi_cli --version '=0.25.2' --locked
      - run: brew install xcodegen
      - run: cd apple && just build
```

- [ ] **Step 2: Write `README.md`**

```markdown
# Daily (codename Yardstick)

A calm todo + daily-notes app for macOS. Crux (Rust) core, SwiftUI shell,
SQLite via Rust-side effect handling, embedded MCP server for AI agents.

- Product/design spec: `docs/design/handoff/README.md`
- Architecture decisions: `docs/superpowers/specs/2026-07-02-daily-app-design.md`
- Current plan: `docs/superpowers/plans/2026-07-02-phase-0-walking-skeleton.md`

## Prerequisites

- Rust 1.90 (`rustup`), `cargo-nextest`, `just`
- `boltffi_cli` **=0.25.2** (`cargo install boltffi_cli --version '=0.25.2' --locked`)
- XcodeGen (`brew install xcodegen`), Xcode 16+

## Dev loop

    just test          # all Rust tests
    just generate      # typegen + BoltFFI Swift packages
    just app           # build the macOS app
    cd apple && just run

## MCP

The app serves MCP (streamable HTTP) on 127.0.0.1:52111.
Token: `~/Library/Application Support/Daily/mcp-token`.

    claude mcp add --transport http daily http://127.0.0.1:52111/mcp \
      --header "Authorization: Bearer $(cat ~/Library/Application\ Support/Daily/mcp-token)"
```

- [ ] **Step 3: Commit, open the PR, add the required check**

```bash
git add .github README.md
git commit -m "ci: apple build job + developer README"
git push -u origin p0/t9-apple-ci
gh pr create --fill
```
Verify the `apple` job passes on the PR. After Jon merges:
```bash
gh api -X POST repos/jonyardley/yardstick/branches/main/protection/required_status_checks/contexts --input - <<< '["apple"]'
```

---

## Self-review notes (run against spec §2/§3/§10 Phase 0)

- Spec coverage: workspace+pins (T1), pure core+storage effect (T2), SQLite/WAL/STRICT/space_id/soft-delete (T3), EffectRouter Rust-side storage (T4), typegen+BoltFFI (T5), MCP+auth (T6/T7), SwiftUI shell+live MCP→UI E2E (T8), CI (guardrails/pr-title from the SDLC setup, `rust` in T1, `apple` in T9). Phase 0's definition in spec §10 is fully covered.
- Deliberate deferrals (Phase 1+, per spec): FTS5 table, notes/blocks/pages/briefs schema, `Search` operation, MCP `get_day`/`write_brief`, menu bar, hotkey. The `search` FTS table ships with the notes schema in Phase 1 — nothing in Phase 0 queries it.
- Known soft spots called out inline: EffectRouter exact signatures (T4 Step 5 note), generated Swift names (T8 Step 2 note), rmcp client test syntax (T6 Step 2 note). Each names its canonical upstream example.

## After Phase 0

Phase 1 (shell + notes) gets its own plan once Jon has run the skeleton and the pinned-toolchain risks are retired. Roadmap: spec §10.
