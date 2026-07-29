# Daily — Design Spec

**Date:** 2026-07-02
**Status:** Living — Phases 0–1 shipped; amended via changelog
**Product:** "Daily" — a calm todo + daily-notes app for macOS (repo codename: Yardstick)
**Architecture direction (fixed by Jon):** Crux (Rust core) + native macOS shell

This spec turns the design handoff in [`docs/design/handoff/`](../../design/handoff/README.md) into build-ready decisions. The handoff README is the product spec (principles, data model, screens, tokens); the extraction docs in [`docs/design/reference/`](../../design/reference/) are the pixel/behavior source of truth; the reports in [`docs/research/`](../../research/) ground every technology decision below. This document does not repeat what those cover — it decides what was open.

---

## 1. What we're building (one paragraph)

A macOS-native "second brain" for a user with ADHD: one daily note per day per space, tasks in Now/Next/Later buckets with orthogonal statuses, exactly one focused task at a time in a persistent bar with a timer, freeform notes organised around pages — projects (nested one level to express initiatives), people and meetings — with `@`/`#` mentions that auto-backlink onto both pages and tasks, one All-actions view where every task from every source is managed in a single editable list, an automated external brief whose actions get one-tap triage each morning, and everything readable/writable by external AI agents over MCP. Emotional target: calm — no red alarms, no badge shouting, gentle resurfacing.

## 2. Architecture

### Approaches considered

**A — Single app process: Crux core + Rust-side effect handlers + embedded MCP server (chosen).**
One macOS app process hosts: the pure Crux core (`shared` crate), a Rust runtime layer that handles storage effects in Rust via crux 0.19's `EffectRouter` (SQLite never crosses FFI), a tokio runtime serving MCP over streamable HTTP on `127.0.0.1`, and the SwiftUI shell receiving only UI-relevant effects (Render, etc.) over BoltFFI. One process owns the DB → zero multi-process SQLite concerns; MCP writes dispatch the same core events the UI does, so external writes update the UI live.

**B — GUI app + separate stdio MCP binary sharing the SQLite file (WAL).**
Simpler MCP client config and works when the app is closed, but creates two write paths (schema/FTS/invariant drift risk), no live UI push on external writes, and mid-migration launch footguns. Rejected as the primary topology; remains reachable later because the store and domain logic live in their own crates.

**C — Always-on daemon owns the DB; GUI and MCP are clients.**
Cleanest single-writer story but the highest engineering cost (launchd lifecycle, versioned IPC, upgrade coordination). Overkill for a single-user personal tool. Rejected for v1; same escape hatch applies.

The crate layering (`store` = DB + domain, `mcp` = tool layer, thin binaries choose transport) keeps B and C reachable without rework — this is the insurance policy, not speculative structure.

### Chosen topology (A) — how the pieces talk

```
┌────────────────────────── Daily.app (one process) ──────────────────────────┐
│                                                                              │
│  SwiftUI shell (@Observable Core)  ◄── BoltFFI: serialized effects ──┐       │
│    │ events (bincode)                                                │       │
│    ▼                                                                 │       │
│  EffectRouter ── Render/UI effects ──────────────────────────────────┘       │
│    │                                                                         │
│    ├─ Storage effects ──► StorageHandler thread (rusqlite, FTS5, WAL)        │
│    ├─ Time effects ─────► timer glue                                         │
│    │                                                                         │
│  Crux core (`shared`: pure Model/Event/ViewModel)                            │
│    ▲ events                                                                  │
│  MCP server (rmcp, streamable HTTP, 127.0.0.1:52111, bearer token)           │
│    reads: direct read-only queries via `store`                               │
│    writes: dispatch core Events (same path as the UI)                        │
└──────────────────────────────────────────────────────────────────────────────┘
External agents: Claude Code → HTTP directly; stdio-only clients → `mcp-remote` shim.
Briefing skill → MCP `write_brief` tool.
```

**"App not running" story:** Daily is a menu-bar + login-item app, so in practice it is always on. The MCP setup instructions include the `mcp-remote` stdio bridge which can `open -g -a Daily` and retry. If headless access ever becomes a hard requirement, promote `store`+`mcp` into a stdio binary (topology B).

### Workspace layout

```
Yardstick/
├── Cargo.toml            # workspace, resolver=3
├── rust-toolchain.toml
├── shared/               # PURE Crux app: Model, Event, ViewModel, Effect. No I/O, no tokio.
│   ├── src/app.rs        #   App impl; update() -> Command<Effect, Event>
│   ├── src/model/…       #   domain: tasks, notes, briefs, focus, pages, triage
│   ├── src/view/…        #   ViewModel builders per screen
│   ├── src/effects/…     #   StorageOperation, TimeOperation, … (Operation types)
│   └── src/bin/codegen.rs#   facet typegen → Swift package "App"
├── store/                # rusqlite + rusqlite_migration + FTS5. StorageHandler thread.
│   └── migrations/*.sql
├── mcp/                  # rmcp 2.x tool layer over `store` (reads) + core events (writes)
├── runtime/              # EffectRouter wiring: core + store handler + mcp server + BoltFFI export
│   └── src/ffi.rs        #   #[boltffi::export] CoreFFI + CruxShell callback trait
└── apple/
    ├── Justfile          # typegen / package / generate-project / dev
    ├── project.yml       # XcodeGen; macOS target, deployment 15.0
    ├── generated/        # (gitignored) Swift pkgs: "Shared" (FFI), "App" (types)
    └── Daily/            # SwiftUI sources; DailyKit sub-package for views + FakeBridge previews
```

### Pinned toolchain (from research — exact pins matter)

| Piece | Choice | Version note |
|---|---|---|
| Core framework | `crux_core` | **0.19** (Command-only API, EffectRouter, BoltFFI era) |
| FFI | BoltFFI | pin `boltffi = "=0.25.2"` + `boltffi_cli =0.25.2` (crates.io is ahead; examples pin — follow the examples) |
| Typegen | facet-based (`crux_core::type_generation::facet`) | pin `facet = "=0.44"` exactly |
| DB | `rusqlite` (bundled) + `rusqlite_migration` | FTS5 on by default in bundled builds |
| MCP | `rmcp` | **2.x** — pin minor; 2.0 aligned with MCP 2025-11-25 spec |
| Apple build | XcodeGen + Justfile + SwiftPM local packages | the current Crux counter/weather example pattern |
| Hotkey | sindresorhus/KeyboardShortcuts | Carbon API — no Accessibility permission |
| macOS target | 15.0 minimum | rich `TextEditor` (macOS 26) not required — editor is TextKit 2 |

Known-young pieces (accepted risk, mitigated by Phase 0 walking skeleton): BoltFFI (~1 month old as default), EffectRouter (RFC-stage), rmcp 2.x (weeks old). All three are exercised end-to-end in Phase 0 before any feature work.

## 3. Data model & storage

Entities follow the handoff README §"Information architecture" (Space, DailyNote, Brief, Task, Page, FocusSession), with Page carrying meetings and initiatives as of the 2026-07-29 amendment (§13). Storage decisions:

- **SQLite schema** (see research/persistence-fts.md §3 for the full sketch): `spaces`, `notes` (one per date per space, or one per page — see §3.1), `blocks` (one row per note block; JSON `content` + extracted `plain_text`), `tasks` (bucket + status orthogonal; `parent_id` one level; `prev_status` for checkbox-untick restore), `pages`, `links` (one polymorphic edge table for all refs/backlinks), `source_links` (external provenance URLs), `briefs` (JSON payload + `rendered_text`), `focus_sessions`, and one unified `search` FTS5 table (`entity_type`, `entity_id`, `title`, `body`) maintained in the same transaction as every entity write. All tables STRICT.
- **IDs are UUIDv7** (client-generated, sortable). **`updated_at` on every table. Soft deletes (`deleted_at`).** These three conventions are the cheap sync-readiness insurance; **no CRDT/automerge in v1** (single device, single user — YAGNI, and the event-sourced core + storage-effect boundary means a sync layer can be added inside `store` later without touching core or shell).
- **`space_id` scopes every entity from migration 001** even though the space switcher UI ships late — retrofitting scoping is far more expensive than carrying it.
- **Note content model:** blocks of plain text with explicit token markup — `@[Tomash](person:UUID)`, `#[COAST](project:UUID)`, `#[Weekly COAST sync](meeting:UUID)`, `#[Fix the CI flake](task:UUID)`, task blocks referencing `task:UUID`. Never persist `NSAttributedString`; the Swift editor parses markup → attributed runs/attachments and serializes back.
- **Derived views are queries, not copies:** Now/Next/Later lists, page Actions, Waiting-on, the All-actions view, resurfacing candidates (`bucket='later' AND (age > 30d OR due within 7d)`) are all SQL against the same tasks.
- **WAL + `busy_timeout=5000` + `BEGIN IMMEDIATE`** from day one — costless now, prerequisite for any future second process.

### 3.1 Pages: projects, initiatives, people, meetings (amendment 2026-07-29)

One table carries every kind of "thing you can write notes on and link to". There is no separate meetings table and no separate initiative entity — both were considered and rejected (§13).

```sql
CREATE TABLE pages (
  id          TEXT PRIMARY KEY,
  space_id    TEXT NOT NULL REFERENCES spaces(id),
  kind        TEXT NOT NULL,               -- 'project' | 'person' | 'meeting' | 'page'
  name        TEXT NOT NULL,
  parent_id   TEXT REFERENCES pages(id),   -- ONE level only; project under project = initiative
  accent      TEXT,                        -- project swatch / person avatar hue
  occurred_at INTEGER,                     -- meetings only; NULL for every other kind
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER
) STRICT;
```

- **An initiative is a project nested under a project.** `kind` gains no fourth value and the UI gains no fifth concept: a project page lists its children and rolls up their actions. Nesting is one level, enforced in the core (a page whose `parent_id` is set cannot itself be a parent).
- **A meeting is `kind='meeting'` plus `occurred_at`.** Attendees are `links` rows from the meeting to person pages, so "every meeting with Tomash" and "every meeting touching COAST" are the same query as any other backlink. Meetings inherit the page's freeform note, its aggregated Actions list, search and MCP surface for free.
- **Page notes reuse `notes`/`blocks`.** Migration 003 makes a note belong to *either* a date or a page. `notes.date` is currently `NOT NULL` inside a STRICT table, which SQLite cannot relax via `ALTER`, so 003 **rebuilds `notes`** (copy → drop → rename, inside the migration transaction): `date TEXT` nullable, new `page_id TEXT REFERENCES pages(id)`, a `CHECK` that exactly one of the two is set, and the old `UNIQUE (space_id, date)` replaced by two partial unique indexes (`WHERE date IS NOT NULL` / `WHERE page_id IS NOT NULL`). Pre-release table, negligible data risk, but it is a rebuild rather than an add-column and Phase 3's plan owns proving it with a migration test over a seeded database.
- **`links.dst_type ∈ {'page','task'}`.** Task-to-task links express related work without forcing false parent/child structure; `parent_id` subtasks stay the only hierarchy. Block-level references are explicitly out of scope (they fight the full-rewrite block model and Phase 3's block identity work).
- **Provenance gets one identity-free table**, rewritten with its source exactly like `links`:

```sql
CREATE TABLE source_links (
  src_type TEXT NOT NULL,   -- 'task' | 'page' | 'brief_item'
  src_id   TEXT NOT NULL,
  url      TEXT NOT NULL,
  label    TEXT NOT NULL,   -- the grey source chip's text, e.g. 'Krisp'
  ordinal  INTEGER NOT NULL,
  PRIMARY KEY (src_type, src_id, url)
) STRICT;
```

This replaces the per-entity `sources: [links]` sketch in the handoff data model: one table serves task provenance, meeting recordings, and brief items.

## 4. Crux core design

- **Model** holds the *working set*: current space, loaded day (note blocks, brief, actions triage state), task lists for visible views, focus session state, and UI state that must survive shell restarts. It is not an in-memory copy of the whole DB; the core asks the store for what a view needs via Storage effects.
- **Events** are the single write path for both UI and MCP: `CaptureTask{title, source}`, `TriageTask{id, bucket, priority, due, refs}`, `SetStatus{id, status, reason?}`, `ToggleDone{id}`, `StartFocus{id}` / `PauseFocus` / `SwitchFocus` / `CompleteFocus`, `EditBlock{...}`, `ConvertLineToTask{...}`, `SortBriefAction{...}`, `CombineAction{...}` (merge / add-as-subtask / make-parent), `ResurfaceDecision{...}`, `WriteBrief{date, payload}` (from MCP), `DayRollover`, `NavigateToDay{date}`, `SearchQueryChanged{q}`, … plus, from the 2026-07-29 amendment: `CreatePage{kind, name, parent?}`, `SetPageParent{id, parent?}` (rejects two-level nesting), `EditPageNote{page_id, text}`, `WriteMeeting{payload}` (from MCP), `ActionsQueryChanged{group_by, filters, sort}` and `BulkUpdateTasks{ids, patch}` (the All-actions view).
- **Effects:** `Render`, `Storage(StorageOperation)` (domain-typed operations — `UpsertTask`, `QueryBucket`, `Search{q}`, `GetDay{date}`, `WriteBrief{…}` — not raw SQL strings; keeps core tests meaningful and the store swappable), `Time` (now + notify-at for rollover), `OpenUrl` (Krisp/Gmail/Slack deep links — handled by Swift), `Hud` (transient confirmations, if needed later).
- **Focus timer:** core owns `PersistedTimer {accumulated, running_since, last_checkpoint}` semantics (pure, testable); shell renders elapsed locally (self-updating `Text(timerInterval:)`) — **no per-second FFI traffic**. Checkpoint every 30 s via Storage effect; wall-clock-jump hardening per research/swiftui-macos-ui.md §4. Sleep semantics: timer keeps counting through short sleeps; on wake after >30 min asleep, auto-pause and show the bar in paused state (calm default — decided, cheap to change).
- **Day rollover** (core logic, triggered by a Time effect at local midnight + on wake/launch): unfinished Now tasks stay in Now, age counter derives from `created_at`/`entered_now_at`; done rows purge from Today view; a new daily note is created lazily on first view/edit.
- **ViewModel** is per-screen, cheap to serialize (it crosses FFI on every render), with pre-formatted display strings (ages as "2 days old", timer *not* included — see above).

## 5. MCP surface (v1)

Tools (all space-scoped via a `space` param defaulting to the work space): `search{query, limit}`, `get_day{date}` (note + brief + actions + tasks touched that day), `list_bucket{bucket}`, `create_task{title, bucket?, priority?, due?, refs?, source_links?}`, `update_task{id, …patch}`, `write_brief{date, payload}` (upsert; shape = the Brief JSON from the handoff), `get_page{id|name}`. Resources deliberately skipped in v1 — agents exercise tools more reliably.

Meetings and pages add four tools (amendment 2026-07-29): `write_meeting{occurred_at, title, attendees[], notes, actions[], source_links[]}` (upsert by `(space, title, occurred_at)`; creates the meeting page, its note, its attendee links, and its actions as untriaged tasks carrying links back to the meeting and to the source), `get_meeting{id | title+date}`, `list_meetings{from, to}`, and `create_page{kind, name, parent?}` (the write half of `get_page`, so an agent can file a new project, initiative or person).

**Krisp stays outside the app.** A Claude skill reads Krisp through its own MCP connector and calls `write_meeting`, exactly as the briefing skill calls `write_brief`. No Krisp-specific code, credentials or network dependency enters Daily, and hand-written meeting notes are indistinguishable from ingested ones.

Auth: static bearer token generated on first run, stored `0600` at `~/Library/Application Support/Daily/mcp-token`; bind `127.0.0.1` only; validate Origin/Host (rmcp config). Settings UI gets a "copy Claude Code setup command" button. Port 52111 default; on collision pick a free port and write it to a discovery file next to the token.

**Consistency rule:** MCP reads go straight to `store` (read-only). MCP writes **must** dispatch core Events so invariants, FTS, links, and live UI refresh all follow the one path.

## 6. Swift shell

- **Structure:** `NavigationSplitView`; custom sidebar via `List(.sidebar)` + `scrollContentBackground(.hidden)` + tint (lean into system material rather than fighting for full opacity); `@Observable @MainActor Core` wrapper processing serialized effects; `CoreBridge` protocol + `FakeBridge` so previews never load Rust; per-capability `extension Core` handlers.
- **Daily-note editor — the hardest UI piece:** one `NSTextView` (TextKit 2, `usingTextLayoutManager: true`) in `NSViewRepresentable`. Mentions = `NSTextAttachment` + `NSTextAttachmentViewProvider` hosting SwiftUI chip capsules (atomic delete, native click/hover). Live `@`/`#` picker = `NSPopover` anchored at `firstRect(forCharacterRange:)`, arrow/enter/escape routed via `doCommandBy`. `[ ]` line conversion intercepted in the text-storage delegate → checkbox attachment + `taskID` paragraph attribute (custom `NSTextLayoutFragment` only if design later demands the full widget look). Phase 1 ships plain-text blocks; tokens arrive in Phase 3.
- **Quick capture:** global hotkey via KeyboardShortcuts, **default ⌥Space** (⌘Space is Spotlight — the mock's label follows the user's configured shortcut), non-activating `NSPanel` (`.nonactivatingPanel`, floating, all-Spaces, Esc/click-away dismisses) → `CaptureTask` event → Inbox with source tag.
- **Menu bar:** `MenuBarExtra` — timer in the label via self-updating `Text(timerInterval:)` (monospaced digits), `.window` style body with today's Now list, focus controls, and a capture field. `LSUIElement` stays **NO** (normal Dock app + menu bar presence).
- **Mention pickers use two sigils, not five** (amendment 2026-07-29): `@` picks people; `#` picks projects, initiatives, meetings and tasks in one ranked list, with the chosen type recorded in the markup and shown by the chip's styling. A third sigil per entity kind would be a keystroke tax for no gain.
- **Sidebar sections** (all data-driven, absent when empty): Projects, with initiatives disclosed under their parent project; People; Meetings, listed newest-first by `occurred_at`; Pages. The Meetings section and the initiative disclosure are deliberate additions to design reference §2, which predates this amendment; everything else there still governs.
- **All-actions view** (amendment 2026-07-29): one editable list of every task in the space regardless of bucket, status or page — group, sort, filter, edit inline, multi-select for bulk edits, full keyboard. It **supersedes** the "All tasks · by status" board from the handoff's journey 5 rather than sitting beside it; the board's status grouping becomes one of this view's grouping options. Its detailed shape (columns, filter chips, keyboard map) is decided in the Phase 2 plan against the design reference, not here.
- **Design fidelity:** tokens and metrics live in one `Theme` namespace generated from the handoff token list; the extraction doc (`docs/design/reference/v2-today-view.md`) is the acceptance reference for the Today view, including the two distinct chip systems, 70px meta column, 0.5px hairlines, and state variant catalog.

## 7. Resolved questions

Decisions on the handoff's four open questions (Jon: veto before the affected phase — see §10):

1. **Overdue trace:** silent roll-forward with the grey age label only. No "moved from Tue" footprint in v1 — the age label already carries the information; a footprint is additive later.
2. **Todoist/Craft:** **one-time migration, no two-way sync.** Keep using Todoist/Craft until Daily is daily-drivable (end of Phase 5); then run the importer (Todoist projects→projects, Now/Later sections→buckets, labels→status/focus, subtasks→parent_id, descriptions→notes+links; Craft markdown→note blocks, "## Briefing" sections→Brief records) and switch. Two-way sync would double the surface area of every write path and directly fights the "single knowledge base" principle.
3. **Menu-bar scope:** capture **and** focus timer (title shows mm:ss while a session runs; window = capture field + Now list + focus controls). Research shows the timer label is nearly free.
4. **Yardley space parity:** full parity is automatic — spaces are a data-model dimension and every MCP tool takes a `space` param. Whether the briefing skill writes a Yardley brief is that skill's configuration, not an app feature.

Ambiguities found in the mocks, resolved (source: `docs/design/reference/core-journeys.md` §Open questions):

- **Focus bar is global chrome** across all views (the "home base").
- **Two distinct note-capture mechanisms:** `[ ]` at line start converts in place → defaults **Now** (per Journey 2B); a separate explicit "send to Inbox" capture command (from selection/quick-add while in a note) produces Inbox items tagged "from note" (per Journey 1A). Both exist; no contradiction.
- **Next-up suggestions: strict P1-first, then age** (README rule wins; the mock's ordering was sloppy).
- **`F` targets the hovered row**, else the keyboard-selected row.
- **Priority badges always render when priority is set**; priority is optional.
- **Person page split:** "Waiting on {Name}" = tasks with status `waiting` that ref the person; "Assigned / shared" = every other non-terminal task refing them.
- **Checkbox ↔ status:** checking sets `done` (stores `prev_status`); unchecking restores `prev_status` (default `backlog`). `binned` is only reachable via the status menu / Bin buttons.
- **Blocked reason:** setting status → Blocked prompts for an optional one-line reason (the board shows it).
- **Combine "Tomasz/Tomash"** was a mock typo; suggested matches rank by shared person/project refs.

## 8. Error handling

- **Storage errors** (the only real failure source in-process): storage thread returns typed `StorageResult::Error`; core surfaces a calm inline banner ("Couldn't save — retrying") and retries idempotent writes; the app never crashes on DB errors. DB corruption → startup integrity check, offer to restore from the previous-launch backup copy (cheap `VACUUM INTO` on each clean quit).
- **MCP errors:** tool handlers map domain failures to MCP error responses with actionable messages; malformed `write_brief` payloads are rejected with the expected schema echoed back (the briefing skill iterates against this).
- **FFI:** decode failures at the FFI boundary are treated as typegen-contract violations and panic on both sides (matches the generated BoltFFI contract and the canonical crux shells) — Phases 0–2. A log-and-degrade hardening pass is scheduled with the Phase 3 backlinks work at the latest; revisit when the Effect surface grows beyond Render.
- **External links** (Krisp/Gmail/Slack): fire-and-forget `NSWorkspace.open`; no error UI beyond a silent log if the URL scheme is unhandled.

## 9. Testing strategy

- **Core (the bulk):** pure Rust tests driving `update()` directly with `EffectTestExt` fluent assertions (`crux_core/testing`). Every journey in `docs/design/reference/core-journeys.md` becomes a test module: triage, `[ ]` conversion defaults, focus singleton + chaining, rollover aging, resurfacing (one card max), combine ops, brief action sorting. Target: journeys fully covered before their UI exists.
- **Store:** integration tests against in-memory SQLite — schema migrations (`MIGRATIONS.validate()`), FTS round-trips, link rewrites, WAL/IMMEDIATE behavior.
- **MCP:** in-process rmcp client tests: tool schemas, write→event→store round-trip, auth rejection.
- **Swift:** thin by design — previews via `FakeBridge`; a small XCUITest smoke (launch, render Today, capture panel opens). Pixel fidelity is checked against the extraction doc by eye, not snapshot tests, in v1.
- **Runner:** `cargo nextest` + `just test` covering both worlds; CI on GitHub Actions (macOS runner) from Phase 0.

## 10. Build phases (summary — detail lives in the implementation plan)

0. **Walking skeleton** — workspace, pinned toolchain, trivial core event→render loop, EffectRouter with storage handled in Rust, BoltFFI/XcodeGen app showing live core state, MCP server answering a `ping` tool, CI. *De-risks every young dependency before feature work.*
1. **Shell + notes** — window/sidebar/calendar to spec, daily note editor (plain text blocks), day navigation, persistence.
2. **Tasks** — model, buckets/status/priority, task rows, triage sheet + keyboard (N/E/L, 1/2/3, #), Inbox, **All-actions view** (replaces the status board).
3. **Pages, meetings and backlinks** — @/# pickers + chips in the editor, migration 003 (`pages`, note rebuild, `source_links`), project pages with initiatives nested one level and auto-pulled Actions, person pages, meeting pages + the Meetings sidebar list, task-to-task links, `[ ]` inline conversion. *If the plan exceeds ten tasks, split into 3a (pages + backlinks) and 3b (meetings).*
4. **Focus** — bar, sessions + persisted timer, suggest-on-idle, done→next chaining, momentum segments, dimming.
5. **Brief + MCP v1** — full MCP toolset + auth (including `write_meeting`, `get_meeting`, `list_meetings`, `create_page`), brief render in yesterday's note, Actions-from-yesterday triage + Combine ops, Waiting on. *← the daily-drivable milestone; briefing skill switches to `write_brief` and the Krisp skill to `write_meeting`.*
6. **Calm systems** — resurfacing, gentle rollover polish, collapsed Next/Later, spaces + switcher (Yardley live).
7. **Capture everywhere + migration** — global hotkey panel, menu-bar extra, Todoist/Craft one-time importer, search polish.

Gate: after each phase, Jon uses the build; feedback folds into the next phase before new scope.

## 11. Risks

| Risk | Mitigation |
|---|---|
| BoltFFI/EffectRouter/rmcp churn (all <2 months old) | Phase 0 exercises all three end-to-end; exact version pins; EffectRouter fallback is 0.16-era middleware which shares the handler shape |
| TextKit 2 editor complexity blows up | Phased: plain blocks (P1) → chips/pickers (P3) → layout fragments only if needed; STTextView as fallback base |
| One-person product drift ("calm" erodes feature by feature) | The handoff's 7 product principles are acceptance criteria in every phase's review |
| MCP writes racing UI edits on the same entity | Single event queue through the core serializes all writes; last-writer-wins at event granularity is acceptable for one user |
| Migration fidelity (Todoist/Craft) | Importer is Phase 7, run against real exports with a dry-run diff report before writing |
| Phase 3 overloaded by the meetings/pages amendment | Stated split point: 3a pages + backlinks, 3b meetings, if the plan passes ten tasks |
| Migration 003 rebuilds `notes` (STRICT tables can't relax `NOT NULL`) | Rebuild happens inside the migration transaction, proved by a migration test over a seeded DB; pre-release data volume is a handful of rows |
| All-actions view becomes the "huge list" the product principles exist to avoid | It is a deliberate management surface, not a daily surface. Today's noise budget (principle 4) is unchanged: the view is reached explicitly, never rendered inside Today |

## 12. Open questions for Jon (none block Phases 0–2)

1. **Naming:** repo is *Yardstick*, the designed product is *Daily*. Ship as "Daily" with Yardstick as codename, or rename the product Yardstick?
2. **Quick-capture default shortcut:** ⌥Space proposed (⌘Space is Spotlight's). Fine, or do you have ⌘Space free (e.g. Spotlight remapped) and want the mock's literal binding?
3. **Brief pipeline cutover:** during Phases 0–4 your briefing skill keeps writing to Craft. OK to run both (Craft + `write_brief`) during Phase 5 for a validation week before switching?
4. §7's four decisions stand unless vetoed before their phase.

## 13. Amendment 2026-07-29 — pages, meetings, initiatives, backlinks, All-actions

Jon's requirement, verbatim in substance: notes are freeform but organised around projects, project initiatives, people and meetings; actions can be created anywhere but managed in a single view holding every task with the tools you'd expect for a big list; meetings are their own thing, simple at first, linkable to projects, initiatives and people, most often sourced from Krisp; and items can be backlinked.

Most of it was already specified — pages with freeform notes plus auto-aggregated actions (§3, §6), capture from anywhere (§6), the polymorphic `links` table shipped empty in migration 002, bucket/status/priority as orthogonal dimensions (§3). Three things were genuinely absent and are decided here.

**Meetings.** Considered: (a) `pages.kind='meeting'` with `occurred_at` and source links — **chosen**; (b) a dedicated `meetings` table with attendees, transcript reference and its own note. (b) is more faithful to the domain but duplicates the page machinery (notes, blocks, backlinks, actions aggregation, search indexing, MCP surface) for one extra column's worth of difference, roughly doubling Phase 3's surface. Attendees as person links are strictly more useful than an attendees column, because they make meetings fall out of the existing backlink queries.

**Initiatives.** Considered: (a) a fourth `kind='initiative'` with a project parent; (b) initiatives *are* projects, nested via `parent_id` — **chosen** (Jon's call); (c) an initiative tag on tasks, no page. (b) means one less concept in the model, in the UI, in the picker and in every MCP tool, at the cost of the word "initiative" having no representation in data — which is fine, because the distinction is about altitude, not behaviour. (c) was rejected because you cannot write notes on a tag.

**One view for actions.** Clarified by Jon as a management surface: view and edit every task from everywhere in one list, with grouping, sorting, filtering, tagging and prioritising. It supersedes the handoff's "All tasks · by status" board (delete-don't-duplicate, SDLC §6). It does **not** replace the "Actions from yesterday" block in Today's note: that block is the two-minute morning ritual on a handful of items, sized by the noise budget (principle 4); the All-actions view is where the long tail is worked. Both surfaces write the same events, so neither is a second write path. The view's detailed shape is a Phase 2 plan decision.

**Backlink targets.** `links.dst_type ∈ {'page','task'}` — pages of every kind, plus tasks for related work. Block-level references (Roam-style) were considered and rejected: they need stable block identity, which Phase 1's full-rewrite block model deliberately does not provide, and the editor cost lands in the same phase as the chips and pickers.

Phase impact: Phase 2 gains the All-actions view; Phase 3 is renamed and absorbs pages, meetings and migration 003; Phase 5 gains four MCP tools and the Krisp skill cutover. Phases 0 and 1 as shipped are untouched — nothing already merged is reworked by this amendment.

## Changelog

- 2026-07-29: Pages/meetings/backlinks amendment — new §3.1 (`pages`
  table with meeting kind and one-level project nesting, `notes` rebuild in
  migration 003, `source_links`, `links.dst_type` covering tasks), §1, §4
  (new events), §5 (`write_meeting` / `get_meeting` / `list_meetings` /
  `create_page`; Krisp stays outside the app), §6 (two-sigil picker,
  sidebar sections, All-actions view superseding the status board), §10
  (Phase 2 and 3 rescoped, Phase 3 renamed, Phase 5 tools), §11 (four new
  risks). Rationale and rejected approaches in §13.
- 2026-07-04: §8 FFI error contract amended to record the as-built panic-on-decode-failure decision (final Phase 0 review).
- 2026-07-04: §3 amended — Phase 1 block rewrites hard-delete superseded
  block rows inside the rewrite transaction (the note row is the
  soft-delete unit; `blocks.deleted_at` remains for Phase 3+ block-level
  editing). The `links` edge table carries no entity conventions
  (identity-free edges, rewritten with their source).
- 2026-07-05: §6 shell-structure deltas as built in Phase 1 — custom
  sidebar layout over a flat tint (not `List(.sidebar)` + system
  material), fixed two-pane `HStack` (not `NavigationSplitView`), previews
  via dumb value-passing views (not `CoreBridge`/`FakeBridge`). §6's prose
  above is unchanged; this line records the delta.
- 2026-07-05: §8 as-built Phase 1 error behavior — storage errors surface
  the raw message in the ViewModel with no retry (no "Couldn't save —
  retrying" banner yet), and DB corruption/migration failure is a calm
  alert + Quit with no backup/restore. The §8 retry + VACUUM-INTO
  backup/restore story is deferred; revisit in the phase that adds the
  briefing pipeline (Phase 5 at the latest).
