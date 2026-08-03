import XCTest

final class StatusJourneyTests: UITestCase {
    /// Pick a status from a row's context menu.
    ///
    /// The menu rows read "<status> — <hint>" (StatusMenuItems pairs each label
    /// with its one-line description, and the current one carries a checkmark),
    /// so `app.menuItems["Blocked"]` matches nothing — the query has to be a
    /// prefix match. It also has to look at `title`: a MenuItem carries its
    /// text there, and matching only on `label` found nothing on CI.
    private func setStatus(_ status: String, on title: String) {
        let row = rowElement(title)
        XCTAssertTrue(
            row.exists,
            """
            row "\(title)" to right-click not found. \(rowQueryReport(title))
            AX tree:
            \(app.debugDescription)
            """)
        row.rightClick()
        let submenu = app.menuItems["Set status"]
        XCTAssertTrue(
            submenu.waitForExistence(timeout: 3),
            "Set status submenu. AX tree:\n\(app.debugDescription)")
        submenu.hover()
        let item = app.menuItems.matching(
            NSPredicate(format: "title BEGINSWITH %@ OR label BEGINSWITH %@",
                        status, status)).firstMatch
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
    ///
    /// Runs in All actions, NOT the Inbox as the plan sketched: the Inbox is a
    /// vanishing list (`retainsDoneRows: false`, §7.2), so a ticked row leaves
    /// and there is nothing left to untick. Untick-in-place only exists in a
    /// list that keeps done rows, so that is where this checklist item lives.
    func testUntickRestoresBlockedWithReason() {
        capture("Chase supplier")
        openSidebarView("all")
        setStatus("Blocked", on: "Chase supplier")
        giveReason("waiting on legal")
        assertRowVisible("waiting on legal")

        app.buttons["Mark done"].firstMatch.click()
        XCTAssertFalse(app.staticTexts["waiting on legal"].exists) // §7.2 row 4 (PR #37 fix 1)
        app.buttons["Mark not done"].firstMatch.click()
        assertRowVisible("waiting on legal")
    }
}
