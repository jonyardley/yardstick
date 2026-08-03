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
        // Without this the FIRST click of every test is silently eaten
        // activating the window — the app comes up inactive on a CI runner
        // (the AX dump showed the whole window `Disabled`), so e.g. a sidebar
        // click appeared to succeed while the route never changed.
        app.activate()
        XCTAssertTrue(
            app.wait(for: .runningForeground, timeout: 10), "app in foreground")
        XCTAssertTrue(
            app.windows.firstMatch.waitForExistence(timeout: 10), "main window")
    }

    /// Terminate + launch again, same data — the persistence check.
    func relaunch() {
        app.terminate()
        launch()
    }

    /// Capture a task through the toolbar + popover (Journey 1A).
    ///
    /// Both queries here are deliberately narrowed, from the AX tree CI
    /// printed: the toolbar item wraps our SwiftUI Button in an AppKit one and
    /// the identifier lands on BOTH, and the toolbar button's own label is
    /// "Add" — the same as the popover's — with the Popover nested INSIDE it:
    ///
    ///     Toolbar → Button 'quickadd.plus' label 'Add'
    ///                 → Button 'quickadd.plus' label 'Add'
    ///                     → Popover → Group → Button label 'Add'
    ///
    /// So `app.buttons["quickadd.plus"]` and `app.buttons["Add"]` each raise
    /// "Multiple matching elements found". `firstMatch` takes the outer
    /// toolbar button; scoping to `app.popovers` takes the real Add.
    func capture(_ title: String) {
        app.buttons.matching(identifier: "quickadd.plus").firstMatch.click()
        let popover = app.popovers.firstMatch
        XCTAssertTrue(popover.waitForExistence(timeout: 3), "quick-add popover")
        let field = popover.textFields["New task"]
        XCTAssertTrue(field.waitForExistence(timeout: 3), "quick-add field")
        field.click()
        field.typeText(title)
        // Pin the failure: Add is disabled on an empty draft, so if focus or
        // typing went astray this must blame the field, not the row query.
        XCTAssertEqual(field.value as? String, title, "typed quick-add draft")
        popover.buttons["Add"].click()
    }

    /// Wait for a row with `title` to be on screen. On failure the whole AX
    /// tree goes into the message — macOS element types are the churn (Risk 1)
    /// and a bare "XCTAssertTrue failed" costs a CI round-trip to diagnose.
    func assertRowVisible(_ title: String, timeout: TimeInterval = 3) {
        XCTAssertTrue(
            app.staticTexts[title].waitForExistence(timeout: timeout),
            "row \"\(title)\" not found. AX tree:\n\(app.debugDescription)")
    }

    /// Click a sidebar Views row by the core's kind string.
    func openSidebarView(_ kind: String) {
        let row = app.buttons["sidebar.\(kind)"]
        XCTAssertTrue(row.waitForExistence(timeout: 3), "sidebar row \(kind)")
        row.click()
    }
}
