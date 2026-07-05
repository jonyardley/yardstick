# Daily — Phase 1: Shell + Notes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Daily window looks like the design: real macOS chrome, the 238px sidebar with space row, mini calendar, and Views list, and a daily-note column with a plain-text editor. Typing persists (notes → blocks → SQLite + FTS), the calendar navigates between days, and everything survives a relaunch. Under the surface, the Phase 0 review's hardening mandates are retired: migration failures return errors instead of crash-looping, the MCP reader reads SQLite instead of the core view, test helpers are consolidated, and the StorageOperation growth strategy is decided and applied.

**Architecture:** Unchanged from Phase 0 (spec §2): pure Crux core (`shared`) emits typed `StorageOperation` effects; `runtime` routes storage to a Rust thread owning rusqlite; non-storage effects reach the SwiftUI shell over BoltFFI; the embedded rmcp server dispatches core events for writes and now reads via a read-only SQLite connection. Phase 1 grows the core's domain (days, notes, blocks, calendar), the store's schema (migration 002: `notes`, `blocks`, `search` FTS5, `links`), and replaces the walking-skeleton UI with the real shell per spec §6 and the design reference.

**Tech Stack:** Rust edition 2024 / crux_core 0.19 / boltffi =0.25.2 / facet =0.44 / rusqlite 0.39 (bundled) / rusqlite_migration 2.5 / rmcp =2.1.0 / axum 0.8 / tokio 1 / SwiftUI (macOS 15.0) / XcodeGen / just / cargo-nextest 0.9.128 locally.

## Global Constraints

- **Pins (exact, never float silently):** `facet = "=0.44"`, `boltffi = "=0.25.2"` (+ `boltffi_cli =0.25.2`), `rmcp = "=2.1.0"`, `rusqlite = "0.39"` (bundled), `rusqlite_migration = "2.5"`, `crux_core = "0.19"`, toolchain `1.90`, macOS deployment target 15.0. CI installs with `--locked`. Pin changes require a spec amendment PR first.
- **No new external dependencies.** Phase 1 needs date math in the core — it is hand-rolled in `shared/src/civil.rs` (~100 lines of pure integer math), NOT chrono. The only new dependency edge is `serde_json` into `store` (already a workspace dependency used by `mcp`; same precedent as Phase 0's `subtle`).
- **Crate DAG (never violate):** `shared → crux_core` only; `store → shared`; `mcp → shared, store`; `runtime → shared, store, mcp`. `mcp` must NOT depend on `runtime`. No I/O, clocks, randomness, or tokio in `shared`. IDs (UUIDv7) are generated in `store`, never in `shared`. All new SQL tables STRICT (FTS5 virtual tables excepted — SQLite does not support STRICT there); every entity table has `space_id`, `created_at`, `updated_at`, `deleted_at`.
- **Workflow (docs/SDLC.md):** each task runs on its own branch `p1/t<N>-<slug>` cut from latest `main`. TDD strictly: failing test → observe failure → minimal implementation → observe pass → commit. A task's final "Commit + PR" step means: push, open the PR (conventional title, template filled including the **"Spec deltas introduced"** section, TDD evidence pasted, this plan's checkboxes ticked in the same PR), then STOP — Jon reviews and squash-merges. Never merge your own PR. Run `just test` before claiming any Rust task done, and `just app` before claiming any Swift task done.
- **Riders rule (new this phase):** every task below carries a **Riders** line naming which Phase 0 ledger riders it absorbs (`none` explicitly otherwise). A rider may not be silently dropped; if a rider turns out not to fit its task, move it to another task in a plan-amendment commit inside the same PR.
- **Pixel-fidelity rule (shell tasks 6–8):** `docs/design/reference/v2-today-view.md` **§1 (title bar), §2 (sidebar, all of §§2.1–2.9), §5 (daily note)** are the acceptance criteria for all chrome built this phase, with these explicit carve-outs, decided here so no UI looks dead and no data is fake:
  - Real macOS window chrome (traffic lights, title bar, toolbar) replaces the mocked ones — the handoff README's stated fidelity adaptation.
  - The toolbar **search field is omitted** in Phase 1 (search ships Phase 5; a non-functional search field is dead UI). The `+` button ships and works (quick-add popover → `CreateTask`, the Phase 0 write path).
  - Sidebar **Views rows render with live counts**: Now/Next/Later/Waiting on are genuinely 0 (buckets arrive Phase 2) and render in the muted empty-count color `#b8b8b6`; **Inbox count is real** (every task is inbox until buckets exist). Views rows are non-interactive this phase except "Today".
  - **Projects / People / Pages sections render only when they have rows** — they are data-driven from the ViewModel (empty vectors in Phase 1), so they are simply absent, not greyed-out mockery.
  - The space switcher row renders the real current space ("Red Badger" = `store::DEFAULT_SPACE_ID`'s row) and is non-interactive until Phase 6.
- **Canonical upstream references** when an API doesn't match this plan: crux repo `examples/counter-routing` (EffectRouter/FFI), `examples/counter` + `examples/weather` (Swift shell mechanics), rust-sdk `examples/servers` + this repo's `mcp/tests/tools.rs` as-built client syntax (task-6 report: `auth_header` takes the bare token, `CallToolRequestParams` plural, `ContentBlock` not `Content`). For AppKit editor details, Apple's TextKit 2 docs / WWDC22 sample are canonical and the named arbiter test decides. Do not invent APIs from training data; mirror the example and note the deviation in the PR.
- Commit after every green test cycle. Run Rust tests with `cargo nextest run -p <crate>`; the whole suite with `just test`.

## Decisions made in this plan (so they aren't re-litigated mid-task)

1. **StorageOperation growth: ONE enum, domain-grouped variants** (not multiple `Operation` types). Rationale: every additional `Operation` type costs a new `Effect` variant, a new router arm, a new handler channel in `runtime`, and more typegen surface — while only partially improving shape-safety (each domain's output would still be an enum). The single enum keeps one storage lane, one executor, one thread. The real safety fix is made in Task 5: **every result-carrying event handler matches its expected `StorageResult` variant explicitly and routes anything else to a visible `wrong_shape` error** (no silent catch-all re-render). Revisit at the Phase 2 gate: if the enum passes ~15 variants or two domains need separate handler threads, split then.
2. **Editor: `NSTextView` (TextKit 2) in `NSViewRepresentable` from day one**, per spec §6 — not SwiftUI `TextEditor`. TextEditor on macOS 15 is plain-string (sufficient for Phase 1) but the Phase 3 swap cost would be the entire editor plumbing (coordinator, debounce, version guard, first-responder behavior) rebuilt on NSTextView anyway, and TextEditor cannot cleanly hit the 14px/1.65 typographic spec. The Phase 1 wrapper is deliberately thin (~120 lines): no tokens, no pickers, no attachments — those are Phase 3 additions to this same view.
3. **Phase 1 block model: full-rewrite paragraphs.** The editor edits the whole day as one text flow; the core splits on `\n` into paragraph blocks (pure, testable); the store rewrites the note's blocks in one transaction (get-or-create note row, delete old blocks + their FTS rows, insert new blocks + FTS rows). Consequences, recorded: block IDs are not stable across saves (fine until Phase 3's block-level editing, which changes the editing model anyway), and superseded block rows are **hard-deleted** inside the transaction — the note row is the soft-delete/tombstone unit. The `deleted_at` column stays on `blocks` for Phase 3+. This is a deliberate, narrow delta against spec §3's blanket soft-delete convention; Task 4's PR records it in the spec changelog.
4. **MCP with an in-memory database is an error**, not a silently-different reader. Phase 0's `ViewReader` (reads from the core view) existed only because an in-memory runtime has no shareable DB file. Phase 1 deletes it (delete-don't-pause): `start_mcp` requires an on-disk path and returns `Err` otherwise; tests use temp-file databases. Production is always on-disk, so nothing user-visible changes.
5. **Migration-failure UX: message window + Quit.** If the DB can't be opened/migrated (e.g. a future schema version after a downgrade), `CoreFFI` reports a non-empty `init_error()` instead of panicking; the shell shows a calm explanation window with a Quit button and never sends events. No auto-retry, no crash-loop. The FFI *decode* contract (panic on typegen mismatch, spec §8 as amended) is unchanged — that is a programmer error; a failing migration is an environment condition.
6. **Startup date comes from the shell** (`Event::Startup { today }`): the core stays clock-free. The Time capability (rollover at midnight) is Phase 4+ scope; until then "today" is fixed per launch — acceptable and recorded (relaunching after midnight picks up the new day).

## File structure (locked decomposition)

```
shared/src/
├── lib.rs                 # modified: export civil + new types
├── civil.rs               # NEW (T5): pure Gregorian date math, no deps
├── app.rs                 # modified (T5): day model, events, ViewModel
└── effects/storage.rs     # modified (T4): GetDay/ReplaceDayBlocks, DayData
store/
├── migrations/002_notes.sql  # NEW (T4): notes, blocks, search FTS5, links
├── src/db.rs              # modified (T1: Err on migration failure; T3: open_read_only)
└── src/executor.rs        # modified (T4): get_day / replace_day_blocks
mcp/src/
├── lib.rs                 # modified (T2: test_support; T3: export StoreReader)
├── reader.rs              # NEW (T3): StoreReader over a read-only Connection
└── test_support.rs        # NEW (T2): feature-gated rmcp client helper
runtime/
├── src/router.rs          # modified (T3): AppRuntime retains db_path
├── src/mcp_glue.rs        # modified (T3): StoreReader, ViewReader deleted
├── src/ffi.rs             # modified (T1: init_error; T3: pass db_path)
├── tests/common/mod.rs    # NEW (T2): RecordingShell/NullShell/poll_until
└── tests/notes_flow.rs    # NEW (T9): navigation + persistence proofs
apple/
├── project.yml            # modified (T6): DailyTests target + test scheme
├── Justfile               # modified (T6): test target
├── Daily/
│   ├── Core.swift         # modified (T1: init_error + let-ified handler; T5: startup date; T8: debounce)
│   ├── DailyApp.swift     # modified (T1: startup-failure window)
│   ├── ContentView.swift  # modified (T5 minimal; T7 replaced by real shell)
│   ├── Theme.swift        # NEW (T6): tokens + Color(oklch:)
│   ├── SidebarView.swift  # NEW (T7)
│   ├── CalendarCard.swift # NEW (T7)
│   ├── DayColumn.swift    # NEW (T7)
│   ├── QuickAddView.swift # NEW (T7)
│   └── NoteEditor.swift   # NEW (T8): NSTextView representable
└── DailyTests/
    └── ThemeTests.swift   # NEW (T6)
justfile                   # modified (T6): app-test target
.github/workflows/ci.yml   # modified (T10): apple job runs tests
```

## Task overview

| # | Branch | PR title (conventional) | Riders absorbed |
|---|---|---|---|
| 1 | `p1/t1-startup-hardening` | `fix(store): migration failures return Err and surface as alert+quit, not a crash loop` | migration-error path; Swift-6 let-ify `onEffects`; render-refetch idempotency comment |
| 2 | `p1/t2-test-helpers` | `test: consolidate MCP client and shell test helpers` | test-helper consolidation |
| 3 | `p1/t3-mcp-reader` | `feat(mcp): serve reads from a read-only SQLite connection` | ViewReader→SQLite reader; AppRuntime retains db_path |
| 4 | `p1/t4-notes-schema` | `feat(store): schema 002 — notes, blocks, unified FTS5 search, links` | StorageOperation growth decision (recorded above, applied here) |
| 5 | `p1/t5-core-day-model` | `feat(core): day model — navigation, calendar, note text, sidebar view-model` | re-examine update() catch-alls (explicit wrong-shape arms) |
| 6 | `p1/t6-theme` | `feat(apple): Theme tokens with OKLCH support and a Swift unit-test target` | none |
| 7 | `p1/t7-shell-chrome` | `feat(apple): window chrome, sidebar, and calendar to design spec` | none |
| 8 | `p1/t8-note-editor` | `feat(apple): plain-text daily-note editor with debounced saves` | none |
| 9 | `p1/t9-day-navigation` | `test(runtime): day-navigation and persistence proofs end to end` | none |
| 10 | `p1/t10-phase-close` | `chore(p1): phase close — apple CI test step, docs, review sweep` | none |

Deviation from the suggested arc, noted: the ledger's five early-Phase-1 mandates are spread across T1 (migration path + Swift 6 prep), T2 (helpers), T3 (reader swap), T4 (growth decision), and T5 (catch-alls) rather than one mega-opener — each is one PR-sized concern. MCP `get_day`/`search` tools are **not** in this phase (spec §10 puts them in Phase 5); T3 does only the reader swap and notes that boundary.

---

### Task 1: Startup hardening — migration errors, FFI `init_error`, Swift 6 prep

**Files:**
- Modify: `store/src/db.rs`, `runtime/src/ffi.rs`, `apple/Daily/Core.swift`, `apple/Daily/DailyApp.swift`
- Test: `store/src/db.rs` (inline), `runtime/tests/ffi.rs`

**Interfaces:**
- Consumes: `rusqlite_migration::Error` (implements `std::error::Error`, `From<rusqlite::Error>` — verified in the 2.5.0 source), existing `AppRuntime::new -> anyhow::Result<Arc<AppRuntime>>`.
- Produces:
  - `store::db::open(path: &Path) -> Result<Connection, OpenError>` and `store::db::open_in_memory() -> Result<Connection, OpenError>` where `pub use rusqlite_migration::Error as OpenError;` — **no `.expect()` on migrations anywhere in `store`**.
  - `CoreFFI` gains `pub fn init_error(&self) -> String` (empty = healthy); `new` never panics on DB failure; `update`/`resolve_serialized`/`view`/`start_mcp` are inert no-ops (`view` returns empty bytes, `start_mcp` returns 0) on a broken core.
  - Swift: `Core.startupError: String?`; `ShellHandler.onEffects` is `let`; `DailyApp` renders `StartupFailureView` (message + Quit) when set.

**Riders:** migration-error path (T3 ledger); Swift-6 let-ify `ShellHandler.onEffects` + render-refetch-idempotency constraint comment (T8 ledger).

- [x] **Step 1: Write the failing store tests**

Append to the `tests` module in `store/src/db.rs`:

```rust
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
```

- [x] **Step 2: Run to verify failure**

Run: `cargo nextest run -p store`
Expected: `open_returns_err_not_panic_on_a_future_schema_version` FAILS — the current `MIGRATIONS.to_latest(&mut conn).expect("migrations failed")` panics (`test panicked: migrations failed`). The corrupt-file test may already pass (pragma errors already propagate) — it stays as a pin.

- [x] **Step 3: Implement the error path in `store/src/db.rs`**

Replace `open`/`open_in_memory` (and add the re-export near the top of the file):

```rust
/// Everything that can go wrong opening the database: rusqlite errors
/// convert into it (`From<rusqlite::Error>`), so `?` works throughout.
pub use rusqlite_migration::Error as OpenError;

pub fn open(path: &Path) -> Result<Connection, OpenError> {
    let mut conn = Connection::open(path)?;
    configure(&mut conn)?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection, OpenError> {
    let mut conn = Connection::open_in_memory()?;
    // In-memory DBs don't support WAL; skip journal_mode, keep the rest.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(conn)
}
```

Add `OpenError` to the re-exports in `store/src/lib.rs`:

```rust
pub use db::{DEFAULT_SPACE_ID, MIGRATIONS, OpenError, open, open_in_memory};
```

`runtime/src/storage_handler.rs` needs no change: `store::open(path)?` now yields `OpenError`, which implements `std::error::Error + Send + Sync`, so `anyhow` absorbs it. (If the compiler disagrees about `Send + Sync`, wrap at the call site with `.map_err(|e| anyhow::anyhow!("{e}"))` and note it in the PR.)

- [x] **Step 4: Run to verify green**

Run: `cargo nextest run -p store && cargo nextest run -p runtime -p mcp -p shared`
Expected: all store tests PASS (5 prior + 2 new); no fallout elsewhere.

- [x] **Step 5: Write the failing FFI test**

Append to `runtime/tests/ffi.rs`:

```rust
/// A database that cannot be opened must not panic the constructor (macOS
/// would relaunch-loop the app): the core reports the failure via
/// `init_error()` and every other entry point is inert.
#[test]
fn core_ffi_with_unopenable_db_reports_init_error_and_is_inert() {
    let dir = std::env::temp_dir().join(format!("daily-ffi-baddb-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("corrupt.db");
    std::fs::write(&path, b"this is not a sqlite database").unwrap();

    let core = CoreFFI::new(path.to_string_lossy().into_owned(), Arc::new(NullShell));
    assert!(
        !core.init_error().is_empty(),
        "expected a non-empty init error for a corrupt db"
    );

    // Inert, not panicking (on a healthy core, update(&[]) would panic on
    // decode — here it must return before decoding anything):
    core.update(&[]);
    core.resolve_serialized(0, &[]);
    assert!(core.view().is_empty());
    assert_eq!(core.start_mcp(0, "t".into()), 0);

    std::fs::remove_dir_all(&dir).ok();
}
```

- [x] **Step 6: Run to verify failure**

Run: `cargo nextest run -p runtime`
Expected: FAIL to compile — `init_error` not defined (and `CoreFFI::new` would panic anyway).

- [x] **Step 7: Implement the fallible-but-not-panicking `CoreFFI`**

In `runtime/src/ffi.rs`, change the struct and impl (`ShellAdapter` and the `CruxShell` trait are unchanged):

```rust
/// The main interface used by the Swift shell.
///
/// Construction cannot fail across FFI, so a database-open failure parks
/// the core in an inert state: `init_error()` returns the message (the
/// shell must check it right after construction and show alert+quit UX);
/// every other method is a no-op until then.
pub struct CoreFFI {
    runtime: Option<Arc<AppRuntime>>,
    init_error: Option<String>,
}

#[boltffi::export]
impl CoreFFI {
    /// Build the core. `db_path` is the SQLite database file path; an empty
    /// string opens an in-memory database (used by tests).
    #[must_use]
    pub fn new(db_path: String, shell: Arc<dyn CruxShell>) -> Self {
        let path = (!db_path.is_empty()).then(|| std::path::PathBuf::from(db_path));
        match AppRuntime::new(path.as_deref(), Arc::new(ShellAdapter(shell))) {
            Ok(runtime) => Self { runtime: Some(runtime), init_error: None },
            Err(e) => Self { runtime: None, init_error: Some(format!("{e:#}")) },
        }
    }

    /// Empty string = healthy. Non-empty = fatal init failure (the database
    /// could not be opened or migrated); the shell surfaces it and quits.
    #[must_use]
    pub fn init_error(&self) -> String {
        self.init_error.clone().unwrap_or_default()
    }

    /// Send a bincode-serialized `shared::Event` to the app. Any resulting
    /// effects arrive asynchronously via [`CruxShell::process_effects`].
    ///
    /// # Panics
    /// If the event cannot be deserialized (a shell/typegen mismatch) —
    /// unchanged contract (spec §8). No-op on a failed-init core.
    pub fn update(&self, data: &[u8]) {
        let Some(runtime) = &self.runtime else { return };
        runtime
            .router
            .routes
            .serialized
            .update(data)
            .expect("event should deserialize");
    }

    /// Resolve a serialized-lane effect with its bincode-serialized output.
    ///
    /// # Panics
    /// If the id is unknown or the output cannot be deserialized (a shell
    /// bug — mirrors counter-routing). No-op on a failed-init core.
    pub fn resolve_serialized(&self, effect_id: u32, data: &[u8]) {
        let Some(runtime) = &self.runtime else { return };
        runtime
            .router
            .routes
            .serialized
            .resolve(EffectId(effect_id), data)
            .expect("failed to resolve effect");
    }

    /// Get the current view model, bincode-serialized (`shared::ViewModel`).
    /// Empty bytes on a failed-init core (the shell never gets here: it
    /// checks `init_error` first and shows the failure screen).
    #[must_use]
    pub fn view(&self) -> Vec<u8> {
        let Some(runtime) = &self.runtime else { return Vec::new() };
        runtime
            .router
            .routes
            .serialized
            .view()
            .expect("view model should serialize")
    }

    /// Starts the embedded MCP server on `port` (0 = ephemeral), guarded by
    /// `token`. Returns the bound port, or 0 on failure.
    pub fn start_mcp(&self, port: u16, token: String) -> u16 {
        let Some(runtime) = &self.runtime else { return 0 };
        crate::start_mcp(runtime.clone(), None, port, token)
            .map_err(|e| eprintln!("daily: MCP server failed to start: {e:#}"))
            .unwrap_or(0)
    }
}
```

(The existing `should_panic` test `resolving_unknown_effect_id_panics` uses a healthy in-memory core, so the panic contract there still holds.)

- [x] **Step 8: Run to verify green**

Run: `cargo nextest run -p runtime`
Expected: all runtime tests PASS (4 prior + 1 new).

- [x] **Step 9: Swift — surface the failure, let-ify the handler, add the idempotency comment**

Rewrite `apple/Daily/Core.swift`'s wiring (full replacement of the `Core` class init/plumbing and `ShellHandler`; `send`, `processEffects` bodies, `appSupportURL`, `loadOrCreateToken` are unchanged except the comment noted below):

```swift
/// Breaks the init-order cycle: `ShellHandler` wants its closure at
/// construction time (immutable — Swift 6 friendly), but the closure's
/// target (`Core`) doesn't exist until after `CoreFFI` is built. No effects
/// can arrive before the first `update`, so wiring `target` after
/// construction is race-free (same argument as Phase 0's late closure).
@MainActor
final class EffectRelay {
    weak var target: Core?
}

/// Bridges the BoltFFI `CruxShell` protocol (invoked from arbitrary Rust
/// threads) to an immutable Swift closure. The closure owns the
/// main-actor hop.
final class ShellHandler: CruxShell {
    private let onEffects: @Sendable (Data) -> Void
    init(_ onEffects: @escaping @Sendable (Data) -> Void) { self.onEffects = onEffects }
    func processEffects(bytes: Data) { onEffects(bytes) }
}
```

And in `Core`:

```swift
@Observable @MainActor
final class Core {
    private(set) var view = ViewModel(tasks: [], count: 0, error: nil)
    private(set) var mcpPort: UInt16 = 0
    /// Non-nil when the Rust core failed to open/migrate its database at
    /// startup. The app shows a message + Quit and sends no events.
    private(set) var startupError: String?

    private let ffi: CoreFFI
    private let shell: ShellHandler

    init() {
        let dbURL = Self.appSupportURL().appendingPathComponent("daily.db")
        let relay = EffectRelay()
        let shell = ShellHandler { bytes in
            // The callback arrives on an arbitrary Rust thread — hop to the
            // main actor before touching observable state.
            _Concurrency.Task { @MainActor in
                relay.target?.processEffects(bytes)
            }
        }
        self.shell = shell
        self.ffi = CoreFFI(dbPath: dbURL.path, shell: shell)

        let initError = ffi.initError()
        guard initError.isEmpty else {
            startupError = initError
            return // inert core: no relay target, no MCP, no startup event
        }

        relay.target = self
        mcpPort = ffi.startMcp(port: 52111, token: Self.loadOrCreateToken())
        send(.startup)
    }
    // send / processEffects / appSupportURL / loadOrCreateToken unchanged
}
```

Add this comment directly above the `case .render:` line inside `processEffects` (the T8-ledger idempotency rider):

```swift
            // CONSTRAINT: Render carries no payload — its contract is
            // "re-fetch the whole view model", and that refetch is
            // idempotent. That idempotency is exactly what makes the
            // unstructured `_Concurrency.Task` hop from the Rust callback
            // thread safe: if renders coalesce, reorder, or double-fire we
            // still converge on the latest view. This breaks the day Render
            // ever carries a diff — restructure the hop before doing that.
```

In `apple/Daily/DailyApp.swift`, branch on the failure:

```swift
import SwiftUI

@main
struct DailyApp: App {
    @State private var core = Core()

    var body: some Scene {
        WindowGroup("Daily") {
            if let message = core.startupError {
                StartupFailureView(message: message)
            } else {
                ContentView().environment(core)
            }
        }
    }
}

/// Calm failure screen for an unopenable database (decision #5): explain,
/// offer Quit. No auto-retry, no crash-loop, no red styling.
struct StartupFailureView: View {
    let message: String

    var body: some View {
        VStack(spacing: 12) {
            Text("Daily can't open its database")
                .font(.headline)
            Text(message)
                .font(.caption)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
                .frame(maxWidth: 380)
            Text("Your data has not been touched. This usually means the database was created by a newer version of Daily.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(maxWidth: 380)
            Button("Quit Daily") { NSApplication.shared.terminate(nil) }
                .keyboardShortcut(.defaultAction)
        }
        .padding(32)
        .frame(minWidth: 460, minHeight: 220)
    }
}
```

- [x] **Step 10: Build the app and verify manually**

Run: `just test && just app`
Expected: all Rust tests PASS; `BUILD SUCCEEDED`.
Manual check (paste the outcome into the PR): `echo garbage > ~/Library/Application\ Support/Daily/daily.db.bak` is NOT needed — instead run the app once normally (works), then temporarily corrupt a **copy**: point nothing at the real DB. Simplest safe check: `cd apple && just run` with `~/Library/Application Support/Daily/daily.db` moved aside and a garbage file put in its place → the failure window appears with a message and Quit works; restore the real file afterwards.

- [x] **Step 11: Commit + PR**

```bash
git add store runtime apple/Daily
git commit -m "fix(store): migration failures return Err and surface as alert+quit, not a crash loop"
git push -u origin p1/t1-startup-hardening
gh pr create --fill   # fill the template; spec-deltas: none (spec §8 decode contract unchanged; this implements the recorded alert+quit decision)
```
Tick this task's checkboxes in this PR. STOP for review.

---

### Task 2: Test-helper consolidation

**Files:**
- Create: `mcp/src/test_support.rs`, `runtime/tests/common/mod.rs`
- Modify: `mcp/Cargo.toml`, `mcp/src/lib.rs`, `mcp/tests/tools.rs`, `runtime/Cargo.toml`, `runtime/tests/headless.rs`, `runtime/tests/ffi.rs`, `runtime/tests/mcp_end_to_end.rs`

**Interfaces:**
- Produces:
  - `mcp::test_support::connect(addr: impl std::fmt::Display, token: &str) -> RunningService<RoleClient, ClientInfo>` (async), behind `feature = "test-support"` — compiled only for test builds via self-referential dev-dependency; never in production builds.
  - `runtime/tests/common/mod.rs`: `RecordingShell` (implements both `runtime::ShellCallback` and `runtime::ffi::CruxShell`), `NullShell` (both traits), `poll_until(secs: u64, what: &str, check: impl FnMut() -> bool)`.

**Riders:** test-helper consolidation (final-review ledger: "connect()/RecordingShell/poll-loop helpers duplicated across mcp/tests and runtime/tests get extracted once").

This is a pure refactor: **no behavior change, no new tests**. TDD evidence for the PR = the full green suite before and after with identical test names/counts (paste both `cargo nextest run --workspace` summaries).

- [x] **Step 1: Record the baseline**

Run: `cargo nextest run --workspace`
Expected: all tests PASS (23 at Phase 0 close, plus Task 1's additions). Save the summary for the PR.

- [x] **Step 2: Add the feature-gated client helper to `mcp`**

`mcp/Cargo.toml` — add:

```toml
[features]
# Test-only rmcp client helpers, shared with `runtime`'s tests. Enabled
# exclusively via dev-dependencies (self-referential below, and in
# runtime's [dev-dependencies]) so production builds never compile it.
test-support = ["rmcp/client", "rmcp/transport-streamable-http-client-reqwest"]
```

and add to `[dev-dependencies]` (the standard self-referential trick so this crate's own integration tests see the feature):

```toml
mcp = { path = ".", features = ["test-support"] }
```

`mcp/src/test_support.rs`:

```rust
//! Test-only helpers shared by this crate's tests and `runtime`'s tests.
//! Compiled only under `feature = "test-support"` (dev-dependencies only).

use rmcp::{
    ServiceExt,
    model::{ClientCapabilities, ClientInfo, Implementation},
    service::{RoleClient, RunningService},
    transport::{
        StreamableHttpClientTransport,
        streamable_http_client::StreamableHttpClientTransportConfig,
    },
};

/// Connect an rmcp streamable-HTTP client to `http://{addr}/mcp` with a
/// bearer token (task-6 report: `auth_header` takes the bare token, no
/// "Bearer " prefix). Panics on handshake failure — these are tests.
pub async fn connect(
    addr: impl std::fmt::Display,
    token: &str,
) -> RunningService<RoleClient, ClientInfo> {
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(format!("http://{addr}/mcp"))
            .auth_header(token),
    );
    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("daily-test-client", "0.0.1"),
    );
    client_info.serve(transport).await.expect("client handshake")
}
```

`mcp/src/lib.rs` — add:

```rust
#[cfg(feature = "test-support")]
pub mod test_support;
```

- [x] **Step 3: Add the runtime tests-common module**

`runtime/tests/common/mod.rs`:

```rust
//! Shared helpers for runtime integration tests. Each tests/*.rs binary
//! that needs them declares `mod common;` (cargo compiles tests/common/
//! as a module, not a test binary).
#![allow(dead_code)] // not every test binary uses every helper

use std::sync::Mutex;

/// Records every effect batch pushed to the shell.
#[derive(Default)]
pub struct RecordingShell {
    pub batches: Mutex<Vec<Vec<u8>>>,
}

impl runtime::ShellCallback for RecordingShell {
    fn process_effects(&self, effects_bincode: Vec<u8>) {
        self.batches.lock().unwrap().push(effects_bincode);
    }
}

impl runtime::ffi::CruxShell for RecordingShell {
    fn process_effects(&self, bytes: Vec<u8>) {
        self.batches.lock().unwrap().push(bytes);
    }
}

/// Discards effects.
pub struct NullShell;

impl runtime::ShellCallback for NullShell {
    fn process_effects(&self, _: Vec<u8>) {}
}

impl runtime::ffi::CruxShell for NullShell {
    fn process_effects(&self, _: Vec<u8>) {}
}

/// Poll `check` every 10 ms until it returns true, panicking with `what`
/// after `secs` seconds. Blocking by design: callers in async tests use it
/// only after all client I/O has completed (the runtime's storage and MCP
/// work happen on their own OS threads, so blocking here starves nothing).
pub fn poll_until(secs: u64, what: &str, mut check: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    while !check() {
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for {what}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
```

`runtime/Cargo.toml` `[dev-dependencies]` — add (keep the existing rmcp/serde_json lines; the explicit rmcp client features stay so runtime tests do not depend on implicit feature unification):

```toml
mcp = { path = "../mcp", features = ["test-support"] }
```

- [x] **Step 4: Rewrite the duplicated call sites**

- `mcp/tests/tools.rs`: delete the local `connect` fn; replace calls with `mcp::test_support::connect(bound, TOKEN).await`. `StubSink`/`StubReader` stay local — they are this file's fixtures, used nowhere else.
- `runtime/tests/headless.rs`: add `mod common;`, delete local `RecordingShell`, use `common::RecordingShell`; replace the hand-rolled deadline loop with `common::poll_until(5, "view to show the created task", || { ... })` keeping the same assertions.
- `runtime/tests/ffi.rs`: add `mod common;`, delete local `RecordingShell`/`NullShell`, use `common::*`; replace the deadline loop with `poll_until`.
- `runtime/tests/mcp_end_to_end.rs`: add `mod common;`, delete local `NullShell` and `connect`, use `common::NullShell` and `mcp::test_support::connect(format!("127.0.0.1:{port}"), "sekrit").await`; replace the deadline loop with `poll_until` (after `client.cancel().await` — see the helper's doc comment).

- [x] **Step 5: Verify identical green**

Run: `cargo nextest run --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check && cargo build --workspace`
Expected: same test list/count as Step 1, all PASS; clippy/fmt clean; the plain `cargo build --workspace` proves `test_support` is NOT compiled into production builds (it would fail there if the cfg gate were wrong, since the client features are off).

- [x] **Step 6: Commit + PR**

```bash
git add mcp runtime
git commit -m "test: consolidate MCP client and shell test helpers"
git push -u origin p1/t2-test-helpers
gh pr create --fill   # TDD evidence = before/after identical green summaries; spec-deltas: none
```
STOP for review.

---

### Task 3: MCP reads from a read-only SQLite connection

**Files:**
- Create: `mcp/src/reader.rs`
- Modify: `store/src/db.rs`, `store/src/lib.rs`, `mcp/src/lib.rs`, `runtime/src/router.rs`, `runtime/src/mcp_glue.rs`, `runtime/src/ffi.rs`, `runtime/tests/mcp_end_to_end.rs`, `runtime/tests/ffi.rs`
- Test: `store/src/db.rs` (inline), `mcp/tests/tools.rs` (one new test), `runtime/tests/mcp_end_to_end.rs`

**Interfaces:**
- Consumes: `store::execute`, `shared::{StorageOperation, StorageResult, Task}`, existing `mcp::TaskReader`.
- Produces:
  - `store::db::open_read_only(path: &Path) -> Result<Connection, OpenError>` — read-only flags, busy_timeout, **no migrations** (the writer owns those).
  - `mcp::StoreReader::new(conn: rusqlite::Connection) -> StoreReader` implementing `TaskReader` (Connection behind a `Mutex` — `Send` but not `Sync`).
  - `runtime::AppRuntime::db_path(&self) -> Option<&std::path::Path>` — the runtime retains its path.
  - `runtime::start_mcp(runtime, db_path: Option<PathBuf>, port, token) -> anyhow::Result<u16>` now **errors on `None`** ("MCP requires an on-disk database"). `CoreFFI::start_mcp` passes the runtime's retained path instead of the Phase 0 hardcoded `None`.
  - `ViewReader` is **deleted** (decision #4).

**Phase boundary, recorded:** spec §10 puts `search`/`get_day`/`write_brief` MCP tools in Phase 5. This task changes only *how* the existing `list_tasks` read is served. No new tools.

**Riders:** ViewReader → read-only SQLite reader; AppRuntime retains db_path (the "promised swap is false at FFI layer today" ledger item).

- [x] **Step 1: Write the failing store test**

Append to the `tests` module in `store/src/db.rs`:

```rust
    #[test]
    fn read_only_connection_sees_committed_writes_and_rejects_writes() {
        let dir = std::env::temp_dir().join(format!("daily-ro-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ro.db");

        let writer = open(&path).unwrap();
        crate::executor::execute(
            &writer,
            &shared::StorageOperation::InsertTask { title: "seen by reader".into() },
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
```

- [x] **Step 2: Run to verify failure**

Run: `cargo nextest run -p store`
Expected: FAIL to compile — `open_read_only` not defined.

- [x] **Step 3: Implement `open_read_only`**

In `store/src/db.rs`:

```rust
/// Open an existing database read-only. No migrations run here — the
/// writer connection (same process) owns the schema. WAL means this reader
/// never blocks the storage thread and always sees committed writes.
pub fn open_read_only(path: &Path) -> Result<Connection, OpenError> {
    use rusqlite::OpenFlags;
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(Duration::from_millis(5000))?;
    Ok(conn)
}
```

Export it from `store/src/lib.rs` (`pub use db::{..., open_read_only, ...}`). Run: `cargo nextest run -p store` — expected: PASS.

- [x] **Step 4: Write the failing mcp reader test**

Append to `mcp/tests/tools.rs`:

```rust
/// The Phase 1 reader contract: `list_tasks` served through a real
/// read-only SQLite connection over the same file a writer connection
/// mutates — not through core state.
#[tokio::test]
async fn store_reader_serves_list_tasks_from_the_database_file() {
    let dir = std::env::temp_dir().join(format!("daily-mcp-reader-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("reader.db");

    let writer = store::open(&path).unwrap();
    store::execute(
        &writer,
        &shared::StorageOperation::InsertTask { title: "from disk".into() },
    );

    let sink = Arc::new(StubSink::default());
    let reader = Arc::new(mcp::StoreReader::new(store::open_read_only(&path).unwrap()));
    let daily = DailyMcp::new(reader, sink);
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (bound, server) = mcp::serve_http_on(daily, addr, TOKEN.into()).await.unwrap();
    tokio::spawn(server);

    let client = mcp::test_support::connect(bound, TOKEN).await;
    let result = client
        .call_tool(CallToolRequestParams::new("list_tasks"))
        .await
        .unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let tasks: Vec<Task> = serde_json::from_str(&text).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "from disk");

    client.cancel().await.unwrap();
    drop(writer);
    std::fs::remove_dir_all(&dir).ok();
}
```

(Add `store` usage — it is already a dependency of `mcp`; this test finally uses the Phase 0 "declared-unused" dep, retiring that ledger Minor.)

- [x] **Step 5: Run to verify failure**

Run: `cargo nextest run -p mcp`
Expected: FAIL to compile — `mcp::StoreReader` not defined.

- [x] **Step 6: Implement `StoreReader`**

`mcp/src/reader.rs`:

```rust
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
        Self { conn: Mutex::new(conn) }
    }
}

impl TaskReader for StoreReader {
    fn list_tasks(&self) -> Result<Vec<Task>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        match store::execute(&conn, &StorageOperation::ListTasks) {
            StorageResult::Tasks(tasks) => Ok(tasks),
            StorageResult::Error(e) => Err(e),
            other => Err(format!("unexpected storage result for ListTasks: {other:?}")),
        }
    }
}
```

`mcp/src/lib.rs`: add `mod reader;` and `pub use reader::StoreReader;`.
`mcp/Cargo.toml`: add `rusqlite = { workspace = true }` to `[dependencies]` (needed to name `Connection` in the public API; the actual SQLite build is already linked via `store`).

Run: `cargo nextest run -p mcp` — expected: all PASS (7 prior + 1 new).

- [x] **Step 7: Write the failing runtime tests**

In `runtime/tests/mcp_end_to_end.rs`, rework the existing test to use an on-disk DB and prove the reader reads the same file the storage thread writes (this also removes the Phase 0 "FIFO ordering" flake risk noted in the ledger — the poll now goes through the reader):

```rust
mod common;

use std::sync::Arc;

use common::NullShell;
use rmcp::model::CallToolRequestParams;
use runtime::AppRuntime;
use shared::Event;

/// The full loop the product depends on: MCP tool call -> core event ->
/// storage thread writes the FILE -> the read-only MCP reader sees it,
/// and so does the core view.
#[tokio::test]
async fn mcp_create_task_reaches_the_database_and_the_view() {
    let dir = std::env::temp_dir().join(format!("daily-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("e2e.db");

    let rt = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
    rt.send_event(Event::Startup);

    let port = runtime::start_mcp(rt.clone(), Some(db.clone()), 0, "sekrit".into()).unwrap();
    let client = mcp::test_support::connect(format!("127.0.0.1:{port}"), "sekrit").await;

    client
        .call_tool(
            CallToolRequestParams::new("create_task").with_arguments(
                serde_json::json!({"title": "Via MCP"})
                    .as_object()
                    .cloned()
                    .unwrap(),
            ),
        )
        .await
        .unwrap();

    // Poll THROUGH THE READER (read-only SQLite conn on the same file):
    let ro = store::open_read_only(&db).unwrap();
    common::poll_until(5, "MCP write to land in the database", || {
        matches!(
            store::execute(&ro, &shared::StorageOperation::ListTasks),
            shared::StorageResult::Tasks(tasks) if tasks.iter().any(|t| t.title == "Via MCP")
        )
    });
    // ...and the core view agrees (same event drove both).
    common::poll_until(5, "MCP write to reach the core view", || {
        rt.view().tasks.iter().any(|t| t.title == "Via MCP")
    });

    client.cancel().await.unwrap();
    std::fs::remove_dir_all(&dir).ok();
}

/// Decision #4: an in-memory runtime has no shareable database file, so
/// starting MCP against it is an error — not a silently different reader.
#[test]
fn start_mcp_without_a_db_path_is_an_error() {
    let rt = AppRuntime::new(None, Arc::new(NullShell)).unwrap();
    let result = runtime::start_mcp(rt, None, 0, "sekrit".into());
    assert!(result.is_err());
}
```

Also in `runtime/tests/ffi.rs`, update `start_mcp_on_a_busy_port_returns_zero_and_does_not_hang` to construct `CoreFFI` with a **temp-file** db path (in-memory would now return 0 for the wrong reason — no db file — masking the bind-failure behavior under test). Keep every assertion; only the `CoreFFI::new(String::new(), ...)` line changes to a temp path plus cleanup.

- [x] **Step 8: Run to verify failure**

Run: `cargo nextest run -p runtime`
Expected: `start_mcp_without_a_db_path_is_an_error` FAILS (start_mcp currently succeeds with the ViewReader); the e2e test FAILS to compile or panics depending on order — both red.

- [x] **Step 9: Implement the swap**

`runtime/src/router.rs` — retain the path:

```rust
pub struct AppRuntime {
    pub(crate) router: Arc<EffectRouter<Daily, DailyRoutes>>,
    db_path: Option<std::path::PathBuf>,
}
```

In `AppRuntime::new`, capture `let db_path_owned = db_path.map(std::path::Path::to_path_buf);` before the router is built and store it in the returned struct (`Ok(Arc::new(Self { router, db_path: db_path_owned }))`). Add:

```rust
    /// The on-disk database path this runtime was opened with (`None` =
    /// in-memory). Retained so the MCP reader can open the same file.
    #[must_use]
    pub fn db_path(&self) -> Option<&std::path::Path> {
        self.db_path.as_deref()
    }
```

`runtime/src/mcp_glue.rs` — delete `ViewReader` entirely (and its doc comment about the in-memory constraint); replace the reader wiring in `start_mcp`:

```rust
pub fn start_mcp(
    runtime: Arc<AppRuntime>,
    db_path: Option<PathBuf>,
    port: u16,
    token: String,
) -> anyhow::Result<u16> {
    let Some(db_path) = db_path else {
        anyhow::bail!("MCP requires an on-disk database (in-memory runtimes have no shareable file)");
    };
    let reader = mcp::StoreReader::new(store::open_read_only(&db_path)?);
    let daily = mcp::DailyMcp::new(Arc::new(reader), Arc::new(RuntimeSink(runtime)));
    // ... (thread + tokio + serve_http_on + port_rx plumbing UNCHANGED)
}
```

Update the module doc comment (reads are now "served from a read-only SQLite connection over the runtime's database file").

`runtime/src/ffi.rs` — `start_mcp` passes the retained path (the ledger's "hardcodes None" fix):

```rust
    pub fn start_mcp(&self, port: u16, token: String) -> u16 {
        let Some(runtime) = &self.runtime else { return 0 };
        let db_path = runtime.db_path().map(std::path::Path::to_path_buf);
        crate::start_mcp(runtime.clone(), db_path, port, token)
            .map_err(|e| eprintln!("daily: MCP server failed to start: {e:#}"))
            .unwrap_or(0)
    }
```

`runtime/Cargo.toml` `[dev-dependencies]`: ensure `mcp` (test-support) is present from Task 2 — the e2e test now also calls `store::open_read_only` (store is already a normal dependency).

- [x] **Step 10: Run to verify green**

Run: `cargo nextest run --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all PASS, clippy clean. Note: the Task 1 inert-FFI test asserts `start_mcp` returns 0 on a broken core — still true. The healthy in-memory `CoreFFI` (empty db_path, Swift never uses it) now returns 0 from `start_mcp` by decision #4 — the ffi.rs busy-port test was moved to a file DB in Step 7 precisely so it still exercises the bind-failure path.

- [x] **Step 11: Manual E2E sanity (the money shot still works)**

Run: `cd apple && just run`, then from a Claude Code session with the `daily` MCP server configured (Phase 0 Task 8 setup): call `create_task` with title "phase 1 reader" → the row appears in the running app AND `list_tasks` returns it. Paste the outcome into the PR.

- [x] **Step 12: Commit + PR**

```bash
git add store mcp runtime
git commit -m "feat(mcp): serve reads from a read-only SQLite connection"
git push -u origin p1/t3-mcp-reader
gh pr create --fill   # spec-deltas: none (implements spec §5's stated read path; ViewReader was the recorded Phase 0 stopgap)
```
STOP for review.

---

### Task 4: Schema 002 — notes, blocks, unified FTS5 search, links — and the storage ops

**Files:**
- Create: `store/migrations/002_notes.sql`
- Modify: `shared/src/effects/storage.rs`, `shared/src/lib.rs`, `store/src/db.rs` (MIGRATIONS vec), `store/src/executor.rs`, `store/Cargo.toml` (serde_json)
- Test: `store/src/executor.rs` (inline), `store/src/db.rs` (existing `migrations_are_valid` now covers 002)

**Interfaces:**
- Consumes: migration 001's `spaces` table, `store::DEFAULT_SPACE_ID`.
- Produces (Task 5 and Phase 3+ rely on these exact shapes):
  - `shared::effects::storage::BlockData { pub id: String, pub kind: String, pub text: String }`
  - `shared::effects::storage::DayData { pub date: String, pub blocks: Vec<BlockData> }`
  - `StorageOperation::{GetDay { date: String }, ReplaceDayBlocks { date: String, paragraphs: Vec<String> }}` (added to the ONE enum per decision #1, domain-grouped with section comments)
  - `StorageResult::{Day(DayData), DaySaved { date: String }}` (added variants)
  - Builders `storage::get_day(date)`, `storage::replace_day_blocks(date, paragraphs)` (same `RequestBuilder` shape as Phase 0's)
  - Executor behavior: `GetDay` on a date with no note returns `Day` with empty blocks and **creates nothing** (lazy creation is on first edit, spec §4); `ReplaceDayBlocks` get-or-creates the note row, rewrites all blocks + their `search` rows in ONE transaction, returns `DaySaved`.

**Riders:** StorageOperation growth decision (decision #1 — applied here; the enum stays single, variants domain-grouped).

- [x] **Step 1: Write migration 002**

`store/migrations/002_notes.sql` (spec §3 + research/persistence-fts.md §3, with `space_id` scoping added per the spec's every-entity rule):

```sql
-- One daily note per date per space.
CREATE TABLE notes (
  id          TEXT PRIMARY KEY,
  space_id    TEXT NOT NULL REFERENCES spaces(id),
  date        TEXT NOT NULL,             -- 'YYYY-MM-DD'
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER,
  UNIQUE (space_id, date)
) STRICT;

-- One row per note block. Phase 1 blocks are plain paragraphs rewritten
-- wholesale on save (plan decision #3): superseded rows are hard-deleted
-- in the rewrite transaction; the note row is the tombstone unit.
-- deleted_at stays for Phase 3+ block-level editing.
CREATE TABLE blocks (
  id          TEXT PRIMARY KEY,
  space_id    TEXT NOT NULL REFERENCES spaces(id),
  note_id     TEXT NOT NULL REFERENCES notes(id),
  order_key   TEXT NOT NULL,             -- positional in P1; fractional index later
  kind        TEXT NOT NULL,             -- 'paragraph' | (later: 'heading' | 'todo' | ...)
  content     TEXT NOT NULL,             -- JSON: {"text": ...} in P1; rich spans later
  plain_text  TEXT NOT NULL,             -- extracted text; feeds `search`
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER
) STRICT;
CREATE INDEX blocks_by_note ON blocks(note_id, order_key);

-- One polymorphic edge table for all refs/backlinks (note->page, task->page,
-- block->task, ...). Shipped empty in Phase 1; populated from Phase 3.
-- Deliberately NOT an entity table (no space_id/timestamps): rows are
-- identity-free edges rewritten wholesale with their source entity.
CREATE TABLE links (
  src_type TEXT NOT NULL,
  src_id   TEXT NOT NULL,
  dst_type TEXT NOT NULL,
  dst_id   TEXT NOT NULL,
  PRIMARY KEY (src_type, src_id, dst_type, dst_id)
) STRICT;
CREATE INDEX links_backlinks ON links(dst_type, dst_id);

-- Unified search index, maintained transactionally by the single writer
-- (research/persistence-fts.md §3: no triggers — all writes flow through
-- one Rust handler). FTS5 virtual tables cannot be STRICT.
CREATE VIRTUAL TABLE search USING fts5(
  entity_type UNINDEXED,   -- 'block' | 'task' | 'brief' | 'page'
  entity_id   UNINDEXED,
  title,
  body,
  tokenize = 'porter unicode61 remove_diacritics 2'
);
```

Register it in `store/src/db.rs`:

```rust
pub static MIGRATIONS: LazyLock<Migrations> = LazyLock::new(|| {
    Migrations::new(vec![
        M::up(include_str!("../migrations/001_initial.sql")),
        M::up(include_str!("../migrations/002_notes.sql")),
    ])
});
```

- [x] **Step 2: Run migration validation**

Run: `cargo nextest run -p store`
Expected: PASS — `migrations_are_valid` and `open_applies_migrations_and_seeds_spaces` now exercise 002 (a SQL syntax error fails right here; this is the red/green gate for the schema itself).

- [x] **Step 3: Add the shared types and builders**

`shared/src/effects/storage.rs` — extend (existing items unchanged; enums reorganized with domain comments per decision #1):

```rust
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockData {
    pub id: String,
    pub kind: String,
    pub text: String,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DayData {
    pub date: String,
    pub blocks: Vec<BlockData>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum StorageOperation {
    // -- tasks (Phase 0; buckets/status/priority arrive Phase 2) --
    InsertTask { title: String },
    ListTasks,
    // -- daily notes (Phase 1) --
    /// Read a day's blocks. Never creates the note (lazy creation is on
    /// first edit — spec §4).
    GetDay { date: String },
    /// Rewrite the day's blocks from paragraphs, creating the note row if
    /// needed. One transaction including the FTS index.
    ReplaceDayBlocks { date: String, paragraphs: Vec<String> },
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum StorageResult {
    // -- tasks --
    Task(Task),
    Tasks(Vec<Task>),
    // -- daily notes --
    Day(DayData),
    DaySaved { date: String },
    // -- any operation --
    Error(String),
}
```

And the builders (same pattern as `insert_task`/`list_tasks`):

```rust
pub fn get_day<Effect, Event>(
    date: impl Into<String>,
) -> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::GetDay { date: date.into() })
}

pub fn replace_day_blocks<Effect, Event>(
    date: impl Into<String>,
    paragraphs: Vec<String>,
) -> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::ReplaceDayBlocks {
        date: date.into(),
        paragraphs,
    })
}
```

Re-export in `shared/src/lib.rs`:

```rust
pub use effects::storage::{BlockData, DayData, StorageOperation, StorageResult, Task};
```

- [x] **Step 4: Write the failing executor tests**

Append to the `tests` module in `store/src/executor.rs`:

```rust
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
        let StorageResult::Day(day) =
            execute(&conn, &StorageOperation::GetDay { date: "2026-07-04".into() })
        else {
            panic!("expected Day");
        };
        assert_eq!(day.date, "2026-07-04");
        assert!(day.blocks.is_empty());
        let notes: i64 = conn
            .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(notes, 0, "GetDay must not create a note (lazy creation is on edit)");
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
        assert_eq!(saved, StorageResult::DaySaved { date: "2026-07-04".into() });
        assert_eq!(day_text(&conn, "2026-07-04"), "Release Meeting\n\nCopy changes?");
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
        assert_eq!(blocks, 1, "superseded blocks are hard-deleted (decision #3)");
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
        assert_eq!(hits("milk"), 0, "stale FTS rows must be gone after a rewrite");
        assert_eq!(hits("nothing"), 1);
    }
```

- [x] **Step 5: Run to verify failure**

Run: `cargo nextest run -p store`
Expected: FAIL to compile — the executor's `run` match is non-exhaustive (`GetDay`/`ReplaceDayBlocks` unhandled). This is decision #1 working as intended: adding a variant breaks the executor's compile until every arm exists.

- [x] **Step 6: Implement the executor arms**

`store/Cargo.toml` `[dependencies]`: add `serde_json = { workspace = true }` (content-JSON with correct escaping; workspace-existing dep, same precedent as Phase 0's `subtle` — noted in the PR).

`store/src/executor.rs` — extend the import and `run`:

```rust
use shared::{BlockData, DayData, StorageOperation, StorageResult, Task};
```

```rust
        StorageOperation::GetDay { date } => get_day(conn, date),
        StorageOperation::ReplaceDayBlocks { date, paragraphs } => {
            replace_day_blocks(conn, date, paragraphs)
        }
```

```rust
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
    Ok(StorageResult::Day(DayData { date: date.to_owned(), blocks }))
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
            (&block_id, DEFAULT_SPACE_ID, &note_id, &order_key, &content, text),
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
    Ok(StorageResult::DaySaved { date: date.to_owned() })
}
```

- [x] **Step 7: Run to verify green**

Run: `cargo nextest run --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all PASS (store gains 5 tests). `shared`/`mcp`/`runtime` compile untouched — nothing outside the executor matches exhaustively on the grown enums except `shared`'s update() catch-alls, which still swallow the new variants (Task 5 replaces them with explicit arms; that known gap is why Task 5 is the very next PR).

- [x] **Step 8: Amend the spec changelog (same PR)**

Append to `docs/superpowers/specs/2026-07-02-daily-app-design.md` Changelog:

```markdown
- 2026-07-04: §3 amended — Phase 1 block rewrites hard-delete superseded
  block rows inside the rewrite transaction (the note row is the
  soft-delete unit; `blocks.deleted_at` remains for Phase 3+ block-level
  editing). The `links` edge table carries no entity conventions
  (identity-free edges, rewritten with their source).
```

- [x] **Step 9: Commit + PR**

```bash
git add shared store docs/superpowers/specs
git commit -m "feat(store): schema 002 — notes, blocks, unified FTS5 search, links"
git push -u origin p1/t4-notes-schema
gh pr create --fill   # spec-deltas: the two §3 amendments above (included in this PR)
```
STOP for review.

---

### Task 5: Core day model — navigation, calendar, note text, sidebar ViewModel

**Files:**
- Create: `shared/src/civil.rs`
- Modify: `shared/src/lib.rs`, `shared/src/app.rs`, `apple/Daily/Core.swift` (startup date + initial VM), `apple/Daily/ContentView.swift` (minimal compile-fix; T7 replaces it), `runtime/tests/headless.rs`, `runtime/tests/ffi.rs`, `runtime/tests/mcp_end_to_end.rs` (view-polling fields)
- Test: `shared/src/civil.rs` (inline), `shared/src/app.rs` (inline)

**Interfaces:**
- Consumes: `storage::{get_day, replace_day_blocks, list_tasks, insert_task}`, `StorageResult::{Day, DaySaved, Tasks, Task, Error}`, `crux_core::Command::{all, done}` + `effects()` iterator (both verified in the 0.19.0 source: `src/command/mod.rs:513` / `:460`).
- Produces (the shell binds to these exact shapes; typegen regenerates them for Swift):

```rust
pub enum Event {
    Startup { today: String },              // shell supplies 'YYYY-MM-DD' (decision #6)
    NavigateToDay { date: String },
    GoToToday,
    ShiftMonth { delta: i32 },              // <0 = previous month, >0 = next
    EditDay { date: String, text: String }, // debounced by the shell (T8)
    DayLoaded(StorageResult),
    DaySaved(StorageResult),
    CreateTask { title: String },
    TaskSaved(StorageResult),
    TasksLoaded(StorageResult),
}

pub struct ViewModel { pub sidebar: SidebarVm, pub calendar: CalendarVm, pub day: DayVm, pub error: Option<String> }
pub struct SidebarVm {
    pub space_name: String,            // "Red Badger" (single space until Phase 6)
    pub space_initials: String,        // "RB"
    pub today_label: String,           // "Jul 4" — the Today row's date
    pub views: Vec<ViewRowVm>,         // exactly: Now, Next · This week, Later, Waiting on, Inbox
    pub projects: Vec<SidebarEntryVm>, // empty in P1 → section hidden (Global Constraints)
    pub people: Vec<SidebarEntryVm>,   // empty in P1 → section hidden
    pub pages: Vec<SidebarEntryVm>,    // empty in P1 → section hidden
}
pub struct ViewRowVm { pub kind: String, pub label: String, pub count: u64 }
pub struct SidebarEntryVm { pub label: String, pub count: u64 }
pub struct CalendarVm { pub month_label: String, pub cells: Vec<CalendarCellVm> }
pub struct CalendarCellVm {
    pub day: u8,        // 0 = leading blank cell
    pub date: String,   // "" for blanks
    pub is_today: bool,
    pub is_selected: bool,
    pub is_weekend: bool,
}
pub struct DayVm {
    pub date: String,        // "YYYY-MM-DD"
    pub title: String,       // "Saturday, July 4"
    pub note_text: String,
    pub editor_version: u64, // bumps ONLY on loads/external changes — the
                             // editor re-applies text only when this moves
}
```
  - `shared::civil`: `CivilDate { year: i32, month: u32, day: u32 }` with `parse`, `iso`, `weekday` (0 = Monday), `add_days`, `display_title`, `short_label`; free fns `is_leap_year`, `days_in_month`, `prev_month`, `next_month`, `month_label`. Pure integer math, zero dependencies (Global Constraints: no chrono).

**Riders:** re-examine update() catch-all arms as StorageResult grows — resolved here: every result-carrying event matches its expected variant(s) explicitly; anything else routes to a `wrong_shape` helper that sets a **visible** `model.error` and renders. No silent catch-all remains.

- [x] **Step 1: Write the failing civil-date tests**

`shared/src/civil.rs` — start with the test module (implementation in Step 3 goes above it), and add `pub mod civil;` to `shared/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_iso_round_trip() {
        let d = CivilDate::parse("2026-07-04").unwrap();
        assert_eq!((d.year, d.month, d.day), (2026, 7, 4));
        assert_eq!(d.iso(), "2026-07-04");
    }

    #[test]
    fn parse_rejects_garbage() {
        for bad in ["", "garbage", "2026-13-01", "2026-02-30", "2026-07-04-05", "07-04-2026"] {
            assert!(CivilDate::parse(bad).is_none(), "should reject {bad:?}");
        }
    }

    #[test]
    fn weekdays_match_known_dates() {
        // Anchors from the design reference: July 1 2026 is a Wednesday,
        // July 2 a Thursday. Unix epoch day zero was a Thursday.
        assert_eq!(CivilDate::parse("1970-01-01").unwrap().weekday(), 3);
        assert_eq!(CivilDate::parse("2026-07-01").unwrap().weekday(), 2);
        assert_eq!(CivilDate::parse("2026-07-02").unwrap().weekday(), 3);
        assert_eq!(CivilDate::parse("2026-07-04").unwrap().weekday(), 5); // Saturday
    }

    #[test]
    fn add_days_crosses_months_years_and_leap_days() {
        let jump = |s: &str, n: i64| CivilDate::parse(s).unwrap().add_days(n).iso();
        assert_eq!(jump("2026-07-04", 28), "2026-08-01");
        assert_eq!(jump("2025-12-31", 1), "2026-01-01");
        assert_eq!(jump("2024-02-28", 1), "2024-02-29"); // leap
        assert_eq!(jump("2026-02-28", 1), "2026-03-01"); // not leap
        assert_eq!(jump("2026-07-04", -4), "2026-06-30");
    }

    #[test]
    fn display_strings_match_the_design_reference() {
        let d = CivilDate::parse("2026-07-02").unwrap();
        assert_eq!(d.display_title(), "Thursday, July 2"); // reference §5
        assert_eq!(d.short_label(), "Jul 2");              // reference §2.2
        assert_eq!(month_label(2026, 7), "July 2026");     // reference §2.3
    }

    #[test]
    fn month_arithmetic_wraps_years() {
        assert_eq!(prev_month(2026, 1), (2025, 12));
        assert_eq!(next_month(2026, 12), (2027, 1));
        assert_eq!(prev_month(2026, 7), (2026, 6));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2026, 6), 30);
    }
}
```

- [x] **Step 2: Run to verify failure**

Run: `cargo nextest run -p shared`
Expected: FAIL to compile — `CivilDate` etc. not defined.

- [x] **Step 3: Implement `shared/src/civil.rs`**

```rust
//! Minimal pure Gregorian-calendar math for the day model. Deliberately
//! dependency-free (Global Constraints: no chrono in `shared`): Howard
//! Hinnant's civil-days algorithms plus total-function helpers. The shell
//! supplies "today"; nothing here reads a clock.

pub const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];
pub const MONTH_ABBREV: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
/// Monday-first, matching the design reference's calendar (§2.3).
pub const WEEKDAY_NAMES: [&str; 7] = [
    "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilDate {
    pub year: i32,
    pub month: u32, // 1-12
    pub day: u32,   // 1-31, validated by parse/from_days
}

impl CivilDate {
    /// Parse strict `YYYY-MM-DD`; rejects impossible dates.
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split('-');
        let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
            return None;
        }
        let year: i32 = y.parse().ok()?;
        let month: u32 = m.parse().ok()?;
        let day: u32 = d.parse().ok()?;
        ((1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day))
            .then_some(Self { year, month, day })
    }

    #[must_use]
    pub fn iso(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Days since 1970-01-01 (negative before). Hinnant's days_from_civil.
    fn to_days(&self) -> i64 {
        let y = i64::from(if self.month <= 2 { self.year - 1 } else { self.year });
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400; // [0, 399]
        let m = i64::from(self.month);
        let d = i64::from(self.day);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Hinnant's civil_from_days.
    fn from_days(z: i64) -> Self {
        let z = z + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097; // [0, 146096]
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        Self {
            year: (if m <= 2 { y + 1 } else { y }) as i32,
            month: m as u32,
            day: d as u32,
        }
    }

    /// 0 = Monday … 6 = Sunday. 1970-01-01 (days = 0) was a Thursday (= 3).
    #[must_use]
    pub fn weekday(&self) -> u32 {
        ((self.to_days() + 3).rem_euclid(7)) as u32
    }

    #[must_use]
    pub fn add_days(&self, delta: i64) -> Self {
        Self::from_days(self.to_days() + delta)
    }

    /// "Thursday, July 2" — the daily-note title (reference §5).
    #[must_use]
    pub fn display_title(&self) -> String {
        format!(
            "{}, {} {}",
            WEEKDAY_NAMES[self.weekday() as usize],
            MONTH_NAMES[(self.month - 1) as usize],
            self.day
        )
    }

    /// "Jul 2" — the sidebar Today-row date (reference §2.2).
    #[must_use]
    pub fn short_label(&self) -> String {
        format!("{} {}", MONTH_ABBREV[(self.month - 1) as usize], self.day)
    }
}

#[must_use]
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[must_use]
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

#[must_use]
pub fn prev_month(year: i32, month: u32) -> (i32, u32) {
    if month <= 1 { (year - 1, 12) } else { (year, month - 1) }
}

#[must_use]
pub fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month >= 12 { (year + 1, 1) } else { (year, month + 1) }
}

/// "July 2026" — the calendar header (reference §2.3).
#[must_use]
pub fn month_label(year: i32, month: u32) -> String {
    format!("{} {}", MONTH_NAMES[(month - 1) as usize], year)
}
```

Run: `cargo nextest run -p shared` — expected: 6 civil tests PASS (Phase 0 app tests still green, untouched so far).

- [x] **Step 4: Write the failing app tests**

Replace the `tests` module in `shared/src/app.rs`. The four Phase 0 tests are superseded, not dropped: every prior assertion has a successor below (startup → storage request; loaded → model+render; create → insert+append; error → surfaced).

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::storage::{BlockData, DayData, StorageOperation, StorageResult, Task};

    const TODAY: &str = "2026-07-04";

    fn started() -> (Daily, Model) {
        let app = Daily;
        let mut model = Model::default();
        let _ = app.update(Event::Startup { today: TODAY.into() }, &mut model);
        (app, model)
    }

    fn day(date: &str, texts: &[&str]) -> StorageResult {
        StorageResult::Day(DayData {
            date: date.into(),
            blocks: texts
                .iter()
                .enumerate()
                .map(|(i, t)| BlockData {
                    id: format!("b{i}"),
                    kind: "paragraph".into(),
                    text: (*t).into(),
                })
                .collect(),
        })
    }

    #[test]
    fn startup_requests_today_and_the_task_list() {
        let app = Daily;
        let mut model = Model::default();
        let mut cmd = app.update(Event::Startup { today: TODAY.into() }, &mut model);
        let ops: Vec<StorageOperation> = cmd
            .effects()
            .map(|e| e.expect_storage().operation)
            .collect();
        assert_eq!(ops.len(), 2);
        assert!(ops.contains(&StorageOperation::GetDay { date: TODAY.into() }));
        assert!(ops.contains(&StorageOperation::ListTasks));
        assert_eq!(model.selected_date, TODAY);
    }

    #[test]
    fn day_loaded_joins_blocks_bumps_editor_version_and_renders() {
        let (app, mut model) = started();
        let v0 = app.view(&model).day.editor_version;
        let mut cmd = app.update(
            Event::DayLoaded(day(TODAY, &["Release Meeting", "", "Copy changes?"])),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        let view = app.view(&model);
        assert_eq!(view.day.note_text, "Release Meeting\n\nCopy changes?");
        assert_eq!(view.day.title, "Saturday, July 4");
        assert!(view.day.editor_version > v0);
    }

    #[test]
    fn stale_day_load_for_a_departed_date_is_ignored() {
        let (app, mut model) = started();
        let _ = app.update(Event::NavigateToDay { date: "2026-07-03".into() }, &mut model);
        // A slow load for the OLD day arrives after navigation:
        let mut cmd = app.update(Event::DayLoaded(day(TODAY, &["old day text"])), &mut model);
        assert_eq!(cmd.effects().count(), 0, "stale load must be dropped");
        assert_eq!(app.view(&model).day.note_text, "");
    }

    #[test]
    fn navigate_to_day_updates_selection_calendar_and_requests_the_day() {
        let (app, mut model) = started();
        let mut cmd = app.update(Event::NavigateToDay { date: "2026-06-30".into() }, &mut model);
        let effects: Vec<Effect> = cmd.effects().collect();
        assert_eq!(effects.len(), 2); // Render (selection highlight) + GetDay
        assert_eq!(model.selected_date, "2026-06-30");
        assert_eq!(app.view(&model).calendar.month_label, "June 2026");
    }

    #[test]
    fn go_to_today_returns_from_elsewhere() {
        let (app, mut model) = started();
        let _ = app.update(Event::NavigateToDay { date: "2026-06-30".into() }, &mut model);
        let _ = app.update(Event::GoToToday, &mut model);
        assert_eq!(model.selected_date, TODAY);
        assert_eq!(app.view(&model).calendar.month_label, "July 2026");
    }

    #[test]
    fn shift_month_moves_the_calendar_without_changing_the_selected_day() {
        let (app, mut model) = started();
        let _ = app.update(Event::ShiftMonth { delta: -1 }, &mut model);
        assert_eq!(app.view(&model).calendar.month_label, "June 2026");
        let _ = app.update(Event::ShiftMonth { delta: 1 }, &mut model);
        let _ = app.update(Event::ShiftMonth { delta: 1 }, &mut model);
        assert_eq!(app.view(&model).calendar.month_label, "August 2026");
        assert_eq!(model.selected_date, TODAY, "paging the calendar is not navigation");
    }

    #[test]
    fn calendar_grid_matches_july_2026() {
        let (app, mut model) = started();
        let cal = app.view(&model).calendar;
        // July 1 2026 is a Wednesday; Monday-first ⇒ two leading blanks (§2.3).
        assert_eq!(cal.cells[0].day, 0);
        assert_eq!(cal.cells[1].day, 0);
        assert_eq!(cal.cells[2].day, 1);
        assert_eq!(cal.cells.len(), 2 + 31);
        let today_cell = cal.cells.iter().find(|c| c.is_today).unwrap();
        assert_eq!(today_cell.day, 4);
        assert!(today_cell.is_selected);
        assert!(today_cell.is_weekend); // 2026-07-04 is a Saturday
        assert!(!cal.cells.iter().find(|c| c.day == 3).unwrap().is_weekend); // Friday
        assert!(cal.cells.iter().find(|c| c.day == 5).unwrap().is_weekend); // Sunday
        let _ = app.update(Event::ShiftMonth { delta: -1 }, &mut model);
        let june = app.view(&model).calendar;
        assert_eq!(june.cells[0].day, 1, "June 1 2026 is a Monday: no leading blanks");
    }

    #[test]
    fn edit_day_echoes_text_saves_paragraphs_and_leaves_the_editor_alone() {
        let (app, mut model) = started();
        let v0 = app.view(&model).day.editor_version;
        let mut cmd = app.update(
            Event::EditDay { date: TODAY.into(), text: "line one\n\nline two".into() },
            &mut model,
        );
        let request = cmd.expect_one_effect().expect_storage();
        assert_eq!(
            request.operation,
            StorageOperation::ReplaceDayBlocks {
                date: TODAY.into(),
                paragraphs: vec!["line one".into(), String::new(), "line two".into()],
            }
        );
        let view = app.view(&model);
        assert_eq!(view.day.note_text, "line one\n\nline two");
        assert_eq!(view.day.editor_version, v0, "own-typing echo must not bump the version");
    }

    #[test]
    fn day_saved_ack_is_silent_and_save_errors_surface_calmly() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::DaySaved(StorageResult::DaySaved { date: TODAY.into() }),
            &mut model,
        );
        assert_eq!(cmd.effects().count(), 0, "a successful save changes nothing visible");

        let mut cmd = app.update(
            Event::DaySaved(StorageResult::Error("disk full".into())),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        assert_eq!(app.view(&model).error.as_deref(), Some("disk full"));
    }

    #[test]
    fn wrong_shape_results_surface_visibly_not_silently() {
        let (app, mut model) = started();
        // A Tasks result arriving where a Day belongs is a handler bug —
        // it must become a visible error, not a shrug-and-render.
        let mut cmd = app.update(Event::DayLoaded(StorageResult::Tasks(vec![])), &mut model);
        cmd.expect_one_effect().expect_render();
        let err = app.view(&model).error.expect("wrong shape must set error");
        assert!(err.contains("DayLoaded"), "error names the handler: {err}");
    }

    #[test]
    fn tasks_feed_the_inbox_count_and_task_flow_still_works() {
        let (app, mut model) = started();
        let mut cmd = app.update(Event::CreateTask { title: "Ship it".into() }, &mut model);
        let request = cmd.expect_one_effect().expect_storage();
        assert_eq!(
            request.operation,
            StorageOperation::InsertTask { title: "Ship it".into() }
        );
        let mut cmd = app.update(
            Event::TaskSaved(StorageResult::Task(Task {
                id: "t1".into(),
                title: "Ship it".into(),
            })),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();

        let view = app.view(&model);
        let inbox = view.sidebar.views.iter().find(|v| v.kind == "inbox").unwrap();
        assert_eq!(inbox.count, 1);
        let now = view.sidebar.views.iter().find(|v| v.kind == "now").unwrap();
        assert_eq!(now.count, 0, "buckets do not exist yet — honest zero");
        assert_eq!(view.sidebar.views.len(), 5);
        assert!(view.sidebar.projects.is_empty(), "no fake sidebar data");
        assert_eq!(view.sidebar.space_name, "Red Badger");
        assert_eq!(view.sidebar.today_label, "Jul 4");
    }

    #[test]
    fn storage_error_on_load_is_surfaced_not_fatal() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::TasksLoaded(StorageResult::Error("disk full".into())),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        assert_eq!(app.view(&model).error.as_deref(), Some("disk full"));
    }
}
```

- [x] **Step 5: Run to verify failure**

Run: `cargo nextest run -p shared`
Expected: FAIL to compile — new `Event` variants, `Model` fields, and `ViewModel` shape not defined.

- [x] **Step 6: Implement the app**

Replace the type definitions and `App` impl in `shared/src/app.rs`:

```rust
use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::civil::{self, CivilDate};
use crate::effects::storage::{self, StorageOperation, StorageResult, Task};

#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum Event {
    Startup { today: String },
    NavigateToDay { date: String },
    GoToToday,
    ShiftMonth { delta: i32 },
    EditDay { date: String, text: String },
    DayLoaded(StorageResult),
    DaySaved(StorageResult),
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
    pub today: String,
    pub selected_date: String,
    pub calendar_year: i32,
    pub calendar_month: u32,
    pub note_text: String,
    pub editor_version: u64,
    pub tasks: Vec<Task>,
    pub error: Option<String>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ViewModel {
    pub sidebar: SidebarVm,
    pub calendar: CalendarVm,
    pub day: DayVm,
    pub error: Option<String>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SidebarVm {
    pub space_name: String,
    pub space_initials: String,
    pub today_label: String,
    pub views: Vec<ViewRowVm>,
    pub projects: Vec<SidebarEntryVm>,
    pub people: Vec<SidebarEntryVm>,
    pub pages: Vec<SidebarEntryVm>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ViewRowVm {
    pub kind: String,
    pub label: String,
    pub count: u64,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SidebarEntryVm {
    pub label: String,
    pub count: u64,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CalendarVm {
    pub month_label: String,
    pub cells: Vec<CalendarCellVm>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CalendarCellVm {
    pub day: u8,
    pub date: String,
    pub is_today: bool,
    pub is_selected: bool,
    pub is_weekend: bool,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DayVm {
    pub date: String,
    pub title: String,
    pub note_text: String,
    pub editor_version: u64,
}

#[derive(Default)]
pub struct Daily;

/// A storage result arrived with a shape its handler doesn't expect — a
/// handler bug. Surface it visibly (calm banner), never silently.
fn wrong_shape(model: &mut Model, handler: &str, got: &StorageResult) -> Command<Effect, Event> {
    model.error = Some(format!(
        "internal: unexpected storage result in {handler}: {got:?}"
    ));
    render()
}

/// Select a date: update the selection + visible month, clear the editor
/// (bumping its version so the view empties immediately), and request the
/// day's blocks.
fn select_date(model: &mut Model, date: String) -> Command<Effect, Event> {
    if let Some(d) = CivilDate::parse(&date) {
        model.calendar_year = d.year;
        model.calendar_month = d.month;
    }
    model.selected_date = date.clone();
    model.note_text = String::new();
    model.editor_version += 1;
    Command::all([render(), storage::get_day(date).then_send(Event::DayLoaded)])
}

impl App for Daily {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Startup { today } => {
                model.today = today.clone();
                model.selected_date = today.clone();
                if let Some(d) = CivilDate::parse(&today) {
                    model.calendar_year = d.year;
                    model.calendar_month = d.month;
                }
                Command::all([
                    storage::get_day(today).then_send(Event::DayLoaded),
                    storage::list_tasks().then_send(Event::TasksLoaded),
                ])
            }
            Event::NavigateToDay { date } => select_date(model, date),
            Event::GoToToday => select_date(model, model.today.clone()),
            Event::ShiftMonth { delta } => {
                let (y, m) = if delta < 0 {
                    civil::prev_month(model.calendar_year, model.calendar_month)
                } else {
                    civil::next_month(model.calendar_year, model.calendar_month)
                };
                model.calendar_year = y;
                model.calendar_month = m;
                render()
            }
            Event::EditDay { date, text } => {
                if date == model.selected_date {
                    // Echo of the user's own typing: keep the model in step
                    // WITHOUT bumping editor_version (the editor owns the
                    // caret; see DayVm.editor_version contract).
                    model.note_text = text.clone();
                }
                let paragraphs: Vec<String> = text.split('\n').map(str::to_owned).collect();
                storage::replace_day_blocks(date, paragraphs).then_send(Event::DaySaved)
            }
            Event::DayLoaded(result) => match result {
                StorageResult::Day(day) if day.date == model.selected_date => {
                    model.note_text = day
                        .blocks
                        .iter()
                        .map(|b| b.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    model.editor_version += 1;
                    model.error = None;
                    render()
                }
                // A load for a day we've since navigated away from: drop it.
                StorageResult::Day(_) => Command::done(),
                StorageResult::Error(e) => {
                    model.error = Some(e);
                    render()
                }
                other => wrong_shape(model, "DayLoaded", &other),
            },
            Event::DaySaved(result) => match result {
                StorageResult::DaySaved { .. } => Command::done(),
                StorageResult::Error(e) => {
                    model.error = Some(e);
                    render()
                }
                other => wrong_shape(model, "DaySaved", &other),
            },
            Event::CreateTask { title } => {
                storage::insert_task(title).then_send(Event::TaskSaved)
            }
            Event::TaskSaved(result) => match result {
                StorageResult::Task(task) => {
                    model.error = None;
                    model.tasks.push(task);
                    render()
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
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        ViewModel {
            sidebar: build_sidebar(model),
            calendar: build_calendar(model),
            day: build_day(model),
            error: model.error.clone(),
        }
    }
}

fn build_sidebar(model: &Model) -> SidebarVm {
    let view_row = |kind: &str, label: &str, count: u64| ViewRowVm {
        kind: kind.into(),
        label: label.into(),
        count,
    };
    SidebarVm {
        // Single space until Phase 6 (spec §10); this names the row
        // store::DEFAULT_SPACE_ID seeds. Honest constant, not sample data.
        space_name: "Red Badger".into(),
        space_initials: "RB".into(),
        today_label: CivilDate::parse(&model.today)
            .map(|d| d.short_label())
            .unwrap_or_default(),
        views: vec![
            view_row("now", "Now", 0),
            view_row("next", "Next · This week", 0),
            view_row("later", "Later", 0),
            view_row("waiting", "Waiting on", 0),
            // Every task is inbox until buckets exist (Phase 2).
            view_row("inbox", "Inbox", model.tasks.len() as u64),
        ],
        projects: Vec::new(),
        people: Vec::new(),
        pages: Vec::new(),
    }
}

fn build_calendar(model: &Model) -> CalendarVm {
    let (year, month) = (model.calendar_year, model.calendar_month);
    if !(1..=12).contains(&month) {
        return CalendarVm::default(); // pre-Startup: nothing to draw
    }
    let first = CivilDate { year, month, day: 1 };
    let mut cells = Vec::with_capacity(37);
    for _ in 0..first.weekday() {
        cells.push(CalendarCellVm::default()); // leading blanks (day 0)
    }
    for day in 1..=civil::days_in_month(year, month) {
        let date = CivilDate { year, month, day };
        let iso = date.iso();
        cells.push(CalendarCellVm {
            day: day as u8,
            is_today: iso == model.today,
            is_selected: iso == model.selected_date,
            is_weekend: date.weekday() >= 5,
            date: iso,
        });
    }
    CalendarVm {
        month_label: civil::month_label(year, month),
        cells,
    }
}

fn build_day(model: &Model) -> DayVm {
    DayVm {
        date: model.selected_date.clone(),
        title: CivilDate::parse(&model.selected_date)
            .map(|d| d.display_title())
            .unwrap_or_else(|| model.selected_date.clone()),
        note_text: model.note_text.clone(),
        editor_version: model.editor_version,
    }
}
```

Update `shared/src/lib.rs`:

```rust
pub mod app;
pub mod civil;
pub mod effects;

pub use app::{
    CalendarCellVm, CalendarVm, Daily, DayVm, Effect, Event, Model, SidebarEntryVm, SidebarVm,
    ViewModel, ViewRowVm,
};
pub use crux_core::Core;
pub use effects::storage::{BlockData, DayData, StorageOperation, StorageResult, Task};
```

- [x] **Step 7: Run to verify green, then fix the ripple in runtime tests**

Run: `cargo nextest run -p shared`
Expected: 6 civil + 12 app tests PASS.

Run: `cargo nextest run --workspace`
Expected: runtime tests FAIL to compile (`Event::Startup` gained a field; `ViewModel` lost `tasks`/`count`). Fix them, preserving each test's intent:
- `headless.rs`: `Event::Startup { today: "2026-07-04".into() }`; poll the inbox count instead of `view.count`: `rt.view().sidebar.views.iter().any(|v| v.kind == "inbox" && v.count == 1)`.
- `ffi.rs` round-trip test: same inbox-count check after decoding the `ViewModel`.
- `mcp_end_to_end.rs`: `Event::Startup { today: ... }`; the core-view poll becomes the inbox-count check (the DB poll from Task 3 is unchanged).

Run: `cargo nextest run --workspace` — expected: all PASS.

- [x] **Step 8: Keep the app building (minimal Swift patch — Task 7 replaces this UI)**

The `apple` CI job builds Swift against freshly generated types, so this PR must keep it green. In `apple/Daily/Core.swift`, replace `send(.startup)` and add the date source:

```swift
        send(.startup(today: Self.todayString()))
```

```swift
    /// The core is clock-free (decision #6): the shell supplies today's
    /// date, in the user's current timezone, as 'YYYY-MM-DD'.
    private static func todayString() -> String {
        let fmt = DateFormatter()
        fmt.calendar = Calendar(identifier: .gregorian)
        fmt.locale = Locale(identifier: "en_US_POSIX")
        fmt.timeZone = .current
        fmt.dateFormat = "yyyy-MM-dd"
        return fmt.string(from: Date())
    }
```

and the initial view value becomes the empty Phase 1 ViewModel:

```swift
    private(set) var view = ViewModel(
        sidebar: SidebarVm(
            space_name: "", space_initials: "", today_label: "",
            views: [], projects: [], people: [], pages: []),
        calendar: CalendarVm(month_label: "", cells: []),
        day: DayVm(date: "", title: "", note_text: "", editor_version: 0),
        error: nil
    )
```

`apple/Daily/ContentView.swift` — minimal honest interim (day title + read-only note text + the Phase 0 capture field so task creation stays reachable; the `List`/`TaskRow` go away with `ViewModel.tasks`):

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
                Text(error).foregroundStyle(.red)
            }
            Text(core.view.day.title).font(.title.bold())
            ScrollView {
                Text(core.view.day.note_text)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            Text(footer).font(.caption).foregroundStyle(.secondary)
        }
        .padding(16)
        .frame(minWidth: 420, minHeight: 480)
    }

    private var footer: String {
        let inbox = core.view.sidebar.views.first { $0.kind == "inbox" }?.count ?? 0
        let mcp = core.mcpPort == 0 ? "MCP failed to start" : "MCP on 127.0.0.1:\(core.mcpPort)"
        return "\(inbox) in inbox · \(mcp)"
    }

    private func create() {
        let title = draft.trimmingCharacters(in: .whitespaces)
        guard !title.isEmpty else { return }
        core.send(.createTask(title: title))
        draft = ""
    }
}
```

**GENERATED-NAME CAVEAT** (applies to this and every later Swift step): enum cases come out camelCased per Phase 0 (`.createTask(title:)` → so `.startup(today:)`, `.editDay(date:text:)`, …). Struct **field** spelling (`note_text` vs `noteText`) is whatever `just generate` emits — Phase 0's fields were single words, so it is unconfirmed; this plan writes snake_case and the first `just app` build of this task is the arbiter. Adjust call sites to the generated spelling, never the generated files. Keep the `typealias DailyTask` in Core.swift — `App.Task` still exists and Phase 2 reuses it.

Run: `just app` — expected: `BUILD SUCCEEDED`.

- [x] **Step 9: Commit + PR**

```bash
git add shared runtime apple/Daily
git commit -m "feat(core): day model — navigation, calendar, note text, sidebar view-model"
git push -u origin p1/t5-core-day-model
gh pr create --fill   # spec-deltas: none (implements the spec §4 Model/Event slice for P1; Time capability deferred per decision #6, already recorded in this plan)
```
STOP for review.

---

### Task 6: Theme tokens (OKLCH-faithful) + Swift unit-test target

**Files:**
- Create: `apple/Daily/Theme.swift`, `apple/DailyTests/ThemeTests.swift`
- Modify: `apple/project.yml` (DailyTests target + test scheme), `apple/Justfile` (`test` target), root `justfile` (`app-test`)

**Interfaces:**
- Produces: `enum Theme` — every design token from `docs/design/handoff/README.md` §Design Tokens (values below are verbatim from that list, plus the extraction doc's §11 exact-color additions used by Phase 1 chrome), `Color(oklch:_:_:)` + `Color(hex:)` initializers, `Theme.Metrics` / `Theme.Typography` constants; a `DailyTests` XCTest bundle runnable via `just app-test`.

**Riders:** none.

Design tokens are specified in OKLCH; SwiftUI has no OKLCH initializer, and hand-converting each to sRGB would put untraceable magic numbers in code. Instead the tokens are written **literally as the design doc states them** through a small, tested OKLCH→sRGB converter (Björn Ottosson's reference OKLab math — fixed constants, pure function).

- [x] **Step 1: Add the test target to the project**

`apple/project.yml` — add under `targets:` and extend the scheme:

```yaml
  DailyTests:
    type: bundle.unit-test
    platform: macOS
    deploymentTarget: "15.0"
    sources: [DailyTests]
    dependencies:
      - target: Daily
    settings:
      base:
        CODE_SIGN_STYLE: Manual
        CODE_SIGN_IDENTITY: "-"
```

```yaml
schemes:
  Daily:
    build:
      targets:
        Daily: all
    run:
      config: Debug
    test:
      config: Debug
      targets: [DailyTests]
```

`apple/Justfile` — add:

```make
# Run the Swift unit tests (builds the app as the test host).
test: generate
    xcodebuild -project Daily.xcodeproj -scheme Daily -configuration Debug test
```

Root `justfile` — add:

```make
app-test:
    cd apple && just test
```

- [x] **Step 2: Write the failing Swift tests**

`apple/DailyTests/ThemeTests.swift`:

```swift
import XCTest
@testable import Daily
import SwiftUI

final class ThemeTests: XCTestCase {
    private func srgb(_ color: Color) -> (r: CGFloat, g: CGFloat, b: CGFloat) {
        let ns = NSColor(color).usingColorSpace(.sRGB)!
        return (ns.redComponent, ns.greenComponent, ns.blueComponent)
    }

    func testOklchAchromaticEndpoints() {
        // oklch(1 0 h) is white and oklch(0 0 h) is black for any hue.
        let white = srgb(Color(oklch: 1.0, 0.0, 123))
        XCTAssertEqual(white.r, 1.0, accuracy: 0.01)
        XCTAssertEqual(white.g, 1.0, accuracy: 0.01)
        XCTAssertEqual(white.b, 1.0, accuracy: 0.01)

        let black = srgb(Color(oklch: 0.0, 0.0, 0))
        XCTAssertEqual(black.r, 0.0, accuracy: 0.01)
        XCTAssertEqual(black.g, 0.0, accuracy: 0.01)
        XCTAssertEqual(black.b, 0.0, accuracy: 0.01)
    }

    func testAccentBlueIsActuallyBlue() {
        // oklch(0.62 0.13 250): blue-dominant, red-recessive — ordering is a
        // robust invariant without golden values.
        let c = srgb(Theme.accent)
        XCTAssertGreaterThan(c.b, c.g)
        XCTAssertGreaterThan(c.g, c.r)
    }

    func testPriority1RedIsActuallyRed() {
        let c = srgb(Theme.priority1)  // oklch(0.6 0.16 25)
        XCTAssertGreaterThan(c.r, c.g)
        XCTAssertGreaterThan(c.r, c.b)
    }

    func testHexTokensDecodeExactly() {
        let sidebar = srgb(Theme.sidebarBg)  // #edecea
        XCTAssertEqual(sidebar.r, 0xED / 255.0, accuracy: 0.005)
        XCTAssertEqual(sidebar.g, 0xEC / 255.0, accuracy: 0.005)
        XCTAssertEqual(sidebar.b, 0xEA / 255.0, accuracy: 0.005)
    }

    func testMetricsMatchTheReference() {
        XCTAssertEqual(Theme.Metrics.sidebarWidth, 238)
        XCTAssertEqual(Theme.Metrics.contentMaxWidth, 760)
        XCTAssertEqual(Theme.Metrics.noteMaxWidth, 640)
    }
}
```

- [x] **Step 3: Run to verify failure**

Run: `cd apple && just test`
Expected: BUILD FAILS — `Theme` and `Color(oklch:)` not defined. (This is the Swift red; xcodebuild's compile failure is the observed failure.)

- [x] **Step 4: Implement `apple/Daily/Theme.swift`**

```swift
import SwiftUI

extension Color {
    /// OKLCH → sRGB (Björn Ottosson's reference OKLab constants), so tokens
    /// can be written exactly as the design doc specifies them.
    init(oklch L: Double, _ C: Double, _ hDegrees: Double) {
        let h = hDegrees * .pi / 180
        let a = C * cos(h)
        let b = C * sin(h)

        let l_ = L + 0.3963377774 * a + 0.2158037573 * b
        let m_ = L - 0.1055613458 * a - 0.0638541728 * b
        let s_ = L - 0.0894841775 * a - 1.2914855480 * b

        let l = l_ * l_ * l_
        let m = m_ * m_ * m_
        let s = s_ * s_ * s_

        let rLin = +4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s
        let gLin = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s
        let bLin = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s

        func gamma(_ c: Double) -> Double {
            let x = min(max(c, 0), 1)
            return x <= 0.0031308 ? 12.92 * x : 1.055 * pow(x, 1 / 2.4) - 0.055
        }
        self.init(.sRGB, red: gamma(rLin), green: gamma(gLin), blue: gamma(bLin), opacity: 1)
    }

    /// #RRGGBB hex token.
    init(hex: UInt32) {
        self.init(
            .sRGB,
            red: Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue: Double(hex & 0xFF) / 255,
            opacity: 1)
    }
}

/// Design tokens — values verbatim from docs/design/handoff/README.md
/// §Design Tokens, plus the exact-color additions in
/// docs/design/reference/v2-today-view.md §11 that Phase 1 chrome uses.
/// One namespace; no view hardcodes a color/metric (spec §6).
enum Theme {
    // MARK: Accent (blue)
    static let accent = Color(oklch: 0.62, 0.13, 250)          // accent blue
    static let accentEyebrow = Color(oklch: 0.55, 0.13, 250)   // focus-bar eyebrow text
    static let accentText = Color(oklch: 0.50, 0.13, 250)      // link/chip text
    static let accentTextDark = Color(oklch: 0.48, 0.13, 250)  // pill text
    static let focusTintBg = Color(oklch: 0.965, 0.025, 250)
    static let focusTintBorder = Color(oklch: 0.88, 0.05, 250)
    static let chipTintBorder = Color(oklch: 0.85, 0.05, 250)
    static let selectedChipTint = Color(oklch: 0.96, 0.02, 250)
    static let pillTint = Color(oklch: 0.95, 0.04, 250)

    // MARK: Priority
    static let priority1 = Color(oklch: 0.60, 0.16, 25)
    static let priority2 = Color(oklch: 0.70, 0.13, 70)
    static let priority3 = Color(hex: 0xB0B0AE)

    // MARK: Status
    static let statusInProgress = accent
    static let statusWaiting = Color(oklch: 0.70, 0.12, 70)
    static let statusWaitingBg = Color(oklch: 0.96, 0.05, 70)
    static let statusBlocked = Color(oklch: 0.60, 0.16, 25)
    static let statusBlockedBg = Color(oklch: 0.96, 0.04, 25)
    static let statusDone = Color(oklch: 0.62, 0.12, 150)
    static let statusBacklog = Color(hex: 0xB0B0AE)
    static let statusBinned = Color(hex: 0xC4C3C0)

    // MARK: People / projects
    static let personAccent = Color(oklch: 0.58, 0.14, 300)
    static let personTintBg = Color(oklch: 0.96, 0.03, 300)
    static let spaceBadge = Color(oklch: 0.62, 0.16, 25)       // "RB" avatar (§2.1)
    static let projectGreen = Color(oklch: 0.62, 0.12, 150)
    static let amberDot = Color(oklch: 0.70, 0.12, 70)

    // MARK: Neutrals (handoff token list + §11 tiers)
    static let textPrimary = Color(hex: 0x1D1D1F)
    static let textBody = Color(hex: 0x3A3A3C)                  // note body
    static let textSecondary = Color(hex: 0x6E6E73)
    static let textTertiary = Color(hex: 0x86868B)
    static let textQuiet = Color(hex: 0x9A9A98)                 // ghost line, metas
    static let textMuted = Color(hex: 0xA0A09E)                 // section labels, counts
    static let textQuaternary = Color(hex: 0xB0B0AE)            // bullets, chevrons
    static let countEmpty = Color(hex: 0xB8B8B6)                // zero counts (never red)
    static let textDisabled = Color(hex: 0xC0C0BE)              // other-month/weekend days
    static let chipBg = Color(hex: 0xF1F0EE)
    static let blockBg = Color(hex: 0xFAF9F7)
    static let hoverBg = Color(hex: 0xF8F7F5)
    static let sidebarBg = Color(hex: 0xEDECEA)
    static let segmentRemaining = Color(hex: 0xDEDCD8)
    static let calendarCardBg = Color.white.opacity(0.55)
    static let calendarOutline = Color(hex: 0xC9C8C4)           // yesterday's ring (§2.3)

    // MARK: Hairlines (rgba(0,0,0,0.06–0.12) family)
    static let hairline06 = Color.black.opacity(0.06)
    static let hairline08 = Color.black.opacity(0.08)
    static let hairline09 = Color.black.opacity(0.09)           // sidebar right edge
    static let hairline10 = Color.black.opacity(0.10)
    static let hairline12 = Color.black.opacity(0.12)

    // MARK: Metrics (reference §§0–5)
    enum Metrics {
        static let sidebarWidth: CGFloat = 238
        static let contentMaxWidth: CGFloat = 760
        static let noteMaxWidth: CGFloat = 640
        static let sidebarRowHeight: CGFloat = 29
        static let sidebarActiveRowHeight: CGFloat = 30
        static let sidebarRowRadius: CGFloat = 7
        static let calendarCardRadius: CGFloat = 9
        static let calendarCellHeight: CGFloat = 22
        static let calendarDayCircle: CGFloat = 21
        static let cardRadius: CGFloat = 12
        static let rowRadius: CGFloat = 8
        static let buttonRadius: CGFloat = 7
        static let plusButtonSize: CGFloat = 28
        static let contentPaddingH: CGFloat = 28
        static let contentPaddingTop: CGFloat = 22
    }

    // MARK: Typography (handoff §Design Tokens "Type")
    enum Typography {
        static let dateTitle = Font.system(size: 25, weight: .bold)      // -0.02em kerning at use site
        static let sectionHeader = Font.system(size: 16, weight: .bold)
        static let body = Font.system(size: 14)                          // line-height 1.65 at use site
        static let sidebarRow = Font.system(size: 13)
        static let sidebarRowActive = Font.system(size: 13, weight: .medium)
        static let spaceName = Font.system(size: 13, weight: .semibold)
        static let capsLabel = Font.system(size: 11, weight: .bold)      // +0.06em, uppercase at use site
        static let count = Font.system(size: 11)
        static let calendarHeader = Font.system(size: 12.5, weight: .semibold)
        static let calendarWeekday = Font.system(size: 9.5, weight: .semibold)
        static let calendarDay = Font.system(size: 11.5)
        static let meta = Font.system(size: 11.5)
        static let ghost = Font.system(size: 14)
    }
}
```

- [x] **Step 5: Run to verify green**

Run: `cd apple && just test`
Expected: `TEST SUCCEEDED` — 5 tests pass. Also run `just app` (plain build stays green).

- [x] **Step 6: Commit + PR**

```bash
git add apple justfile
git commit -m "feat(apple): Theme tokens with OKLCH support and a Swift unit-test target"
git push -u origin p1/t6-theme
gh pr create --fill   # spec-deltas: none (spec §6 mandates the Theme namespace; test target implements spec §9's thin Swift testing)
```
STOP for review.

---

### Task 7: Window chrome, sidebar, and calendar to design spec

**Files:**
- Create: `apple/Daily/SidebarView.swift`, `apple/Daily/CalendarCard.swift`, `apple/Daily/DayColumn.swift`, `apple/Daily/QuickAddView.swift`
- Modify: `apple/Daily/ContentView.swift` (replaced wholesale — the Task 5 interim UI is superseded and deleted), `apple/Daily/Core.swift` (navigation helpers), `apple/Daily/DailyApp.swift` (window sizing)
- Test: manual acceptance checklist against the reference (Step 6) — pixel fidelity is checked by eye per spec §9; automated Swift tests cover Theme (T6) and, later, logic-bearing helpers

**Interfaces:**
- Consumes: `ViewModel.{sidebar, calendar, day}`, `Core.send(_:)`, `Theme`.
- Produces: `SidebarView(sidebar:calendar:onGoToToday:onSelectDate:onShiftMonth:)`, `CalendarCard(calendar:onSelect:onShift:)`, `DayColumn(day:)` (read-only note text this task; T8 swaps in the editor), `QuickAddView(onSubmit:)`; `Core.navigate(to:)`, `Core.goToToday()`, `Core.shiftMonth(_:)` (thin `send` wrappers this task; T8 adds flush semantics).
- **Acceptance criteria:** reference §1 (title bar — adapted to real chrome), §2 (sidebar §§2.1–2.9), §5 (daily note header/typography), with the Global Constraints carve-outs. All views are dumb functions of ViewModel values, so SwiftUI previews work by passing sample values — no bridge, no Rust loaded (this is the plan's stand-in for spec §6's `FakeBridge`, recorded in the PR's spec-deltas).

**Riders:** none.

- [ ] **Step 1: Core navigation helpers**

Add to `Core` in `apple/Daily/Core.swift`:

```swift
    // Navigation entry points. Thin today; Task 8 gives them flush-pending-
    // edit semantics, so ALL UI navigation must route through these, never
    // send(.navigateToDay) directly.
    func navigate(to date: String) { send(.navigateToDay(date: date)) }
    func goToToday() { send(.goToToday) }
    func shiftMonth(_ delta: Int32) { send(.shiftMonth(delta: delta)) }
```

- [ ] **Step 2: Sidebar + calendar views**

`apple/Daily/CalendarCard.swift` (reference §2.3 — card `rgba(255,255,255,0.55)` radius 9, header 12.5/600 with `‹ ›` arrows, Monday-first weekday row 9.5/600 `#b0b0ae`, 22px cells, today = 21px accent circle w/ white 11.5/600 numeral, weekends `#c0c0be`; interpretation recorded: the mock's outlined circle on "yesterday" marks **today when it is not the selected day** — filled circle always marks the selected day):

```swift
import SwiftUI

struct CalendarCard: View {
    let calendar: CalendarVm
    let onSelect: (String) -> Void
    let onShift: (Int32) -> Void

    private let columns = Array(
        repeating: GridItem(.flexible(), spacing: 1), count: 7)
    private let weekdays = ["M", "T", "W", "T", "F", "S", "S"]

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(calendar.month_label)
                    .font(Theme.Typography.calendarHeader)
                    .foregroundStyle(Theme.textPrimary)
                Spacer()
                HStack(spacing: 11) {
                    Button { onShift(-1) } label: { Text("‹") }
                    Button { onShift(1) } label: { Text("›") }
                }
                .buttonStyle(.plain)
                .font(.system(size: 11))
                .foregroundStyle(Theme.textQuaternary)
            }
            .padding(.horizontal, 2)
            .padding(.bottom, 7)

            LazyVGrid(columns: columns, spacing: 1) {
                ForEach(Array(weekdays.enumerated()), id: \.offset) { _, d in
                    Text(d)
                        .font(Theme.Typography.calendarWeekday)
                        .foregroundStyle(Theme.textQuaternary)
                        .padding(.bottom, 3)
                }
                ForEach(Array(calendar.cells.enumerated()), id: \.offset) { _, cell in
                    CalendarCellView(cell: cell, onSelect: onSelect)
                }
            }
        }
        .padding(9)
        .background(Theme.calendarCardBg)
        .clipShape(RoundedRectangle(cornerRadius: Theme.Metrics.calendarCardRadius))
        .padding(.horizontal, 4)
        .padding(.top, 10)
        .padding(.bottom, 12)
    }
}

private struct CalendarCellView: View {
    let cell: CalendarCellVm
    let onSelect: (String) -> Void

    var body: some View {
        Group {
            if cell.day == 0 {
                Color.clear
            } else {
                Button { onSelect(cell.date) } label: {
                    ZStack {
                        if cell.is_selected {
                            Circle().fill(Theme.accent)
                                .frame(width: Theme.Metrics.calendarDayCircle,
                                       height: Theme.Metrics.calendarDayCircle)
                        } else if cell.is_today {
                            Circle().strokeBorder(Theme.calendarOutline, lineWidth: 1)
                                .frame(width: Theme.Metrics.calendarDayCircle,
                                       height: Theme.Metrics.calendarDayCircle)
                        }
                        Text("\(cell.day)")
                            .font(cell.is_selected
                                  ? .system(size: 11.5, weight: .semibold)
                                  : Theme.Typography.calendarDay)
                            .foregroundStyle(
                                cell.is_selected ? .white
                                : cell.is_weekend ? Theme.textDisabled
                                : Theme.textBody)
                    }
                }
                .buttonStyle(.plain)
            }
        }
        .frame(height: Theme.Metrics.calendarCellHeight)
    }
}
```

`apple/Daily/SidebarView.swift` (reference §2 — `#edecea`, 12/10 padding, right hairline; §2.1 space row; §2.2 active Today row 30px accent radius-7; §2.4 section headers 11/700/0.06em uppercase `#9a9a98`; §§2.5–2.6 view rows 29px with per-kind dots and right-aligned counts; §§2.7–2.9 sections render only with rows — empty in Phase 1):

```swift
import SwiftUI

struct SidebarView: View {
    let sidebar: SidebarVm
    let calendar: CalendarVm
    let onGoToToday: () -> Void
    let onSelectDate: (String) -> Void
    let onShiftMonth: (Int32) -> Void
    let mcpStatus: String

    var body: some View {
        VStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    spaceRow
                    todayRow
                    CalendarCard(calendar: calendar,
                                 onSelect: onSelectDate,
                                 onShift: onShiftMonth)
                    sectionHeader("Views", topPadding: 4)
                    ForEach(Array(sidebar.views.enumerated()), id: \.offset) { _, row in
                        viewRow(row)
                    }
                    // Projects / People / Pages: data-driven; empty in
                    // Phase 1 ⇒ absent, not dead (Global Constraints).
                    entrySection("Projects", sidebar.projects)
                    entrySection("People", sidebar.people)
                    entrySection("Pages", sidebar.pages)
                }
                .padding(.horizontal, 10)
                .padding(.top, 12)
            }
            // Dev-useful, honest status until the Phase 5 Settings UI.
            Text(mcpStatus)
                .font(.system(size: 10))
                .foregroundStyle(Theme.textMuted)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(10)
        }
        .frame(width: Theme.Metrics.sidebarWidth)
        .background(Theme.sidebarBg)
        .overlay(alignment: .trailing) {
            Theme.hairline09.frame(width: 0.5)
        }
    }

    private var spaceRow: some View {
        HStack(spacing: 8) {
            Text(sidebar.space_initials)
                .font(.system(size: 10, weight: .bold))
                .foregroundStyle(.white)
                .frame(width: 22, height: 22)
                .background(Theme.spaceBadge)
                .clipShape(RoundedRectangle(cornerRadius: 6))
            Text(sidebar.space_name)
                .font(Theme.Typography.spaceName)
            Spacer()
            Text("⌄")
                .font(.system(size: 10))
                .foregroundStyle(Theme.textQuaternary)
        }
        .padding(EdgeInsets(top: 5, leading: 8, bottom: 12, trailing: 8))
    }

    private var todayRow: some View {
        Button(action: onGoToToday) {
            HStack(spacing: 9) {
                RoundedRectangle(cornerRadius: 2)
                    .fill(.white)
                    .frame(width: 9, height: 9)
                Text("Today")
                    .font(Theme.Typography.sidebarRowActive)
                Spacer()
                Text(sidebar.today_label)
                    .font(.system(size: 11))
                    .opacity(0.85)
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 9)
            .frame(height: Theme.Metrics.sidebarActiveRowHeight)
            .background(Theme.accent)
            .clipShape(RoundedRectangle(cornerRadius: Theme.Metrics.sidebarRowRadius))
        }
        .buttonStyle(.plain)
    }

    private func sectionHeader(_ title: String, topPadding: CGFloat = 12) -> some View {
        Text(title.uppercased())
            .font(Theme.Typography.capsLabel)
            .kerning(0.66) // 0.06em of 11px
            .foregroundStyle(Theme.textQuiet)
            .padding(EdgeInsets(top: topPadding, leading: 8, bottom: 6, trailing: 8))
    }

    private func viewRow(_ row: ViewRowVm) -> some View {
        HStack(spacing: 9) {
            viewIcon(kind: row.kind)
            Text(row.label)
                .font(Theme.Typography.sidebarRow)
                .foregroundStyle(Theme.textPrimary)
            Spacer()
            Text("\(row.count)")
                .font(Theme.Typography.count)
                .foregroundStyle(row.count == 0 ? Theme.countEmpty : Theme.textMuted)
        }
        .padding(.horizontal, 9)
        .frame(height: Theme.Metrics.sidebarRowHeight)
    }

    /// Reference §2.6: Now/Next = filled dots, Later = grey dot, Waiting on
    /// = hollow amber ring, Inbox = tray outline.
    @ViewBuilder
    private func viewIcon(kind: String) -> some View {
        switch kind {
        case "now":
            Circle().fill(Theme.accent).frame(width: 8, height: 8)
        case "next":
            Circle().fill(Theme.amberDot).frame(width: 8, height: 8)
        case "later":
            Circle().fill(Theme.textDisabled).frame(width: 8, height: 8)
        case "waiting":
            Circle().strokeBorder(Theme.amberDot, lineWidth: 1.5)
                .frame(width: 8, height: 8)
        default: // inbox tray
            UnevenRoundedRectangle(
                bottomLeadingRadius: 2, bottomTrailingRadius: 2)
                .strokeBorder(Theme.textMuted, lineWidth: 1.5)
                .frame(width: 11, height: 8)
        }
    }

    @ViewBuilder
    private func entrySection(_ title: String, _ entries: [SidebarEntryVm]) -> some View {
        if !entries.isEmpty {
            sectionHeader(title)
            ForEach(Array(entries.enumerated()), id: \.offset) { _, entry in
                HStack(spacing: 9) {
                    Text(entry.label)
                        .font(Theme.Typography.sidebarRow)
                    Spacer()
                    if entry.count > 0 {
                        Text("\(entry.count)")
                            .font(Theme.Typography.count)
                            .foregroundStyle(Theme.textMuted)
                    }
                }
                .padding(.horizontal, 9)
                .frame(height: Theme.Metrics.sidebarRowHeight)
            }
        }
    }
}
```

Layout decision, recorded: the sidebar is a plain `ScrollView` + `VStack` with `Theme.sidebarBg`, not `List(.sidebar)` + system material as spec §6 sketches — the reference's exact metrics (29px rows, radius-7, precise paddings) are the acceptance criteria and `List` fights all of them. The PR records this as a spec §6 delta (one changelog line: "sidebar is custom-layout over a flat tint in P1; revisit material look at a polish pass").

- [ ] **Step 3: Day column and quick-add**

`apple/Daily/DayColumn.swift` (reference §5 — eyebrow 11/700/0.06em uppercase `#a0a09e`, title 25/700/-0.02em, body 14/1.65 `#3a3a3c`, note text max-width 640; read-only `Text` this task, T8 swaps in the editor):

```swift
import SwiftUI

struct DayColumn: View {
    let day: DayVm

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                Text("DAILY NOTE")
                    .font(Theme.Typography.capsLabel)
                    .kerning(0.66)
                    .foregroundStyle(Theme.textMuted)
                    .padding(.bottom, 3)
                Text(day.title)
                    .font(Theme.Typography.dateTitle)
                    .kerning(-0.5) // -0.02em of 25px
                    .foregroundStyle(Theme.textPrimary)
                    .padding(.bottom, 12)
                if day.note_text.isEmpty {
                    Text("Type to keep writing…")
                        .font(Theme.Typography.body)
                        .foregroundStyle(Theme.textQuiet)
                } else {
                    Text(day.note_text)
                        .font(Theme.Typography.body)
                        .lineSpacing(14 * 0.65)
                        .foregroundStyle(Theme.textBody)
                }
            }
            .frame(maxWidth: Theme.Metrics.noteMaxWidth, alignment: .leading)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(EdgeInsets(
                top: Theme.Metrics.contentPaddingTop,
                leading: Theme.Metrics.contentPaddingH,
                bottom: 40,
                trailing: Theme.Metrics.contentPaddingH))
        }
        .background(Color.white)
    }
}
```

`apple/Daily/QuickAddView.swift` (the toolbar `+` popover — keeps the Phase 0 task-creation path reachable; Inbox count updates live):

```swift
import SwiftUI

struct QuickAddView: View {
    let onSubmit: (String) -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var title = ""

    var body: some View {
        HStack {
            TextField("New task", text: $title)
                .textFieldStyle(.roundedBorder)
                .frame(width: 260)
                .onSubmit(submit)
            Button("Add", action: submit)
                .disabled(title.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .padding(12)
    }

    private func submit() {
        let trimmed = title.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        onSubmit(trimmed)
        title = ""
        dismiss()
    }
}
```

- [ ] **Step 4: Replace ContentView and size the window**

`apple/Daily/ContentView.swift` (wholesale replacement; the Task 5 interim UI is deleted here — delete-don't-pause):

```swift
import SwiftUI

struct ContentView: View {
    @Environment(Core.self) private var core
    @State private var showQuickAdd = false

    var body: some View {
        HStack(spacing: 0) {
            SidebarView(
                sidebar: core.view.sidebar,
                calendar: core.view.calendar,
                onGoToToday: { core.goToToday() },
                onSelectDate: { core.navigate(to: $0) },
                onShiftMonth: { core.shiftMonth($0) },
                mcpStatus: core.mcpPort == 0
                    ? "MCP not running"
                    : "MCP · 127.0.0.1:\(core.mcpPort)")
            VStack(spacing: 0) {
                if let error = core.view.error {
                    Text(error)
                        .font(.system(size: 12))
                        .foregroundStyle(Theme.textSecondary)
                        .frame(maxWidth: .infinity)
                        .padding(6)
                        .background(Theme.blockBg)
                }
                DayColumn(day: core.view.day)
            }
        }
        .navigationTitle("Today")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button { showQuickAdd = true } label: {
                    Image(systemName: "plus")
                        .foregroundStyle(.white)
                        .frame(width: Theme.Metrics.plusButtonSize,
                               height: Theme.Metrics.plusButtonSize)
                        .background(Theme.accent)
                        .clipShape(RoundedRectangle(cornerRadius: 7))
                }
                .buttonStyle(.plain)
                .popover(isPresented: $showQuickAdd) {
                    QuickAddView { core.send(.createTask(title: $0)) }
                }
            }
        }
    }
}
```

Layout decision, recorded: a fixed-width `HStack` instead of `NavigationSplitView` — the reference sidebar is a fixed 238px pane with no collapse affordance in the design; `NavigationSplitView`'s collapsible/resizable behavior and material sidebar would have to be fought to match §2. Recorded in the same spec §6 changelog line as Step 2's. (If Jon wants sidebar collapse later, that is a deliberate product change, not chrome fidelity.)

In `apple/Daily/DailyApp.swift`, set a sensible default size on the content:

```swift
            } else {
                ContentView()
                    .environment(core)
                    .frame(minWidth: 900, minHeight: 600)
            }
```

- [ ] **Step 5: Build**

Run: `just app && just app-test`
Expected: `BUILD SUCCEEDED`, `TEST SUCCEEDED`. Iterate on generated-name mismatches per the Task 5 caveat.

- [ ] **Step 6: Manual acceptance checklist (against the reference; paste results into the PR)**

Run `cd apple && just run` and verify each item:
1. **§1 chrome:** window title "Today"; toolbar shows the accent `+` (28×28, radius 7, white plus); NO search field (recorded carve-out).
2. **§2.1:** space row — 22px rounded-6 red badge "RB", "Red Badger" 13/600, `⌄` chevron; not clickable.
3. **§2.2:** Today row — accent fill, white text, radius 7, 30px, white 9px square icon, right date label (e.g. "Jul 4") at 85% opacity; clicking returns to today from any day.
4. **§2.3:** calendar — "July 2026" header, `‹ ›` arrows page months (June/August correct grids), Monday-first M T W T F S S, current month starts in the correct column, selected day = filled accent circle w/ white numeral, today ringed when not selected, weekends `#c0c0be`.
5. **§2.6:** Views — five rows, correct dot styles (blue/amber/grey filled, amber ring, tray icon), counts right-aligned; Inbox count matches reality and increments live when `+` adds a task (also via MCP `create_task` — external write updates the UI without user action); zero counts render muted, never red.
6. **§2.7–2.9:** no Projects/People/Pages sections visible (empty data ⇒ absent).
7. **§5:** content column — "DAILY NOTE" eyebrow, date title 25px bold ("Saturday, July 4" format), body area max-width 640, ghost line "Type to keep writing…" on an empty day.
8. Clicking a calendar day swaps the note column's title to that date (text is read-only until T8).

- [ ] **Step 7: Commit + PR**

```bash
git add apple/Daily apple/project.yml
git commit -m "feat(apple): window chrome, sidebar, and calendar to design spec"
git push -u origin p1/t7-shell-chrome
gh pr create --fill   # spec-deltas: §6 sidebar implementation (custom layout over flat tint instead of List(.sidebar)+material; fixed HStack instead of NavigationSplitView; previews via dumb value-passing views instead of FakeBridge) — one changelog amendment included in this PR
```
STOP for review.

---

### Task 8: Plain-text daily-note editor with debounced saves

**Files:**
- Create: `apple/Daily/NoteEditor.swift`
- Modify: `apple/Daily/DayColumn.swift` (swap read-only text for the editor), `apple/Daily/Core.swift` (debounce + flush semantics), `apple/Daily/DailyApp.swift` (terminate flush)
- Test: manual acceptance checklist (Step 4) — the editor's Rust-side contract is already covered by shared/store tests; T9 adds the headless persistence proofs

**Interfaces:**
- Consumes: `DayVm.{note_text, editor_version, date}`, `Event::EditDay`, `Core.navigate/goToToday` from T7.
- Produces:
  - `NoteEditor(text:version:onEdit:)` — `NSViewRepresentable` wrapping ONE `NSTextView` created with `usingTextLayoutManager: true` (TextKit 2, spec §6). Plain text only: `isRichText = false`, no attachments, no pickers (Phase 3).
  - `Core.noteEdited(_ text: String)` — 500 ms debounce, then `send(.editDay(date:text:))` with the date captured at edit time.
  - `Core.flushPendingEdit()` — cancels the timer and sends immediately; called by `navigate(to:)`/`goToToday()` **before** the navigation event, and on app termination.
- **The version contract (the render-refetch-idempotency comment's editor-side twin):** `updateNSView` pushes `text` into the text view ONLY when `editor_version` differs from the last version it applied. Renders caused by the user's own typing echo (version unchanged) never touch the view — no caret jumps, no fighting the typist. Day switches and loads bump the version (T5) and thus replace the text.

**Riders:** none.

**Known-unknowable, named up front:** exact TextKit 2 wiring details (`NSTextView(usingTextLayoutManager:)` + manual `NSScrollView` assembly, `typingAttributes` persistence) are AppKit-contact territory. Canonical references: Apple's TextKit 2 documentation and the WWDC22 sample; the arbiter is Step 4's manual checklist (type → persists; navigate → correct text; no caret jump while typing during a background render). Adjust the wrapper to what AppKit actually does and note deviations in the PR.

- [ ] **Step 1: Debounce + flush in Core**

Add to `Core` in `apple/Daily/Core.swift`:

```swift
    // MARK: Note editing (debounced)

    private var pendingEditWork: DispatchWorkItem?
    private var pendingEditText: String?
    private var pendingEditDate: String?

    /// Called on every keystroke (via NoteEditor). Debounced 500 ms: the
    /// core is pure and eager — timing policy lives here in the shell.
    func noteEdited(_ text: String) {
        pendingEditText = text
        pendingEditDate = view.day.date // captured NOW, not at fire time
        pendingEditWork?.cancel()
        let work = DispatchWorkItem { [weak self] in self?.flushPendingEdit() }
        pendingEditWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5, execute: work)
    }

    /// Send any pending edit immediately. Safe to call when idle.
    func flushPendingEdit() {
        pendingEditWork?.cancel()
        pendingEditWork = nil
        guard let text = pendingEditText, let date = pendingEditDate else { return }
        pendingEditText = nil
        pendingEditDate = nil
        send(.editDay(date: date, text: text))
    }
```

and give the T7 navigation helpers their real semantics (replace the thin versions):

```swift
    // Flush-then-navigate: an in-flight edit for the outgoing day must be
    // saved (against ITS date, captured in noteEdited) before the day
    // changes under the editor.
    func navigate(to date: String) {
        flushPendingEdit()
        send(.navigateToDay(date: date))
    }

    func goToToday() {
        flushPendingEdit()
        send(.goToToday)
    }

    func shiftMonth(_ delta: Int32) { send(.shiftMonth(delta: delta)) }
```

In `DailyApp` (or `Core.init`), flush on quit. Add to `Core.init` after the startup send:

```swift
        NotificationCenter.default.addObserver(
            forName: NSApplication.willTerminateNotification,
            object: nil, queue: .main
        ) { [weak self] _ in
            MainActor.assumeIsolated {
                self?.flushPendingEdit()
                // The storage thread persists asynchronously; give it a
                // bounded beat before the process dies. Worst case (force
                // quit mid-debounce) loses ≤500 ms of typing — recorded
                // limitation; a storage-drain ack replaces this if real
                // losses are ever observed.
                Thread.sleep(forTimeInterval: 0.2)
            }
        }
```

- [ ] **Step 2: The editor view**

`apple/Daily/NoteEditor.swift`:

```swift
import AppKit
import SwiftUI

/// ONE plain-text NSTextView (TextKit 2) for the daily note — spec §6.
/// Phase 1 scope: plain paragraphs only. Phase 3 adds mention chips and
/// pickers to THIS view; do not replace it with TextEditor.
struct NoteEditor: NSViewRepresentable {
    let text: String
    let version: UInt64
    let onEdit: (String) -> Void

    func makeCoordinator() -> Coordinator { Coordinator(onEdit: onEdit) }

    func makeNSView(context: Context) -> NSScrollView {
        let textView = NSTextView(usingTextLayoutManager: true) // TextKit 2 opt-in
        textView.delegate = context.coordinator
        textView.isRichText = false
        textView.allowsUndo = true
        textView.drawsBackground = false
        textView.isVerticallyResizable = true
        textView.isHorizontallyResizable = false
        textView.autoresizingMask = [.width]
        textView.textContainer?.widthTracksTextView = true
        textView.textContainerInset = .zero

        // Reference §5: 14px / 1.65 line height / #3a3a3c.
        let paragraph = NSMutableParagraphStyle()
        paragraph.lineHeightMultiple = 1.65
        let attributes: [NSAttributedString.Key: Any] = [
            .font: NSFont.systemFont(ofSize: 14),
            .foregroundColor: NSColor(Theme.textBody),
            .paragraphStyle: paragraph,
        ]
        textView.defaultParagraphStyle = paragraph
        textView.typingAttributes = attributes
        textView.font = .systemFont(ofSize: 14)
        textView.textColor = NSColor(Theme.textBody)

        let scroll = NSScrollView()
        scroll.documentView = textView
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        scroll.drawsBackground = false
        context.coordinator.textView = textView
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        context.coordinator.onEdit = onEdit
        guard let textView = scroll.documentView as? NSTextView else { return }
        // The version contract: only push core text into the view when the
        // document changed underneath us (day switch, load). Never on the
        // render echo of the user's own typing — that would fight the caret.
        if context.coordinator.appliedVersion != version {
            context.coordinator.appliedVersion = version
            if textView.string != text {
                textView.string = text
            }
        }
    }

    @MainActor
    final class Coordinator: NSObject, NSTextViewDelegate {
        var onEdit: (String) -> Void
        var appliedVersion: UInt64 = 0
        weak var textView: NSTextView?

        init(onEdit: @escaping (String) -> Void) { self.onEdit = onEdit }

        func textDidChange(_ notification: Notification) {
            guard let textView else { return }
            onEdit(textView.string)
        }
    }
}
```

- [ ] **Step 3: Swap it into DayColumn**

In `apple/Daily/DayColumn.swift`, the column header stays fixed and the editor fills/scrolls the rest (recorded deviation: the reference scrolls header+body together; with only the note in the Phase 1 column this is visually identical, and the layout gets revisited when the briefing/task sections join the column in Phase 2):

```swift
import SwiftUI

struct DayColumn: View {
    let day: DayVm
    let onEdit: (String) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("DAILY NOTE")
                .font(Theme.Typography.capsLabel)
                .kerning(0.66)
                .foregroundStyle(Theme.textMuted)
                .padding(.bottom, 3)
            Text(day.title)
                .font(Theme.Typography.dateTitle)
                .kerning(-0.5)
                .foregroundStyle(Theme.textPrimary)
                .padding(.bottom, 12)
            ZStack(alignment: .topLeading) {
                NoteEditor(text: day.note_text,
                           version: day.editor_version,
                           onEdit: onEdit)
                if day.note_text.isEmpty {
                    Text("Type to keep writing…")
                        .font(Theme.Typography.ghost)
                        .foregroundStyle(Theme.textQuiet)
                        .allowsHitTesting(false)
                        .padding(.top, 4)
                }
            }
            .frame(maxWidth: Theme.Metrics.noteMaxWidth, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .padding(EdgeInsets(
            top: Theme.Metrics.contentPaddingTop,
            leading: Theme.Metrics.contentPaddingH,
            bottom: 40,
            trailing: Theme.Metrics.contentPaddingH))
        .background(Color.white)
    }
}
```

(Ghost-overlay nuance: `day.note_text` follows keystrokes only after the debounced echo round-trip, so the placeholder can linger ~½ s after the first keystroke. If that looks wrong in practice, track emptiness in the Coordinator and surface it via a `@Binding isEmpty` instead — a contained change; note whichever variant ships in the PR.)

Update the call site in `ContentView.swift`:

```swift
                DayColumn(day: core.view.day, onEdit: { core.noteEdited($0) })
```

- [ ] **Step 4: Build + manual acceptance checklist (paste results into the PR)**

Run: `just app && cd apple && just run`, then verify:
1. Click into the note area, type two paragraphs (with an empty line between) → wait 1 s → quit (⌘Q) → relaunch → text is back exactly, including the empty line.
2. Type continuously for several seconds → the caret never jumps and no characters drop (the version guard: background renders from the save acks must not reset the view).
3. Type on Today, then immediately (within the 500 ms debounce) click yesterday in the calendar → both days keep their own text when you navigate back and forth (flush-before-navigate against the captured date).
4. Empty day shows the ghost line; it disappears once text lands.
5. Undo (⌘Z) works within a day's editing session.
6. While the app is running, `sqlite3 ~/Library/Application\ Support/Daily/daily.db "SELECT entity_type, body FROM search WHERE search MATCH 'YOUR_TYPED_WORD';"` returns the block — FTS is live end-to-end.
7. Typography: 14px body, visibly ~1.65 line height, `#3a3a3c` text.

- [ ] **Step 5: Commit + PR**

```bash
git add apple/Daily
git commit -m "feat(apple): plain-text daily-note editor with debounced saves"
git push -u origin p1/t8-note-editor
gh pr create --fill   # spec-deltas: none (spec §6's Phase-1 editor exactly: NSTextView/TextKit2, plain text, tokens deferred to Phase 3)
```
STOP for review.

---

### Task 9: Day-navigation and persistence proofs, end to end

**Files:**
- Create: `runtime/tests/notes_flow.rs`
- Modify: none expected (this task exists to CATCH integration bugs; any fix it forces lands in this PR with a plan-amendment note)
- Test: `runtime/tests/notes_flow.rs` + the whole-phase manual checklist (Step 3)

**Interfaces:**
- Consumes: everything Tasks 1–8 built. No new surface.

**Riders:** none.

**TDD cadence note:** these are integration *proofs*, not feature drivers — if Tasks 4/5 are correct they pass on first run, and that green run is the evidence. If any fails, that is a real cross-seam bug: fix it in this PR and note the fix. Do not weaken a proof to make it pass.

- [ ] **Step 1: Write the proofs**

`runtime/tests/notes_flow.rs`:

```rust
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
        rt.send_event(Event::Startup { today: TODAY.into() });
        rt.send_event(Event::EditDay {
            date: TODAY.into(),
            text: "persisted line\n\nsecond".into(),
        });
        poll_until(5, "day text to reach the database", || {
            db_day_text(&db, TODAY).as_deref() == Some("persisted line\n\nsecond")
        });
    } // runtime dropped = the app quit

    // "Relaunch": a fresh runtime over the same file shows the text.
    let rt2 = AppRuntime::new(Some(&db), Arc::new(NullShell)).unwrap();
    rt2.send_event(Event::Startup { today: TODAY.into() });
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
    rt.send_event(Event::Startup { today: TODAY.into() });

    rt.send_event(Event::EditDay { date: TODAY.into(), text: "alpha".into() });
    rt.send_event(Event::NavigateToDay { date: YESTERDAY.into() });
    rt.send_event(Event::EditDay { date: YESTERDAY.into(), text: "beta".into() });
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
    rt.send_event(Event::Startup { today: TODAY.into() });
    poll_until(5, "startup day load", || rt.view().day.date == TODAY);
    let after_load = rt.view().day.editor_version;

    rt.send_event(Event::EditDay { date: TODAY.into(), text: "typed by hand".into() });
    poll_until(5, "edit to reach the database", || {
        db_day_text(&db, TODAY).as_deref() == Some("typed by hand")
    });
    assert_eq!(
        rt.view().day.editor_version, after_load,
        "the save round-trip of the user's own edit must not bump the version"
    );

    rt.send_event(Event::NavigateToDay { date: YESTERDAY.into() });
    poll_until(5, "navigation to bump the editor version", || {
        rt.view().day.editor_version > after_load
    });
    std::fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 2: Run the whole suite**

Run: `just test && just app-test`
Expected: all Rust tests PASS (the three proofs included), Swift tests PASS. Any failure here is a real integration bug — fix it in this PR (plan-amendment note in the PR description).

- [ ] **Step 3: Whole-phase manual E2E checklist (the phase-gate dry run; paste results into the PR)**

Run `cd apple && just run`:
1. Fresh launch on a new day → today's title, empty note with ghost line, calendar has today selected.
2. Type a note → relaunch → still there (SQLite persistence, the Phase 1 headline).
3. Calendar: click yesterday → empty note for yesterday, title correct; type something; click Today in the sidebar → today's note intact; back to yesterday → its text intact.
4. Month arrows page to June and August and back; clicking a day in another month navigates to it and the calendar follows.
5. `+` in the toolbar → add a task → Inbox count increments; MCP `create_task` from Claude Code → Inbox count increments live with no user action.
6. MCP `list_tasks` returns the tasks (read-only reader over the live file).
7. Quit via ⌘Q immediately after typing → relaunch → at most the final ~½ s of typing lost, everything else present (recorded T8 limitation).
8. The whole window against the reference §§1/2/5 one more time (T7's checklist), now with live editing.

- [ ] **Step 4: Commit + PR**

```bash
git add runtime
git commit -m "test(runtime): day-navigation and persistence proofs end to end"
git push -u origin p1/t9-day-navigation
gh pr create --fill   # spec-deltas: none
```
STOP for review.

---

### Task 10: Phase close — apple CI test step, docs, review sweep

**Files:**
- Modify: `.github/workflows/ci.yml` (apple job runs the Swift tests), `README.md` (dev loop + current-plan pointer)
- Test: CI itself (the job must pass on this PR)

**Interfaces:**
- Consumes: `just app-test` (T6).

**Riders:** none. (For the avoidance of doubt: the ledger's Phase 5 items — port-collision fallback + discovery file — are NOT this phase's scope and are re-recorded in After Phase 1 below.)

- [ ] **Step 1: CI — the apple job builds AND tests**

In `.github/workflows/ci.yml`, replace the apple job's final step (`- run: just app`) with:

```yaml
      - run: just app-test
```

(`xcodebuild test` builds the app as the test host, so this supersedes the build-only step — one step, both proofs. The job name `apple` is unchanged, so the required status check needs no branch-protection edit.)

- [ ] **Step 2: README**

Update `README.md`: the dev-loop block gains `just app-test   # Swift unit tests`, and the "Current plan" link moves to `docs/superpowers/plans/2026-07-04-phase-1-shell-and-notes.md`.

- [ ] **Step 3: Review sweep (do, then record in the PR description)**

1. `just test && just app-test` — paste both green summaries.
2. Walk this plan top to bottom: every checkbox ticked across the merged PRs; every task's Riders line satisfied or explicitly moved. List any stragglers (there must be none).
3. Walk the reference acceptance sections (§§1, 2, 5) against the running build; list every residual deviation with its recorded rationale (expected: real chrome adaptation, omitted search field, hidden empty sections, header-fixed scroll region, sidebar flat tint). This list is Jon's phase-gate briefing.
4. Grep sweep: `grep -rn "TODO\|FIXME\|unimplemented\|todo!" shared store mcp runtime apple/Daily` → empty (CI guards this too).
5. Confirm `apple/generated/` is untouched in every merged PR.

- [ ] **Step 4: Commit + PR**

```bash
git add .github README.md
git commit -m "chore(p1): phase close — apple CI test step, docs, review sweep"
git push -u origin p1/t10-phase-close
gh pr create --fill   # spec-deltas: none
```
After Jon merges and uses the build, he tags: `git tag phase-1 && git push origin phase-1` (Jon's action, per SDLC — the phase gate is his call, not this plan's).

---

## Self-review notes (checks performed while writing this plan; findings fixed in place)

**Spec §10 Phase 1 coverage walk** — "Shell + notes — window/sidebar/calendar to spec, daily note editor (plain text blocks), day navigation, persistence":
- Window/sidebar/calendar to spec → T6 (tokens), T7 (chrome/sidebar/calendar), acceptance = reference §§1/2/5 with recorded carve-outs.
- Daily note editor, plain text blocks → T4 (notes/blocks/FTS schema + ops), T5 (core model), T8 (NSTextView editor).
- Day navigation → T5 (events/calendar), T7 (UI wiring), T9 (proofs).
- Persistence → T4 (transactional writes + FTS), T9 (restart proofs), T8 checklist (relaunch).
- Ledger early-Phase-1 mandates, all five present: migration-error path (T1), ViewReader→SQLite reader incl. db_path retention (T3), StorageOperation growth decision (decision #1, applied T4), Swift-6 let-ify + idempotency comment (T1), catch-all re-examination (T5's explicit wrong-shape arms), test-helper consolidation (T2).
- Deliberate exclusions (later phases, per spec §10): focus bar, task rows/buckets, briefing block, Waiting on, resurfaced card, collapsed Next/Later (P2–P6); toolbar search + `search`/`get_day` MCP tools (P5); space switcher behavior (P6); Time capability/rollover (P4); Todoist/Craft import (P7).

**Placeholder scan** — no TODO/FIXME/stub code anywhere in the plan's code; every "placeholder-looking" UI decision renders real data or is data-driven-absent (Global Constraints); the T5 interim ContentView is working UI explicitly deleted in T7 (delete-don't-pause).

**Type-consistency check** — issues found while cross-checking task interfaces, each fixed in the task text:
1. `runtime/tests/ffi.rs`'s busy-port test would silently stop testing bind failure once T3 makes in-memory cores return 0 from `start_mcp` for a different reason → T3 Step 7 moves it to a temp-file DB.
2. Removing `ViewModel.tasks` (T5) would have broken `ViewReader` — resolved by ordering T3 (deletes ViewReader, repoints the e2e poll at the SQLite reader) before T5, and T5 repoints remaining view polls at the sidebar inbox count.
3. `Event::Startup` gaining a field breaks the Swift build inside the same PR (the `apple` CI job regenerates types) → T5 Step 8 carries the minimal Swift patch; same pattern for the ViewModel reshape.
4. `DayVm` carries no `is_today`/`today` — checked every consumer: Today-row styling uses `sidebar.today_label`, and `GoToToday` means the shell never needs today's date back. No dead fields.
5. Every `StorageResult` variant ↔ handler pairing audited: each event matches its expected variant(s) + `Error` + `wrong_shape` fallback; the executor match is exhaustive by construction (a new variant fails compilation there — decision #1's safety property).
6. `poll_until` is blocking; audited every async call site — always after client I/O completes, and the runtime's storage/MCP work runs on dedicated OS threads. Documented on the helper.
7. Reference §2.3 ambiguity (outlined circle on July 1) resolved and recorded in T7: filled circle = selected day; outline = today when not selected.

**Known soft spots, each with its named arbiter:** BoltFFI regeneration of the changed `CoreFFI` surface (T1 — arbiter: `just generate` + task-5-report shapes); generated Swift field casing (T5 Step 8 caveat — arbiter: first `just app` build); TextKit 2 wiring details (T8 — arbiter: manual checklist); `Command::all`/`effects()` verified against the vendored crux_core 0.19.0 source but re-check on any patch bump.

## After Phase 1

Phase 2 (**Tasks** — model, buckets/status/priority, task rows, triage sheet + keyboard, Inbox, status board) gets its own plan after Jon uses this build and the feedback folds in. Carried forward explicitly so nothing silently drops:
- Phase 2 revisits the StorageOperation single-enum decision at the gate (decision #1's stated checkpoint) and replaces the Phase 0 `Task {id, title}` type with the full task model.
- Phase 3 takes the ghost-line binding variant (if the T8 overlay lags annoy), block-identity/fractional order keys, and the FFI log-and-degrade pass (spec §8's scheduled hardening).
- Phase 4 takes the Time capability (day rollover replaces the launch-time `today`).
- Phase 5 takes the toolbar search field + `search`/`get_day` MCP tools, and the ledger's port-collision fallback + discovery file.
- Polish backlog (any phase): sidebar system-material look, storage-drain ack on quit (replacing the 200 ms bound), DailyKit sub-package extraction if the Swift file count starts to hurt.
