import XCTest

final class AllActionsJourneyTests: UITestCase {
    /// Capture `titles`, then open All actions grouped by NOTHING.
    ///
    /// The grouping switch is not cosmetic here: All actions groups by status
    /// by default and a freshly captured task is Backlog — a collapsed group,
    /// rendered only as a "Backlog · N" footer, so its row is absent from the
    /// AX tree entirely (found in Task 3). Grouping "None" draws every row.
    private func seed(_ titles: [String]) {
        for title in titles { capture(title) }
        openSidebarView("all")
        groupBy("None")
        for title in titles { assertRowVisible(title) }
    }

    private func groupBy(_ option: String) {
        let segment = app.radioButtons[option]
        XCTAssertTrue(
            segment.waitForExistence(timeout: 3),
            "grouping segment \(option). AX tree:\n\(app.debugDescription)")
        segment.click()
    }

    /// The selectable element for a row: the enclosing Cell, not the title
    /// StaticText. A click on the title text does not reach the `List`'s row —
    /// nothing selected and no "N selected" appeared — which matches PR #39's
    /// note that selection only ever worked on the padding around the content.
    private func selectableRow(_ title: String) -> XCUIElement {
        app.cells.containing(.staticText, identifier: title).firstMatch
    }

    /// ⌘-click adds to the selection; the count appears in the controls row.
    private func select(_ first: String, _ others: String...) {
        selectableRow(first).click()
        for title in others {
            let row = selectableRow(title)
            XCUIElement.perform(withKeyModifiers: .command) { row.click() }
        }
        let count = others.count + 1
        XCTAssertTrue(
            app.staticTexts["\(count) selected"].waitForExistence(timeout: 3),
            "\(count) selected. AX tree:\n\(app.debugDescription)")
    }

    /// Checklist E4 part 1 + PR #39's selection fix: ⌘-clicking rows BY THEIR
    /// CONTENT selects them, and the bulk keyboard path moves all of them in
    /// one go while leaving everything else alone.
    func testCommandClickSelectsAndLMovesSelectionToLater() {
        seed(["Alpha", "Bravo", "Charlie"])
        select("Alpha", "Bravo")

        app.typeKey("l", modifierFlags: [])

        openSidebarView("later")
        assertRowVisible("Alpha")
        assertRowVisible("Bravo")
        XCTAssertFalse(app.staticTexts["Charlie"].exists, "Charlie must be untouched")
    }

    /// Checklist E6: a selection plus Set status → Waiting changes them all.
    func testBulkSetStatusWaiting() {
        seed(["Alpha", "Bravo"])
        select("Alpha", "Bravo")

        let menu = app.menuButtons.firstMatch // the toolbar StatusMenuItems control
        XCTAssertTrue(
            menu.waitForExistence(timeout: 3),
            "bulk status menu. AX tree:\n\(app.debugDescription)")
        menu.click()
        // Menu rows read "<status> — <hint>" and carry their text in `title`
        // (Task 3), so this must be a prefix match on title.
        let waiting = app.menuItems.matching(
            NSPredicate(format: "title BEGINSWITH 'Waiting' OR label BEGINSWITH 'Waiting'"))
            .firstMatch
        XCTAssertTrue(
            waiting.waitForExistence(timeout: 3),
            "Waiting menu item. AX tree:\n\(app.debugDescription)")
        waiting.click()

        // Both rows now live under Waiting: group by status again and the
        // Waiting group holds them, which is the honest end-to-end check.
        groupBy("Status")
        assertRowVisible("Alpha")
        assertRowVisible("Bravo")
    }

    /// Checklist E2–E3: grouping switches, and filters compose and clear.
    func testGroupingAndFilters() {
        seed(["Alpha", "Bravo"])

        groupBy("Bucket")
        XCTAssertTrue(
            app.staticTexts["Inbox"].waitForExistence(timeout: 3),
            "Bucket grouping should show an Inbox group. AX tree:\n\(app.debugDescription)")
        groupBy("None")

        // Filter to Now: both tasks are in Inbox, so the list empties.
        let bucketFilter = app.popUpButtons.element(boundBy: 0)
        XCTAssertTrue(
            bucketFilter.waitForExistence(timeout: 3),
            "bucket filter. AX tree:\n\(app.debugDescription)")
        bucketFilter.click()
        app.menuItems["Now"].click()
        XCTAssertFalse(app.staticTexts["Alpha"].exists, "Inbox tasks are not in Now")

        // `Clear filters` is a `.buttonStyle(.link)` Button, which macOS
        // exposes as a Link — `app.buttons["Clear filters"]` found nothing.
        let clear = app.links["Clear filters"].exists
            ? app.links["Clear filters"]
            : app.buttons["Clear filters"]
        XCTAssertTrue(
            clear.waitForExistence(timeout: 3),
            "Clear filters control. AX tree:\n\(app.debugDescription)")
        clear.click()
        assertRowVisible("Alpha")
    }

    /// Checklist E5 + PR #39's gesture fix: double-click edits, Return
    /// commits, Escape cancels.
    ///
    /// Typing REPLACES the title: the editor opens with the existing text
    /// selected, so ` renamed` produced the title `renamed`, not
    /// `Alpha renamed`. Each edit therefore types the whole new title.
    func testDoubleClickEditsTitleReturnCommitsEscapeCancels() {
        seed(["Alpha"])
        editTitle(of: "Alpha", to: "Alpha renamed\n")
        assertRowVisible("Alpha renamed")

        rowElement("Alpha renamed").doubleClick()
        let again = app.textFields.firstMatch
        XCTAssertTrue(again.waitForExistence(timeout: 3), "editor reopens")
        again.typeText("junk")
        app.typeKey(.escape, modifierFlags: [])
        assertRowVisible("Alpha renamed")
        XCTAssertFalse(app.staticTexts["junk"].exists, "Escape discards the draft")
    }

    /// Double-click the title (the opaque content the edit gesture is attached
    /// to — a Cell click would land on the row's transparent middle) and type
    /// the replacement.
    private func editTitle(of title: String, to replacement: String) {
        rowElement(title).doubleClick()
        let editor = app.textFields.firstMatch
        XCTAssertTrue(
            editor.waitForExistence(timeout: 3),
            "inline title editor. AX tree:\n\(app.debugDescription)")
        editor.typeText(replacement)
    }

    /// Checklist E "Finish": an inline edit survives a relaunch.
    func testAllActionsEditsSurviveRelaunch() {
        seed(["Alpha"])
        editTitle(of: "Alpha", to: "Alpha renamed\n")
        assertRowVisible("Alpha renamed")

        relaunch()
        openSidebarView("all")
        groupBy("None")
        assertRowVisible("Alpha renamed", timeout: 5)
    }
}
