# Phase 2 Gate Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the five interaction/layout defects Jon found in his Phase 2 gate walk (2026-08-03), plus the rows-and-checkboxes restyle he asked for, so the app is a comfortable daily driver while Phase 3 is planned.

**Architecture:** No core, store, or MCP changes — every task is Swift-shell-only (`apple/Yardstick/`), plus one design-reference amendment. The Crux core, events and ViewModels are untouched; nothing here needs typegen.

**Tech Stack:** SwiftUI (macOS), XCTest via `just app-test`.

## Where this plan came from (gate feedback routing)

Jon's gate feedback, routed per SDLC §1 (feedback becomes spec/plan revision before new scope):

| Feedback item | Lands where |
|---|---|
| Sidebar shifts + right gap on Now/Next/Later/Waiting | **This plan, Task 1** |
| Checkbox/row only clickable on the ring's border (views + All-actions select) | **This plan, Task 2** |
| No Escape to leave All-actions selection mode | **This plan, Task 2** |
| Double-click edit: checkbox vanishes, layout jumps, second click to focus | **This plan, Task 3** |
| Note cursor much taller than the text's line height | **This plan, Task 4** |
| Things 3-style rows and checkboxes ("rows + checkboxes only" per Jon's answer) | **This plan, Task 5** |
| Tasks created inside notes (`[ ]` → real task), headings/bullets/numbered lists in the editor | Phase 3 plan (next doc) — already the heart of Phase 3's spec scope |
| All-actions Done group at the bottom + collapsible | Phase 3 plan |
| Drag-to-prioritise → Jon: manual order matters more than P1/2/3, plus a "focus on 1–a-few things" cue | **Spec amendment first** (ordering model + overlap with Phase 4's focus scope), then planned |
| Subtasks like Things | Phase 4 (Jon's answer), arrives with `parent_id` pulled forward from Phase 5 |
| Stale blocked reason under Done rows | Already fixed on `main` (#39, `RowStyle.showsBlockedReason`) — no task |
| Done tick vanishes instantly — hold the done styling, then animate out (Jon's follow-up note after #43 merged) | **This plan, Task 6** (added by amendment in the Task 5 PR) |

## Global Constraints

- Only `apple/Yardstick/`, `apple/YardstickTests/` and `docs/design/reference/v2-today-view.md` may change. No Rust, no `apple/generated/` (never committed), no new dependencies.
- Every PR: conventional-commit title, template filled, TDD evidence pasted, `just test` (unchanged, 107) and `just app-test` green.
- Where a step has no automatable test (pure layout/hit-testing), the step names its manual arbiter and the PR must say the manual check was or was not run — the Phase 2 precedent (Tasks 6–9), never a silent claim.
- Pixel-fidelity arbiter: `docs/design/reference/v2-today-view.md` §7.2 **as amended by Task 5 Step 1** of this plan.

## Task overview

| # | Branch | PR title | Contents |
|---|---|---|---|
| 1–3 | `fix/gate-interactions` | `fix(apple): fill bucket routes, real hit targets, edit-in-place` | One PR, three tasks — same files, one reviewer gate |
| 4 | `fix/gate-note-caret` | `fix(apple): note caret matches the text line height` | Own PR |
| 5 | `fix/gate-row-restyle` | `feat(apple): Things-style task rows and checkboxes` | Own PR (includes the reference amendment) |
| 6 | `fix/gate-done-grace` | `feat(apple): hold done styling, then animate the row out` | Own PR — added by amendment; approving the Task 5 PR approves this task |

---

### Task 1: Bucket routes fill the window

**Files:**
- Modify: `apple/Yardstick/ContentView.swift` (the `default:` route branch, ~lines 53–66)

**Interfaces:**
- Consumes: `TaskListView` (unchanged), `Theme.Metrics.contentPadding*`.
- Produces: nothing new — layout only.

**Riders:** none.

**Cause (verified in code):** a vertical `ScrollView` adopts its *content's* width, and the `default:` branch's content is `TaskListView` capped at `Theme.Metrics.contentMaxWidth` (760) with no expanding frame — so the whole main column collapses to 760+padding, the window's root centres the narrower `HStack`, the sidebar drifts right and a white gap opens on the trailing edge. `InboxView` already has the fix (`.frame(maxWidth: .infinity, alignment: .topLeading)` on the padded scroll content, `ContentView.swift`-adjacent pattern at `InboxView.swift:30`); the `default:` branch never got it.

- [x] **Step 1: Apply the InboxView pattern to the default branch**

In `ContentView.swift`, replace the `default:` branch's `ScrollView` content:

```swift
default:
    // now / next / later / waiting: the same list surface.
    ScrollView {
        TaskListView(list: core.view.list,
                     onToggleDone: { core.toggleDone(id: $0) },
                     onOpenTriage: { core.triageTarget = $0 },
                     onSetStatus: { core.setStatus(id: $0, status: $1) })
            .padding(EdgeInsets(top: Theme.Metrics.contentPaddingTop,
                                 leading: Theme.Metrics.contentPaddingH,
                                 bottom: 40,
                                 trailing: Theme.Metrics.contentPaddingH))
            .frame(maxWidth: .infinity, alignment: .topLeading)
    }
    .background(Color.white)
```

(The only change is the added `.frame(maxWidth: .infinity, alignment: .topLeading)` line.)

- [x] **Step 2: Verify**

Run: `just app-test`
Expected: PASS (no behavioural tests cover layout; this confirms the build).

Manual arbiter (paste result in the PR): launch via `cd apple && just run`, click Now, Next, Later, Waiting on — the content column reaches the window's trailing edge, the sidebar does not move, no gap. Compare against Inbox, which was already correct.

- [x] **Step 3: Commit**

```bash
git add apple/Yardstick/ContentView.swift
git commit -m "fix(apple): bucket routes fill the main column like Inbox"
```

---

### Task 2: Real hit targets, and Escape leaves selection

**Files:**
- Modify: `apple/Yardstick/TaskRow.swift` (checkbox button + row shape)
- Modify: `apple/Yardstick/AllActionsView.swift` (Escape handling)

**Interfaces:**
- Consumes: `TaskRow` as shipped; `AllActionsView.selection: Set<String>`.
- Produces: nothing new — hit-testing and one key handler.

**Riders:** none.

**Cause (verified in code):** the checkbox is `Circle().strokeBorder(...)` inside a `.plain` button — the stroked ring is the only non-transparent content, so only the ~1.5px border is hittable (`TaskRow.swift:61,137`). The row `HStack` likewise has no `contentShape`, so `List` selection clicks over transparent padding fall through — Jon's "have to click on the border" in both places. And nothing maps Escape to clearing `selection`.

- [x] **Step 1: Make the whole checkbox circle hittable, with breathing room**

In `TaskRow.swift`, change the button:

```swift
Button(action: onToggleDone) {
    checkbox
        .contentShape(Circle().inset(by: -4))
}
.buttonStyle(.plain)
.accessibilityLabel(row.isDone ? "Mark not done" : "Mark done")
```

- [x] **Step 2: Make the whole row hittable**

Still in `TaskRow.swift`, add one modifier to the row `HStack`, directly after `.padding(.horizontal, Theme.Metrics.taskRowHPadding)`:

```swift
.contentShape(Rectangle())
```

This makes hover, context-menu and `List` selection respond across the full row, including the empty meta spacer.

- [x] **Step 3: Escape clears the All-actions selection**

In `AllActionsView.swift`, on the `List` (alongside the existing `.onKeyPress`):

```swift
.onExitCommand {
    selection.removeAll()
}
```

- [x] **Step 4: Verify**

Run: `just app-test`
Expected: PASS (hit-testing has no XCTest surface; this confirms the build and that the 36 existing tests still pass).

Manual arbiter (paste result in the PR): (a) clicking anywhere inside a checkbox circle toggles done, in a bucket view and in All actions; (b) clicking a row body in All actions selects it without ⌘-hunting for padding; (c) with rows selected, Escape deselects and the "n selected" chrome disappears.

- [x] **Step 5: Commit**

```bash
git add apple/Yardstick/TaskRow.swift apple/Yardstick/AllActionsView.swift
git commit -m "fix(apple): full-shape hit targets; Escape clears selection"
```

---

### Task 3: Edit-in-place keeps the row's shape and focuses immediately

**Files:**
- Create: `apple/Yardstick/TitleEdit.swift`
- Modify: `apple/Yardstick/AllActionsView.swift` (`rowView(_:)`)
- Test: `apple/YardstickTests/TitleEditTests.swift`

**Interfaces:**
- Consumes: `AllActionsView.editingID/draftTitle` state, `onEditTitle` callback (unchanged signature `(String, String) -> Void`).
- Produces: `enum TitleEdit { static func commit(draft: String, original: String) -> String? }` — returns the trimmed title to save, or `nil` when the edit should be dropped (empty or unchanged). Task 3 is its only consumer today.

**Riders:** none.

**Cause (verified in code):** `rowView(_:)` swaps the entire `TaskRow` for a bare `TextField` (`AllActionsView.swift:97–122`) — checkbox, badges and meta all vanish and the row height changes; and the field is never given focus, hence Jon's second click.

- [x] **Step 1: Write the failing test for the commit rule**

`apple/YardstickTests/TitleEditTests.swift`:

```swift
import XCTest
@testable import Yardstick

final class TitleEditTests: XCTestCase {
    func testTrimsAndReturnsAChangedTitle() {
        XCTAssertEqual(TitleEdit.commit(draft: "  New title ", original: "Old"), "New title")
    }

    func testDropsAnEmptyOrWhitespaceDraft() {
        XCTAssertNil(TitleEdit.commit(draft: "   ", original: "Old"))
        XCTAssertNil(TitleEdit.commit(draft: "", original: "Old"))
    }

    func testDropsAnUnchangedTitleIncludingWhitespaceOnlyChanges() {
        XCTAssertNil(TitleEdit.commit(draft: "Old", original: "Old"))
        XCTAssertNil(TitleEdit.commit(draft: "  Old  ", original: "Old"))
    }
}
```

- [x] **Step 2: Run it, observe the failure**

Run: `just app-test`
Expected: FAIL — `cannot find 'TitleEdit' in scope`.

- [x] **Step 3: Implement `TitleEdit`**

`apple/Yardstick/TitleEdit.swift`:

```swift
/// The rule for committing an inline title edit, kept out of the view so it
/// is testable without a host app: trim, drop empties, drop no-ops.
enum TitleEdit {
    static func commit(draft: String, original: String) -> String? {
        let trimmed = draft.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty, trimmed != original else { return nil }
        return trimmed
    }
}
```

- [x] **Step 4: Run it, observe the pass**

Run: `just app-test`
Expected: PASS (3 new tests).

- [x] **Step 5: Rebuild `rowView` to edit the title in place**

In `AllActionsView.swift`, add a focus state to the view:

```swift
@FocusState private var titleFieldFocused: Bool
```

and replace `rowView(_:)`:

```swift
@ViewBuilder
private func rowView(_ row: TaskRowVm) -> some View {
    if editingID == row.id {
        // The same anatomy as TaskRow — checkbox, field, meta — so entering
        // edit mode changes nothing but the title becoming a field.
        HStack(spacing: Theme.Metrics.taskRowGap) {
            TaskRow(row: row,
                    onToggleDone: { onToggleDone(row.id) },
                    onOpenTriage: { onOpenTriage(row.id) },
                    onSetStatus: { onSetStatus(row.id, $0) })
                .hidden()
                .overlay(alignment: .leading) {
                    TextField("", text: $draftTitle)
                        .textFieldStyle(.plain)
                        .font(Theme.Typography.body)
                        .focused($titleFieldFocused)
                        .padding(.leading, Theme.Metrics.taskRowHPadding
                                 + Theme.Metrics.checkboxSize
                                 + Theme.Metrics.taskRowGap)
                        .onSubmit {
                            if let title = TitleEdit.commit(draft: draftTitle, original: row.title) {
                                onEditTitle(row.id, title)
                            }
                            editingID = nil
                        }
                        .onExitCommand { editingID = nil }
                        .onAppear { titleFieldFocused = true }
                }
        }
    } else {
        TaskRow(row: row,
                onToggleDone: { onToggleDone(row.id) },
                onOpenTriage: { onOpenTriage(row.id) },
                onSetStatus: { onSetStatus(row.id, $0) })
            // simultaneousGesture, not onTapGesture: a plain double-tap
            // recognizer swallows single clicks over the row content, so
            // ⌘-click selection only worked on the padding around it.
            .simultaneousGesture(TapGesture(count: 2).onEnded {
                draftTitle = row.title
                editingID = row.id
            })
    }
}
```

**Check against reality, not this document:** the `.hidden()`+`overlay` trick keeps the exact row height and leaves the checkbox visible only if `hidden()` is applied to the *whole* row. If the hidden row's checkbox must stay visible and interactive during an edit (nicer), the alternative is passing an `isEditing` flag into `TaskRow` and swapping only its title `Text` for the field — prefer whichever compiles cleanly and keeps the checkbox visible; the arbiter is the manual check in Step 6, and if the `TaskRow`-flag route is taken, record it as a deviation in the PR.

- [x] **Step 6: Verify**

Run: `just app-test`
Expected: PASS.

Manual arbiter (paste result in the PR): double-click a title in All actions → the row keeps its height and checkbox, the field is focused immediately (type without clicking), Return saves, Escape reverts, a whitespace-only edit saves nothing.

- [x] **Step 7: Commit + PR (ends the `fix/gate-interactions` branch)**

```bash
git add apple/Yardstick apple/YardstickTests
git commit -m "fix(apple): edit titles in place with immediate focus"
git push -u origin fix/gate-interactions
gh pr create --fill   # spec-deltas: none
```

STOP for review.

**Deviations recorded while implementing (plan amended in the `fix/gate-interactions` PR):**

1. **Task 3 Step 5 took the step's own sanctioned alternative.** The `.hidden()`+`overlay` snippet hides the checkbox with the rest of the row and needs a hand-computed leading offset that would drift if row metrics change (Task 5 changes them). Implemented instead: `TaskRow` gains an optional `titleEditing: TitleEditing?` (draft binding + submit/cancel callbacks, default `nil` so all previews and call sites compile unchanged), and renders the title as a focused `.plain` `TextField` in place of the `Text` — same anatomy, checkbox visible and live, no offset arithmetic. `AllActionsView.rowView` passes it with `TitleEdit.commit` in the submit closure.
2. **One compile fix against this SDK:** `onExitCommand` requires its argument label — `.onExitCommand(perform: editing.onCancel)`, not `.onExitCommand(editing.onCancel)`. First `just app-test` failed on exactly that line; labelled, the suite is 39/39.

---

### Task 4: The note caret matches the text's line height

**Files:**
- Create: `apple/Yardstick/NoteTypography.swift`
- Modify: `apple/Yardstick/NoteEditor.swift` (the `noteAttributes` block)
- Test: `apple/YardstickTests/NoteTypographyTests.swift`

**Interfaces:**
- Consumes: `NoteEditor.noteAttributes` (private static, `NoteEditor.swift:26–34`).
- Produces: `enum NoteTypography { static let fontSize: CGFloat; static let lineHeightMultiple: CGFloat; static func lineSpacing(for font: NSFont) -> CGFloat }`.

**Riders:** none.

**Cause (verified in code):** `noteAttributes` uses `paragraph.lineHeightMultiple = 1.65` — that inflates the line *fragment* to ~23px, and AppKit draws the insertion point the full fragment height, so the caret towers over 14px text. Expressing the same 1.65 rhythm as `lineSpacing` (leading *between* fragments) keeps each fragment — and the caret — at the font's natural height.

- [ ] **Step 1: Write the failing test**

`apple/YardstickTests/NoteTypographyTests.swift`:

```swift
import AppKit
import XCTest
@testable import Yardstick

final class NoteTypographyTests: XCTestCase {
    /// Reference §5: 14px text at 1.65 line height. The total vertical
    /// rhythm (font's own line height + our spacing) must land on
    /// 14 × 1.65 = 23.1, without inflating the fragment the caret spans.
    func testLineSpacingPreservesTheReferenceRhythm() {
        let font = NSFont.systemFont(ofSize: NoteTypography.fontSize)
        let base = NSLayoutManager().defaultLineHeight(for: font)
        let total = base + NoteTypography.lineSpacing(for: font)
        XCTAssertEqual(total, 14 * 1.65, accuracy: 0.01)
    }

    func testLineSpacingIsNeverNegativeEvenForATallFont() {
        // A font whose natural line height already exceeds the target must
        // clamp to zero, not pull lines together.
        let tall = NSFont.systemFont(ofSize: 40)
        XCTAssertGreaterThanOrEqual(NoteTypography.lineSpacing(for: tall), 0)
    }
}
```

- [ ] **Step 2: Run it, observe the failure**

Run: `just app-test`
Expected: FAIL — `cannot find 'NoteTypography' in scope`.

- [ ] **Step 3: Implement `NoteTypography`**

`apple/Yardstick/NoteTypography.swift`:

```swift
import AppKit

/// Reference §5's type rhythm (14px / 1.65) expressed as inter-line spacing
/// rather than lineHeightMultiple: the multiple inflates every line fragment
/// and AppKit draws the insertion point the full fragment height, so the
/// caret towered over the text. Spacing keeps the fragment (and caret) at
/// the font's natural height and puts the extra leading between lines.
enum NoteTypography {
    static let fontSize: CGFloat = 14
    static let lineHeightMultiple: CGFloat = 1.65

    static func lineSpacing(for font: NSFont) -> CGFloat {
        let base = NSLayoutManager().defaultLineHeight(for: font)
        return max(0, fontSize * lineHeightMultiple - base)
    }
}
```

- [ ] **Step 4: Run it, observe the pass**

Run: `just app-test`
Expected: PASS (2 new tests).

- [ ] **Step 5: Use it in `NoteEditor`**

Replace the `noteAttributes` initializer in `NoteEditor.swift`:

```swift
private static let noteAttributes: [NSAttributedString.Key: Any] = {
    let font = NSFont.systemFont(ofSize: NoteTypography.fontSize)
    let paragraph = NSMutableParagraphStyle()
    paragraph.lineSpacing = NoteTypography.lineSpacing(for: font)
    return [
        .font: font,
        .foregroundColor: NSColor(Theme.textBody),
        .paragraphStyle: paragraph,
    ]
}()
```

(The doc comment above it should be updated to say spacing, not multiple — it currently cites `lineHeightMultiple`.)

- [ ] **Step 6: Verify**

Run: `just test && just app-test`
Expected: both green.

Manual arbiter (paste result in the PR): in the Today note, the caret is the height of the text, and multi-line notes keep visibly the same line rhythm as before (compare a screenshot against `main`). **Check against reality:** if the caret is still tall (TextKit 2 caret behaviour differs across macOS releases), do not invent an override — record it in the PR, leave `lineSpacing` in place (it is independently correct), and file the caret as its own follow-up with what was observed.

- [ ] **Step 7: Commit + PR**

```bash
git add apple/Yardstick apple/YardstickTests
git commit -m "fix(apple): note caret matches the text line height"
git push -u origin fix/gate-note-caret
gh pr create --fill   # spec-deltas: none (reference §5 rhythm preserved)
```

STOP for review.

---

### Task 5: Things-style rows and checkboxes

**Files:**
- Modify: `docs/design/reference/v2-today-view.md` (§7.2 amendment — Step 1)
- Modify: `apple/YardstickTests/ThemeTests.swift` (`testMetricsMatchTheReference`)
- Modify: `apple/Yardstick/Theme.swift` (`Metrics`)
- Modify: `apple/Yardstick/TaskRow.swift` (checkbox shape)

**Interfaces:**
- Consumes: `Theme.Metrics.{checkboxSize, taskRowVPadding, taskRowGap}`, `RowStyle.checkbox(_:)` (mapping unchanged).
- Produces: `Theme.Metrics.checkboxRadius: CGFloat` (new token).

**Riders:** Jon's gate ask, scoped by his own answer: **rows + checkboxes only** — sidebar, headers and the rest of the chrome keep the current reference.

**This task changes acceptance criteria**, so the reference is amended *first* and the pixel test drives the code. Values proposed from Jon's Things 3 screenshot; he adjudicates them on this plan PR before any code is written.

- [x] **Step 1: Amend the design reference**

Append to `docs/design/reference/v2-today-view.md`, at the end of §7.2:

```markdown
**Amendment 2026-08-03 (Phase 2 gate feedback — Things-style rows, rows + checkboxes only):**
- Checkbox is a **16×16 rounded square, corner radius 4.5** (was a 17px circle). Empty:
  `border: 1.5px solid #c4c3c0`, transparent fill. In-progress: accent-blue border, 25%
  accent centre (shape change only). Done: green fill + white check (unchanged colours).
- The full checkbox shape plus a 4px halo is the click target, and the full row rect
  is the hover/selection target — hit targets are part of the spec, not an implementation detail.
- Row padding tightens to `7px 6px` (was `9px 6px`); flex gap `10px` (was `11px`).
- Everything else in §7.2 (title 14px, priority badge, pills, chips, 70px meta column,
  done-row opacity 0.55) is unchanged. Sidebar and headers are explicitly out of scope.
```

- [x] **Step 2: Drive the metric change through the failing pixel test**

`testMetricsMatchTheReference` (`ThemeTests.swift:45`) currently asserts only `sidebarWidth`/`contentMaxWidth`/`noteMaxWidth` — the row metrics were never pinned. Add the amended values to it:

```swift
// §7.2 as amended 2026-08-03 (Things-style rows).
XCTAssertEqual(Theme.Metrics.taskRowVPadding, 7)
XCTAssertEqual(Theme.Metrics.taskRowGap, 10)
XCTAssertEqual(Theme.Metrics.checkboxSize, 16)
XCTAssertEqual(Theme.Metrics.checkboxRadius, 4.5)
```

- [x] **Step 3: Run it, observe the failure**

Run: `just app-test`
Expected: FAIL — the three changed values mismatch and `checkboxRadius` does not exist.

- [x] **Step 4: Implement in `Theme.Metrics`**

In `apple/Yardstick/Theme.swift`:

```swift
static let taskRowVPadding: CGFloat = 7
static let taskRowGap: CGFloat = 10
static let checkboxSize: CGFloat = 16
static let checkboxRadius: CGFloat = 4.5
```

- [x] **Step 5: Swap the checkbox shape**

In `TaskRow.swift`, replace the `checkbox` builder's shapes — `Circle()` becomes `RoundedRectangle(cornerRadius: Theme.Metrics.checkboxRadius)` in all three cases (and the Task 2 `contentShape` on the button becomes `RoundedRectangle(cornerRadius: Theme.Metrics.checkboxRadius).inset(by: -4)`):

```swift
@ViewBuilder
private var checkbox: some View {
    let size = Theme.Metrics.checkboxSize
    let shape = RoundedRectangle(cornerRadius: Theme.Metrics.checkboxRadius)
    switch RowStyle.checkbox(row.checkbox) {
    case .ring:
        shape.strokeBorder(Theme.checkboxRing, lineWidth: 1.5)
            .frame(width: size, height: size)
    case .ringWithSoftCentre:
        shape.strokeBorder(Theme.accent, lineWidth: 1.5)
            .frame(width: size, height: size)
            .overlay(shape.fill(Theme.accent).opacity(0.25).padding(3))
    case .filledCheck:
        shape.fill(Theme.statusDone)
            .frame(width: size, height: size)
            .overlay(
                Image(systemName: "checkmark")
                    .font(.system(size: 9, weight: .semibold))
                    .foregroundStyle(.white))
    }
}
```

- [x] **Step 6: Run everything, observe the pass**

Run: `just test && just app-test`
Expected: both green, including the updated `ThemeTests`.

Manual arbiter (paste result in the PR): side-by-side with Jon's Things 3 screenshot — square-ish checkboxes, tighter rows; all four §7.2 row states still render correctly in the `TaskRow` previews.

- [x] **Step 7: Commit + PR**

```bash
git add docs/design/reference apple/Yardstick apple/YardstickTests
git commit -m "feat(apple): Things-style task rows and checkboxes"
git push -u origin fix/gate-row-restyle
gh pr create --fill   # spec-deltas: none (design reference amended in this PR)
```

STOP for review.

---

### Task 6: Hold the done styling, then animate the row out

*Added by plan amendment in the Task 5 PR, from Jon's follow-up note after #43 merged: "when I check an item as done the item disappears instantly… nice to see it marked as done using the done styling and then animate out after a delay." The behaviour is specced in the same §7.2 reference amendment Task 5 lands (1.2s grace, ~250ms ease-out fade, second click cancels).*

**Files:**
- Create: `apple/Yardstick/TickGrace.swift`
- Modify: `apple/Yardstick/TaskRow.swift` (toggle handler + pending styling), `apple/Yardstick/TaskListView.swift` (flag + leave animation), `apple/Yardstick/InboxView.swift`, `apple/Yardstick/ContentView.swift` (default branch), `apple/Yardstick/DayColumn.swift`, `apple/Yardstick/AllActionsView.swift` (pass the flag)
- Test: `apple/YardstickTests/TickGraceTests.swift`

**Interfaces:**
- Consumes: `TaskRow`, `TaskListView` as shipped after Tasks 2–5.
- Produces: `enum TickGrace { static let holdSeconds: TimeInterval; enum Decision { case toggleNow, beginGrace, cancelGrace }; static func decide(isDone: Bool, graceActive: Bool, listRetainsDoneRows: Bool) -> Decision }`; `TaskRow` and `TaskListView` gain `var retainsDoneRows: Bool = true`.

**Riders:** none.

**Accepted trade-off (record in the PR):** the tick's `ToggleDone` event is dispatched *after* the 1.2s grace, so quitting inside that window loses that one tick. The alternative — dispatch immediately and cache the vanished row shell-side for redisplay — needs a row cache with expiry merged into every list build, which is disproportionate for a polish item. Revisit only if the loss is ever actually observed.

- [ ] **Step 1: Write the failing decision-table test**

`apple/YardstickTests/TickGraceTests.swift`:

```swift
import XCTest

@testable import Yardstick

final class TickGraceTests: XCTestCase {
    func testFirstTickInAVanishingListBeginsTheGrace() {
        XCTAssertEqual(
            TickGrace.decide(isDone: false, graceActive: false, listRetainsDoneRows: false),
            .beginGrace)
    }

    func testSecondTickDuringTheGraceCancelsIt() {
        XCTAssertEqual(
            TickGrace.decide(isDone: false, graceActive: true, listRetainsDoneRows: false),
            .cancelGrace)
    }

    func testRetainingListsToggleImmediatelyWithNoGrace() {
        // Now and All actions keep done rows visible: they restyle in place.
        XCTAssertEqual(
            TickGrace.decide(isDone: false, graceActive: false, listRetainsDoneRows: true),
            .toggleNow)
    }

    func testUntickingIsAlwaysImmediate() {
        XCTAssertEqual(
            TickGrace.decide(isDone: true, graceActive: false, listRetainsDoneRows: false),
            .toggleNow)
        XCTAssertEqual(
            TickGrace.decide(isDone: true, graceActive: false, listRetainsDoneRows: true),
            .toggleNow)
    }
}
```

- [ ] **Step 2: Run it, observe the failure**

Run: `just app-test`
Expected: FAIL — `cannot find 'TickGrace' in scope`.

- [ ] **Step 3: Implement `TickGrace`**

`apple/Yardstick/TickGrace.swift`:

```swift
import Foundation

/// §7.2 amendment (2026-08-03): ticking a row that would leave its list
/// holds the done styling for a grace window, then the row animates out.
/// Pure decision table — the view owns the timer, this owns the choices.
enum TickGrace {
    static let holdSeconds: TimeInterval = 1.2

    enum Decision: Equatable {
        case toggleNow    // dispatch ToggleDone immediately
        case beginGrace   // show done styling now, dispatch after the hold
        case cancelGrace  // second click during the hold: revert, dispatch nothing
    }

    static func decide(isDone: Bool, graceActive: Bool, listRetainsDoneRows: Bool) -> Decision {
        if graceActive { return .cancelGrace }
        if isDone || listRetainsDoneRows { return .toggleNow }
        return .beginGrace
    }
}
```

- [ ] **Step 4: Run it, observe the pass**

Run: `just app-test`
Expected: PASS (4 new tests).

- [ ] **Step 5: Wire the grace into `TaskRow`**

In `TaskRow.swift`, add below `titleEditing`:

```swift
/// Whether this row's list keeps done rows visible (Now, All actions) —
/// they restyle in place; vanishing lists get the §7.2 grace instead.
var retainsDoneRows: Bool = true
```

add state:

```swift
@State private var pendingDone = false
@State private var graceTask: Task<Void, Never>?
```

change the button action from `onToggleDone` to `handleToggle`, and add:

```swift
private func handleToggle() {
    switch TickGrace.decide(isDone: row.isDone,
                            graceActive: pendingDone,
                            listRetainsDoneRows: retainsDoneRows) {
    case .toggleNow:
        onToggleDone()
    case .beginGrace:
        pendingDone = true
        graceTask = Task {
            try? await Task.sleep(for: .seconds(TickGrace.holdSeconds))
            guard !Task.isCancelled else { return }
            pendingDone = false
            onToggleDone()
        }
    case .cancelGrace:
        graceTask?.cancel()
        graceTask = nil
        pendingDone = false
    }
}
```

and render the pending state as done: the checkbox switch keys on
`RowStyle.checkbox(pendingDone ? "done" : row.checkbox)`, and every
`row.isDone` used for styling in `body` (title colour, strikethrough, meta
colour, row opacity — not the accessibility label's action) becomes
`(row.isDone || pendingDone)`.

- [ ] **Step 6: Thread the flag and animate the leave**

`TaskListView` gains `var retainsDoneRows: Bool = true`, passes it to every `TaskRow`, and animates row departures on its rows `VStack`:

```swift
.animation(.easeOut(duration: 0.25),
           value: list.groups.flatMap(\.rows).map(\.id))
```

with `.transition(.opacity)` on each `TaskRow` in its `ForEach`. Call sites: `InboxView` and `ContentView`'s `default:` branch pass `retainsDoneRows: false`; `DayColumn` (the Now section) and `AllActionsView` pass nothing (default `true`).

- [ ] **Step 7: Run everything**

Run: `just test && just app-test`
Expected: both green.

Manual arbiter (paste result in the PR): in the Inbox, ticking a task shows the green check + strikethrough for ~1.2s, then the row fades out; clicking again during the hold reverts it with nothing saved; in Now and All actions the tick restyles in place immediately, as before.

- [ ] **Step 8: Commit + PR**

```bash
git add apple/Yardstick apple/YardstickTests
git commit -m "feat(apple): hold done styling, then animate the row out"
git push -u origin fix/gate-done-grace
gh pr create --fill   # spec-deltas: none (reference §7.2 amendment landed with Task 5)
```

STOP for review.

---

## Self-review notes

- **Coverage walk:** all six in-scope feedback items map to Tasks 1–5; the five out-of-scope items are each routed by name in the table at the top, none silently dropped.
- **Type consistency:** `TitleEdit.commit(draft:original:)` is defined in Task 3 and used only there; `Theme.Metrics.checkboxRadius` is defined in Task 5 Step 4 before its uses in Step 5; Task 5's `contentShape` line supersedes Task 2's circle version and says so.
- **Ordering:** Tasks 2, 3 and 5 all touch `TaskRow.swift` — 1–3 ship as one PR, 5 lands after it merges; the plan is serial where files overlap, so no wave table.
- **Placeholder scan:** the two "check against reality" notes (Task 3 Step 5's hidden/overlay vs. flag choice; Task 4 Step 6's caret-on-other-macOS caveat) name their arbiter and the deviation procedure — the Phase 1/2 pattern, not a placeholder.
