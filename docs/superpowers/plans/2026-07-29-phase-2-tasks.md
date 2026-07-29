# Yardstick — Phase 2: Tasks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tasks become real. A captured item lands in Inbox with a source tag, a triage sheet sets when/priority/due from the keyboard, the Today column grows a Now section with the reference's task rows and momentum cue, every task carries one of six statuses (Blocked prompting for a reason), the sidebar's Views rows navigate to live bucket lists with live counts, and one All-actions view manages every task in the space with grouping, filtering, sorting, inline edit and multi-select bulk edits. Everything survives a relaunch.

**Architecture:** Unchanged from Phases 0–1 (spec §2). The pure core (`shared`) grows a task domain (`Bucket`, `Status`, `TaskData`) and per-surface ViewModel builders; `store` migration 003 widens the `tasks` table; the single `StorageOperation` enum gains three task operations and loses Phase 0's two; the SwiftUI shell gains task rows, a triage sheet, a status menu and the All-actions view. The core holds every non-deleted task for the space in `Model.tasks` and every list surface is a pure function over that vector — one query, no per-surface SQL (decision #3).

**Tech Stack:** Rust edition 2024 / crux_core 0.19 / boltffi =0.25.2 / facet =0.44 / rusqlite 0.39 (bundled) / rusqlite_migration 2.5 / rmcp =2.1.0 / axum 0.8 / tokio 1 / SwiftUI (macOS 15.0) / XcodeGen / just / cargo-nextest.

## Global Constraints

- **Pins (exact, never float silently):** `facet = "=0.44"`, `boltffi = "=0.25.2"` (+ `boltffi_cli =0.25.2`), `rmcp = "=2.1.0"`, `rusqlite = "0.39"` (bundled), `rusqlite_migration = "2.5"`, `crux_core = "0.19"`, toolchain `1.90`, macOS deployment target 15.0. CI installs with `--locked`. Pin changes require a spec amendment PR first.
- **No new external dependencies.** Phase 2 needs no new crates and no new Swift packages. Date arithmetic uses `shared/src/civil.rs` (Phase 1), never chrono.
- **Crate DAG (never violate):** `shared → crux_core` only; `store → shared`; `mcp → shared, store`; `runtime → shared, store, mcp`. `mcp` must NOT depend on `runtime`. No I/O, clocks, randomness, or tokio in `shared`. IDs (UUIDv7) are generated in `store`, never in `shared`. Every entity table keeps `space_id`, `created_at`, `updated_at`, `deleted_at`; new tables are STRICT.
- **The core is clock-free.** Today's date arrives from the shell via `Event::Startup { today }` (Phase 1 decision #6). Anything needing wall-clock time-of-day is either formatted in the shell or deferred to the Time capability in Phase 4.
- **Workflow (docs/SDLC.md):** each task runs on its own branch `p2/t<N>-<slug>` cut from latest `main`. TDD strictly: failing test → observe the failure → minimal implementation → observe the pass → commit. A task's final "Commit + PR" step means: push, open the PR (conventional title, template filled including **"Spec deltas introduced"**, TDD evidence pasted, this plan's checkboxes ticked in the same PR), then STOP — Jon reviews and squash-merges. Never merge your own PR. Run `just test` before claiming any Rust task done and `just app-test` before claiming any Swift task done.
- **Riders rule:** every task carries a **Riders** line naming which carried-forward items it absorbs (`none` explicitly otherwise). A rider may not be silently dropped; if it does not fit its task, move it in a plan-amendment commit inside the same PR.
- **Pixel-fidelity rule:** `docs/design/reference/v2-today-view.md` **§7 (Now section: header, momentum pips, task-row anatomy, all four row states)** and `docs/design/reference/core-journeys.md` **Journey 1 (Inbox + triage sheet) and Journey 5 (status menu, six statuses with verbatim descriptions)** are the acceptance criteria for this phase's UI, with the carve-outs below.
- **Verbatim microcopy** (core-journeys §Microcopy inventory — ship these strings exactly): "Captured today · unsorted", "Triage · {task}", "WHEN", "PRIORITY", "DUE", "Now", "Next", "Later", "Now" + "Today", "Next" + "This week", "{n} done · {n} to go", "Set status", "Backlog"/"Someday / unstarted", "In progress"/"Actively on it", "Blocked"/"Can't proceed", "Waiting"/"On someone else", "Done"/"Complete", "Binned"/"Dropped", "Backlog · {n}", "Binned · {n}".

### Carve-outs decided here (so no task ships dead UI)

Each one is a thing the reference shows that Phase 2 deliberately omits, with the phase that owns it:

1. **No project/person chips on task rows, and no `#` field in the triage sheet.** Chips and the linker need the `pages` table, which is Phase 3 (spec §3.1). `TaskRowVm.chips` exists and is empty in Phase 2, so rows render no chips — data-driven absence, exactly as Phase 1 handled empty sidebar sections. The triage sheet ships WHEN, PRIORITY and DUE only; its `#` row and the `@`/`#` pickers are Phase 3. **The `#` key is therefore not bound in Phase 2** — binding it to nothing would be worse than not binding it.
2. **No "Focusing" pill, no `F` key, no focus bar, no dimming.** Phase 4. `TaskRowVm.status_pill` covers Blocked / Waiting / In progress only.
3. **No completion time in the done row's meta column.** The reference's `10:15` needs a wall clock the core does not have; the row renders the reference's empty 70px spacer instead. Phase 4's Time capability adds it.
4. **No "Actions from yesterday" block, no Waiting-on brief rows, no resurfaced card, no collapsed Next/Later summaries in Today.** Phases 5 and 6. The Waiting-on *view* (sidebar row → list of tasks with status `waiting`) does ship, because it is pure task data.
5. **No red Inbox count badge in the title bar** (Journey 1A). The handoff's principle 4 forbids badge shouting and spec §7 resolved counts to the muted style; the sidebar Inbox count is the count. Recorded deviation from Journey 1A, consistent with Phase 1's sidebar treatment.
6. **No subtasks.** `parent_id` arrives with the Combine operations in Phase 5, so migration 003 does not add the column.
7. **No drag and drop.** Bucket changes happen through triage, the row menu, and bulk edit.

---

## Decisions made in this plan (so they aren't re-litigated mid-task)

1. **The single `StorageOperation` enum stays** — this is the Phase 1 decision #1 checkpoint, due at this gate. After Phase 2 the enum has **five** variants (`CreateTask`, `SaveTask`, `QueryTasks`, `GetDay`, `ReplaceDayBlocks`), against the stated split threshold of ~15, and no domain needs a second handler thread. Phase 0's `InsertTask`/`ListTasks` are **deleted** (delete-don't-pause), not kept alongside. Revisit again at the Phase 3 gate when `pages` arrives.

2. **Whole-row saves, not patches.** `SaveTask { task: TaskData }` upserts every mutable column from the core's own copy of the task. The core already holds the task, so patch semantics (and the `Option<Option<T>>` clearing problem across FFI typegen) buy nothing. Last-writer-wins at event granularity is the accepted concurrency model (spec §11). `created_at` and `id` are never written by a save; `updated_at` is set by the store.

3. **The core holds all tasks; every surface is a pure function.** One `QueryTasks` (no arguments) returns every non-deleted task for the space; `Model.tasks` holds them; the Now list, Inbox, bucket views, Waiting-on, All-actions groups and every sidebar count are pure functions over that vector. Justification: one user, hundreds of tasks, and it makes every surface a table-driven core test with no SQL. The alternative (per-surface filtered queries) adds a filter type, more executor SQL, and re-query orchestration on every write for no user-visible gain. Revisit if the task count ever reaches five figures, which for a personal tool it will not.

4. **Writes re-query.** Every successful `SaveTask`/`CreateTask` is followed by a `QueryTasks` so the model is a faithful mirror rather than a hand-patched copy that can drift from the database. The cost is one indexed read per write on a local SQLite file.

5. **Enums cross the FFI as Rust enums**, not strings: `Bucket` and `Status` derive `Facet` and are generated into Swift. Phase 1's stringly-typed `ViewRowVm.kind` stays as it is (it is a rendering hint, not domain state) — new domain state is typed.

6. **Civil dates for anything the core reasons about.** `entered_now_on` and `done_on` are `TEXT` `YYYY-MM-DD` civil dates set by the **core** from `model.today`, so age arithmetic is pure `civil.rs` subtraction. `created_at`/`updated_at` stay epoch integers set by the store (ordering only, never displayed).

7. **Routing lives in the core.** `Model.route` plus `Event::SelectView { kind }` decide which surface the main column shows; the ViewModel carries both `day: DayVm` and `list: TaskListVm` on every render, with `route: String` telling the shell which to draw. An enum-of-payloads ViewModel would be tidier in Rust and worse in Swift (every `switch` re-binding the payload); two always-present fields and a tag keep the shell dumb.

8. **Migration 003 uses `ALTER TABLE ADD COLUMN`, not a table rebuild.** STRICT tables accept added columns with defaults, but not added `CHECK` constraints, so bucket/status validity is enforced by the Rust enums that are the only writers. A rebuild would buy CHECK constraints on a database with a single writer and a handful of pre-release rows. Migration 003's test proves existing rows land as `inbox`/`backlog`.

9. **The All-actions view is a `List` with `Section`s, not a `Table`.** There is no mock for it (it arrives from the 2026-07-29 spec amendment, which explicitly delegates its shape to this plan). `List` gives sections, multi-selection and keyboard handling on macOS 15 without fighting `Table`'s column model, and grouping by status is the primary axis. Its acceptance criteria are written out in Task 9 rather than pointing at a reference file.

10. **Bulk edits are one event, not N events.** `BulkUpdateTasks { ids, bucket, priority, status }` saves each affected task inside one storage round trip, so a 30-row bulk edit is one re-query, not 30.

---

## File structure (locked decomposition)

```
store/
├── migrations/003_tasks.sql       # NEW (T1): widen tasks + indexes
└── src/executor.rs                # modified (T1): create_task/save_task/query_tasks
shared/src/
├── effects/storage.rs             # modified (T1): TaskData, CreateTask/SaveTask/QueryTasks
├── task.rs                        # NEW (T2): Bucket, Status, ordering + age helpers (pure)
├── app.rs                         # modified (T2): task events; (T3) route + list ViewModel
├── view/                          # NEW (T3): per-surface builders, split out of app.rs
│   ├── mod.rs                     #   pub(crate) use; build_view(model) -> ViewModel
│   ├── sidebar.rs                 #   live counts + Views rows
│   ├── task_row.rs                #   TaskRowVm formatting (age/due/source meta)
│   └── task_list.rs               #   Now / Inbox / bucket / waiting / all-actions groups
└── lib.rs                         # modified: export the new types
mcp/src/
├── server.rs                      # modified (T2): CreateTask -> CaptureTask{source:"mcp"}
└── reader.rs                      # modified (T2): QueryTasks
apple/Yardstick/
├── TaskRow.swift                  # NEW (T4): reference §7.2 row, all four states
├── TaskListView.swift             # NEW (T5): section header + momentum pips + rows
├── InboxView.swift                # NEW (T6): "Captured today · unsorted" + source tags
├── TriageSheet.swift              # NEW (T7): WHEN/PRIORITY/DUE + N/E/L, 1/2/3
├── StatusMenu.swift               # NEW (T8): six statuses + blocked-reason prompt
├── AllActionsView.swift           # NEW (T9): grouped, filtered, multi-select list
├── ContentView.swift              # modified (T5,T6,T9): route switch
├── SidebarView.swift              # modified (T6): Views rows become buttons
├── DayColumn.swift                # modified (T5): Now section under the note
├── Core.swift                     # modified (T6): selectView/triage/status senders
└── Theme.swift                    # modified (T4): row metrics, pip + checkbox tokens
apple/YardstickTests/
├── TaskRowFormattingTests.swift   # NEW (T4): pure Swift formatting helpers
└── TriageKeyboardTests.swift      # NEW (T7): key -> intent mapping
runtime/tests/
└── tasks_flow.rs                  # NEW (T10): capture -> triage -> done, persisted
```

`shared/src/app.rs` is 700+ lines after Phase 1 and would pass 1,200 with five new surfaces in it. Task 3 extracts the view builders into `shared/src/view/` (four focused files) and leaves `app.rs` owning the model and `update()`. That is the one structural change to existing code in this phase; it is a pure move plus the new builders, and the existing view tests keep passing throughout.

## Task overview

| # | Wave | Branch | PR title (conventional) | Riders absorbed |
|---|---|---|---|---|
| 1 | 0 | `p2/t1-task-schema` | `feat(store): migration 003 — task buckets, statuses, priority, due` | StorageOperation single-enum checkpoint (decision #1, recorded + applied) |
| 2 | 1 | `p2/t2-core-task-model` | `feat(core): task domain — capture, triage, status, done` | Phase 0 `Task {id,title}` replacement; MCP capture source |
| 3 | 2 | `p2/t3-view-builders` | `refactor(core): split view builders out of app.rs and add task surfaces` | none |
| 4 | 3 | `p2/t4-task-row` | `feat(apple): task row to reference §7.2 with all four states` | none |
| 10a | 3 | `p2/t10a-e2e-proofs` | `test(runtime): end-to-end task-flow proofs` | none |
| 5 | 4 | `p2/t5-now-section` | `feat(apple): Now section with momentum cue under the daily note` | none |
| 7 | 4 | `p2/t7-triage` | `feat(apple): triage sheet with N/E/L and 1/2/3 keyboard` | none |
| 8 | 4 | `p2/t8-status` | `feat(apple): six-status menu with blocked reason and untick restore` | none |
| 6 | 5 | `p2/t6-routing-inbox` | `feat(apple): sidebar Views navigate; Inbox with source tags` | none |
| 9 | 6 | `p2/t9-all-actions` | `feat(apple): All-actions view — group, filter, sort, bulk edit` | supersedes the status board (spec §6) |
| 10 | 7 | `p2/t10-phase-close` | `chore(p2): phase close — docs, review sweep, phase-gate dry run` | none |

Ordering rationale: the store and core land first because every Swift task consumes their generated types; Task 3's split happens before the Swift work so the surfaces exist to render; Task 4 gates the Swift lanes because it owns the row and the Theme tokens they all render; the All-actions view (T9) comes last of the UI because it reuses T4's row, T7's keyboard map and T8's status menu. Task numbers are stable — the wave column, not the number, sets the order.

---

## Execution plan (read this before dispatching any agent)

### Dependency graph

```
T1 ──► T2 ──► T3 ──┬──► T4 ──┬──► T5 ──► T6 ──► T9 ──► T10
                   │         ├──► T7 ────────────┤
                   │         └──► T8 ────────────┘
                   └──► T10a (Rust only; no Swift dependency)
```

### Waves

| Wave | Runs | Concurrency | Gate to the next wave |
|---|---|---|---|
| 0 | T1 | 1 | merged |
| 1 | T2 | 1 | merged |
| 2 | T3 | 1 | merged **and `just generate` clean** — every Swift task consumes its generated types |
| 3 | T4, T10a | 2 | T4 merged (T10a may still be in review; nothing depends on it) |
| 4 | T5, T7, T8 | 3 | all three merged |
| 5 | T6 | 1 | merged |
| 6 | T9 | 1 | merged |
| 7 | T10 | 1 | Jon tags `phase-2` |

Waves 0–2 are a hard serial spine: each defines the types the next consumes, and T2 and T3 both rewrite `app.rs`.

**T10a is Task 10's Steps 1 and 2 only** (`runtime/tests/tasks_flow.rs` and its run), lifted out to run as its own PR in wave 3 on branch `p2/t10a-e2e-proofs`. Those five proofs exercise core → router → store with no Swift at all, so they can land before any UI is built on them and act as the guard for every wave after. Task 10 proper then begins at its Step 3.

### Per-task model and effort

| Tasks | Model | Effort | Why |
|---|---|---|---|
| T1, T2, T3 | Opus 5 | high | Schema, domain design, the `app.rs` split — judgement where the plan meets real APIs |
| T4, T5, T6, T7, T8, T10a | Sonnet 5 | high | Transcription against complete code in this plan |
| T9, T10 | Opus 5 | high | The All-actions view has no mock; T10 is the phase-gate sweep |

Never below `high`: every task is test-first with an observable failure to reason about, and low effort is where "I'll skip the failing run" creeps in.

### Isolation

One git worktree per task, branched from latest `main`. Wave 3 and wave 4 run 2 and 3 worktrees concurrently; each gets its own `.xcodeproj` and DerivedData path, so Xcode builds do not collide. They do compete for CPU — expect each concurrent `just app-test` to take roughly twice as long as a solo run, which is still faster than three in series.

### Merge-contention rules for wave 4 (the only place three lanes touch the same files)

T5, T7 and T8 all add to `Core.swift` and `ContentView.swift`. File ownership plus a fixed insertion discipline makes the three merges conflict-free without inventing a scaffolding task:

| Lane | Owns outright | Adds to shared files |
|---|---|---|
| T5 | `TaskListView.swift`, `DayColumn.swift` | `ContentView`: the `DayColumn(...)` call site. `Core`: `// MARK: Capture` section |
| T7 | `TriageSheet.swift`, `shared/src/view/task_row.rs` | `ContentView`: **one** `.sheet` modifier, attached last in the chain. `Core`: `// MARK: Triage` section |
| T8 | `StatusMenu.swift` | `ContentView`: **one** `.sheet` modifier, attached above T7's. `Core`: `// MARK: Status` section |

Rules, in every wave-4 agent's prompt:
1. Add senders inside your own `// MARK: <Lane>` section at the **end** of `Core.swift`, never interleaved with another lane's.
2. Attach exactly one `.sheet` in `ContentView`, in the order given above, each on its own line.
3. Do not touch another lane's file, even to fix something — report it instead.
4. Rebase on `main` before pushing; if a wave-4 sibling landed first, re-run `just app-test` after the rebase and paste the second run in the PR.

### The prompt to hand each agent

```text
Read docs/SDLC.md, then docs/superpowers/plans/2026-07-29-phase-2-tasks.md.

Create a git worktree for this task, branched from latest main, then implement
Task <N> only — its steps exactly, in order: write the failing test, run it and
paste the failure, minimal implementation, run it and paste the pass, commit.

Open the PR with the template filled in (TDD evidence, spec deltas, plan
checkboxes ticked in the same PR) and STOP. Do not merge. Do not start another
task. If a step's API does not exist as written, follow the plan's named
arbiter for that step, mirror the canonical example, and record the deviation
in the PR description.

<for wave-4 lanes only, paste the four merge-contention rules from the plan's
Execution plan section, plus this lane's file-ownership row>
```

### Review batching

Waves 3 and 4 put two and three PRs in Jon's queue at once, reviewable in one sitting each. The critical path is seven review rounds instead of ten. If a wave-4 PR is rejected, its siblings are unaffected — that is the point of the file-ownership split.

---

### Task 1: Migration 003 and the task storage operations

**Files:**
- Create: `store/migrations/003_tasks.sql`
- Modify: `store/src/db.rs` (register 003), `store/src/executor.rs` (replace the two Phase 0 task arms), `shared/src/effects/storage.rs` (`TaskData` + three operations)
- Test: `store/src/executor.rs` (`mod tests`), `store/src/db.rs` (`mod tests`)

**Interfaces:**
- Produces, consumed by every later task:
  - `shared::TaskData { id: String, title: String, bucket: Bucket, status: Status, priority: u8, due: String, prev_status: Option<Status>, blocked_reason: String, source: String, entered_now_on: String, done_on: String, created_at: i64 }` — `priority` 0 means none; `due`, `blocked_reason`, `entered_now_on`, `done_on` use `""` for absent (one absent-representation across the FFI boundary; `Option<String>` would make every Swift call site unwrap for no gain).
  - `shared::{Bucket, Status}` — defined in Task 2's `shared/src/task.rs`. **Task 1 defines them inline in `shared/src/effects/storage.rs` and Task 2 moves them**, so Task 1 compiles alone.
  - `StorageOperation::CreateTask { title: String, source: String }` → `StorageResult::Task(TaskData)`
  - `StorageOperation::SaveTask { task: TaskData }` → `StorageResult::TaskSaved { id: String }`
  - `StorageOperation::QueryTasks` → `StorageResult::Tasks(Vec<TaskData>)`
- Consumes: `store::db::DEFAULT_SPACE_ID`, the Phase 1 `execute(conn, op)` shape.

**Riders:** the Phase 1 decision #1 checkpoint. Record in the PR description: five variants after this task, threshold ~15, one handler thread still sufficient → single enum retained; `InsertTask`/`ListTasks` deleted.

- [x] **Step 1: Write the failing migration test**

In `store/src/db.rs`'s `mod tests`, add:

```rust
#[test]
fn migration_003_widens_tasks_and_defaults_existing_rows() {
    // A database created before 003 (schema 002) with one task row in it.
    let dir = std::env::temp_dir().join(format!("ys-003-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t.db");
    {
        let mut conn = Connection::open(&path).unwrap();
        let up_to_002 = Migrations::new(vec![
            M::up(include_str!("../migrations/001_initial.sql")),
            M::up(include_str!("../migrations/002_notes.sql")),
        ]);
        up_to_002.to_latest(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO tasks (id, space_id, title, created_at, updated_at)
             VALUES ('t1', ?1, 'pre-existing', unixepoch(), unixepoch())",
            [DEFAULT_SPACE_ID],
        )
        .unwrap();
    }

    let conn = open(&path).unwrap();

    let (title, bucket, status, priority, due): (String, String, String, Option<i64>, Option<String>) =
        conn.query_row(
            "SELECT title, bucket, status, priority, due FROM tasks WHERE id = 't1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(title, "pre-existing", "the row survives the migration");
    assert_eq!(bucket, "inbox", "untriaged is the only honest default");
    assert_eq!(status, "backlog");
    assert_eq!(priority, None, "priority is optional (handoff §Task)");
    assert_eq!(due, None);

    std::fs::remove_dir_all(&dir).ok();
}
```

- [x] **Step 2: Run it to verify it fails**

Run: `cargo nextest run -p store migration_003`
Expected: FAIL — `no such column: bucket`.

- [x] **Step 3: Write migration 003**

Create `store/migrations/003_tasks.sql`:

```sql
-- Phase 2 widens tasks: bucket (when) and status (state) are orthogonal
-- (handoff §Task). Added, not rebuilt: STRICT tables accept ADD COLUMN with
-- a default but not added CHECK constraints, and the typed Rust enums in
-- shared/src/task.rs are the only writers (plan decision #8).
--
-- Absent values are NULL in the database and "" / 0 across the FFI boundary
-- (see shared::TaskData). Civil dates (entered_now_on, done_on) are set by
-- the clock-free core from Event::Startup's `today`; created_at/updated_at
-- stay epoch integers set here and are never displayed.
ALTER TABLE tasks ADD COLUMN bucket         TEXT NOT NULL DEFAULT 'inbox';
ALTER TABLE tasks ADD COLUMN status         TEXT NOT NULL DEFAULT 'backlog';
ALTER TABLE tasks ADD COLUMN priority       INTEGER;
ALTER TABLE tasks ADD COLUMN due            TEXT;
ALTER TABLE tasks ADD COLUMN prev_status    TEXT;
ALTER TABLE tasks ADD COLUMN blocked_reason TEXT;
ALTER TABLE tasks ADD COLUMN source         TEXT NOT NULL DEFAULT 'quick_add';
ALTER TABLE tasks ADD COLUMN entered_now_on TEXT;
ALTER TABLE tasks ADD COLUMN done_on        TEXT;

CREATE INDEX tasks_by_bucket ON tasks(space_id, bucket) WHERE deleted_at IS NULL;
CREATE INDEX tasks_by_status ON tasks(space_id, status) WHERE deleted_at IS NULL;
```

Register it in `store/src/db.rs`'s `MIGRATIONS`:

```rust
        M::up(include_str!("../migrations/003_tasks.sql")),
```

- [x] **Step 4: Run it to verify green**

Run: `cargo nextest run -p store`
Expected: PASS, including the pre-existing `migrations_are_valid` and `upgrading_a_real_001_database_lands_on_002_with_data_intact`.

- [x] **Step 5: Write the failing executor tests**

Replace the Phase 0 task tests (`insert_then_list_round_trips`, `list_is_oldest_first_and_ignores_soft_deleted`) in `store/src/executor.rs`'s `mod tests` with:

```rust
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
        let result = execute(&conn, &StorageOperation::SaveTask { task: saved.clone() });
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
```

Add to that module's imports: `use shared::{Bucket, Status, TaskData};`

- [x] **Step 6: Run them to verify they fail**

Run: `cargo nextest run -p store`
Expected: FAIL to compile — `no variant named CreateTask`, `cannot find type TaskData`.

- [x] **Step 7: Define the storage surface in `shared`**

In `shared/src/effects/storage.rs`, delete `pub struct Task`, `InsertTask`, `ListTasks`, `insert_task()`, `list_tasks()` and add:

```rust
/// When a task should happen. Orthogonal to [`Status`] (handoff §Task).
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Bucket {
    Inbox,
    Now,
    Next,
    Later,
}

/// What state a task is in. Orthogonal to [`Bucket`]; six states, one at a
/// time (core-journeys Journey 5).
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Status {
    Backlog,
    InProgress,
    Blocked,
    Waiting,
    Done,
    Binned,
}

/// One task, whole. Absent values are `""` / `0` across the FFI boundary and
/// NULL in the database (plan Task 1 interfaces).
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TaskData {
    pub id: String,
    pub title: String,
    pub bucket: Bucket,
    pub status: Status,
    /// 1, 2, 3 — or 0 for "no priority" (priority is optional).
    pub priority: u8,
    /// 'YYYY-MM-DD' or "".
    pub due: String,
    /// Restored when a done checkbox is unticked (spec §7).
    pub prev_status: Option<Status>,
    pub blocked_reason: String,
    /// Capture provenance: 'quick_add' | 'note' | 'menu_bar' | 'mcp'.
    pub source: String,
    /// Civil date the task entered the Now bucket — the age label's origin.
    pub entered_now_on: String,
    pub done_on: String,
    /// The store's clock; ordering only, never displayed.
    pub created_at: i64,
}
```

Extend the operation and result enums:

```rust
    // -- tasks (Phase 2) --
    /// Capture: the store generates the id and the timestamps; everything
    /// else takes its untriaged default (bucket=inbox, status=backlog).
    CreateTask {
        title: String,
        source: String,
    },
    /// Upsert every mutable column from the core's own copy (decision #2).
    /// Errors if the id is unknown — a save is never a disguised insert.
    SaveTask {
        task: TaskData,
    },
    /// Every non-deleted task in the space, oldest first (decision #3).
    QueryTasks,
```

```rust
    // -- tasks --
    Task(TaskData),
    Tasks(Vec<TaskData>),
    TaskSaved { id: String },
```

And the three request builders, replacing `insert_task`/`list_tasks`:

```rust
pub fn create_task<Effect, Event>(
    title: impl Into<String>,
    source: impl Into<String>,
) -> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::CreateTask {
        title: title.into(),
        source: source.into(),
    })
}

pub fn save_task<Effect, Event>(
    task: TaskData,
) -> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::SaveTask { task })
}

pub fn query_tasks<Effect, Event>()
-> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::QueryTasks)
}
```

Update `shared/src/lib.rs`'s re-export line to `pub use effects::storage::{BlockData, Bucket, DayData, Status, StorageOperation, StorageResult, TaskData};`.

`shared/src/app.rs` will not compile yet (it still references `Task`, `insert_task`, `list_tasks`). Task 1's Step 9 carries the minimal edit; Task 2 does the real work.

- [x] **Step 8: Implement the executor arms**

In `store/src/executor.rs`, replace the `InsertTask`/`ListTasks` arms with:

```rust
        StorageOperation::CreateTask { title, source } => create_task(conn, title, source),
        StorageOperation::SaveTask { task } => save_task(conn, task),
        StorageOperation::QueryTasks => query_tasks(conn),
```

and add the three functions:

```rust
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
```

Update the file's import line to `use shared::{BlockData, Bucket, DayData, Status, StorageOperation, StorageResult, TaskData};`.

- [x] **Step 9: Keep the workspace compiling**

`shared/src/app.rs`, `mcp/src/{server,reader}.rs` and `runtime/tests/*` still reference the deleted `Task`/`insert_task`/`list_tasks`. Task 2 rewrites them properly; this step does the minimum so the store tests can run:

- In `shared/src/app.rs`: change `Model.tasks` to `Vec<TaskData>`, `Event::CreateTask{title}`'s arm to `storage::create_task(title, "quick_add").then_send(Event::TaskSaved)`, `Event::TaskSaved`'s `StorageResult::Task(task)` arm unchanged (it pushes), `TasksLoaded`'s arm unchanged, and `Startup`'s `storage::list_tasks()` to `storage::query_tasks()`.
- In `mcp/src/reader.rs`: `StorageOperation::ListTasks` → `StorageOperation::QueryTasks`, and `TaskReader::list_tasks` returns `Vec<TaskData>`.
- In `mcp/src/lib.rs` and `mcp/tests/tools.rs`: `Task` → `TaskData` in imports and fixtures (the fixture builder is `sample()`-shaped; copy the one from Step 5).

- [x] **Step 10: Run the whole suite to verify green**

Run: `just test`
Expected: PASS — the five new executor tests, the new migration test, and every Phase 0/1 test.

Then: `cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --check`

- [x] **Step 11: Commit + PR**

```bash
git add store shared mcp
git commit -m "feat(store): migration 003 — task buckets, statuses, priority, due"
git push -u origin p2/t1-task-schema
gh pr create --fill
```

PR description must record: the decision #1 checkpoint result (5 variants, single enum retained), and the spec delta **none** (spec §3 already specifies this schema; `parent_id` deferred to Phase 5 is a plan carve-out, listed under carve-out 6).
STOP for review.

**Deviations recorded while implementing (plan amended in the Task 1 PR):**

1. **Step 4 does not pass as written.** Phase 1's `upgrading_a_real_001_database_lands_on_002_with_data_intact` asserts `user_version == 2`, which 003 makes 3. It is now `upgrading_a_real_001_database_lands_on_the_latest_schema_with_data_intact`, asserting 3 ("reopening must apply every later migration"). Its fixture row also had to move from `execute(&conn, InsertTask{..})` to raw SQL: the executor writes 003's columns, which do not exist on the 001-only database the test builds.
2. **Step 9's file list is incomplete.** Three more call sites reference the deleted Phase 0 surface and had to move for the workspace to compile: `store/src/db.rs` (two `InsertTask` uses in its tests → `CreateTask`), `runtime/tests/mcp_end_to_end.rs` (`ListTasks` → `QueryTasks`), and `apple/Yardstick/Core.swift` (`typealias YardstickTask = App.Task` → `App.TaskData`, one line — without it the `apple` CI job cannot compile the shell against the regenerated types).
3. **Task 1 therefore touches Swift** (that one typealias line) and runs `just app-test`, though it remains a Rust task. Confirmed green: 13 Swift tests, 0 failures.

---

### Task 2: Core task domain — capture, triage, status, done

**Files:**
- Create: `shared/src/task.rs`
- Modify: `shared/src/effects/storage.rs` (move `Bucket`/`Status` out), `shared/src/app.rs` (events, model, update arms), `shared/src/lib.rs`, `mcp/src/server.rs` (capture source `"mcp"`), `mcp/tests/tools.rs`
- Test: `shared/src/app.rs` (`mod tests`), `shared/src/task.rs` (`mod tests`), `mcp/tests/tools.rs`

**Interfaces:**
- Consumes: Task 1's `TaskData`, `Bucket`, `Status`, `storage::{create_task, save_task, query_tasks}`.
- Produces, consumed by Tasks 3–10:
  - `shared::Event::CaptureTask { title: String, source: String }` — replaces Phase 0's `CreateTask`.
  - `shared::Event::TriageTask { id: String, bucket: Bucket, priority: u8, due: String }`
  - `shared::Event::SetStatus { id: String, status: Status, reason: String }`
  - `shared::Event::ToggleDone { id: String }`
  - `shared::Event::EditTaskTitle { id: String, title: String }`
  - `shared::Event::BulkUpdateTasks { ids: Vec<String>, bucket: Option<Bucket>, priority: Option<u8>, status: Option<Status> }`
  - `shared::task::{sort_key, age_in_days, is_open}` — pure helpers used by Task 3's builders.

**Riders:** replace Phase 0's `Task {id, title}` everywhere (rider from the Phase 1 ledger); give MCP-captured tasks an honest source tag.

- [x] **Step 1: Write the failing pure-helper tests**

Create `shared/src/task.rs` containing only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::storage::{Bucket, Status, TaskData};

    fn task(priority: u8, entered: &str, status: Status) -> TaskData {
        TaskData {
            id: "id".into(),
            title: "t".into(),
            bucket: Bucket::Now,
            status,
            priority,
            due: String::new(),
            prev_status: None,
            blocked_reason: String::new(),
            source: "quick_add".into(),
            entered_now_on: entered.into(),
            done_on: String::new(),
            created_at: 0,
        }
    }

    #[test]
    fn open_covers_everything_except_done_and_binned() {
        assert!(is_open(Status::Backlog));
        assert!(is_open(Status::InProgress));
        assert!(is_open(Status::Blocked));
        assert!(is_open(Status::Waiting));
        assert!(!is_open(Status::Done));
        assert!(!is_open(Status::Binned));
    }

    #[test]
    fn age_counts_whole_days_from_entering_now() {
        assert_eq!(age_in_days("2026-07-02", "2026-07-04"), Some(2));
        assert_eq!(age_in_days("2026-07-04", "2026-07-04"), Some(0));
        assert_eq!(age_in_days("", "2026-07-04"), None, "never entered Now");
        assert_eq!(age_in_days("garbage", "2026-07-04"), None);
        assert_eq!(
            age_in_days("2026-07-05", "2026-07-04"),
            None,
            "a future origin is not negative age, it is no age"
        );
    }

    #[test]
    fn sort_is_priority_first_then_oldest_then_done_last() {
        // Spec §7: "strict P1-first, then age". Priority 0 (none) sorts after
        // 3, and done rows sink below every open row (reference §7.2 row 4).
        let mut rows = vec![
            task(0, "2026-07-01", Status::Backlog),
            task(2, "2026-07-03", Status::Backlog),
            task(1, "2026-07-03", Status::Backlog),
            task(2, "2026-07-01", Status::Backlog),
            task(1, "2026-07-02", Status::Done),
        ];
        rows.sort_by_key(|t| sort_key(t, "2026-07-04"));
        let shape: Vec<(u8, &str, bool)> = rows
            .iter()
            .map(|t| (t.priority, t.entered_now_on.as_str(), t.status == Status::Done))
            .collect();
        assert_eq!(
            shape,
            vec![
                (1, "2026-07-03", false),
                (2, "2026-07-01", false), // older beats newer at equal priority
                (2, "2026-07-03", false),
                (0, "2026-07-01", false), // no priority sorts last among open
                (1, "2026-07-02", true),  // done sinks regardless of priority
            ]
        );
    }
}
```

- [x] **Step 2: Run it to verify it fails**

Run: `cargo nextest run -p shared task::`
Expected: FAIL to compile — `cannot find function is_open` (the module has tests and no implementation).

- [x] **Step 3: Implement the pure helpers**

At the top of `shared/src/task.rs`:

```rust
//! Pure task-domain helpers. No I/O, no clocks: "today" is always a
//! parameter, supplied by the model from `Event::Startup`.

use crate::civil::CivilDate;
use crate::effects::storage::{Status, TaskData};

/// Open = not terminal. Done and Binned are the two terminal states
/// (core-journeys Journey 5: Binned is "dropped", distinct from Done).
#[must_use]
pub fn is_open(status: Status) -> bool {
    !matches!(status, Status::Done | Status::Binned)
}

/// Whole days between entering the Now bucket and `today`. `None` when the
/// task never entered Now, when the stored date is unparseable, or when it
/// is in the future — all three render as "no age label", never as a
/// negative or a zero that lies.
#[must_use]
pub fn age_in_days(entered_now_on: &str, today: &str) -> Option<i64> {
    let from = CivilDate::parse(entered_now_on)?;
    let to = CivilDate::parse(today)?;
    let days = to.days_since_epoch() - from.days_since_epoch();
    (days >= 0).then_some(days)
}

/// Ordering for every open list: done rows last, then priority 1→3 with
/// "no priority" after 3, then oldest first (spec §7).
#[must_use]
pub fn sort_key(task: &TaskData, today: &str) -> (bool, u8, i64) {
    let done_last = task.status == Status::Done;
    let priority_rank = if task.priority == 0 { 4 } else { task.priority };
    let age = age_in_days(&task.entered_now_on, today).unwrap_or(0);
    (done_last, priority_rank, -age)
}
```

`CivilDate::days_since_epoch` does not exist yet: `civil.rs` has the day-number arithmetic as a **private** `fn to_days(self) -> i64` (Hinnant's algorithm, already proved by the Phase 1 civil tests). Publish it in this task rather than duplicating it — in `shared/src/civil.rs`, rename `to_days` to `days_since_epoch`, make it `pub`, add `#[must_use]`, update its two internal callers (`weekday`, `add_days`), and add the driving test:

```rust
    #[test]
    fn days_since_epoch_is_the_day_number_used_for_ages() {
        assert_eq!(CivilDate::parse("1970-01-01").unwrap().days_since_epoch(), 0);
        assert_eq!(CivilDate::parse("2026-07-04").unwrap().days_since_epoch(), 20638);
        let a = CivilDate::parse("2026-02-28").unwrap().days_since_epoch();
        let b = CivilDate::parse("2026-03-01").unwrap().days_since_epoch();
        assert_eq!(b - a, 1, "2026 is not a leap year");
        let c = CivilDate::parse("2024-02-28").unwrap().days_since_epoch();
        let d = CivilDate::parse("2024-03-01").unwrap().days_since_epoch();
        assert_eq!(d - c, 2, "2024 is");
    }
```

If `20638` is wrong, take the number the failing assertion prints — the two leap-year deltas are the real invariants and the epoch anchor is `0`.

Declare the module in `shared/src/lib.rs` (`pub mod task;`) and re-export: `pub use task::{age_in_days, is_open, sort_key};`

- [x] **Step 4: Run it to verify green**

Run: `cargo nextest run -p shared task::`
Expected: PASS (3 tests).

- [x] **Step 5: Write the failing core write-path tests**

In `shared/src/app.rs`'s `mod tests`, add:

```rust
    fn task_fixture(id: &str, bucket: Bucket, status: Status) -> TaskData {
        TaskData {
            id: id.into(),
            title: "Finalize vendor contract".into(),
            bucket,
            status,
            priority: 0,
            due: String::new(),
            prev_status: None,
            blocked_reason: String::new(),
            source: "quick_add".into(),
            entered_now_on: String::new(),
            done_on: String::new(),
            created_at: 1,
        }
    }

    /// Puts one task into the model the way the runtime does: a query result.
    fn with_task(app: &Yardstick, model: &mut Model, task: TaskData) {
        let _ = app.update(
            Event::TasksLoaded(StorageResult::Tasks(vec![task])),
            model,
        );
    }

    #[test]
    fn capture_creates_an_inbox_task_carrying_its_source() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::CaptureTask {
                title: "Book dentist".into(),
                source: "quick_add".into(),
            },
            &mut model,
        );
        let op = cmd.expect_one_effect().expect_storage().operation;
        assert_eq!(
            op,
            StorageOperation::CreateTask {
                title: "Book dentist".into(),
                source: "quick_add".into(),
            }
        );
    }

    #[test]
    fn a_created_task_triggers_a_requery_not_a_hand_patched_model() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::TaskCreated(StorageResult::Task(task_fixture(
                "t1",
                Bucket::Inbox,
                Status::Backlog,
            ))),
            &mut model,
        );
        let ops: Vec<StorageOperation> = cmd
            .effects()
            .map(|e| e.expect_storage().operation)
            .collect();
        assert!(
            ops.contains(&StorageOperation::QueryTasks),
            "decision #4: writes re-query so the model cannot drift"
        );
    }

    #[test]
    fn triage_sets_bucket_priority_due_and_stamps_entering_now() {
        let (app, mut model) = started();
        with_task(&app, &mut model, task_fixture("t1", Bucket::Inbox, Status::Backlog));

        let mut cmd = app.update(
            Event::TriageTask {
                id: "t1".into(),
                bucket: Bucket::Now,
                priority: 1,
                due: "2026-07-31".into(),
            },
            &mut model,
        );
        let op = cmd.expect_one_effect().expect_storage().operation;
        let StorageOperation::SaveTask { task } = op else {
            panic!("expected SaveTask, got {op:?}")
        };
        assert_eq!(task.bucket, Bucket::Now);
        assert_eq!(task.priority, 1);
        assert_eq!(task.due, "2026-07-31");
        assert_eq!(
            task.entered_now_on, TODAY,
            "the age label's origin is stamped when the task enters Now"
        );
    }

    #[test]
    fn triage_out_of_now_and_back_does_not_reset_the_age() {
        let (app, mut model) = started();
        let mut aged = task_fixture("t1", Bucket::Now, Status::Backlog);
        aged.entered_now_on = "2026-07-01".into();
        with_task(&app, &mut model, aged);

        let mut cmd = app.update(
            Event::TriageTask {
                id: "t1".into(),
                bucket: Bucket::Now,
                priority: 2,
                due: String::new(),
            },
            &mut model,
        );
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(
            task.entered_now_on, "2026-07-01",
            "re-triaging within Now must not restart the age clock"
        );
    }

    #[test]
    fn toggle_done_remembers_the_previous_status_and_stamps_the_day() {
        let (app, mut model) = started();
        with_task(
            &app,
            &mut model,
            task_fixture("t1", Bucket::Now, Status::InProgress),
        );

        let mut cmd = app.update(Event::ToggleDone { id: "t1".into() }, &mut model);
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(task.status, Status::Done);
        assert_eq!(task.prev_status, Some(Status::InProgress));
        assert_eq!(task.done_on, TODAY);
    }

    #[test]
    fn unticking_restores_the_previous_status_and_clears_the_day() {
        let (app, mut model) = started();
        let mut done = task_fixture("t1", Bucket::Now, Status::Done);
        done.prev_status = Some(Status::Blocked);
        done.done_on = TODAY.into();
        with_task(&app, &mut model, done);

        let mut cmd = app.update(Event::ToggleDone { id: "t1".into() }, &mut model);
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(task.status, Status::Blocked, "spec §7: prev_status restores");
        assert_eq!(task.prev_status, None);
        assert_eq!(task.done_on, "");
    }

    #[test]
    fn unticking_a_task_with_no_remembered_status_falls_back_to_backlog() {
        let (app, mut model) = started();
        let mut done = task_fixture("t1", Bucket::Now, Status::Done);
        done.done_on = TODAY.into();
        with_task(&app, &mut model, done);

        let mut cmd = app.update(Event::ToggleDone { id: "t1".into() }, &mut model);
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(task.status, Status::Backlog, "spec §7's stated default");
    }

    #[test]
    fn setting_blocked_keeps_the_reason_and_clearing_the_status_drops_it() {
        let (app, mut model) = started();
        with_task(&app, &mut model, task_fixture("t1", Bucket::Now, Status::Backlog));

        let mut cmd = app.update(
            Event::SetStatus {
                id: "t1".into(),
                status: Status::Blocked,
                reason: "Legal review".into(),
            },
            &mut model,
        );
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(task.status, Status::Blocked);
        assert_eq!(task.blocked_reason, "Legal review");

        with_task(&app, &mut model, task);
        let mut cmd = app.update(
            Event::SetStatus {
                id: "t1".into(),
                status: Status::InProgress,
                reason: String::new(),
            },
            &mut model,
        );
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(
            task.blocked_reason, "",
            "a reason belongs to Blocked only — it must not linger"
        );
    }

    #[test]
    fn a_bulk_update_saves_every_selected_task_in_one_round_trip() {
        let (app, mut model) = started();
        let _ = app.update(
            Event::TasksLoaded(StorageResult::Tasks(vec![
                task_fixture("t1", Bucket::Inbox, Status::Backlog),
                task_fixture("t2", Bucket::Inbox, Status::Backlog),
                task_fixture("t3", Bucket::Inbox, Status::Backlog),
            ])),
            &mut model,
        );

        let mut cmd = app.update(
            Event::BulkUpdateTasks {
                ids: vec!["t1".into(), "t3".into()],
                bucket: Some(Bucket::Later),
                priority: Some(3),
                status: None,
            },
            &mut model,
        );
        let ops: Vec<StorageOperation> = cmd
            .effects()
            .map(|e| e.expect_storage().operation)
            .collect();
        assert_eq!(ops.len(), 2, "one save per selected task, no re-query yet");
        for op in ops {
            let StorageOperation::SaveTask { task } = op else {
                panic!("expected SaveTask")
            };
            assert!(task.id == "t1" || task.id == "t3", "unselected must not move");
            assert_eq!(task.bucket, Bucket::Later);
            assert_eq!(task.priority, 3);
            assert_eq!(task.status, Status::Backlog, "None means leave it alone");
        }
    }

    #[test]
    fn editing_an_unknown_task_surfaces_an_error_and_saves_nothing() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::EditTaskTitle {
                id: "ghost".into(),
                title: "nope".into(),
            },
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        assert!(
            app.view(&model).error.is_some(),
            "a missing task is a visible failure, never a silent no-op"
        );
    }
```

Add to the test module's imports: `use crate::effects::storage::{Bucket, Status, TaskData};`

- [x] **Step 6: Run them to verify they fail**

Run: `cargo nextest run -p shared app::`
Expected: FAIL to compile — `no variant named CaptureTask`, `TriageTask`, `SetStatus`, `ToggleDone`, `EditTaskTitle`, `BulkUpdateTasks`, `TaskCreated`.

- [x] **Step 7: Move the domain types and add the events**

Move `Bucket` and `Status` from `shared/src/effects/storage.rs` into `shared/src/task.rs` (keep `TaskData` in `storage.rs` — it is the storage payload) and have `storage.rs` `use crate::task::{Bucket, Status};`. Re-export both from `lib.rs` as before, so no downstream import changes.

In `shared/src/app.rs`, replace `Event::CreateTask`/`TaskSaved`/`TasksLoaded` with:

```rust
    // -- tasks (Phase 2) --
    CaptureTask { title: String, source: String },
    TriageTask { id: String, bucket: Bucket, priority: u8, due: String },
    SetStatus { id: String, status: Status, reason: String },
    ToggleDone { id: String },
    EditTaskTitle { id: String, title: String },
    BulkUpdateTasks {
        ids: Vec<String>,
        bucket: Option<Bucket>,
        priority: Option<u8>,
        status: Option<Status>,
    },
    TaskCreated(StorageResult),
    TaskSaved(StorageResult),
    TasksLoaded(StorageResult),
```

- [x] **Step 8: Implement the update arms**

Add above `impl App for Yardstick`:

```rust
/// Find a task by id, or set a visible error. Every task write goes through
/// this — a write against an id the model has never seen is a bug worth
/// surfacing, not a silent no-op.
fn task_mut<'m>(model: &'m mut Model, id: &str, handler: &str) -> Option<&'m mut TaskData> {
    let found = model.tasks.iter().position(|t| t.id == id);
    match found {
        Some(i) => Some(&mut model.tasks[i]),
        None => {
            model.error = Some(format!("internal: {handler} for unknown task {id}"));
            None
        }
    }
}
```

and the arms inside `update()`:

```rust
            Event::CaptureTask { title, source } => {
                storage::create_task(title, source).then_send(Event::TaskCreated)
            }
            Event::TriageTask {
                id,
                bucket,
                priority,
                due,
            } => {
                let today = model.today.clone();
                let Some(task) = task_mut(model, &id, "TriageTask") else {
                    return render();
                };
                // Entering Now starts the age clock; staying in Now keeps it.
                if bucket == Bucket::Now && task.entered_now_on.is_empty() {
                    task.entered_now_on = today;
                } else if bucket != Bucket::Now {
                    task.entered_now_on = String::new();
                }
                task.bucket = bucket;
                task.priority = priority;
                task.due = due;
                storage::save_task(task.clone()).then_send(Event::TaskSaved)
            }
            Event::SetStatus { id, status, reason } => {
                let today = model.today.clone();
                let Some(task) = task_mut(model, &id, "SetStatus") else {
                    return render();
                };
                apply_status(task, status, reason, &today);
                storage::save_task(task.clone()).then_send(Event::TaskSaved)
            }
            Event::ToggleDone { id } => {
                let today = model.today.clone();
                let Some(task) = task_mut(model, &id, "ToggleDone") else {
                    return render();
                };
                if task.status == Status::Done {
                    // Spec §7: unticking restores prev_status, default backlog.
                    let restored = task.prev_status.take().unwrap_or(Status::Backlog);
                    apply_status(task, restored, String::new(), &today);
                } else {
                    let previous = task.status;
                    apply_status(task, Status::Done, String::new(), &today);
                    task.prev_status = Some(previous);
                }
                storage::save_task(task.clone()).then_send(Event::TaskSaved)
            }
            Event::EditTaskTitle { id, title } => {
                let Some(task) = task_mut(model, &id, "EditTaskTitle") else {
                    return render();
                };
                task.title = title;
                storage::save_task(task.clone()).then_send(Event::TaskSaved)
            }
            Event::BulkUpdateTasks {
                ids,
                bucket,
                priority,
                status,
            } => {
                let today = model.today.clone();
                let mut saves = Vec::new();
                for id in ids {
                    let Some(task) = task_mut(model, &id, "BulkUpdateTasks") else {
                        continue;
                    };
                    if let Some(bucket) = bucket {
                        if bucket == Bucket::Now && task.entered_now_on.is_empty() {
                            task.entered_now_on = today.clone();
                        } else if bucket != Bucket::Now {
                            task.entered_now_on = String::new();
                        }
                        task.bucket = bucket;
                    }
                    if let Some(priority) = priority {
                        task.priority = priority;
                    }
                    if let Some(status) = status {
                        apply_status(task, status, String::new(), &today);
                    }
                    saves.push(storage::save_task(task.clone()).then_send(Event::TaskSaved));
                }
                Command::all(saves)
            }
            Event::TaskCreated(result) | Event::TaskSaved(result) => match result {
                StorageResult::Task(_) | StorageResult::TaskSaved { .. } => {
                    model.error = None;
                    // Decision #4: re-query rather than patch the model.
                    Command::all([render(), storage::query_tasks().then_send(Event::TasksLoaded)])
                }
                StorageResult::Error(e) => {
                    model.error = Some(e);
                    render()
                }
                other => wrong_shape(model, "TaskSaved", &other),
            },
            Event::TasksLoaded(result) => match result {
                StorageResult::Tasks(tasks) => {
                    model.error = None;
                    model.tasks = tasks;
                    render()
                }
                StorageResult::Error(e) => {
                    model.error = Some(e);
                    render()
                }
                other => wrong_shape(model, "TasksLoaded", &other),
            },
```

and the one shared status transition, next to `task_mut`:

```rust
/// The single place a status changes, so the fields that hang off a status
/// can never drift from it: a reason belongs to Blocked, a done day belongs
/// to Done.
fn apply_status(task: &mut TaskData, status: Status, reason: String, today: &str) {
    task.status = status;
    task.blocked_reason = if status == Status::Blocked { reason } else { String::new() };
    task.done_on = if status == Status::Done {
        today.to_owned()
    } else {
        String::new()
    };
}
```

Update `Startup` to request `storage::query_tasks()`, and remove the Phase 0 `Event::CreateTask` arm.

- [x] **Step 9: Run them to verify green**

Run: `cargo nextest run -p shared`
Expected: PASS — the eleven new write-path tests plus every Phase 1 day/navigation test.

- [x] **Step 10: Point MCP at the new event and source tag**

In `mcp/src/server.rs`'s `create_task` tool, send:

```rust
        self.events.send_event(shared::Event::CaptureTask {
            title: p.title.clone(),
            source: "mcp".into(),
        });
```

In `mcp/tests/tools.rs`, update the assertion that inspects the recorded event to expect `Event::CaptureTask { title, source }` with `source == "mcp"`, and add:

```rust
#[tokio::test]
async fn mcp_captured_tasks_are_tagged_as_coming_from_an_agent() {
    // Provenance is a product requirement (core-journeys Journey 1A: every
    // capture carries a source tag), so an agent's writes must be
    // distinguishable from the user's in the Inbox.
    let (client, sink) = super::helpers::connected_client().await;
    client
        .call_tool(CallToolRequestParams {
            name: "create_task".into(),
            arguments: Some(serde_json::json!({ "title": "from an agent" }).as_object().unwrap().clone()),
        })
        .await
        .unwrap();
    let events = sink.events();
    assert!(matches!(
        &events[0],
        shared::Event::CaptureTask { source, .. } if source == "mcp"
    ));
}
```

Use whatever the file's existing client-fixture helper is named (Phase 1 Task 2 consolidated it into `mcp::test_support`); mirror the neighbouring test's setup exactly rather than inventing a helper.

- [x] **Step 11: Run the whole suite**

Run: `just test && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --check`
Expected: all PASS. `runtime` tests compile because they only use `Event::` names that still exist plus the ones renamed here — fix any that reference `Event::CreateTask` by switching to `CaptureTask { title, source: "quick_add".into() }`.

- [x] **Step 12: Commit + PR**

```bash
git add shared mcp runtime
git commit -m "feat(core): task domain — capture, triage, status, done"
git push -u origin p2/t2-core-task-model
gh pr create --fill   # spec-deltas: none
```
STOP for review.

---

### Task 3: View builders — split `app.rs` and add the task surfaces

**Files:**
- Create: `shared/src/view/mod.rs`, `shared/src/view/sidebar.rs`, `shared/src/view/task_row.rs`, `shared/src/view/task_list.rs`
- Modify: `shared/src/app.rs` (delete the three `build_*` functions, add `route`/grouping/filter state and their events, `view()` delegates), `shared/src/civil.rs` (`WEEKDAY_ABBREV` + `weekday_short`), `shared/src/lib.rs`
- Test: `shared/src/view/task_row.rs`, `shared/src/view/task_list.rs`, `shared/src/view/sidebar.rs` (each has its own `mod tests`), `shared/src/civil.rs`

**Interfaces:**
- Consumes: Task 2's `Model.tasks`, `task::{sort_key, age_in_days, is_open}`, `CivilDate`.
- Produces, consumed by Tasks 4–10:
  - `ViewModel { sidebar, calendar, route: String, day: DayVm, list: TaskListVm, error }` — `route` is one of `"today" | "now" | "next" | "later" | "waiting" | "inbox" | "all"`.
  - `TaskRowVm { id, title, checkbox: String, priority: u8, status_pill: String, status_kind: String, chips: Vec<String>, meta: String, is_done: bool, blocked_reason: String }` — `checkbox` is `"open" | "in_progress" | "done"`.
  - `TaskListVm { title, subtitle, groups: Vec<TaskGroupVm>, momentum: Option<MomentumVm>, collapsed: Vec<CollapsedGroupVm>, group_by: String, filter_bucket: String, filter_status: String }`
  - `TaskGroupVm { label: String, kind: String, count: u64, rows: Vec<TaskRowVm> }`
  - `MomentumVm { done: u64, remaining: u64, label: String }`
  - `CollapsedGroupVm { label: String, count: u64 }`
  - `Event::SelectView { kind: String }`, `Event::SetGrouping { group_by: String }`, `Event::SetFilter { bucket: String, status: String }`
- `DayVm`, `SidebarVm`, `CalendarVm` and their fields are unchanged, so Phase 1's Swift chrome keeps compiling.

**Riders:** none.

- [x] **Step 1: Write the failing row-formatting tests**

Create `shared/src/view/task_row.rs` with its test module only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Bucket, Status};

    fn task(bucket: Bucket, status: Status) -> TaskData {
        TaskData {
            id: "t1".into(),
            title: "Give probation feedback on Thabang to Pieter".into(),
            bucket,
            status,
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

    const TODAY: &str = "2026-07-04";

    #[test]
    fn checkbox_follows_status_not_bucket() {
        assert_eq!(build_row(&task(Bucket::Now, Status::Backlog), TODAY).checkbox, "open");
        assert_eq!(
            build_row(&task(Bucket::Now, Status::InProgress), TODAY).checkbox,
            "in_progress",
            "reference §7.2 row 1: blue ring with a soft filled centre"
        );
        assert_eq!(build_row(&task(Bucket::Now, Status::Done), TODAY).checkbox, "done");
        assert_eq!(
            build_row(&task(Bucket::Now, Status::Blocked), TODAY).checkbox,
            "open",
            "blocked is carried by the pill, not by the checkbox"
        );
    }

    #[test]
    fn done_rows_are_flagged_for_dimming_and_strikethrough() {
        let row = build_row(&task(Bucket::Now, Status::Done), TODAY);
        assert!(row.is_done);
        assert_eq!(row.status_pill, "", "a struck-through row needs no Done pill");
    }

    #[test]
    fn notable_statuses_get_a_pill_and_ordinary_ones_do_not() {
        assert_eq!(build_row(&task(Bucket::Now, Status::Backlog), TODAY).status_pill, "");
        assert_eq!(
            build_row(&task(Bucket::Now, Status::InProgress), TODAY).status_pill,
            "In progress"
        );
        assert_eq!(build_row(&task(Bucket::Now, Status::Blocked), TODAY).status_pill, "Blocked");
        assert_eq!(build_row(&task(Bucket::Now, Status::Waiting), TODAY).status_pill, "Waiting");
        assert_eq!(build_row(&task(Bucket::Now, Status::Binned), TODAY).status_pill, "Binned");
    }

    #[test]
    fn a_blocked_row_carries_its_reason_for_the_board_card() {
        let mut blocked = task(Bucket::Next, Status::Blocked);
        blocked.blocked_reason = "Legal review".into();
        assert_eq!(build_row(&blocked, TODAY).blocked_reason, "Legal review");
    }

    #[test]
    fn now_rows_show_age_once_a_day_has_passed_and_never_zero_days() {
        let mut aged = task(Bucket::Now, Status::Backlog);
        aged.entered_now_on = "2026-07-02".into();
        assert_eq!(build_row(&aged, TODAY).meta, "2 days old");

        aged.entered_now_on = "2026-07-03".into();
        assert_eq!(build_row(&aged, TODAY).meta, "1 day old", "singular, not '1 days'");

        aged.entered_now_on = TODAY.into();
        assert_eq!(
            build_row(&aged, TODAY).meta, "",
            "'0 days old' is noise; today's arrivals say nothing"
        );
    }

    #[test]
    fn a_now_row_with_no_age_falls_back_to_provenance() {
        let mut fresh = task(Bucket::Now, Status::Backlog);
        fresh.entered_now_on = TODAY.into();
        fresh.source = "note".into();
        assert_eq!(
            build_row(&fresh, TODAY).meta, "from note",
            "reference §7.2 row 2 shows provenance where there is no age"
        );
    }

    #[test]
    fn inbox_rows_always_show_their_source_tag() {
        // core-journeys Journey 1A: every capture carries a visible source.
        for (source, expected) in [
            ("quick_add", "quick add"),
            ("note", "from note"),
            ("menu_bar", "menu bar"),
            ("mcp", "from an agent"),
            ("something_new", "something_new"),
        ] {
            let mut t = task(Bucket::Inbox, Status::Backlog);
            t.source = source.into();
            assert_eq!(build_row(&t, TODAY).meta, expected, "source {source}");
        }
    }

    #[test]
    fn dated_rows_outside_now_show_the_due_weekday() {
        let mut t = task(Bucket::Next, Status::Backlog);
        t.due = "2026-07-31".into();
        assert_eq!(
            build_row(&t, TODAY).meta, "Fri",
            "Journey 1C: due dates render as an abbreviated weekday in list rows"
        );
    }

    #[test]
    fn chips_are_empty_until_pages_exist() {
        // Phase 2 carve-out 1: no project/person chips until Phase 3 —
        // the field exists so rows render none, rather than faking any.
        assert!(build_row(&task(Bucket::Now, Status::Backlog), TODAY).chips.is_empty());
    }
}
```

- [x] **Step 2: Run it to verify it fails**

Run: `cargo nextest run -p shared view::`
Expected: FAIL to compile — `cannot find function build_row`.

- [x] **Step 3: Implement the row builder and the weekday abbreviation**

In `shared/src/civil.rs` add, beside `WEEKDAY_NAMES`:

```rust
pub const WEEKDAY_ABBREV: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
```

and on `CivilDate`:

```rust
    /// "Fri" — the due chip in list rows (core-journeys Journey 1C).
    #[must_use]
    pub fn weekday_short(&self) -> String {
        WEEKDAY_ABBREV[self.weekday() as usize].to_owned()
    }
```

with the driving test in that file's `mod tests`:

```rust
    #[test]
    fn weekday_short_matches_known_dates() {
        assert_eq!(CivilDate::parse("2026-07-31").unwrap().weekday_short(), "Fri");
        assert_eq!(CivilDate::parse("2026-07-04").unwrap().weekday_short(), "Sat");
        assert_eq!(CivilDate::parse("2026-07-05").unwrap().weekday_short(), "Sun");
    }
```

Then `shared/src/view/task_row.rs`:

```rust
//! `TaskData` → `TaskRowVm`: every display string a row needs, computed
//! once in the core (spec §4). The shell renders; it never formats domain
//! data. The one exception, recorded: wall-clock times of day (Phase 4).

use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::civil::CivilDate;
use crate::effects::storage::TaskData;
use crate::task::{Bucket, Status, age_in_days};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskRowVm {
    pub id: String,
    pub title: String,
    /// "open" | "in_progress" | "done" (reference §7.2 checkbox states).
    pub checkbox: String,
    /// 1–3, or 0 for no badge.
    pub priority: u8,
    /// "" when the status is unremarkable; otherwise the pill's text.
    pub status_pill: String,
    /// Which tint the pill takes: "in_progress" | "blocked" | "waiting" | "binned".
    pub status_kind: String,
    /// Project/person chips — empty until Phase 3 (carve-out 1).
    pub chips: Vec<String>,
    /// The 70px right column: age, provenance, or due weekday.
    pub meta: String,
    pub is_done: bool,
    pub blocked_reason: String,
}

/// Source tag copy (core-journeys Journey 1A). An unknown source shows its
/// raw value rather than being swallowed — a new capture point should be
/// visible the moment it starts writing, not silently unlabelled.
fn source_label(source: &str) -> String {
    match source {
        "quick_add" => "quick add".into(),
        "note" => "from note".into(),
        "menu_bar" => "menu bar".into(),
        "mcp" => "from an agent".into(),
        other => other.into(),
    }
}

fn age_label(days: i64) -> String {
    if days == 1 {
        "1 day old".into()
    } else {
        format!("{days} days old")
    }
}

/// The 70px meta column, one rule per bucket:
/// - Inbox: always the source tag (nothing else is known yet).
/// - Now: age once a day has passed, else provenance.
/// - Next/Later: the due weekday, else nothing.
fn meta(task: &TaskData, today: &str) -> String {
    match task.bucket {
        Bucket::Inbox => source_label(&task.source),
        Bucket::Now => match age_in_days(&task.entered_now_on, today) {
            Some(days) if days >= 1 => age_label(days),
            _ => source_label(&task.source),
        },
        Bucket::Next | Bucket::Later => CivilDate::parse(&task.due)
            .map(|d| d.weekday_short())
            .unwrap_or_default(),
    }
}

#[must_use]
pub fn build_row(task: &TaskData, today: &str) -> TaskRowVm {
    let (pill, kind) = match task.status {
        Status::InProgress => ("In progress", "in_progress"),
        Status::Blocked => ("Blocked", "blocked"),
        Status::Waiting => ("Waiting", "waiting"),
        Status::Binned => ("Binned", "binned"),
        Status::Backlog | Status::Done => ("", ""),
    };
    TaskRowVm {
        id: task.id.clone(),
        title: task.title.clone(),
        checkbox: match task.status {
            Status::Done => "done".into(),
            Status::InProgress => "in_progress".into(),
            _ => "open".into(),
        },
        priority: task.priority,
        status_pill: pill.into(),
        status_kind: kind.into(),
        chips: Vec::new(),
        meta: meta(task, today),
        is_done: task.status == Status::Done,
        blocked_reason: task.blocked_reason.clone(),
    }
}
```

Note the `Bucket::Now` fallback ordering: a Now row that arrived today shows provenance, which is exactly reference §7.2 row 2 ("from Slack") sitting beside row 1's "2 days old".

- [x] **Step 4: Run it to verify green**

Run: `cargo nextest run -p shared view::task_row`
Expected: PASS (9 tests).

- [x] **Step 5: Write the failing list-builder tests**

Create `shared/src/view/task_list.rs` with its test module only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::storage::TaskData;
    use crate::task::{Bucket, Status};

    const TODAY: &str = "2026-07-04";

    fn t(id: &str, bucket: Bucket, status: Status, priority: u8) -> TaskData {
        TaskData {
            id: id.into(),
            title: format!("task {id}"),
            bucket,
            status,
            priority,
            due: String::new(),
            prev_status: None,
            blocked_reason: String::new(),
            source: "quick_add".into(),
            entered_now_on: String::new(),
            done_on: String::new(),
            created_at: 0,
        }
    }

    fn ids(group: &TaskGroupVm) -> Vec<&str> {
        group.rows.iter().map(|r| r.id.as_str()).collect()
    }

    #[test]
    fn the_now_list_is_titled_from_the_reference_and_orders_by_priority_then_age() {
        let tasks = vec![
            t("none", Bucket::Now, Status::Backlog, 0),
            t("p2", Bucket::Now, Status::Backlog, 2),
            t("p1", Bucket::Now, Status::Backlog, 1),
            t("elsewhere", Bucket::Next, Status::Backlog, 1),
        ];
        let list = build_list("now", &tasks, TODAY, "status", "", "");
        assert_eq!(list.title, "Now");
        assert_eq!(list.subtitle, "Today", "reference §7.1");
        assert_eq!(list.groups.len(), 1);
        assert_eq!(ids(&list.groups[0]), vec!["p1", "p2", "none"]);
    }

    #[test]
    fn done_now_tasks_stay_visible_at_the_bottom_and_feed_the_momentum_cue() {
        // Principle 6: done stays visible for the rest of the day.
        let tasks = vec![
            t("open1", Bucket::Now, Status::Backlog, 0),
            t("done1", Bucket::Now, Status::Done, 1),
            t("open2", Bucket::Now, Status::Backlog, 0),
            t("open3", Bucket::Now, Status::Backlog, 0),
        ];
        let list = build_list("now", &tasks, TODAY, "status", "", "");
        assert_eq!(ids(&list.groups[0]).last(), Some(&"done1"));
        let momentum = list.momentum.expect("the Now list has a momentum cue");
        assert_eq!(momentum.done, 1);
        assert_eq!(momentum.remaining, 3);
        assert_eq!(momentum.label, "1 done · 3 to go", "reference §7.1 verbatim");
    }

    #[test]
    fn only_the_now_list_has_a_momentum_cue() {
        let tasks = vec![t("a", Bucket::Inbox, Status::Backlog, 0)];
        assert!(build_list("inbox", &tasks, TODAY, "status", "", "").momentum.is_none());
        assert!(build_list("all", &tasks, TODAY, "status", "", "").momentum.is_none());
    }

    #[test]
    fn the_inbox_says_captured_today_unsorted_and_holds_only_untriaged_tasks() {
        let tasks = vec![
            t("in1", Bucket::Inbox, Status::Backlog, 0),
            t("now1", Bucket::Now, Status::Backlog, 0),
        ];
        let list = build_list("inbox", &tasks, TODAY, "status", "", "");
        assert_eq!(list.title, "Inbox");
        assert_eq!(list.subtitle, "Captured today · unsorted", "Journey 1A verbatim");
        assert_eq!(ids(&list.groups[0]), vec!["in1"]);
    }

    #[test]
    fn binned_and_done_tasks_never_appear_in_a_bucket_list() {
        let tasks = vec![
            t("open", Bucket::Next, Status::Backlog, 0),
            t("binned", Bucket::Next, Status::Binned, 0),
            t("done", Bucket::Next, Status::Done, 0),
        ];
        let list = build_list("next", &tasks, TODAY, "status", "", "");
        assert_eq!(
            ids(&list.groups[0]),
            vec!["open"],
            "only the Now list keeps done rows visible (principle 6 is about today)"
        );
        assert_eq!(list.subtitle, "This week", "Journey 1C verbatim");
    }

    #[test]
    fn the_waiting_view_is_a_status_query_across_every_bucket() {
        let tasks = vec![
            t("w1", Bucket::Now, Status::Waiting, 0),
            t("w2", Bucket::Later, Status::Waiting, 0),
            t("other", Bucket::Now, Status::Blocked, 0),
        ];
        let list = build_list("waiting", &tasks, TODAY, "status", "", "");
        assert_eq!(list.title, "Waiting on");
        let mut found = ids(&list.groups[0]);
        found.sort_unstable();
        assert_eq!(found, vec!["w1", "w2"]);
    }

    #[test]
    fn all_actions_groups_by_status_with_backlog_and_binned_collapsed() {
        // Journey 5B: four columns; Backlog and Binned collapse to counts.
        let tasks = vec![
            t("ip", Bucket::Now, Status::InProgress, 0),
            t("bl", Bucket::Next, Status::Blocked, 0),
            t("wa", Bucket::Later, Status::Waiting, 0),
            t("dn", Bucket::Now, Status::Done, 0),
            t("bk1", Bucket::Inbox, Status::Backlog, 0),
            t("bk2", Bucket::Inbox, Status::Backlog, 0),
            t("bn", Bucket::Later, Status::Binned, 0),
        ];
        let list = build_list("all", &tasks, TODAY, "status", "", "");
        assert_eq!(list.title, "All actions");
        let labels: Vec<&str> = list.groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, vec!["In progress", "Blocked", "Waiting", "Done"]);
        let collapsed: Vec<(&str, u64)> = list
            .collapsed
            .iter()
            .map(|c| (c.label.as_str(), c.count))
            .collect();
        assert_eq!(collapsed, vec![("Backlog", 2), ("Binned", 1)]);
    }

    #[test]
    fn all_actions_can_group_by_bucket_instead() {
        let tasks = vec![
            t("n", Bucket::Now, Status::Backlog, 0),
            t("e", Bucket::Next, Status::Backlog, 0),
            t("i", Bucket::Inbox, Status::Backlog, 0),
        ];
        let list = build_list("all", &tasks, TODAY, "bucket", "", "");
        let labels: Vec<&str> = list.groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, vec!["Inbox", "Now", "Next", "Later"]);
        assert!(
            list.collapsed.is_empty(),
            "collapsing is a status-grouping concept only"
        );
        assert_eq!(list.groups[3].count, 0, "an empty bucket group still reports zero");
    }

    #[test]
    fn all_actions_filters_narrow_the_same_grouping() {
        let tasks = vec![
            t("a", Bucket::Now, Status::Blocked, 0),
            t("b", Bucket::Next, Status::Blocked, 0),
            t("c", Bucket::Now, Status::Waiting, 0),
        ];
        let by_bucket = build_list("all", &tasks, TODAY, "status", "now", "");
        let rows: Vec<&str> = by_bucket.groups.iter().flat_map(|g| ids(g)).collect();
        assert_eq!(rows, vec!["a", "c"], "bucket filter");

        let by_status = build_list("all", &tasks, TODAY, "status", "", "blocked");
        let rows: Vec<&str> = by_status.groups.iter().flat_map(|g| ids(g)).collect();
        assert_eq!(rows, vec!["a", "b"], "status filter");

        let both = build_list("all", &tasks, TODAY, "status", "now", "blocked");
        let rows: Vec<&str> = both.groups.iter().flat_map(|g| ids(g)).collect();
        assert_eq!(rows, vec!["a"], "filters compose");
    }

    #[test]
    fn all_actions_with_no_grouping_is_one_flat_list() {
        let tasks = vec![
            t("p1", Bucket::Now, Status::Backlog, 1),
            t("p3", Bucket::Later, Status::Backlog, 3),
        ];
        let list = build_list("all", &tasks, TODAY, "none", "", "");
        assert_eq!(list.groups.len(), 1);
        assert_eq!(list.groups[0].label, "");
        assert_eq!(ids(&list.groups[0]), vec!["p1", "p3"], "priority order holds");
    }

    #[test]
    fn an_unknown_route_is_an_empty_list_not_a_panic() {
        let list = build_list("nonsense", &[], TODAY, "status", "", "");
        assert!(list.groups.iter().all(|g| g.rows.is_empty()));
    }
}
```

- [x] **Step 6: Run them to verify they fail**

Run: `cargo nextest run -p shared view::task_list`
Expected: FAIL to compile — `cannot find function build_list`.

- [x] **Step 7: Implement the list builder**

`shared/src/view/task_list.rs`:

```rust
//! One builder for every task surface (plan decision #3): Now, Inbox, the
//! bucket views, Waiting on, and All actions are the same pure function over
//! `Model.tasks` with a different route.

use facet::Facet;
use serde::{Deserialize, Serialize};

use super::task_row::{TaskRowVm, build_row};
use crate::effects::storage::TaskData;
use crate::task::{Bucket, Status, is_open, sort_key};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TaskListVm {
    pub title: String,
    pub subtitle: String,
    pub groups: Vec<TaskGroupVm>,
    /// Only the Now list carries the momentum cue (reference §7.1).
    pub momentum: Option<MomentumVm>,
    /// Journey 5B: Backlog and Binned collapse to counts.
    pub collapsed: Vec<CollapsedGroupVm>,
    pub group_by: String,
    pub filter_bucket: String,
    pub filter_status: String,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TaskGroupVm {
    pub label: String,
    /// "" for ungrouped; otherwise the status or bucket key, for tinting.
    pub kind: String,
    pub count: u64,
    pub rows: Vec<TaskRowVm>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct MomentumVm {
    pub done: u64,
    pub remaining: u64,
    pub label: String,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CollapsedGroupVm {
    pub label: String,
    pub count: u64,
}

fn bucket_key(b: Bucket) -> &'static str {
    match b {
        Bucket::Inbox => "inbox",
        Bucket::Now => "now",
        Bucket::Next => "next",
        Bucket::Later => "later",
    }
}

fn bucket_label(b: Bucket) -> &'static str {
    match b {
        Bucket::Inbox => "Inbox",
        Bucket::Now => "Now",
        Bucket::Next => "Next",
        Bucket::Later => "Later",
    }
}

fn status_key(s: Status) -> &'static str {
    match s {
        Status::Backlog => "backlog",
        Status::InProgress => "in_progress",
        Status::Blocked => "blocked",
        Status::Waiting => "waiting",
        Status::Done => "done",
        Status::Binned => "binned",
    }
}

/// Verbatim from core-journeys Journey 5A.
fn status_label(s: Status) -> &'static str {
    match s {
        Status::Backlog => "Backlog",
        Status::InProgress => "In progress",
        Status::Blocked => "Blocked",
        Status::Waiting => "Waiting",
        Status::Done => "Done",
        Status::Binned => "Binned",
    }
}

fn group(label: &str, kind: &str, tasks: &[&TaskData], today: &str) -> TaskGroupVm {
    let mut sorted: Vec<&TaskData> = tasks.to_vec();
    sorted.sort_by_key(|t| sort_key(t, today));
    TaskGroupVm {
        label: label.into(),
        kind: kind.into(),
        count: sorted.len() as u64,
        rows: sorted.iter().map(|t| build_row(t, today)).collect(),
    }
}

#[must_use]
pub fn build_list(
    route: &str,
    tasks: &[TaskData],
    today: &str,
    group_by: &str,
    filter_bucket: &str,
    filter_status: &str,
) -> TaskListVm {
    let base = TaskListVm {
        group_by: group_by.into(),
        filter_bucket: filter_bucket.into(),
        filter_status: filter_status.into(),
        ..TaskListVm::default()
    };

    // A single bucket or status view: one group, no collapsing.
    let simple = |title: &str, subtitle: &str, rows: Vec<&TaskData>| TaskListVm {
        title: title.into(),
        subtitle: subtitle.into(),
        groups: vec![group("", "", &rows, today)],
        ..base.clone()
    };

    match route {
        "now" => {
            // Principle 6: done rows stay for the rest of the day, and drive
            // the momentum cue.
            let rows: Vec<&TaskData> = tasks
                .iter()
                .filter(|t| t.bucket == Bucket::Now && t.status != Status::Binned)
                .collect();
            let done = rows.iter().filter(|t| t.status == Status::Done).count() as u64;
            let remaining = rows.len() as u64 - done;
            let mut list = simple("Now", "Today", rows);
            list.momentum = Some(MomentumVm {
                done,
                remaining,
                label: format!("{done} done · {remaining} to go"),
            });
            list
        }
        "next" => simple(
            "Next",
            "This week",
            tasks
                .iter()
                .filter(|t| t.bucket == Bucket::Next && is_open(t.status))
                .collect(),
        ),
        "later" => simple(
            "Later",
            "",
            tasks
                .iter()
                .filter(|t| t.bucket == Bucket::Later && is_open(t.status))
                .collect(),
        ),
        "inbox" => simple(
            "Inbox",
            "Captured today · unsorted",
            tasks
                .iter()
                .filter(|t| t.bucket == Bucket::Inbox && is_open(t.status))
                .collect(),
        ),
        "waiting" => simple(
            "Waiting on",
            "",
            tasks.iter().filter(|t| t.status == Status::Waiting).collect(),
        ),
        "all" => {
            let kept: Vec<&TaskData> = tasks
                .iter()
                .filter(|t| filter_bucket.is_empty() || bucket_key(t.bucket) == filter_bucket)
                .filter(|t| filter_status.is_empty() || status_key(t.status) == filter_status)
                .collect();
            let mut list = TaskListVm {
                title: "All actions".into(),
                subtitle: String::new(),
                ..base
            };
            match group_by {
                "bucket" => {
                    for b in [Bucket::Inbox, Bucket::Now, Bucket::Next, Bucket::Later] {
                        let rows: Vec<&TaskData> =
                            kept.iter().copied().filter(|t| t.bucket == b).collect();
                        list.groups
                            .push(group(bucket_label(b), bucket_key(b), &rows, today));
                    }
                }
                "none" => {
                    list.groups.push(group("", "", &kept, today));
                }
                _ => {
                    // Journey 5B's four columns, then the two collapsed counts.
                    for s in [
                        Status::InProgress,
                        Status::Blocked,
                        Status::Waiting,
                        Status::Done,
                    ] {
                        let rows: Vec<&TaskData> =
                            kept.iter().copied().filter(|t| t.status == s).collect();
                        list.groups
                            .push(group(status_label(s), status_key(s), &rows, today));
                    }
                    for s in [Status::Backlog, Status::Binned] {
                        list.collapsed.push(CollapsedGroupVm {
                            label: status_label(s).into(),
                            count: kept.iter().filter(|t| t.status == s).count() as u64,
                        });
                    }
                }
            }
            list
        }
        // "today" and anything unrecognised: the Today column draws the note
        // plus the Now list, which it fetches under the "now" route.
        _ => TaskListVm { ..base },
    }
}
```

`TaskListVm` needs `Clone` for the `base.clone()` above; it derives it already.

- [x] **Step 8: Run them to verify green**

Run: `cargo nextest run -p shared view::task_list`
Expected: PASS (11 tests).

- [x] **Step 9: Write the failing sidebar-count test**

Create `shared/src/view/sidebar.rs` by **moving** `build_sidebar` out of `app.rs` unchanged, then add to its new `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::storage::TaskData;
    use crate::task::{Bucket, Status};

    fn t(bucket: Bucket, status: Status) -> TaskData {
        TaskData {
            id: "x".into(),
            title: "x".into(),
            bucket,
            status,
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

    fn counts(tasks: Vec<TaskData>) -> Vec<(String, u64)> {
        let model = Model {
            today: "2026-07-04".into(),
            tasks,
            ..Model::default()
        };
        build_sidebar(&model)
            .views
            .into_iter()
            .map(|r| (r.kind, r.count))
            .collect()
    }

    #[test]
    fn views_rows_count_open_work_per_bucket_and_status() {
        let rows = counts(vec![
            t(Bucket::Now, Status::Backlog),
            t(Bucket::Now, Status::Done),
            t(Bucket::Next, Status::Backlog),
            t(Bucket::Later, Status::Backlog),
            t(Bucket::Later, Status::Binned),
            t(Bucket::Inbox, Status::Backlog),
            t(Bucket::Next, Status::Waiting),
        ]);
        assert_eq!(
            rows,
            vec![
                ("now".to_string(), 1),
                ("next".to_string(), 2),
                ("later".to_string(), 1),
                ("waiting".to_string(), 1),
                ("inbox".to_string(), 1),
            ],
            "counts are open work: done and binned are not outstanding, and \
             Waiting counts in both its bucket and the Waiting row"
        );
    }

    #[test]
    fn an_empty_database_shows_zeroes_and_no_projects_people_or_pages() {
        let rows = counts(vec![]);
        assert!(rows.iter().all(|(_, count)| *count == 0));
        let model = Model::default();
        let sidebar = build_sidebar(&model);
        assert!(sidebar.projects.is_empty(), "pages arrive in Phase 3");
        assert!(sidebar.people.is_empty());
        assert!(sidebar.pages.is_empty());
    }
}
```

- [x] **Step 10: Run it to verify it fails**

Run: `cargo nextest run -p shared view::sidebar`
Expected: FAIL — the `now` row is hard-coded to 0 and `inbox` counts `model.tasks.len()` (Phase 1's honest placeholder), so the assertion prints `("now", 0), ..., ("inbox", 7)`.

- [x] **Step 11: Make the counts live**

In `shared/src/view/sidebar.rs`, replace the hard-coded `views` vector with:

```rust
    let open_in = |bucket: Bucket| {
        model
            .tasks
            .iter()
            .filter(|t| t.bucket == bucket && is_open(t.status))
            .count() as u64
    };
    let waiting = model
        .tasks
        .iter()
        .filter(|t| t.status == Status::Waiting)
        .count() as u64;
    // ...
        views: vec![
            view_row("now", "Now", open_in(Bucket::Now)),
            view_row("next", "Next · This week", open_in(Bucket::Next)),
            view_row("later", "Later", open_in(Bucket::Later)),
            view_row("waiting", "Waiting on", waiting),
            view_row("inbox", "Inbox", open_in(Bucket::Inbox)),
        ],
```

- [x] **Step 12: Run it to verify green**

Run: `cargo nextest run -p shared view::sidebar`
Expected: PASS (2 tests).

- [x] **Step 13: Wire routing into the model and the ViewModel**

In `shared/src/view/mod.rs`:

```rust
//! Per-surface ViewModel builders. `app.rs` owns the model and `update()`;
//! everything that turns model state into display data lives here.

pub mod sidebar;
pub mod task_list;
pub mod task_row;

mod calendar;
mod day;

pub use calendar::build_calendar;
pub use day::build_day;
pub use sidebar::build_sidebar;
pub use task_list::{CollapsedGroupVm, MomentumVm, TaskGroupVm, TaskListVm, build_list};
pub use task_row::TaskRowVm;
```

Move `build_calendar` into `shared/src/view/calendar.rs` and `build_day` into `shared/src/view/day.rs`, both unchanged, together with the `CalendarVm`/`CalendarCellVm`/`DayVm` definitions they build. Move `SidebarVm`/`ViewRowVm`/`SidebarEntryVm` into `sidebar.rs`. `app.rs` keeps `ViewModel` itself and re-exports the rest through `lib.rs` so no other crate's imports change.

In `app.rs`:

```rust
#[derive(Default)]
pub struct Model {
    // ... Phase 1 fields unchanged ...
    pub tasks: Vec<TaskData>,
    /// Which surface the main column shows: "today" | "now" | "next" |
    /// "later" | "waiting" | "inbox" | "all" (plan decision #7).
    pub route: String,
    /// All-actions view state. Empty strings mean "no filter".
    pub group_by: String,
    pub filter_bucket: String,
    pub filter_status: String,
    pub error: Option<String>,
}
```

three new event arms:

```rust
            Event::SelectView { kind } => {
                model.route = kind;
                render()
            }
            Event::SetGrouping { group_by } => {
                model.group_by = group_by;
                render()
            }
            Event::SetFilter { bucket, status } => {
                model.filter_bucket = bucket;
                model.filter_status = status;
                render()
            }
```

`Startup` sets the defaults (`model.route = "today".into(); model.group_by = "status".into();`) before its commands, and `view()` becomes:

```rust
    fn view(&self, model: &Model) -> ViewModel {
        // The Today column draws the note plus the Now list, so "today"
        // builds the Now list; every other route builds its own.
        let list_route = if model.route == "today" { "now" } else { &model.route };
        ViewModel {
            sidebar: build_sidebar(model),
            calendar: build_calendar(model),
            route: model.route.clone(),
            day: build_day(model),
            list: build_list(
                list_route,
                &model.tasks,
                &model.today,
                &model.group_by,
                &model.filter_bucket,
                &model.filter_status,
            ),
            error: model.error.clone(),
        }
    }
```

- [x] **Step 14: Write the failing routing test**

In `app.rs`'s `mod tests`:

```rust
    #[test]
    fn startup_lands_on_today_and_today_carries_the_now_list() {
        let (app, mut model) = started();
        with_task(&app, &mut model, {
            let mut t = task_fixture("t1", Bucket::Now, Status::Backlog);
            t.title = "Chase COAST support docs response".into();
            t
        });
        let view = app.view(&model);
        assert_eq!(view.route, "today");
        assert_eq!(view.day.title, "Saturday, July 4", "the note is still there");
        assert_eq!(
            view.list.title, "Now",
            "Today draws the note plus the Now section, not an empty list"
        );
        assert_eq!(view.list.groups[0].rows.len(), 1);
    }

    #[test]
    fn selecting_a_sidebar_view_switches_the_surface_without_touching_the_day() {
        let (app, mut model) = started();
        let before = app.view(&model).day.editor_version;
        let mut cmd = app.update(
            Event::SelectView {
                kind: "inbox".into(),
            },
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        let view = app.view(&model);
        assert_eq!(view.route, "inbox");
        assert_eq!(view.list.title, "Inbox");
        assert_eq!(
            view.day.editor_version, before,
            "switching surfaces must not disturb the editor (R1 gate)"
        );
    }

    #[test]
    fn grouping_and_filter_changes_only_reshape_the_all_actions_list() {
        let (app, mut model) = started();
        let _ = app.update(Event::SelectView { kind: "all".into() }, &mut model);
        let _ = app.update(
            Event::SetGrouping {
                group_by: "bucket".into(),
            },
            &mut model,
        );
        let labels: Vec<String> = app
            .view(&model)
            .list
            .groups
            .into_iter()
            .map(|g| g.label)
            .collect();
        assert_eq!(labels, vec!["Inbox", "Now", "Next", "Later"]);

        let _ = app.update(
            Event::SetFilter {
                bucket: "now".into(),
                status: String::new(),
            },
            &mut model,
        );
        let view = app.view(&model);
        assert_eq!(view.list.filter_bucket, "now", "the shell renders the active chip");
        assert_eq!(view.list.group_by, "bucket");
    }
```

- [x] **Step 15: Run the whole suite**

Run: `just test`
Expected: PASS. Then `cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --check`.

- [x] **Step 16: Regenerate the Swift types and keep the app building**

Run: `just generate && just app`
Expected: the app builds. `ContentView.swift` needs the new required `ViewModel` fields in its previews/initialisers only if it constructs one — `Core.swift` does, in its `private(set) var view = ViewModel(...)` seed. Add `route: "today"`, `list: TaskListVm(...)` (empty) to that literal, matching the generated Swift signature exactly (field order follows the Rust struct). No behaviour change: the shell ignores `list` until Task 5.

- [x] **Step 17: Commit + PR**

```bash
git add shared apple/Yardstick/Core.swift
git commit -m "refactor(core): split view builders out of app.rs and add task surfaces"
git push -u origin p2/t3-view-builders
gh pr create --fill
```

PR description records: `app.rs` shrank from ~700 to ~450 lines; every Phase 1 view test still passes unchanged (proof the move was a move); spec deltas **none**.
STOP for review.

**Deviations recorded while implementing (plan amended in the Task 3 PR):**

1. **Step 3's `meta()` contradicts Step 1's own test.** The plan's `Bucket::Now` arm falls back to `source_label(&task.source)` for any row with no age, which makes a Now row entered today read `quick add` — but Step 1's `now_rows_show_age_once_a_day_has_passed_and_never_zero_days` asserts `""` for exactly that row, while `a_now_row_with_no_age_falls_back_to_provenance` asserts `from note` for the same zero-age case with a different source. Arbiter: the pixel-fidelity rule's `v2-today-view.md` §7.2, which shows three distinct Now-row metas — `2 days old` (row 1), `from Slack` (row 2) and an **empty 70px spacer** (row 3). `quick_add` is the default in-app path, so it is row 3's case: the Now arm now suppresses it and shows provenance only for a source worth naming. Inbox rows are unaffected (Journey 1A: the Inbox exists to show provenance, `quick add` included).

2. **Steps 13 and 14 are in the wrong order** — Step 13 wires routing, Step 14 writes the test that drives it, so following them literally means no observable failure. Executed swapped: the Step 13 wiring was backed out, Step 14's three tests were run against the unwired core (`no field route on type ViewModel`, `no variant named SelectView`), then the wiring was restored. Future task steps should put the test before the wiring.

3. **Steps 1 and 9 need the module to exist before their tests can compile.** The plan only creates `shared/src/view/mod.rs` at Step 13, so Steps 1/5/9 would fail on "file not found for module" rather than the named failure. `mod.rs` (and `pub mod view;` in `lib.rs`) were created at Step 1 and extended one `pub mod` line at a time; Step 13 then added the re-exports and the `calendar`/`day` moves as written.

4. **Step 13 does not give the `lib.rs` re-export line.** `CalendarCellVm`, `CalendarVm`, `DayVm`, `SidebarEntryVm`, `SidebarVm` and `ViewRowVm` moved from `app` to `view`, so `lib.rs` now re-exports them from `view::` alongside the five new list types. `ViewModel` stays in `app`. `shared::ViewModel` is the only one of these any other crate imports (`runtime/src/router.rs`, `runtime/tests/ffi.rs`), so no downstream import changed.

5. **Step 17's line-count claim is stale.** It was written before Task 2 grew `app.rs`. Actual: implementation lines 491 → 430; the file total is 1088 → 1102 because Step 14 adds 71 lines of routing tests. The moved code is 240 lines across four new `view/` files.

---

### Task 4: The task row (reference §7.2), all four states

**Files:**
- Create: `apple/Yardstick/TaskRow.swift`, `apple/Yardstick/StatusMenu.swift` (the six bare buttons only — Task 8 completes it), `apple/YardstickTests/TaskRowFormattingTests.swift`
- Modify: `apple/Yardstick/Theme.swift` (row metrics + checkbox/pip tokens)
- Test: `apple/YardstickTests/TaskRowFormattingTests.swift`

**Interfaces:**
- Consumes: generated `App.TaskRowVm` (Task 3).
- Produces, consumed by Tasks 5, 6 and 9:
  - `struct TaskRow: View` — `init(row: TaskRowVm, onToggleDone: @escaping () -> Void, onOpenTriage: @escaping () -> Void, onSetStatus: @escaping (Status) -> Void)`
  - `enum RowStyle { static func checkbox(_ key: String) -> CheckboxStyle; static func pill(_ kind: String) -> (fg: Color, bg: Color)? }` — the pure mapping the tests drive.

**Riders:** none.

- [x] **Step 1: Write the failing formatting tests**

`apple/YardstickTests/TaskRowFormattingTests.swift`:

```swift
import SwiftUI
import XCTest
@testable import Yardstick

/// The row's pure mapping from core strings to styling. Pixel fidelity is
/// checked by eye against reference §7.2; these tests pin the parts that
/// silently rot: which state maps to which shape, and that an unknown key
/// degrades to the neutral open state rather than crashing or vanishing.
final class TaskRowFormattingTests: XCTestCase {

    func testCheckboxStatesMapToTheReferenceShapes() {
        XCTAssertEqual(RowStyle.checkbox("open"), .ring)
        XCTAssertEqual(RowStyle.checkbox("in_progress"), .ringWithSoftCentre)
        XCTAssertEqual(RowStyle.checkbox("done"), .filledCheck)
    }

    func testUnknownCheckboxKeyFallsBackToOpen() {
        // A core that grows a new state must not produce an invisible row.
        XCTAssertEqual(RowStyle.checkbox("something_new"), .ring)
        XCTAssertEqual(RowStyle.checkbox(""), .ring)
    }

    func testOnlyNotableStatusesGetAPillTint() {
        XCTAssertNil(RowStyle.pill(""), "an ordinary task has no pill")
        XCTAssertNotNil(RowStyle.pill("in_progress"))
        XCTAssertNotNil(RowStyle.pill("blocked"))
        XCTAssertNotNil(RowStyle.pill("waiting"))
        XCTAssertNotNil(RowStyle.pill("binned"))
    }

    func testPillTintsComeFromTheStatusTokensNotAdHocColours() {
        XCTAssertEqual(RowStyle.pill("blocked")?.bg, Theme.statusBlockedBg)
        XCTAssertEqual(RowStyle.pill("blocked")?.fg, Theme.statusBlocked)
        XCTAssertEqual(RowStyle.pill("waiting")?.bg, Theme.statusWaitingBg)
        XCTAssertEqual(RowStyle.pill("in_progress")?.bg, Theme.pillTint)
    }

    func testPriorityBadgeColoursFollowTheTokenScale() {
        XCTAssertEqual(RowStyle.priorityColour(1), Theme.priority1)
        XCTAssertEqual(RowStyle.priorityColour(2), Theme.priority2)
        XCTAssertEqual(RowStyle.priorityColour(3), Theme.priority3)
        XCTAssertNil(RowStyle.priorityColour(0), "priority is optional — no badge")
        XCTAssertNil(RowStyle.priorityColour(9), "out of range renders nothing")
    }
}
```

- [x] **Step 2: Run it to verify it fails**

Run: `just app-test`
Expected: FAIL — `cannot find 'RowStyle' in scope`.

- [x] **Step 3: Implement `RowStyle` and the row**

Add to `Theme.Metrics`:

```swift
        static let taskRowVPadding: CGFloat = 9        // §7.2
        static let taskRowHPadding: CGFloat = 6
        static let taskRowGap: CGFloat = 11
        static let checkboxSize: CGFloat = 17
        static let priorityBadgeSize: CGFloat = 19
        static let priorityBadgeRadius: CGFloat = 5
        static let metaColumnWidth: CGFloat = 70       // fixed, even when empty
        static let pillRadius: CGFloat = 20
        static let pipWidth: CGFloat = 16
        static let pipHeight: CGFloat = 5
        static let pipRadius: CGFloat = 3
```

and `Theme.checkboxRing = Color(hex: 0xC4C3C0)` beside the neutrals (§7.2 row 2's `#c4c3c0` ring).

`apple/Yardstick/TaskRow.swift`:

```swift
import App
import SwiftUI

enum CheckboxStyle: Equatable {
    case ring                 // open: 1.5px #c4c3c0 ring
    case ringWithSoftCentre   // in progress: blue ring + 25% blue centre
    case filledCheck          // done: green fill + white check
}

/// Pure mapping from the core's strings to styling. Kept out of the view so
/// it can be tested without a host app (TaskRowFormattingTests).
enum RowStyle {
    static func checkbox(_ key: String) -> CheckboxStyle {
        switch key {
        case "in_progress": return .ringWithSoftCentre
        case "done": return .filledCheck
        default: return .ring
        }
    }

    static func pill(_ kind: String) -> (fg: Color, bg: Color)? {
        switch kind {
        case "in_progress": return (Theme.accentTextDark, Theme.pillTint)
        case "blocked": return (Theme.statusBlocked, Theme.statusBlockedBg)
        case "waiting": return (Theme.statusWaiting, Theme.statusWaitingBg)
        case "binned": return (Theme.textSecondary, Theme.chipBg)
        default: return nil
        }
    }

    static func priorityColour(_ priority: UInt8) -> Color? {
        switch priority {
        case 1: return Theme.priority1
        case 2: return Theme.priority2
        case 3: return Theme.priority3
        default: return nil
        }
    }
}

/// Reference §7.2 — 17px checkbox, 14px title, optional priority badge and
/// status pill, then the fixed 70px right-aligned meta column (present even
/// when empty, so titles stay aligned down the list).
struct TaskRow: View {
    let row: TaskRowVm
    let onToggleDone: () -> Void
    let onOpenTriage: () -> Void
    let onSetStatus: (Status) -> Void

    @State private var isHovered = false

    var body: some View {
        HStack(spacing: Theme.Metrics.taskRowGap) {
            Button(action: onToggleDone) { checkbox }
                .buttonStyle(.plain)
                .accessibilityLabel(row.isDone ? "Mark not done" : "Mark done")

            Text(row.title)
                .font(Theme.Typography.body)
                .foregroundStyle(row.isDone ? Theme.textTertiary : Theme.textPrimary)
                .strikethrough(row.isDone)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)

            if let colour = RowStyle.priorityColour(row.priority) {
                Text("\(row.priority)")
                    .font(.system(size: 11, weight: .bold))
                    .foregroundStyle(.white)
                    .frame(width: Theme.Metrics.priorityBadgeSize,
                           height: Theme.Metrics.priorityBadgeSize)
                    .background(colour)
                    .clipShape(RoundedRectangle(cornerRadius: Theme.Metrics.priorityBadgeRadius))
            }

            if let tint = RowStyle.pill(row.statusKind), !row.statusPill.isEmpty {
                HStack(spacing: 5) {
                    Circle().fill(tint.fg).frame(width: 6, height: 6)
                    Text(row.statusPill)
                }
                .font(Theme.Typography.meta)
                .foregroundStyle(tint.fg)
                .padding(.horizontal, 9)
                .padding(.vertical, 3)
                .background(tint.bg)
                .clipShape(Capsule())
            }

            // Chips are empty until Phase 3 (carve-out 1); the loop renders
            // nothing today and needs no change when pages arrive.
            ForEach(row.chips, id: \.self) { chip in
                Text(chip)
                    .font(Theme.Typography.meta)
                    .foregroundStyle(Theme.textSecondary)
                    .padding(.horizontal, 9)
                    .padding(.vertical, 3)
                    .background(Theme.chipBg)
                    .clipShape(Capsule())
            }

            Text(row.meta)
                .font(Theme.Typography.meta)
                .foregroundStyle(row.isDone ? Theme.textTertiary : Theme.textQuiet)
                .frame(width: Theme.Metrics.metaColumnWidth, alignment: .trailing)
        }
        .padding(.vertical, Theme.Metrics.taskRowVPadding)
        .padding(.horizontal, Theme.Metrics.taskRowHPadding)
        .background(isHovered ? Theme.hoverBg : .clear)
        .clipShape(RoundedRectangle(cornerRadius: Theme.Metrics.rowRadius))
        .opacity(row.isDone ? 0.55 : 1)
        .onHover { isHovered = $0 }
        .contextMenu {
            Button("Triage…", action: onOpenTriage)
            StatusMenuItems(current: row.statusKind, onSelect: onSetStatus)
        }
        .overlay(alignment: .bottomLeading) {
            if !row.blockedReason.isEmpty {
                Text(row.blockedReason)
                    .font(Theme.Typography.meta)
                    .foregroundStyle(Theme.statusBlocked)
                    .padding(.leading, 45)
            }
        }
    }

    @ViewBuilder
    private var checkbox: some View {
        let size = Theme.Metrics.checkboxSize
        switch RowStyle.checkbox(row.checkbox) {
        case .ring:
            Circle().strokeBorder(Theme.checkboxRing, lineWidth: 1.5)
                .frame(width: size, height: size)
        case .ringWithSoftCentre:
            Circle().strokeBorder(Theme.accent, lineWidth: 1.5)
                .frame(width: size, height: size)
                .overlay(Circle().fill(Theme.accent).opacity(0.25).padding(3))
        case .filledCheck:
            Circle().fill(Theme.statusDone)
                .frame(width: size, height: size)
                .overlay(
                    Image(systemName: "checkmark")
                        .font(.system(size: 9, weight: .semibold))
                        .foregroundStyle(.white))
        }
    }
}
```

`StatusMenuItems` arrives in Task 8. Until then, put a temporary local definition **in Task 8's file path** — no: Task 4 must compile alone, so include the real `StatusMenuItems` here in `StatusMenu.swift` with just the six buttons, and Task 8 adds the reason prompt and the checkmark/description styling to that same file. Task 8's Interfaces block repeats the signature.

- [x] **Step 4: Run it to verify green**

Run: `just app-test`
Expected: PASS (5 new tests, 13 existing).

- [x] **Step 5: Eyeball the four states**

Add previews to `TaskRow.swift` covering reference §7.2's four rows verbatim (focusing-state row renders as `in_progress` in Phase 2 — carve-out 2), then run `cd apple && just run` is not needed: use Xcode previews or check them inside Task 5's running Now section. Record in the PR which of §7.2's four row states you compared, and any deviation.

- [x] **Step 6: Commit + PR**

```bash
git add apple
git commit -m "feat(apple): task row to reference §7.2 with all four states"
git push -u origin p2/t4-task-row
gh pr create --fill   # spec-deltas: none
```
STOP for review.

---

### Task 5: The Now section under the daily note

**Wave 4, lane A — runs concurrently with Tasks 7 and 8.** Owns `TaskListView.swift` and `DayColumn.swift` outright; in the shared files it may touch only the `DayColumn(...)` call site in `ContentView` and a `// MARK: Capture` section at the end of `Core.swift`. Obey the Execution plan's four merge-contention rules.

**Files:**
- Create: `apple/Yardstick/TaskListView.swift`
- Modify: `apple/Yardstick/DayColumn.swift` (Now section below the note), `apple/Yardstick/ContentView.swift` (pass the list + handlers)
- Test: `runtime/tests/tasks_flow.rs` is Task 10; this task's proof is the core tests from Task 3 plus the manual checklist below

**Interfaces:**
- Consumes: `TaskRow` (Task 4), generated `App.TaskListVm`.
- Produces, consumed by Tasks 6 and 9:
  - `struct TaskListView: View` — `init(list: TaskListVm, showsHeader: Bool = true, onToggleDone: @escaping (String) -> Void, onOpenTriage: @escaping (String) -> Void, onSetStatus: @escaping (String, Status) -> Void)`
  - `struct MomentumPips: View` — `init(done: UInt64, remaining: UInt64)`

**Riders:** none.

- [ ] **Step 1: Build the section header and pips**

`apple/Yardstick/TaskListView.swift`:

```swift
import App
import SwiftUI

/// Reference §7.1 — four 16×5px pips, done ones green, the rest #dedcd8.
/// The pip count follows the row count (capped so a 40-task day does not
/// draw 40 pips); the label carries the exact numbers.
struct MomentumPips: View {
    let done: UInt64
    let remaining: UInt64

    private var total: Int { min(Int(done + remaining), 4) }
    private var filled: Int {
        guard done + remaining > 0 else { return 0 }
        return Int((Double(done) / Double(done + remaining) * Double(total)).rounded())
    }

    var body: some View {
        HStack(spacing: 3) {
            ForEach(0..<total, id: \.self) { index in
                RoundedRectangle(cornerRadius: Theme.Metrics.pipRadius)
                    .fill(index < filled ? Theme.statusDone : Theme.segmentRemaining)
                    .frame(width: Theme.Metrics.pipWidth, height: Theme.Metrics.pipHeight)
            }
        }
    }
}

/// One task surface: the section header (title + subtitle + momentum cue),
/// then grouped rows. Used by the Today column's Now section, every sidebar
/// bucket view, and the All-actions view.
struct TaskListView: View {
    let list: TaskListVm
    var showsHeader = true
    let onToggleDone: (String) -> Void
    let onOpenTriage: (String) -> Void
    let onSetStatus: (String, Status) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if showsHeader { header }
            ForEach(Array(list.groups.enumerated()), id: \.offset) { _, group in
                if !group.label.isEmpty {
                    HStack(spacing: 8) {
                        Text(group.label)
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(Theme.textPrimary)
                        Text("\(group.count)")
                            .font(Theme.Typography.count)
                            .foregroundStyle(Theme.textMuted)
                    }
                    .padding(.top, 14)
                    .padding(.bottom, 2)
                }
                if group.rows.isEmpty && !group.label.isEmpty {
                    Text("Nothing here")
                        .font(Theme.Typography.meta)
                        .foregroundStyle(Theme.countEmpty)
                        .padding(.vertical, 6)
                        .padding(.horizontal, Theme.Metrics.taskRowHPadding)
                }
                ForEach(group.rows, id: \.id) { row in
                    TaskRow(row: row,
                            onToggleDone: { onToggleDone(row.id) },
                            onOpenTriage: { onOpenTriage(row.id) },
                            onSetStatus: { onSetStatus(row.id, $0) })
                }
            }
            ForEach(Array(list.collapsed.enumerated()), id: \.offset) { _, group in
                Text("\(group.label) · \(group.count)")
                    .font(Theme.Typography.meta)
                    .foregroundStyle(Theme.textQuiet)
                    .padding(.top, 10)
                    .padding(.horizontal, Theme.Metrics.taskRowHPadding)
            }
        }
        .frame(maxWidth: Theme.Metrics.contentMaxWidth, alignment: .leading)
    }

    private var header: some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Text(list.title)
                .font(Theme.Typography.sectionHeader)
                .foregroundStyle(Theme.textPrimary)
            if !list.subtitle.isEmpty {
                Text(list.subtitle)
                    .font(Theme.Typography.sidebarRow)
                    .foregroundStyle(Theme.textTertiary)
            }
            Spacer()
            if let momentum = list.momentum {
                HStack(spacing: 8) {
                    MomentumPips(done: momentum.done, remaining: momentum.remaining)
                    Text(momentum.label)
                        .font(.system(size: 12))
                        .foregroundStyle(Theme.textTertiary)
                }
            }
        }
        .padding(.bottom, 4)
        .overlay(alignment: .bottom) { Theme.hairline08.frame(height: 0.5) }
        .padding(.bottom, 4)
    }
}
```

- [ ] **Step 2: Put it under the note**

In `DayColumn.swift`, add the parameters and render the section below the editor:

```swift
    let list: TaskListVm
    let onToggleDone: (String) -> Void
    let onOpenTriage: (String) -> Void
    let onSetStatus: (String, Status) -> Void
```

and, after the editor's `ZStack` (still inside the outer `VStack`):

```swift
            TaskListView(list: list,
                         onToggleDone: onToggleDone,
                         onOpenTriage: onOpenTriage,
                         onSetStatus: onSetStatus)
                .padding(.top, 24)
```

The editor no longer fills the column: give the `ZStack` `.frame(minHeight: 120, alignment: .topLeading)` and wrap the whole `VStack` in a `ScrollView`. This retires the Phase 1 recorded deviation ("the header stays fixed and the editor fills/scrolls the rest ... revisited when the task sections join the column in Phase 2") — say so in the PR, and delete that paragraph from `DayColumn.swift`'s doc comment.

- [ ] **Step 3: Wire the handlers in `ContentView`**

```swift
                DayColumn(day: core.view.day,
                          editable: core.dayIsEditable,
                          onEdit: { core.noteEdited($0) },
                          list: core.view.list,
                          onToggleDone: { core.send(.toggleDone(id: $0)) },
                          onOpenTriage: { core.triageTarget = $0 },
                          onSetStatus: { core.setStatus(id: $0, status: $1) })
```

`core.triageTarget` and `core.setStatus` arrive in Tasks 7 and 8. For this task, use `onOpenTriage: { _ in }` and `onSetStatus: { core.send(.setStatus(id: $0, status: $1, reason: "")) }`, and Task 7/8 replace them. Note it in the PR as a deliberate two-step so this task ships something testable.

- [ ] **Step 4: Build, run, and check against the reference**

Run: `just app-test && cd apple && just run`
Manual checklist (paste results into the PR):
1. Toolbar `+` → add "Test one" → it appears nowhere in Today (it is in Inbox, bucket=inbox) and the sidebar Inbox count reads 1. Correct behaviour, not a bug.
2. Use `sqlite3 ~/Library/Application\ Support/Yardstick/daily.db "UPDATE tasks SET bucket='now'"` to move it, relaunch → it appears under a "Now / Today" header with an empty checkbox, no priority badge, no chips, and `quick add` in the meta column.
3. Click the checkbox → the row dims, strikes through, drops to the bottom, and the header reads "1 done · 0 to go" with one green pip.
4. Click it again → it comes back as open, header back to "0 done · 1 to go".
5. Compare the header and row against reference §7.1/§7.2 and list every deviation with its rationale (expected: no chips, no Focusing pill, no completion time — carve-outs 1–3).

- [ ] **Step 5: Commit + PR**

```bash
git add apple
git commit -m "feat(apple): Now section with momentum cue under the daily note"
git push -u origin p2/t5-now-section
gh pr create --fill   # spec-deltas: none
```
STOP for review.

---

### Task 6: Sidebar navigation and the Inbox

**Files:**
- Modify: `apple/Yardstick/SidebarView.swift` (Views rows become buttons with a selected state), `apple/Yardstick/ContentView.swift` (route switch), `apple/Yardstick/Core.swift` (senders), `apple/Yardstick/QuickAddView.swift` (source tag)
- Create: `apple/Yardstick/InboxView.swift`
- Test: the Task 3 routing tests cover the core; this task's proof is the manual checklist

**Interfaces:**
- Consumes: `TaskListView` (Task 5), `ViewModel.route` (Task 3).
- Produces: `Core.selectView(_ kind: String)`, `Core.capture(_ title: String, source: String)`; `struct InboxView: View`.

**Riders:** none.

- [ ] **Step 1: Make the Views rows navigate**

In `SidebarView.swift`, add `let route: String` and `let onSelectView: (String) -> Void`, then wrap `viewRow(row)` in a button with a selected background, reusing §2.2's active-row treatment:

```swift
                    ForEach(Array(sidebar.views.enumerated()), id: \.offset) { _, row in
                        Button { onSelectView(row.kind) } label: {
                            viewRow(row, isSelected: route == row.kind)
                        }
                        .buttonStyle(.plain)
                    }
```

and in `viewRow`, take `isSelected: Bool`, tint the row (`Theme.accent` fill, white text, radius 7, height 30 when selected — §2.2) and leave the unselected appearance exactly as Phase 1 shipped it. The "Today" row is selected when `route == "today"`, so `todayRow` takes the same treatment and `onGoToToday` also sends `SelectView { kind: "today" }`.

Phase 1's carve-out "Views rows are non-interactive this phase except Today" is retired by this task — say so in the PR.

- [ ] **Step 2: Route the main column**

`apple/Yardstick/InboxView.swift`:

```swift
import App
import SwiftUI

/// Journey 1A — "Captured today · unsorted": no metadata, no ordering
/// promises, one source tag per row, and a Triage button on the selected row.
struct InboxView: View {
    let list: TaskListVm
    let onToggleDone: (String) -> Void
    let onOpenTriage: (String) -> Void
    let onSetStatus: (String, Status) -> Void

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                TaskListView(list: list,
                             onToggleDone: onToggleDone,
                             onOpenTriage: onOpenTriage,
                             onSetStatus: onSetStatus)
                if list.groups.allSatisfy({ $0.rows.isEmpty }) {
                    Text("Nothing to sort.")
                        .font(Theme.Typography.body)
                        .foregroundStyle(Theme.textQuiet)
                        .padding(.top, 18)
                }
            }
            .padding(EdgeInsets(top: Theme.Metrics.contentPaddingTop,
                                leading: Theme.Metrics.contentPaddingH,
                                bottom: 40,
                                trailing: Theme.Metrics.contentPaddingH))
            .frame(maxWidth: .infinity, alignment: .topLeading)
        }
        .background(Color.white)
    }
}
```

In `ContentView.swift`, replace the single `DayColumn` with a switch on `core.view.route`:

```swift
                switch core.view.route {
                case "today":
                    DayColumn(/* as Task 5 */)
                case "inbox":
                    InboxView(list: core.view.list, /* handlers */)
                case "all":
                    AllActionsView(list: core.view.list, /* handlers */)   // Task 9
                default:
                    // now / next / later / waiting: the same list surface
                    ScrollView {
                        TaskListView(list: core.view.list, /* handlers */)
                            .padding(/* content padding as InboxView */)
                    }
                    .background(Color.white)
                }
```

Task 9 adds the `"all"` case; until then that case renders the `default` branch. Do not add a placeholder view for it — route `"all"` is not reachable from the sidebar until Task 9 adds the row.

`.navigationTitle` follows the surface: `core.view.route == "today" ? "Today" : core.view.list.title`.

- [ ] **Step 3: Add the senders and the capture source**

In `Core.swift`:

```swift
    func selectView(_ kind: String) {
        // Flush first: switching away from Today must not lose an in-flight
        // note edit (same contract as navigate(to:)).
        flushPendingEdit()
        send(.selectView(kind: kind))
    }

    func capture(_ title: String, source: String) {
        send(.captureTask(title: title, source: source))
    }
```

and in `ContentView`, the `+` popover calls `core.capture($0, source: "quick_add")`.

- [ ] **Step 4: Build and check**

Run: `just app-test && cd apple && just run`
Manual checklist (paste into the PR):
1. `+` → add three tasks → sidebar Inbox count reads 3.
2. Click Inbox → the three rows appear under "Inbox / Captured today · unsorted", each with `quick add` in the meta column, and the Inbox row is tinted as selected.
3. Click Now / Next / Later / Waiting on → each shows its own empty header with the right title and subtitle ("Next" + "This week"), and nothing crashes on an empty list.
4. Click Today → the note is exactly as it was, with the caret behaviour unchanged, and the Now section below it.
5. Type in the note, immediately click Inbox, then click Today → the text is still there (the flush-then-switch contract).
6. Tick a task in Inbox → it leaves the Inbox list (done is not open) and the count drops.

- [ ] **Step 5: Commit + PR**

```bash
git add apple
git commit -m "feat(apple): sidebar Views navigate; Inbox with source tags"
git push -u origin p2/t6-routing-inbox
gh pr create --fill   # spec-deltas: none
```
STOP for review.

---

### Task 7: The triage sheet and its keyboard

**Wave 4, lane B — runs concurrently with Tasks 5 and 8.** Owns `TriageSheet.swift` and `shared/src/view/task_row.rs` outright; in the shared files it may add one `.sheet` modifier in `ContentView` (attached last in the chain) and a `// MARK: Triage` section at the end of `Core.swift`. Obey the Execution plan's four merge-contention rules.

**Files:**
- Create: `apple/Yardstick/TriageSheet.swift`, `apple/YardstickTests/TriageKeyboardTests.swift`
- Modify: `apple/Yardstick/Core.swift` (`triageTarget`), `apple/Yardstick/ContentView.swift` (present the sheet)
- Test: `apple/YardstickTests/TriageKeyboardTests.swift`

**Interfaces:**
- Consumes: `Event.triageTask` (Task 2), generated `App.Bucket`.
- Produces, consumed by Task 9:
  - `enum TriageKey { static func intent(for character: Character) -> TriageIntent? }`
  - `enum TriageIntent: Equatable { case bucket(Bucket); case priority(UInt8) }`
  - `struct TriageSheet: View` — `init(title: String, initial: TriageDraft, onCommit: @escaping (TriageDraft) -> Void, onCancel: @escaping () -> Void)`
  - `struct TriageDraft: Equatable { var bucket: Bucket; var priority: UInt8; var due: String }`

**Riders:** none.

- [ ] **Step 1: Write the failing keyboard tests**

`apple/YardstickTests/TriageKeyboardTests.swift`:

```swift
import XCTest
@testable import Yardstick

/// Journey 1B's keyboard contract: N/E/L for when, 1/2/3 for priority.
/// `E` for Next (not `N`, which Now already owns) is the reference's own
/// choice and the thing most likely to be "corrected" by mistake later.
final class TriageKeyboardTests: XCTestCase {

    func testWhenKeys() {
        XCTAssertEqual(TriageKey.intent(for: "n"), .bucket(.now))
        XCTAssertEqual(TriageKey.intent(for: "e"), .bucket(.next))
        XCTAssertEqual(TriageKey.intent(for: "l"), .bucket(.later))
    }

    func testWhenKeysAreCaseInsensitive() {
        XCTAssertEqual(TriageKey.intent(for: "N"), .bucket(.now))
        XCTAssertEqual(TriageKey.intent(for: "E"), .bucket(.next))
        XCTAssertEqual(TriageKey.intent(for: "L"), .bucket(.later))
    }

    func testPriorityKeys() {
        XCTAssertEqual(TriageKey.intent(for: "1"), .priority(1))
        XCTAssertEqual(TriageKey.intent(for: "2"), .priority(2))
        XCTAssertEqual(TriageKey.intent(for: "3"), .priority(3))
    }

    func testUnboundKeysDoNothing() {
        // `#` opens the project/person linker in the reference, which needs
        // pages (Phase 3). Binding it now would be a key that lies.
        XCTAssertNil(TriageKey.intent(for: "#"))
        XCTAssertNil(TriageKey.intent(for: "0"))
        XCTAssertNil(TriageKey.intent(for: "4"))
        XCTAssertNil(TriageKey.intent(for: "f"), "F is focus — Phase 4")
        XCTAssertNil(TriageKey.intent(for: " "))
    }

    func testApplyingAnIntentLeavesEverythingElseAlone() {
        var draft = TriageDraft(bucket: .inbox, priority: 0, due: "2026-07-31")
        draft.apply(.bucket(.later))
        XCTAssertEqual(draft.bucket, .later)
        XCTAssertEqual(draft.priority, 0, "a when key must not clear priority")
        XCTAssertEqual(draft.due, "2026-07-31", "nor the due date")

        draft.apply(.priority(2))
        XCTAssertEqual(draft.priority, 2)
        XCTAssertEqual(draft.bucket, .later)
    }

    func testPressingTheSamePriorityAgainClearsIt() {
        // Priority is optional (handoff §Task), so the toggle needs a way
        // back to "none" without reaching for the mouse.
        var draft = TriageDraft(bucket: .now, priority: 2, due: "")
        draft.apply(.priority(2))
        XCTAssertEqual(draft.priority, 0)
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `just app-test`
Expected: FAIL — `cannot find 'TriageKey' in scope`.

- [ ] **Step 3: Implement the sheet**

`apple/Yardstick/TriageSheet.swift`:

```swift
import App
import SwiftUI

enum TriageIntent: Equatable {
    case bucket(Bucket)
    case priority(UInt8)
}

/// Journey 1B: N/E/L and 1/2/3. Kept pure so the mapping is testable
/// without a window (TriageKeyboardTests).
enum TriageKey {
    static func intent(for character: Character) -> TriageIntent? {
        switch Character(character.lowercased()) {
        case "n": return .bucket(.now)
        case "e": return .bucket(.next)
        case "l": return .bucket(.later)
        case "1": return .priority(1)
        case "2": return .priority(2)
        case "3": return .priority(3)
        default: return nil
        }
    }
}

struct TriageDraft: Equatable {
    var bucket: Bucket
    /// 0 = none.
    var priority: UInt8
    /// 'YYYY-MM-DD' or "".
    var due: String

    mutating func apply(_ intent: TriageIntent) {
        switch intent {
        case .bucket(let bucket):
            self.bucket = bucket
        case .priority(let priority):
            // Pressing the same digit again clears it — priority is optional.
            self.priority = self.priority == priority ? 0 : priority
        }
    }
}

/// Journey 1B — one lightweight sheet, three fields, keyboard-first.
/// The reference's fourth field (PROJECT / PERSON, opened with `#`) needs the
/// pages table and lands in Phase 3 (carve-out 1).
struct TriageSheet: View {
    let title: String
    let onCommit: (TriageDraft) -> Void
    let onCancel: () -> Void

    @State private var draft: TriageDraft
    @State private var hasDue: Bool
    @State private var dueDate: Date
    @FocusState private var keyboardFocus: Bool

    init(title: String,
         initial: TriageDraft,
         onCommit: @escaping (TriageDraft) -> Void,
         onCancel: @escaping () -> Void) {
        self.title = title
        self.onCommit = onCommit
        self.onCancel = onCancel
        _draft = State(initialValue: initial)
        _hasDue = State(initialValue: !initial.due.isEmpty)
        _dueDate = State(initialValue: Self.parse(initial.due) ?? Date())
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(title)
                .font(.system(size: 17, weight: .semibold))
                .foregroundStyle(Theme.textPrimary)

            field("WHEN") {
                Picker("", selection: $draft.bucket) {
                    Text("Now").tag(Bucket.now)
                    Text("Next").tag(Bucket.next)
                    Text("Later").tag(Bucket.later)
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .frame(width: 260)
            }

            field("PRIORITY") {
                HStack(spacing: 8) {
                    ForEach(UInt8(1)...UInt8(3), id: \.self) { value in
                        Button {
                            draft.apply(.priority(value))
                        } label: {
                            Text("\(value)")
                                .font(.system(size: 12, weight: .bold))
                                .foregroundStyle(draft.priority == value ? .white : Theme.textSecondary)
                                .frame(width: 26, height: 26)
                                .background(draft.priority == value
                                    ? (RowStyle.priorityColour(value) ?? Theme.priority3)
                                    : Theme.chipBg)
                                .clipShape(RoundedRectangle(cornerRadius: 6))
                        }
                        .buttonStyle(.plain)
                    }
                }
            }

            field("DUE") {
                HStack(spacing: 10) {
                    Toggle("", isOn: $hasDue).labelsHidden()
                    DatePicker("", selection: $dueDate, displayedComponents: .date)
                        .labelsHidden()
                        .disabled(!hasDue)
                        .opacity(hasDue ? 1 : 0.4)
                }
            }

            HStack {
                Spacer()
                Button("Cancel", action: onCancel).keyboardShortcut(.cancelAction)
                Button("Triage") {
                    draft.due = hasDue ? Self.iso(dueDate) : ""
                    onCommit(draft)
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
        .frame(width: 380)
        // Keyboard-first: the sheet takes key events itself, so N/E/L and
        // 1/2/3 work without tabbing to a control.
        .focusable()
        .focused($keyboardFocus)
        .onAppear { keyboardFocus = true }
        .onKeyPress { press in
            guard let character = press.characters.first,
                  let intent = TriageKey.intent(for: character) else { return .ignored }
            draft.apply(intent)
            return .handled
        }
    }

    @ViewBuilder
    private func field(_ label: String, @ViewBuilder content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label)
                .font(Theme.Typography.capsLabel)
                .tracking(0.66)
                .foregroundStyle(Theme.textMuted)
            content()
        }
    }

    private static let formatter: DateFormatter = {
        let f = DateFormatter()
        f.calendar = Calendar(identifier: .gregorian)
        f.locale = Locale(identifier: "en_US_POSIX")
        f.timeZone = .current
        f.dateFormat = "yyyy-MM-dd"
        return f
    }()

    static func iso(_ date: Date) -> String { formatter.string(from: date) }
    static func parse(_ iso: String) -> Date? { formatter.date(from: iso) }
}
```

`onKeyPress` with a single trailing closure exists on macOS 14+; if the signature does not match on this SDK, use `.onKeyPress(phases: .down) { press in ... }` and record the deviation in the PR. Do not fall back to `NSEvent` monitors — a global monitor would eat keys destined for the note editor.

- [ ] **Step 4: Present it from the row and the Inbox**

In `Core.swift`:

```swift
    /// Id of the task whose triage sheet is open, or nil.
    var triageTarget: String?

    func triage(id: String, draft: TriageDraft) {
        send(.triageTask(id: id, bucket: draft.bucket, priority: draft.priority, due: draft.due))
        triageTarget = nil
    }

    /// The row the sheet is editing, so the sheet opens on current values
    /// rather than defaults (principle 5: everything is always editable).
    func row(id: String) -> TaskRowVm? {
        view.list.groups.flatMap(\.rows).first { $0.id == id }
    }
```

`TaskRowVm` carries no bucket/priority/due in editable form (it carries display strings), so add the three raw fields the sheet needs to `TaskRowVm` in this task: `pub bucket: Bucket`, `pub priority: u8` (already there), `pub due: String`. Extend `build_row` accordingly and add one core test asserting they round-trip, in `shared/src/view/task_row.rs`:

```rust
    #[test]
    fn rows_carry_the_raw_values_the_triage_sheet_edits() {
        let mut t = task(Bucket::Next, Status::Backlog);
        t.due = "2026-07-31".into();
        t.priority = 2;
        let row = build_row(&t, TODAY);
        assert_eq!(row.bucket, Bucket::Next);
        assert_eq!(row.due, "2026-07-31", "the sheet opens on the current date");
        assert_eq!(row.priority, 2);
    }
```

In `ContentView.swift`, attach the sheet once at the top level (not per row — one sheet, whichever row asked):

```swift
        .sheet(item: $core.triageTarget) { id in
            let row = core.row(id: id)
            TriageSheet(
                title: "Triage · \(row?.title ?? "")",
                initial: TriageDraft(bucket: row?.bucket ?? .now,
                                     priority: row?.priority ?? 0,
                                     due: row?.due ?? ""),
                onCommit: { core.triage(id: id, draft: $0) },
                onCancel: { core.triageTarget = nil })
        }
```

`sheet(item:)` needs `Identifiable`; use `.sheet(isPresented:)` driven by `core.triageTarget != nil` with a `Binding` if the id-based overload fights `String`. Then point Task 5's `onOpenTriage` at `{ core.triageTarget = $0 }`.

- [ ] **Step 5: Run and check**

Run: `just test && just app-test && cd apple && just run`
Manual checklist (paste into the PR):
1. `+` capture "Finalize vendor contract" → Inbox shows it.
2. Right-click the row → Triage… → the sheet title reads "Triage · Finalize vendor contract".
3. Press `E` then `1` → the WHEN segment moves to Next, the priority 1 square turns red, without touching the mouse.
4. Press `1` again → priority clears.
5. Press `1`, enable DUE, pick a date, press Return → the row leaves Inbox, the Inbox count drops, and clicking Next shows it with a red `1` badge and the due weekday in the meta column.
6. Reopen its triage sheet → the sheet opens on Next / 1 / that date, not on defaults.
7. Relaunch → everything as left.

- [ ] **Step 6: Commit + PR**

```bash
git add apple shared
git commit -m "feat(apple): triage sheet with N/E/L and 1/2/3 keyboard"
git push -u origin p2/t7-triage
gh pr create --fill   # spec-deltas: none
```
STOP for review.

---

### Task 8: Six statuses, the blocked reason, and untick restore

**Wave 4, lane C — runs concurrently with Tasks 5 and 7.** Owns `StatusMenu.swift` outright; in the shared files it may add one `.sheet` modifier in `ContentView` (attached above lane B's) and a `// MARK: Status` section at the end of `Core.swift`. Obey the Execution plan's four merge-contention rules.

**Files:**
- Modify: `apple/Yardstick/StatusMenu.swift` (created in Task 4 with the bare six buttons — this task completes it), `apple/Yardstick/Core.swift` (`blockedReasonTarget`), `apple/Yardstick/ContentView.swift` (reason prompt)
- Create: `apple/YardstickTests/StatusMenuTests.swift`
- Test: `apple/YardstickTests/StatusMenuTests.swift`

**Interfaces:**
- Consumes: `Event.setStatus` (Task 2).
- Produces:
  - `struct StatusOption { let status: Status; let label: String; let hint: String; let key: String; let colour: Color }`, `StatusOption.all: [StatusOption]`
  - `struct StatusMenuItems: View` — `init(current: String, onSelect: @escaping (Status) -> Void)` (signature unchanged from Task 4)

**Riders:** none.

- [ ] **Step 1: Write the failing status-catalogue tests**

`apple/YardstickTests/StatusMenuTests.swift`:

```swift
import XCTest
@testable import Yardstick

/// Journey 5A ships six statuses with verbatim one-line hints, in a fixed
/// order, each with its dot colour. These are product copy: a test is the
/// only thing that stops them drifting.
final class StatusMenuTests: XCTestCase {

    func testAllSixStatusesInTheDesignedOrder() {
        XCTAssertEqual(
            StatusOption.all.map(\.label),
            ["Backlog", "In progress", "Blocked", "Waiting", "Done", "Binned"])
    }

    func testHintsAreVerbatimFromTheReference() {
        XCTAssertEqual(
            StatusOption.all.map(\.hint),
            [
                "Someday / unstarted",
                "Actively on it",
                "Can't proceed",
                "On someone else",
                "Complete",
                "Dropped",
            ])
    }

    func testDotColoursComeFromTheStatusTokens() {
        let byLabel = Dictionary(uniqueKeysWithValues: StatusOption.all.map { ($0.label, $0.colour) })
        XCTAssertEqual(byLabel["Backlog"], Theme.statusBacklog)
        XCTAssertEqual(byLabel["In progress"], Theme.statusInProgress)
        XCTAssertEqual(byLabel["Blocked"], Theme.statusBlocked)
        XCTAssertEqual(byLabel["Waiting"], Theme.statusWaiting)
        XCTAssertEqual(byLabel["Done"], Theme.statusDone)
        XCTAssertEqual(byLabel["Binned"], Theme.statusBinned)
    }

    func testStatusKeysMatchTheCoresStringsSoTheCheckmarkLandsOnTheRightRow() {
        XCTAssertEqual(
            StatusOption.all.map(\.key),
            ["backlog", "in_progress", "blocked", "waiting", "done", "binned"])
    }

    func testOnlyBlockedNeedsAReasonPrompt() {
        XCTAssertTrue(StatusOption.needsReason(.blocked))
        XCTAssertFalse(StatusOption.needsReason(.waiting))
        XCTAssertFalse(StatusOption.needsReason(.done))
        XCTAssertFalse(StatusOption.needsReason(.backlog))
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `just app-test`
Expected: FAIL — `cannot find 'StatusOption' in scope`.

- [ ] **Step 3: Implement the catalogue and the menu**

`apple/Yardstick/StatusMenu.swift`:

```swift
import App
import SwiftUI

/// Journey 5A's six statuses: label, verbatim hint, dot colour, and the
/// core's own key string so the current-selection checkmark can match.
struct StatusOption: Identifiable {
    let status: Status
    let label: String
    let hint: String
    let key: String
    let colour: Color

    var id: String { key }

    static let all: [StatusOption] = [
        .init(status: .backlog, label: "Backlog", hint: "Someday / unstarted",
              key: "backlog", colour: Theme.statusBacklog),
        .init(status: .inProgress, label: "In progress", hint: "Actively on it",
              key: "in_progress", colour: Theme.statusInProgress),
        .init(status: .blocked, label: "Blocked", hint: "Can't proceed",
              key: "blocked", colour: Theme.statusBlocked),
        .init(status: .waiting, label: "Waiting", hint: "On someone else",
              key: "waiting", colour: Theme.statusWaiting),
        .init(status: .done, label: "Done", hint: "Complete",
              key: "done", colour: Theme.statusDone),
        .init(status: .binned, label: "Binned", hint: "Dropped",
              key: "binned", colour: Theme.statusBinned),
    ]

    /// Spec §7: setting Blocked prompts for an optional one-line reason.
    /// Nothing else does — Waiting's "who" comes from person links (Phase 3).
    static func needsReason(_ status: Status) -> Bool { status == .blocked }
}

/// The six menu rows, used from a row's context menu and the All-actions
/// view's bulk menu. A `Menu` in the caller wraps these.
struct StatusMenuItems: View {
    let current: String
    let onSelect: (Status) -> Void

    var body: some View {
        Menu("Set status") {
            ForEach(StatusOption.all) { option in
                Button {
                    onSelect(option.status)
                } label: {
                    // Menu rows cannot draw a coloured dot on macOS, so the
                    // checkmark carries the current state and the hint gives
                    // the one-line description from the reference.
                    if option.key == current {
                        Label("\(option.label) — \(option.hint)", systemImage: "checkmark")
                    } else {
                        Text("\(option.label) — \(option.hint)")
                    }
                }
            }
        }
    }
}
```

The generated Swift case names for `Status` may be `.inProgress` or `.in_progress` depending on facet's casing; check `apple/generated/App` after `just generate` and use what is generated. Same for `Bucket`. Note the actual casing in the PR so later tasks do not guess.

- [ ] **Step 4: Prompt for the blocked reason**

In `Core.swift`:

```swift
    /// Set when the user picks Blocked and a reason has not been given yet.
    var blockedReasonTarget: String?

    func setStatus(id: String, status: Status) {
        if StatusOption.needsReason(status) {
            blockedReasonTarget = id
            return
        }
        send(.setStatus(id: id, status: status, reason: ""))
    }

    func commitBlockedReason(id: String, reason: String) {
        send(.setStatus(id: id, status: .blocked, reason: reason))
        blockedReasonTarget = nil
    }
```

In `ContentView.swift`, a small sheet with one text field, "Blocked" and Cancel; Return commits (an empty reason is allowed — the reference calls it optional), Escape cancels and leaves the status untouched.

- [ ] **Step 5: Run and check**

Run: `just app-test && cd apple && just run`
Manual checklist (paste into the PR):
1. Right-click a Now row → Set status → all six rows appear with their hints, and the current one carries a checkmark.
2. Pick Blocked → the reason prompt appears → type "Legal review" → Return → the row shows a red "Blocked" pill and the reason beneath it.
3. Pick In progress → the pill turns blue, the reason disappears (it belongs to Blocked only), and the checkbox gains the soft blue centre.
4. Pick Blocked → Escape at the prompt → the status is unchanged.
5. Tick the checkbox on the Blocked row, then untick it → the row returns to **Blocked**, not Backlog (spec §7's `prev_status` restore, end to end).
6. Relaunch → statuses and reasons intact.

- [ ] **Step 6: Commit + PR**

```bash
git add apple
git commit -m "feat(apple): six-status menu with blocked reason and untick restore"
git push -u origin p2/t8-status
gh pr create --fill   # spec-deltas: none
```
STOP for review.

---

### Task 9: The All-actions view

**Files:**
- Create: `apple/Yardstick/AllActionsView.swift`, `apple/YardstickTests/BulkEditTests.swift`
- Modify: `apple/Yardstick/ContentView.swift` (the `"all"` route case), `apple/Yardstick/SidebarView.swift` (an "All actions" row), `shared/src/view/sidebar.rs` (that row + its count), `apple/Yardstick/Core.swift` (bulk senders)
- Test: `apple/YardstickTests/BulkEditTests.swift`, `shared/src/view/sidebar.rs`

**Interfaces:**
- Consumes: `TaskListView` (Task 5), `TriageKey`/`TriageIntent` (Task 7), `StatusMenuItems` (Task 8), `Event.bulkUpdateTasks` and `Event.setGrouping`/`setFilter` (Tasks 2, 3).
- Produces: `struct AllActionsView: View`; `enum BulkEdit { static func payload(for intent: TriageIntent, ids: [String]) -> BulkPayload }`, `struct BulkPayload: Equatable { let ids: [String]; let bucket: Bucket?; let priority: UInt8?; let status: Status? }`.

**Riders:** this view supersedes the handoff's "All tasks · by status" board (spec §6). The board is not built, ever — say so in the PR so no later phase resurrects it.

**Acceptance criteria** (there is no mock; spec §6 delegates the shape to this plan):
1. Every task in the space appears, regardless of bucket or status — including Done and Binned, which no other surface shows in full.
2. A **Group by** control: Status (default, Journey 5B's four groups plus Backlog/Binned collapsed to counts), Bucket (Inbox/Now/Next/Later), None (one flat list).
3. **Filter** chips for bucket and status; a filter and a grouping compose, and the active filters are visible and clearable in one click.
4. Rows are the Task 4 row, so a task looks the same here as in Today.
5. **Multi-select** with click, shift-click and ⌘-click; the selection count is visible.
6. With a selection: `N`/`E`/`L` set the bucket, `1`/`2`/`3` set the priority, and the toolbar offers Set status. Each is one `BulkUpdateTasks` event, not one per row.
7. **Inline title editing** on double-click, committing on Return, reverting on Escape.
8. Empty groups render "Nothing here", never disappear silently, so the grouping stays legible.

- [ ] **Step 1: Write the failing bulk-payload tests**

`apple/YardstickTests/BulkEditTests.swift`:

```swift
import XCTest
@testable import Yardstick

/// A bulk edit must be exactly one event carrying every selected id, and it
/// must leave unmentioned fields alone — the core reads `nil` as "don't
/// touch", so a wrong `nil` here silently rewrites 30 tasks.
final class BulkEditTests: XCTestCase {

    func testBucketIntentSetsOnlyTheBucket() {
        let payload = BulkEdit.payload(for: .bucket(.later), ids: ["a", "b"])
        XCTAssertEqual(payload.ids, ["a", "b"])
        XCTAssertEqual(payload.bucket, .later)
        XCTAssertNil(payload.priority)
        XCTAssertNil(payload.status)
    }

    func testPriorityIntentSetsOnlyThePriority() {
        let payload = BulkEdit.payload(for: .priority(1), ids: ["a"])
        XCTAssertEqual(payload.priority, 1)
        XCTAssertNil(payload.bucket)
        XCTAssertNil(payload.status)
    }

    func testAnEmptySelectionProducesNoIds() {
        // The caller must be able to check this and skip the send entirely.
        XCTAssertTrue(BulkEdit.payload(for: .priority(2), ids: []).ids.isEmpty)
    }

    func testSelectionOrderIsPreservedForPredictableUndoTalk() {
        let payload = BulkEdit.payload(for: .bucket(.now), ids: ["c", "a", "b"])
        XCTAssertEqual(payload.ids, ["c", "a", "b"])
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `just app-test`
Expected: FAIL — `cannot find 'BulkEdit' in scope`.

- [ ] **Step 3: Implement the view**

`apple/Yardstick/AllActionsView.swift`:

```swift
import App
import SwiftUI

struct BulkPayload: Equatable {
    let ids: [String]
    let bucket: Bucket?
    let priority: UInt8?
    let status: Status?
}

/// One selection + one intent = one event (plan decision #10).
enum BulkEdit {
    static func payload(for intent: TriageIntent, ids: [String]) -> BulkPayload {
        switch intent {
        case .bucket(let bucket):
            return BulkPayload(ids: ids, bucket: bucket, priority: nil, status: nil)
        case .priority(let priority):
            return BulkPayload(ids: ids, bucket: nil, priority: priority, status: nil)
        }
    }
}

/// Every task in the space, in one editable list. Supersedes the handoff's
/// "All tasks · by status" board (spec §6): status grouping is one option
/// here rather than a separate screen.
struct AllActionsView: View {
    let list: TaskListVm
    let onToggleDone: (String) -> Void
    let onOpenTriage: (String) -> Void
    let onSetStatus: (String, Status) -> Void
    let onEditTitle: (String, String) -> Void
    let onBulk: (BulkPayload) -> Void
    let onGroupBy: (String) -> Void
    let onFilter: (String, String) -> Void

    @State private var selection = Set<String>()
    @State private var editingID: String?
    @State private var draftTitle = ""

    private var orderedSelection: [String] {
        list.groups.flatMap(\.rows).map(\.id).filter { selection.contains($0) }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            controls
            List(selection: $selection) {
                ForEach(Array(list.groups.enumerated()), id: \.offset) { _, group in
                    Section {
                        if group.rows.isEmpty {
                            Text("Nothing here")
                                .font(Theme.Typography.meta)
                                .foregroundStyle(Theme.countEmpty)
                        }
                        ForEach(group.rows, id: \.id) { row in
                            rowView(row)
                                .tag(row.id)
                        }
                    } header: {
                        if !group.label.isEmpty {
                            HStack(spacing: 8) {
                                Text(group.label)
                                Text("\(group.count)").foregroundStyle(Theme.textMuted)
                            }
                        }
                    }
                }
                if !list.collapsed.isEmpty {
                    Section {
                        HStack(spacing: 14) {
                            ForEach(Array(list.collapsed.enumerated()), id: \.offset) { _, group in
                                Text("\(group.label) · \(group.count)")
                                    .font(Theme.Typography.meta)
                                    .foregroundStyle(Theme.textQuiet)
                            }
                        }
                    }
                }
            }
            .listStyle(.inset)
            .onKeyPress { press in
                guard !selection.isEmpty,
                      let character = press.characters.first,
                      let intent = TriageKey.intent(for: character) else { return .ignored }
                onBulk(BulkEdit.payload(for: intent, ids: orderedSelection))
                return .handled
            }
        }
        .background(Color.white)
    }

    @ViewBuilder
    private func rowView(_ row: TaskRowVm) -> some View {
        if editingID == row.id {
            TextField("", text: $draftTitle)
                .textFieldStyle(.plain)
                .font(Theme.Typography.body)
                .onSubmit {
                    let trimmed = draftTitle.trimmingCharacters(in: .whitespaces)
                    if !trimmed.isEmpty, trimmed != row.title {
                        onEditTitle(row.id, trimmed)
                    }
                    editingID = nil
                }
                .onExitCommand { editingID = nil }
        } else {
            TaskRow(row: row,
                    onToggleDone: { onToggleDone(row.id) },
                    onOpenTriage: { onOpenTriage(row.id) },
                    onSetStatus: { onSetStatus(row.id, $0) })
                .onTapGesture(count: 2) {
                    draftTitle = row.title
                    editingID = row.id
                }
        }
    }

    private var controls: some View {
        HStack(spacing: 14) {
            Picker("Group by", selection: Binding(
                get: { list.groupBy },
                set: { onGroupBy($0) })) {
                    Text("Status").tag("status")
                    Text("Bucket").tag("bucket")
                    Text("None").tag("none")
                }
                .pickerStyle(.segmented)
                .frame(width: 220)

            Picker("Bucket", selection: Binding(
                get: { list.filterBucket },
                set: { onFilter($0, list.filterStatus) })) {
                    Text("Any bucket").tag("")
                    Text("Inbox").tag("inbox")
                    Text("Now").tag("now")
                    Text("Next").tag("next")
                    Text("Later").tag("later")
                }
                .frame(width: 130)

            Picker("Status", selection: Binding(
                get: { list.filterStatus },
                set: { onFilter(list.filterBucket, $0) })) {
                    Text("Any status").tag("")
                    ForEach(StatusOption.all) { option in
                        Text(option.label).tag(option.key)
                    }
                }
                .frame(width: 140)

            if !list.filterBucket.isEmpty || !list.filterStatus.isEmpty {
                Button("Clear filters") { onFilter("", "") }
                    .buttonStyle(.link)
            }

            Spacer()

            if !selection.isEmpty {
                Text("\(selection.count) selected")
                    .font(Theme.Typography.meta)
                    .foregroundStyle(Theme.textSecondary)
                StatusMenuItems(current: "") { status in
                    onBulk(BulkPayload(ids: orderedSelection, bucket: nil,
                                       priority: nil, status: status))
                }
            }
        }
        .padding(EdgeInsets(top: 14, leading: Theme.Metrics.contentPaddingH,
                            bottom: 10, trailing: Theme.Metrics.contentPaddingH))
    }
}
```

In `Core.swift`:

```swift
    func bulk(_ payload: BulkPayload) {
        guard !payload.ids.isEmpty else { return }
        send(.bulkUpdateTasks(ids: payload.ids, bucket: payload.bucket,
                              priority: payload.priority, status: payload.status))
    }
```

- [ ] **Step 4: Add the sidebar row**

In `shared/src/view/sidebar.rs`, append one row after Inbox and extend the Step 9 test's expected vector in the same commit:

```rust
            view_row("all", "All actions", model.tasks.iter().filter(|t| is_open(t.status)).count() as u64),
```

Give it its own icon in `SidebarView.viewIcon` (three stacked 1.5px lines, 11px wide, `Theme.textMuted`) so it does not fall through to the Inbox tray.

- [ ] **Step 5: Run and check**

Run: `just test && just app-test && cd apple && just run`
Manual checklist (paste into the PR), against the eight acceptance criteria above:
1. Capture five tasks, triage two to Now with P1/P2, block one with a reason, tick one done.
2. Click All actions → Status grouping shows In progress / Blocked / Waiting / Done groups with counts, and "Backlog · n" / "Binned · n" at the bottom.
3. Switch to Bucket → four groups; switch to None → one flat list in priority-then-age order.
4. Filter to Bucket = Now → only Now tasks, across every group; add Status = Blocked → composes; Clear filters → back.
5. Select three rows with ⌘-click → "3 selected" appears → press `L` → all three move to Later in one go; the sidebar Later count jumps by three.
6. With a selection, use Set status → Waiting → all three become Waiting.
7. Double-click a title → edit → Return → the new title shows everywhere (check Today too); double-click, change, Escape → unchanged.
8. Relaunch → every change intact.

- [ ] **Step 6: Commit + PR**

```bash
git add apple shared
git commit -m "feat(apple): All-actions view — group, filter, sort, bulk edit"
git push -u origin p2/t9-all-actions
gh pr create --fill
```

PR description records: the eight acceptance criteria with pass/fail for each, and the spec delta **none** (spec §6 already records the board's supersession).
STOP for review.

---

### Tasks 10a and 10: end-to-end proofs, then phase close

**This section is two PRs** (see the Execution plan). **Task 10a** is Steps 1–2 below, on branch `p2/t10a-e2e-proofs`, running in wave 3 alongside Task 4: the proofs touch core, router and store only, so they land before any UI is built on them. **Task 10** is Steps 3–6, on branch `p2/t10-phase-close`, in wave 7 once every other task has merged.

**Files — Task 10a:**
- Create: `runtime/tests/tasks_flow.rs`
- Test: `runtime/tests/tasks_flow.rs`

**Files — Task 10:**
- Modify: `README.md` (current-plan pointer), `docs/superpowers/plans/2026-07-29-phase-2-tasks.md` (final checkboxes)

**Interfaces:**
- Consumes: `runtime::AppRuntime`, `runtime/tests/common/{NullShell, poll_until}` (Phase 1 Task 2), the events from Task 2 and the ViewModel from Task 3. **No Swift dependency** — that is what lets 10a run early.

**Riders:** none.

- [ ] **Step 1: Write the failing end-to-end proofs** *(Task 10a)*

`runtime/tests/tasks_flow.rs`:

```rust
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

fn started(name: &str) -> (AppRuntime, std::path::PathBuf) {
    let dir = temp_dir(name);
    let db = dir.join("daily.db");
    let rt = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup { today: TODAY.into() });
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
    assert!(rt.view().list.groups[0].rows.iter().all(|r| r.priority == 3));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn everything_survives_a_restart_on_the_same_database() {
    let dir = temp_dir("restart");
    let db = dir.join("daily.db");
    let id = {
        let rt = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
        rt.send_event(Event::Startup { today: TODAY.into() });
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
    rt.send_event(Event::Startup { today: TODAY.into() });
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
```

Note `Startup` lands on route `"today"`, whose list is the Now list (Task 3's `view()`), which is why the restart test reads `list.title == "Now"` without selecting a view.

- [ ] **Step 2: Run them**

Run: `cargo nextest run -p runtime`
Expected: PASS. A failure here is a real wiring bug between core, router and store — fix it in this PR and note it in the description.

Then `just test && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --check`, and commit + PR — **this ends Task 10a**:

```bash
git add runtime
git commit -m "test(runtime): end-to-end task-flow proofs"
git push -u origin p2/t10a-e2e-proofs
gh pr create --fill   # spec-deltas: none
```
STOP for review. Everything below is Task 10, in wave 7.

- [ ] **Step 3: Run everything** *(Task 10 starts here)*

Run: `just test && just app-test`
Expected: all green. Paste both summaries into the PR.

- [ ] **Step 4: Whole-phase manual E2E (the phase-gate dry run — paste results into the PR)**

Run `cd apple && just run`:
1. Fresh launch → Today: note editable, Now section empty, sidebar counts all zero and muted.
2. Capture four tasks with `+`; Inbox count reads 4; Inbox shows them with `quick add` tags.
3. Triage one to Now/P1/no due, one to Next/P2/Friday, one to Later/no priority; leave one in Inbox.
4. Today → the Now task appears with a red 1 badge; "0 done · 1 to go".
5. Tick it → dimmed, struck through, sinks; "1 done · 0 to go", one green pip.
6. Block the Next task with reason "Legal review"; check the pill and the reason.
7. Tick then untick the blocked task → returns to Blocked.
8. All actions → group by Status, then Bucket, then None; filter and clear; select two rows and press `L`; use Set status → Waiting.
9. Waiting on → both appear, from whichever bucket.
10. `⌘Q`, relaunch → every state above is exactly as left.
11. From Claude Code, call the MCP `create_task` tool → the task appears in the Inbox live, tagged `from an agent`, with no user action.
12. Walk reference §7.1/§7.2 and Journeys 1B/5A against the running build; list every residual deviation with its rationale (expected: carve-outs 1–7).

- [ ] **Step 5: Review sweep (do, then record in the PR)**

1. Every checkbox in this plan ticked across the merged PRs; every task's Riders line satisfied or explicitly moved.
2. `grep -rn "TODO\|FIXME\|unimplemented\|todo!" shared store mcp runtime apple/Yardstick` → empty.
3. Confirm `apple/generated/` is untouched in every merged PR.
4. Confirm no Phase 0 remnants: `grep -rn "InsertTask\|ListTasks\|CreateTask {" shared store mcp runtime` → only `StorageOperation::CreateTask` survives.
5. `cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --check`.
6. Update `README.md`'s "Current plan" link to this plan.

- [ ] **Step 6: Commit + PR**

```bash
git add runtime README.md docs
git commit -m "chore(p2): phase close — end-to-end proofs, docs, review sweep"
git push -u origin p2/t10-phase-close
gh pr create --fill   # spec-deltas: none
```

After Jon merges and uses the build, he tags: `git tag phase-2 && git push origin phase-2` (Jon's action, per SDLC — the phase gate is his call, not this plan's).

---

## Self-review notes (checks performed while writing this plan; findings fixed in place)

**Spec coverage walk** — spec §10 Phase 2 reads "Tasks — model, buckets/status/priority, task rows, triage sheet + keyboard (N/E/L, 1/2/3, #), Inbox, **All-actions view** (replaces the status board)":
- model → T1 (schema), T2 (domain + events).
- buckets/status/priority → T1, T2, and the pure ordering rules in T2's `task.rs`.
- task rows → T4 (row), T5 (Now section), acceptance = reference §7.1/§7.2.
- triage sheet + keyboard → T7. **`#` is deliberately excluded** (carve-out 1) because pages are Phase 3; recorded rather than silently dropped.
- Inbox → T6.
- All-actions view replacing the status board → T9, with the supersession recorded in its Riders line.
- Spec §3's task columns: all present except `parent_id` (carve-out 6, Phase 5) and the task `notes` field (no Phase 2 surface shows it; add it with the surface that does).
- Spec §4's event list: `CaptureTask`, `TriageTask`, `SetStatus`, `ToggleDone` all land in T2. `ConvertLineToTask`, `SortBriefAction`, `CombineAction`, `ResurfaceDecision`, `WriteBrief`, `StartFocus`, `DayRollover` belong to Phases 3–6 and are not touched.
- Spec §9's testing strategy: every journey rule that Phase 2 owns became a core test (triage defaults, done/undone restore, ordering, momentum counts, grouping/collapsing), plus five runtime end-to-end proofs in T10.

**Placeholder scan** — no TBD/TODO in the plan; every code step carries the actual code. Three places name a check the implementer must make against reality rather than trusting this document, each with a named arbiter and an instruction not to invent an API: the generated Swift enum casing (T8 Step 3, arbiter `apple/generated/App` after `just generate`), `onKeyPress`'s signature on this SDK (T7 Step 3, arbiter the first `just app-test`), and `sheet(item:)` versus `isPresented` (T7 Step 4, same arbiter). That is the Phase 1 pattern, not a placeholder.

**Type-consistency check** — issues found while cross-checking task interfaces, each fixed in the task text above:
1. `TaskRowVm` originally carried only display strings, so T7's sheet had nothing to open on — fixed by adding `bucket`/`due` to the row in T7 Step 4 with its own core test, rather than having the shell re-derive them.
2. `StatusMenuItems` is used by T4's context menu but defined in T8's file — fixed by having T4 create `StatusMenu.swift` with the six bare buttons and T8 complete it, so both tasks compile alone.
3. T3's `build_list` takes `route` as `&str` while `Model.route` is `String`; the `view()` snippet dereferences correctly (`&model.route`) and the `"today"` special case is explicit.
4. `TaskListVm` needs `Clone` for T3's `base.clone()`; the derive list includes it.
5. T9's sidebar row addition breaks T3 Step 9's `counts()` assertion (a sixth row appears) — T9 Step 4 says to extend that test in the same commit.
6. The Phase 1 `Core.swift` `ViewModel(...)` seed literal gains two fields in T3 Step 16; without that the app fails to build mid-phase.
7. `age_in_days` needs a public day-number function that `civil.rs` keeps private — T2 Step 3 publishes `days_since_epoch` with its own test rather than duplicating Hinnant's arithmetic.
8. Every `StorageResult` variant a task adds is matched explicitly in `update()` with `Error` and `wrong_shape` arms (T2 Step 8), preserving the Phase 1 rule that no result shape is silently swallowed.

**Right-sizing check** — T2 and T3 are the two largest tasks (eleven and sixteen steps). Both were candidates for splitting and both stay whole: T2's arms share `apply_status` and would be untestable in halves, and T3's split-plus-add is only safe as one commit because the moved functions must keep their tests passing in the same PR. Every other task is under eight steps.

## After Phase 2

Phase 3 (**Pages, meetings and backlinks** — spec §3.1 and §10 as amended 2026-07-29) gets its own plan after Jon uses this build. Carried forward explicitly so nothing silently drops:
- **Phase 3 owns:** `pages` (project/person/meeting kinds, one-level nesting), migration 003's successor rebuilding `notes` for page notes, `source_links`, `links` population, the `@`/`#` pickers and chips in the editor, `TaskRowVm.chips` finally filling, the triage sheet's PROJECT / PERSON field and its `#` key, `[ ]` inline conversion, and the Meetings sidebar section. It also revisits the single-`StorageOperation`-enum decision at its own gate.
- **Phase 4 owns:** the Time capability (day rollover, replacing the launch-time `today`), the focus bar and sessions, the `F` key, the "Focusing" pill, app-wide dimming, and the done row's completion time (carve-out 3).
- **Phase 5 owns:** the brief, `write_brief`/`write_meeting`/`get_meeting`/`list_meetings`/`create_page`, the Actions-from-yesterday block, Combine operations (bringing `parent_id`, carve-out 6), the toolbar search field, `search`/`get_day` MCP tools, and the ledger's port-collision fallback plus discovery file.
- **Phase 6 owns:** resurfacing, rollover polish, collapsed Next/Later summaries in Today, spaces and the switcher.
- **Phase 7 owns:** the global capture hotkey (retiring carve-out 5's context), the menu-bar extra, and the Todoist/Craft importer.
- **Polish backlog (any phase):** sidebar system-material look, storage-drain acknowledgement on quit (replacing the 200 ms bound), a task `notes` field when a surface needs it, and drag-and-drop between buckets (carve-out 7).
