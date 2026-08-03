import XCTest

final class StatusJourneyTests: UITestCase {
    /// Pick a status from a row's context menu.
    ///
    /// The menu rows are labelled "<status> — <hint>" (StatusMenuItems pairs
    /// each label with its one-line description, and the current one carries a
    /// checkmark), so `app.menuItems["Blocked"]` matches nothing — the query
    /// has to be a prefix match.
    private func setStatus(_ status: String, on title: String) {
        let row = app.staticTexts[title]
        XCTAssertTrue(row.waitForExistence(timeout: 3))
        row.rightClick()
        let submenu = app.menuItems["Set status"]
        XCTAssertTrue(
            submenu.waitForExistence(timeout: 3),
            "Set status submenu. AX tree:\n\(app.debugDescription)")
        submenu.hover()
        let item = app.menuItems.matching(
            NSPredicate(format: "label BEGINSWITH %@", status)).firstMatch
        XCTAssertTrue(
            item.waitForExistence(timeout: 3),
            "status item \(status). AX tree:\n\(app.debugDescription)")
        item.click()
    }

    /// The reason prompt: type a reason and commit it.
    private func giveReason(_ reason: String) {
        let field = app.textFields["Reason (optional)"]
        XCTAssertTrue(
            field.waitForExistence(timeout: 3),
            "reason prompt. AX tree:\n\(app.debugDescription)")
        field.click()
        field.typeText(reason)
        // "Blocked" is both the prompt's heading and its commit button, so the
        // query must be a button, scoped to the sheet.
        app.sheets.firstMatch.buttons["Blocked"].click()
    }

    /// Checklist B2: choosing Blocked prompts for a reason; the reason then
    /// renders under the row.
    func testBlockedPromptsForReasonAndShowsIt() {
        capture("Chase supplier")
        openSidebarView("inbox")
        setStatus("Blocked", on: "Chase supplier")
        giveReason("waiting on legal")
        assertRowVisible("waiting on legal")
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
        giveReason("waiting on legal")
        assertRowVisible("waiting on legal")

        setStatus("In progress", on: "Chase supplier")
        XCTAssertFalse(app.staticTexts["waiting on legal"].exists)
    }

    /// Checklist B5 — the T10a data-loss fix end to end: tick a Blocked task
    /// done, untick, and it is Blocked again WITH its reason.
    func testUntickRestoresBlockedWithReason() {
        capture("Chase supplier")
        openSidebarView("inbox")
        setStatus("Blocked", on: "Chase supplier")
        giveReason("waiting on legal")
        assertRowVisible("waiting on legal")

        app.buttons["Mark done"].firstMatch.click()
        XCTAssertFalse(app.staticTexts["waiting on legal"].exists) // §7.2 row 4 (PR #37 fix 1)
        app.buttons["Mark not done"].firstMatch.click()
        assertRowVisible("waiting on legal")
    }
}
