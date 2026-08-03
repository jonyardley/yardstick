# XCUITest Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An XCUITest journey suite that automates the Phase 2 manual GUI checklists (capture, triage keyboard, status menu with blocked reason, All-actions selection/bulk-edit/inline-edit, relaunch persistence) so a human pass is only needed for visual polish.

**Architecture:** A `YardstickUITests` bundle (Apple's built-in UI-automation framework — **no new dependency**) launches the real app against a throwaway support directory injected via environment variables, with the embedded MCP server disabled and "today" frozen. A small pure `LaunchConfig` struct parses those variables (unit-tested); `Core.init` consumes it. A new `apple-ui` CI lane runs the journeys on the existing macOS runner.

**Tech Stack:** XCUITest (ships with Xcode), xcodegen, just, GitHub Actions `macos-15`.

## Global Constraints

- No new SPM/Cargo dependency anywhere in this plan. XCUITest is part of Xcode; if any task seems to need a package, stop and escalate — that would require a spec amendment first (CLAUDE.md).
- Pins that never float: `facet = "=0.44"`, `boltffi = "=0.25.2"` (untouched by this plan).
- `apple/generated/` is never committed or hand-edited.
- The environment variables are exactly: `YARDSTICK_SUPPORT_DIR` (absolute path), `YARDSTICK_DISABLE_MCP` (`"1"` to disable), `YARDSTICK_TODAY` (`YYYY-MM-DD`). Absent or invalid values mean production behaviour.
- Accessibility identifiers introduced by this plan: `quickadd.plus` (toolbar + button) and `sidebar.<kind>` where `<kind>` is the core's view-kind string (`now`, `next`, `later`, `waiting`, `inbox`, `all`). Everything else is queried by its visible label.
- **Agents must not run `just app-ui-test` locally without Jon's explicit OK for that run.** UI tests take over the desktop of the machine they run on, and these worktrees run on Jon's real desktop. The default red/green evidence loop for Tasks 2–4 is CI (push the branch, read the `apple-ui` job log). Local runs are Jon's option, not the agent's.
- Frozen test date: `2026-01-14` (a Wednesday), constant `UITestCase.frozenToday`.
- Branch naming: `chore/uitest-t<N>-<slug>` (this is infrastructure between phases; `p<N>/` is reserved for phase plans).

---

### Task 1: LaunchConfig — environment overrides for isolated launches

The app currently always opens `~/Library/Application Support/Yardstick/daily.db` and binds port 52111. A UI test launching the real binary would therefore read/write Jon's real data and collide with any running dev instance. This task adds the override seam, driven by unit tests.

**Files:**
- Create: `apple/Yardstick/LaunchConfig.swift`
- Modify: `apple/Yardstick/Core.swift` (init + `loadOrCreateToken` + `todayString` call site)
- Test: `apple/YardstickTests/LaunchConfigTests.swift`

**Interfaces:**
- Produces: `LaunchConfig` with `supportDir: String?`, `mcpDisabled: Bool`, `today: String?` and `static func from(_ environment: [String: String]) -> LaunchConfig`. `Core.init(environment:)` defaulting to `ProcessInfo.processInfo.environment`. Tasks 2–4 rely on the three environment variables behaving exactly as specified here.

- [x] **Step 1: Write the failing tests**

`apple/YardstickTests/LaunchConfigTests.swift`:

```swift
import XCTest
@testable import Yardstick

final class LaunchConfigTests: XCTestCase {
    func testAbsentVariablesMeanProductionBehaviour() {
        XCTAssertEqual(
            LaunchConfig.from([:]),
            LaunchConfig(supportDir: nil, mcpDisabled: false, today: nil))
    }

    func testSupportDirMustBeAbsolute() {
        XCTAssertEqual(
            LaunchConfig.from(["YARDSTICK_SUPPORT_DIR": "/tmp/yardstick-test"]).supportDir,
            "/tmp/yardstick-test")
        XCTAssertNil(
            LaunchConfig.from(["YARDSTICK_SUPPORT_DIR": "relative/path"]).supportDir)
        XCTAssertNil(LaunchConfig.from(["YARDSTICK_SUPPORT_DIR": ""]).supportDir)
    }

    func testDisableMcpIsExactlyTheStringOne() {
        XCTAssertTrue(LaunchConfig.from(["YARDSTICK_DISABLE_MCP": "1"]).mcpDisabled)
        XCTAssertFalse(LaunchConfig.from(["YARDSTICK_DISABLE_MCP": "true"]).mcpDisabled)
        XCTAssertFalse(LaunchConfig.from(["YARDSTICK_DISABLE_MCP": "0"]).mcpDisabled)
    }

    func testTodayMustParseAsIsoDate() {
        XCTAssertEqual(LaunchConfig.from(["YARDSTICK_TODAY": "2026-01-14"]).today, "2026-01-14")
        XCTAssertNil(LaunchConfig.from(["YARDSTICK_TODAY": "14/01/2026"]).today)
        XCTAssertNil(LaunchConfig.from(["YARDSTICK_TODAY": "not-a-date"]).today)
    }
}
```

Note: the unit-test target compiles the app sources directly (no `@testable import` today in this project — check how `EditGateTests` imports; mirror it. If sources are compiled into the test bundle, drop the `@testable import Yardstick` line).

- [x] **Step 2: Run and watch them fail**

Run: `just app-test`
Expected: build failure — `Cannot find 'LaunchConfig' in scope` (×4).

- [x] **Step 3: Minimal implementation**

`apple/Yardstick/LaunchConfig.swift`:

```swift
import Foundation

/// Environment overrides for launching the app under test. Absent or invalid
/// variables mean production behaviour; parsing is pure so it is unit-tested
/// (LaunchConfigTests) without launching anything.
struct LaunchConfig: Equatable {
    /// YARDSTICK_SUPPORT_DIR — absolute path replacing the real Application
    /// Support directory (database + MCP token live inside it).
    var supportDir: String?
    /// YARDSTICK_DISABLE_MCP=1 — do not start the embedded MCP server, so a
    /// test-launched app never fights a dev instance for port 52111.
    var mcpDisabled: Bool
    /// YARDSTICK_TODAY — fixed 'YYYY-MM-DD' handed to the clock-free core,
    /// so date-derived copy ("entered today", due weekdays) is deterministic.
    var today: String?

    static func from(_ environment: [String: String]) -> LaunchConfig {
        LaunchConfig(
            supportDir: environment["YARDSTICK_SUPPORT_DIR"]
                .flatMap { $0.hasPrefix("/") ? $0 : nil },
            mcpDisabled: environment["YARDSTICK_DISABLE_MCP"] == "1",
            today: environment["YARDSTICK_TODAY"].flatMap { isoDate($0) ? $0 : nil })
    }

    private static func isoDate(_ candidate: String) -> Bool {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.date(from: candidate) != nil
    }
}
```

- [x] **Step 4: Run and watch them pass**

Run: `just app-test`
Expected: PASS, existing suite still green.

- [x] **Step 5: Wire Core.init through the config**

In `apple/Yardstick/Core.swift` — the init signature grows a defaulted parameter (no call-site changes anywhere):

```swift
init(environment: [String: String] = ProcessInfo.processInfo.environment) {
    let config = LaunchConfig.from(environment)
    let supportDir: URL
    if let override = config.supportDir {
        supportDir = URL(fileURLWithPath: override, isDirectory: true)
        try? FileManager.default.createDirectory(
            at: supportDir, withIntermediateDirectories: true)
    } else {
        supportDir = SupportDirectory.url()
    }
    let dbURL = supportDir.appendingPathComponent("daily.db")
```

and further down, replacing the unconditional MCP start and the startup send:

```swift
    if !config.mcpDisabled {
        mcpPort = ffi.startMcp(port: 52111, token: Self.loadOrCreateToken(in: supportDir))
    }
    editGate.closeForStartup(currentVersion: view.day.editorVersion)
    send(.startup(today: config.today ?? Self.todayString()))
```

`loadOrCreateToken` becomes directory-parameterised (it currently calls `SupportDirectory.url()` itself):

```swift
    private static func loadOrCreateToken(in supportDir: URL) -> String {
        let url = supportDir.appendingPathComponent("mcp-token")
        // body unchanged
```

No other `Core` change. `mcpPort` staying `0` when disabled is already the handled "not running" state (the sidebar footer treats 0 as MCP-off, never fatal).

- [x] **Step 6: Full verification**

Run: `just app-test && just test && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --check`
Expected: all green (this task touches no Rust; the Rust runs prove it).

- [x] **Step 7: Commit + PR**

```bash
git add apple/Yardstick/LaunchConfig.swift apple/Yardstick/Core.swift apple/YardstickTests/LaunchConfigTests.swift
git commit -m "feat(apple): environment overrides for isolated app launches"
git push -u origin chore/uitest-t1-launch-config
gh pr create --fill   # spec deltas: none — dev/test seam, production behaviour unchanged when vars absent
```
STOP for review.

---

### Task 2: The YardstickUITests target, smoke journeys, and the CI lane

The target, the base fixture that guarantees isolation, two smoke tests (capture lands in Inbox; relaunch persists), the `just` recipes, and the `apple-ui` CI job — the job lands **here**, not at the end, because Tasks 3–4's red/green evidence loop is CI.

**Files:**
- Create: `apple/YardstickUITests/UITestCase.swift`
- Create: `apple/YardstickUITests/SmokeTests.swift`
- Modify: `apple/project.yml` (new target + scheme test targets)
- Modify: `apple/Justfile` (`test` gains `-only-testing:YardstickTests`; new `ui-test` recipe)
- Modify: `justfile` (new `app-ui-test` forward)
- Modify: `.github/workflows/ci.yml` (new `apple-ui` job)
- Modify: `apple/Yardstick/ContentView.swift` (identifier on the + button)
- Modify: `apple/Yardstick/SidebarView.swift` (identifier on the view rows)

**Interfaces:**
- Consumes: Task 1's three environment variables.
- Produces: `UITestCase` (base class) with `app: XCUIApplication`, `supportDir: URL`, `static let frozenToday = "2026-01-14"`, `func launch()`, `func relaunch()`, `func capture(_ title: String)`, `func openSidebarView(_ kind: String)`. Tasks 3–4 subclass it and call exactly these.

- [x] **Step 1: Declare the target and recipes**

`apple/project.yml` — add under `targets:`:

```yaml
  YardstickUITests:
    type: bundle.ui-testing
    platform: macOS
    deploymentTarget: "15.0"
    sources: [YardstickUITests]
    dependencies:
      - target: Yardstick
    settings:
      base:
        CODE_SIGN_STYLE: Manual
        CODE_SIGN_IDENTITY: "-"
        GENERATE_INFOPLIST_FILE: YES
        TEST_TARGET_NAME: Yardstick
```

and the scheme's test block becomes:

```yaml
    test:
      config: Debug
      targets: [YardstickTests, YardstickUITests]
```

`apple/Justfile` — pin the existing recipe to unit tests only and add the UI lane:

```just
# Run the Swift unit tests (builds the app as the test host).
test: generate
    xcodebuild -project Yardstick.xcodeproj -scheme Yardstick -configuration Debug test -only-testing:YardstickTests

# Run the XCUITest journeys. Launches the real app on the CURRENT DESKTOP —
# don't run casually on a machine someone is using; CI is the default home.
ui-test: generate
    xcodebuild -project Yardstick.xcodeproj -scheme Yardstick -configuration Debug test -only-testing:YardstickUITests -retry-tests-on-failure
```

Root `justfile` — after `app-test`:

```just
# Run the XCUITest journeys (takes over the desktop while running).
app-ui-test:
    cd apple && just ui-test
```

- [x] **Step 2: Write the base fixture**

`apple/YardstickUITests/UITestCase.swift`:

```swift
import XCTest

/// Base for every journey: a fresh throwaway support directory per test
/// (never the real database), no MCP listener (never a port collision with
/// a dev instance), and a frozen "today" so date-derived copy is stable.
class UITestCase: XCTestCase {
    static let frozenToday = "2026-01-14" // a Wednesday

    var app: XCUIApplication!
    private(set) var supportDir: URL!

    override func setUpWithError() throws {
        continueAfterFailure = false
        supportDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("yardstick-uitest-\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: supportDir, withIntermediateDirectories: true)
        launch()
    }

    override func tearDownWithError() throws {
        app.terminate()
        try? FileManager.default.removeItem(at: supportDir)
    }

    /// (Re)launches against the same support directory.
    func launch() {
        app = XCUIApplication()
        app.launchEnvironment["YARDSTICK_SUPPORT_DIR"] = supportDir.path
        app.launchEnvironment["YARDSTICK_DISABLE_MCP"] = "1"
        app.launchEnvironment["YARDSTICK_TODAY"] = Self.frozenToday
        app.launch()
    }

    /// Terminate + launch again, same data — the persistence check.
    func relaunch() {
        app.terminate()
        launch()
    }

    /// Capture a task through the toolbar + popover (Journey 1A).
    func capture(_ title: String) {
        app.buttons["quickadd.plus"].click()
        let field = app.textFields["New task"]
        XCTAssertTrue(field.waitForExistence(timeout: 3), "quick-add field")
        field.click()
        field.typeText(title)
        app.buttons["Add"].click()
    }

    /// Click a sidebar Views row by the core's kind string.
    func openSidebarView(_ kind: String) {
        let row = app.buttons["sidebar.\(kind)"]
        XCTAssertTrue(row.waitForExistence(timeout: 3), "sidebar row \(kind)")
        row.click()
    }
}
```

- [x] **Step 3: Write the smoke journeys**

`apple/YardstickUITests/SmokeTests.swift`:

```swift
import XCTest

final class SmokeTests: UITestCase {
    /// The isolation proof: a fresh support dir means an EMPTY inbox. If
    /// this ever shows pre-existing tasks, the override seam is broken and
    /// the suite would be chewing on real data — that must fail loudly.
    func testFreshLaunchStartsEmptyAndCaptureLandsInInbox() {
        openSidebarView("inbox")
        XCTAssertFalse(app.staticTexts["Finalize vendor contract"].exists)
        capture("Finalize vendor contract")
        XCTAssertTrue(
            app.staticTexts["Finalize vendor contract"].waitForExistence(timeout: 3))
    }

    func testRelaunchKeepsCapturedTasks() {
        capture("Persistent task")
        relaunch()
        openSidebarView("inbox")
        XCTAssertTrue(app.staticTexts["Persistent task"].waitForExistence(timeout: 5))
    }
}
```

- [x] **Step 4: Run on CI, watch it fail for the right reason**

Add the CI job now (next code block), push the branch, and read the `apple-ui` log. Expected first failure: `quickadd.plus` / `sidebar.inbox` not found — the identifiers don't exist yet. That is this task's observed red.

`.github/workflows/ci.yml` — after the `apple` job:

```yaml
  apple-ui:
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
      # XCUITest journeys — launches the real app in the runner's GUI session
      - run: just app-ui-test
```

- [x] **Step 5: Add the two identifiers, watch CI go green**

`apple/Yardstick/ContentView.swift` — on the + button (the `Button { showQuickAdd = true }` label around line 97):

```swift
                .accessibilityIdentifier("quickadd.plus")
```

`apple/Yardstick/SidebarView.swift` — on the Views-row `Button` (inside the `ForEach` over `sidebar.views`, around line 40):

```swift
                        .accessibilityIdentifier("sidebar.\(row.kind)")
```

Push; expected: `apple-ui` green (2 tests), `apple` still green and no slower (it now runs `-only-testing:YardstickTests`).

**Deviation recorded (observed on CI, Risk 1).** The quick-add corner of the
AX tree is nested and label-ambiguous, exactly as the risk anticipated:

```
Window 'main' → Toolbar
  → Button identifier 'quickadd.plus', label 'Add'      (AppKit wrapper)
      → Button identifier 'quickadd.plus', label 'Add'  (our SwiftUI Button)
          → Popover → Group → Button label 'Add'        (QuickAddView)
```

Two consequences for `UITestCase.capture`, both proven by a red CI run:

1. `.accessibilityIdentifier` lands on the wrapper *and* our button, so
   `app.buttons["quickadd.plus"]` raises "Multiple matching elements found".
   Use `app.buttons.matching(identifier: "quickadd.plus").firstMatch` — the
   outer element is the clickable one.
2. The toolbar button's own label is `Add` (it was `identifier: 'plus',
   label: 'Add'` even before this plan), and the popover lives *inside* it, so
   `app.buttons["Add"]` is ambiguous too. Scope through `app.popovers`.

The sidebar rows need neither workaround: `app.buttons["sidebar.<kind>"]`
resolves uniquely. Tasks 3–4 should expect the same shape of churn around
sheets and menus and budget a CI round-trip per query fix.

**Two more findings, both from the same debugging, both changing what later
tasks can assume:**

3. **A real defect, fixed here rather than in a separate `fix/` PR.** A
   sidebar row only responded to clicks on its label: an unselected row's
   background is `.clear` and the gap to its count is a `Spacer`, so a click
   in the middle of the 200pt row fell through and nothing selected. The plan
   says a real defect gets its own `fix/` PR first, but that fix's driving test
   *is* this harness, which is unmerged — so `.contentShape(Rectangle())` on
   `viewRow` and `todayRow` lands in this PR, where the journey's red-then-green
   is the evidence. Jon's call whether to split it.
4. **`app.launch()` does not make the window key, and that is invisible.**
   macOS delivers toolbar clicks to a non-key window but drops content clicks,
   so an unactivated window silently swallowed the first sidebar click and the
   failure surfaced later as a missing row. `launch()` now activates, waits for
   `.runningForeground`, and clicks the inert MCP footer to spend the
   activating click. Keep that in `launch()` — every journey depends on it.

**Local vs CI divergence, recorded (Risk 1, wider than expected):** on
macOS 26 the toolbar `+` exposes ONE element (no AppKit wrapper) and no
synthesized click at any offset opens the popover, while on the `macos-15`
runner the wrapper exists and the popover opens normally. So the suite is
CI-verified only: a local run of these journeys fails at `capture` for
environmental reasons, which is a fact about the OS, not the app. CI stays the
red/green loop for Tasks 3–4 as the Global Constraints already require.

- [x] **Step 6: Full local verification (unit lanes only)**

Run: `just app-test && just test && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --check`
Expected: all green. Do NOT run `just app-ui-test` locally (Global Constraints).

- [x] **Step 7: Commit + PR**

```bash
git add apple/project.yml apple/Justfile justfile .github/workflows/ci.yml apple/YardstickUITests apple/Yardstick/ContentView.swift apple/Yardstick/SidebarView.swift
git commit -m "test(apple): XCUITest target, isolated-launch fixture, smoke journeys, CI lane"
git push -u origin chore/uitest-t2-target-and-smoke
gh pr create --fill   # spec deltas: none — test infra; PR notes Jon flips apple-ui to a required check when merging (SDLC §5)
```
STOP for review. PR body must paste the failing CI log excerpt (Step 4) and the passing one (Step 5).

---

### Task 3: Triage and status-menu journeys

Automates manual checklists A (T7's seven points) and B (T8's six points), including the blocked-reason prompt, Escape-cancel, and untick-restore.

**Files:**
- Create: `apple/YardstickUITests/TriageJourneyTests.swift`
- Create: `apple/YardstickUITests/StatusJourneyTests.swift`

**Interfaces:**
- Consumes: `UITestCase` exactly as produced by Task 2 (`capture`, `openSidebarView`, `relaunch`, `frozenToday`).
- Produces: nothing new for later tasks.

**Query notes for the implementer** (macOS AX trees are the flaky part; adjust queries with `po app.debugDescription` from a CI log or a Jon-approved local run, and record what differed in this plan file):
- Context menu: `row.rightClick()` then `app.menuItems["Triage…"]`; the status submenu is `app.menuItems["Set status"]` whose children are the six labels from `StatusOption.all` (`Backlog`, `In progress`, `Blocked`, `Waiting`, `Done`, `Binned`).
- The triage sheet's WHEN control is a segmented `Picker` — expect `app.radioButtons["Next"]` (AppKit segments usually expose as radio buttons); fall back to `app.buttons["Next"]` scoped to `app.sheets`.
- Keyboard into the sheet: `app.typeKey("e", modifierFlags: [])` — the sheet grabs key focus on appear (`.focused`, PR #39 kept that behaviour).
- The blocked-reason prompt is the `ContentView` sheet with `TextField "Reason (optional)"` and buttons `Cancel` / `Blocked`.

- [x] **Step 1: Write the triage journey**

`apple/YardstickUITests/TriageJourneyTests.swift`:

```swift
import XCTest

final class TriageJourneyTests: UITestCase {
    private func openTriage(for title: String) {
        let row = app.staticTexts[title]
        XCTAssertTrue(row.waitForExistence(timeout: 3))
        row.rightClick()
        app.menuItems["Triage…"].click()
        XCTAssertTrue(app.sheets.firstMatch.waitForExistence(timeout: 3))
    }

    /// Checklist A3–A5: E then 1 moves WHEN to Next and sets P1, same digit
    /// clears, commit moves the task out of Inbox into Next.
    func testKeyboardTriageMovesTaskToNextWithPriority() {
        capture("Finalize vendor contract")
        openSidebarView("inbox")
        openTriage(for: "Finalize vendor contract")

        app.typeKey("e", modifierFlags: [])
        app.typeKey("1", modifierFlags: [])
        app.buttons["Triage"].click()

        openSidebarView("next")
        XCTAssertTrue(
            app.staticTexts["Finalize vendor contract"].waitForExistence(timeout: 3))
        openSidebarView("inbox")
        XCTAssertFalse(app.staticTexts["Finalize vendor contract"].exists)
    }

    /// Checklist A6: the sheet reopens on current values, never defaults.
    func testSheetReopensOnCurrentValues() {
        capture("Finalize vendor contract")
        openSidebarView("inbox")
        openTriage(for: "Finalize vendor contract")
        app.typeKey("e", modifierFlags: [])
        app.typeKey("2", modifierFlags: [])
        app.buttons["Triage"].click()

        openSidebarView("next")
        openTriage(for: "Finalize vendor contract")
        let sheet = app.sheets.firstMatch
        XCTAssertTrue(sheet.radioButtons["Next"].isSelected)
        app.buttons["Cancel"].click()
    }

    /// Checklist A7: triaged state survives a relaunch.
    func testTriageSurvivesRelaunch() {
        capture("Finalize vendor contract")
        openSidebarView("inbox")
        openTriage(for: "Finalize vendor contract")
        app.typeKey("e", modifierFlags: [])
        app.buttons["Triage"].click()

        relaunch()
        openSidebarView("next")
        XCTAssertTrue(
            app.staticTexts["Finalize vendor contract"].waitForExistence(timeout: 5))
    }
}
```

- [x] **Step 2: Push, watch CI — red must be for a missing behaviour or wrong query, never silent**

Expected on first push: plausibly green (the features exist). If any test fails, diagnose from the CI log: a wrong AX query gets fixed in the test; a real defect gets its own `fix/` PR first (the journey then proves the fix). Paste whichever happened into the PR.

- [x] **Step 3: Write the status journey**

`apple/YardstickUITests/StatusJourneyTests.swift`:

```swift
import XCTest

final class StatusJourneyTests: UITestCase {
    private func setStatus(_ status: String, on title: String) {
        let row = app.staticTexts[title]
        XCTAssertTrue(row.waitForExistence(timeout: 3))
        row.rightClick()
        app.menuItems["Set status"].hover()
        app.menuItems[status].click()
    }

    /// Checklist B2: choosing Blocked prompts for a reason; the reason then
    /// renders under the row.
    func testBlockedPromptsForReasonAndShowsIt() {
        capture("Chase supplier")
        openSidebarView("inbox")
        setStatus("Blocked", on: "Chase supplier")

        let field = app.textFields["Reason (optional)"]
        XCTAssertTrue(field.waitForExistence(timeout: 3))
        field.click()
        field.typeText("waiting on legal")
        app.buttons["Blocked"].click()
        XCTAssertTrue(app.staticTexts["waiting on legal"].waitForExistence(timeout: 3))
    }

    /// Checklist B3: Escape in the reason prompt leaves status untouched.
    func testEscapeInReasonPromptLeavesStatusUntouched() {
        capture("Chase supplier")
        openSidebarView("inbox")
        setStatus("Blocked", on: "Chase supplier")
        XCTAssertTrue(app.textFields["Reason (optional)"].waitForExistence(timeout: 3))
        app.typeKey(.escape, modifierFlags: [])
        XCTAssertFalse(app.staticTexts["Blocked"].exists)
    }

    /// Checklist B4: In progress clears a previous blocked reason.
    func testInProgressClearsTheReason() {
        capture("Chase supplier")
        openSidebarView("inbox")
        setStatus("Blocked", on: "Chase supplier")
        let field = app.textFields["Reason (optional)"]
        XCTAssertTrue(field.waitForExistence(timeout: 3))
        field.click()
        field.typeText("waiting on legal")
        app.buttons["Blocked"].click()

        setStatus("In progress", on: "Chase supplier")
        XCTAssertFalse(app.staticTexts["waiting on legal"].exists)
    }

    /// Checklist B5 — the T10a data-loss fix end to end: tick a Blocked task
    /// done, untick, and it is Blocked again WITH its reason.
    func testUntickRestoresBlockedWithReason() {
        capture("Chase supplier")
        openSidebarView("inbox")
        setStatus("Blocked", on: "Chase supplier")
        let field = app.textFields["Reason (optional)"]
        XCTAssertTrue(field.waitForExistence(timeout: 3))
        field.click()
        field.typeText("waiting on legal")
        app.buttons["Blocked"].click()

        app.buttons["Mark done"].firstMatch.click()
        XCTAssertFalse(app.staticTexts["waiting on legal"].exists) // §7.2 row 4 (PR #37 fix 1)
        app.buttons["Mark not done"].firstMatch.click()
        XCTAssertTrue(app.staticTexts["waiting on legal"].waitForExistence(timeout: 3))
    }
}
```

- [x] **Step 4: Push, watch CI go green**

Expected: `apple-ui` green with 7 journeys total. Any red: same triage rule as Step 2.

**Deviations recorded (all three reds were the plan's queries; the app was
right every time):**

1. **Segment selection lives in `value`, not `isSelected`.** A segmented
   `Picker`'s segments report `value: 1` when selected and `isSelected` stays
   false on all of them, so `sheet.radioButtons["Next"].isSelected` failed
   against a correctly-reopened sheet. `TriageJourneyTests.isSelectedSegment`
   reads `value` (accepting Int or String) and the test also asserts `Now` is
   NOT selected so it cannot pass vacuously.
2. **A MenuItem carries its text in `title`, not `label`.** Matching on
   `label BEGINSWITH` found nothing while the tree plainly showed
   `title: 'Blocked — Can't proceed'`. The predicate now checks both. Note the
   labels are `"<status> — <hint>"`, so the plan's `app.menuItems["Blocked"]`
   could never have matched — prefix-match by design, and `hover()` on
   `Set status` does open the submenu.
3. **Checklist B5 needs BOTH surfaces.** The Inbox is a vanishing list
   (`retainsDoneRows: false`, §7.2): tick a row done and it leaves, so
   "untick and it is Blocked again" has nothing to untick — `Mark not done`
   genuinely does not exist there. So the reason is set in the Inbox and the
   tick/untick happens in All actions, which keeps done rows and toggles in
   place with no grace (TickGraceTests' retaining-list case).
4. **All actions hides freshly captured tasks — Task 4 must handle this.**
   It groups by status by default, a captured task is Backlog, and Backlog is
   a COLLAPSED group: it renders only as the `Backlog · 1` footer, so the row
   is absent from the AX tree entirely (`staticTexts=false cells=0 buttons=0`).
   Any journey that captures a task and expects to see it in All actions must
   first give it a visible status or switch grouping — Task 4's `seed()` as the
   plan sketches it would fail for exactly this reason.
5. **Row queries must stay typed.** A `descendants(matching: .any)` fallback
   timed out evaluating the query on CI (that class went from ~100s to 422s of
   retries) and never even reached its tree dump. `rowElement` tries
   `staticTexts` then `cells`; widen by adding a type, never with `.any`.
   All actions renders rows as `Outline → OutlineRow → Cell`, with the title a
   StaticText inside the cell, so `staticTexts[title]` does work once the row
   is actually rendered.

- [x] **Step 5: Full local verification (unit lanes only)**

Run: `just app-test && just test && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --check`
Expected: all green.

- [x] **Step 6: Commit + PR**

```bash
git add apple/YardstickUITests
git commit -m "test(apple): triage and status-menu XCUITest journeys"
git push -u origin chore/uitest-t3-triage-status
gh pr create --fill   # spec deltas: none
```
STOP for review.

---

### Task 4: All-actions journeys

Automates checklist E (T9's eight points): grouping, filters, ⌘-click multi-select, keyboard bulk edit, bulk status, and inline title editing — including the two interaction risks that turned out real in manual testing (PR #39).

**Files:**
- Create: `apple/YardstickUITests/AllActionsJourneyTests.swift`

**Interfaces:**
- Consumes: `UITestCase` from Task 2. Nothing else.

**Query notes:** the grouping control is a segmented `Picker` (`radioButtons` `Status`/`Bucket`/`None`); the two filter `Picker`s render as pop-up buttons (`app.popUpButtons`); ⌘-click is `XCUIElement.perform(withKeyModifiers: .command) { row.click() }`; inline edit is `row.doubleClick()` then typing into the appearing `TextField`.

- [ ] **Step 1: Write the journeys**

`apple/YardstickUITests/AllActionsJourneyTests.swift`:

```swift
import XCTest

final class AllActionsJourneyTests: UITestCase {
    private func seed(_ titles: [String]) {
        for title in titles { capture(title) }
        openSidebarView("all")
    }

    /// Checklist E4 part 1 + PR #39's selection fix: ⌘-clicking rows BY
    /// THEIR CONTENT selects them, and the bulk keyboard path moves all of
    /// them in one go.
    func testCommandClickSelectsAndLMovesSelectionToLater() {
        seed(["Alpha", "Bravo", "Charlie"])
        app.staticTexts["Alpha"].click()
        XCUIElement.perform(withKeyModifiers: .command) {
            app.staticTexts["Bravo"].click()
        }
        XCTAssertTrue(app.staticTexts["2 selected"].waitForExistence(timeout: 3))

        app.typeKey("l", modifierFlags: [])
        openSidebarView("later")
        XCTAssertTrue(app.staticTexts["Alpha"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Bravo"].exists)
        XCTAssertFalse(app.staticTexts["Charlie"].exists) // Charlie untouched
        openSidebarView("all")
        XCTAssertTrue(app.staticTexts["Charlie"].waitForExistence(timeout: 3))
    }

    /// Checklist E6: a selection plus Set status → Waiting changes them all.
    func testBulkSetStatusWaiting() {
        seed(["Alpha", "Bravo"])
        app.staticTexts["Alpha"].click()
        XCUIElement.perform(withKeyModifiers: .command) {
            app.staticTexts["Bravo"].click()
        }
        XCTAssertTrue(app.staticTexts["2 selected"].waitForExistence(timeout: 3))
        app.menuButtons.firstMatch.click() // the toolbar StatusMenuItems control
        app.menuItems["Waiting"].click()
        XCTAssertTrue(app.staticTexts.matching(
            NSPredicate(format: "label CONTAINS 'Waiting'")).firstMatch
            .waitForExistence(timeout: 3))
    }

    /// Checklist E2–E3: grouping switches and filters compose and clear.
    func testGroupingAndFilters() {
        seed(["Alpha", "Bravo"])
        app.radioButtons["Bucket"].click()
        XCTAssertTrue(app.staticTexts["Inbox"].waitForExistence(timeout: 3))
        app.radioButtons["None"].click()

        app.popUpButtons.element(boundBy: 0).click()
        app.menuItems["Now"].click()
        XCTAssertFalse(app.staticTexts["Alpha"].exists) // both live in Inbox
        app.buttons["Clear filters"].click()
        XCTAssertTrue(app.staticTexts["Alpha"].waitForExistence(timeout: 3))
    }

    /// Checklist E5 + PR #39's gesture fix: double-click edits, Return
    /// commits everywhere, Escape cancels.
    func testDoubleClickEditsTitleReturnCommitsEscapeCancels() {
        seed(["Alpha"])
        app.staticTexts["Alpha"].doubleClick()
        let editor = app.textFields.firstMatch
        XCTAssertTrue(editor.waitForExistence(timeout: 3))
        editor.typeText(" renamed\n")
        XCTAssertTrue(app.staticTexts["Alpha renamed"].waitForExistence(timeout: 3))

        app.staticTexts["Alpha renamed"].doubleClick()
        let again = app.textFields.firstMatch
        XCTAssertTrue(again.waitForExistence(timeout: 3))
        again.typeText(" junk")
        app.typeKey(.escape, modifierFlags: [])
        XCTAssertTrue(app.staticTexts["Alpha renamed"].waitForExistence(timeout: 3))
        XCTAssertFalse(app.staticTexts["Alpha renamed junk"].exists)
    }

    /// Checklist E "Finish": everything above survives a relaunch.
    func testAllActionsEditsSurviveRelaunch() {
        seed(["Alpha"])
        app.staticTexts["Alpha"].doubleClick()
        let editor = app.textFields.firstMatch
        XCTAssertTrue(editor.waitForExistence(timeout: 3))
        editor.typeText(" renamed\n")

        relaunch()
        openSidebarView("all")
        XCTAssertTrue(app.staticTexts["Alpha renamed"].waitForExistence(timeout: 5))
    }
}
```

- [ ] **Step 2: Push, watch CI**

Expected: green, or red pointing at either a wrong AX query (fix the test) or a real defect (own `fix/` PR first, then this journey proves it). Paste the log either way.

- [ ] **Step 3: Full local verification (unit lanes only)**

Run: `just app-test && just test && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --check`
Expected: all green.

- [ ] **Step 4: Commit + PR**

```bash
git add apple/YardstickUITests/AllActionsJourneyTests.swift
git commit -m "test(apple): All-actions XCUITest journeys"
git push -u origin chore/uitest-t4-all-actions
gh pr create --fill   # spec deltas: none
```
STOP for review.

---

### Task 5: Make it policy — SDLC gate row and the CLAUDE.md carve-out

Docs-only. The suite exists and runs on CI; this task records it as a required gate and replaces the ad-hoc "no SwiftUI test harness" exception (taken in Tasks 4/7/8/9 of Phase 2 and PRs #37/#39) with a written, narrow rule.

**Files:**
- Modify: `docs/SDLC.md` (§5 table)
- Modify: `CLAUDE.md` (workflow item 3 + Commands)

**Interfaces:** none — text.

- [ ] **Step 1: SDLC §5 — add the gate row**

In the §5 table, after the `apple` row:

```markdown
| `apple-ui` | `just app-ui-test` (XCUITest journeys against an isolated support dir, added by the 2026-08-03 XCUITest plan) | yes |
```

- [ ] **Step 2: CLAUDE.md — the carve-out and the command**

Workflow item 3 currently ends: *"Code without a driving test does not get written."* Append to that item:

```markdown
   For SwiftUI, the driving test for interaction/wiring is an XCUITest journey (`just app-ui-test`, CI-verified — agents never run it locally without Jon's OK for that run); only pure visual polish (colours, spacing, focus effects) may ship on eyeball verification, flagged in the PR.
```

In **Commands**, after `just app-test`:

```markdown
- `just app-ui-test` — XCUITest journeys (takes over the desktop; CI is the default home)
```

CLAUDE.md is at 40 of its 120-line cap, so this fits; per its own budget rule the PR description must still justify the growth: it converts a five-times-taken undocumented exception into two lines of policy.

- [ ] **Step 3: Verify guardrails locally**

Run: `bash scripts/guardrails.sh`
Expected: clean (line cap respected, no TODO markers).

- [ ] **Step 4: Commit + PR**

```bash
git add docs/SDLC.md CLAUDE.md
git commit -m "docs: record the apple-ui gate and the SwiftUI test carve-out"
git push -u origin chore/uitest-t5-policy
gh pr create --fill   # docs/process-only: no TDD section
```
STOP for review. Jon marks `apple-ui` as a required status check in branch protection when merging this PR (SDLC §5: required checks land with the PR that makes them policy — the job itself has been proving itself since Task 2).

---

## Risks recorded at planning time

1. **macOS AX-tree guesses.** Segmented pickers, pop-up buttons and menu hovers are the classic XCUITest-on-macOS churn. The plan's queries are best-effort; Tasks 3–4 explicitly budget for adjusting them from `app.debugDescription` and recording deviations in this file. This is expected, not failure.
2. **CI GUI session.** GitHub's `macos-15` runners execute XCUITest in a logged-in GUI session; if the runner image ever breaks this, the fallback is splitting `apple-ui` to non-required while it stabilises — a one-line branch-protection change, recorded here if taken.
3. **Runtime cost.** Each journey launches the app (~2–4 s launch + test body); the full suite is roughly 12 tests × ~10–20 s ≈ 2–4 min on top of the `apple-ui` job's build. Acceptable; if it grows past ~10 min, split the job before trimming coverage.
4. **What stays manual:** visual polish (the Phase 2 focus-ring and layout-gap class of bug). Snapshot testing would cover it but needs a new dependency → spec amendment; deliberately out of scope here. Revisit only if visual regressions recur.
