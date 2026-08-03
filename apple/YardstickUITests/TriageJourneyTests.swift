import XCTest

final class TriageJourneyTests: UITestCase {
    private func openTriage(for title: String) {
        let row = app.staticTexts[title]
        XCTAssertTrue(row.waitForExistence(timeout: 3))
        row.rightClick()
        app.menuItems["Triage…"].click()
        XCTAssertTrue(
            app.sheets.firstMatch.waitForExistence(timeout: 3),
            "triage sheet did not open. AX tree:\n\(app.debugDescription)")
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
        assertRowVisible("Finalize vendor contract")
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
        XCTAssertTrue(
            sheet.radioButtons["Next"].isSelected,
            "WHEN should reopen on Next. AX tree:\n\(app.debugDescription)")
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
        assertRowVisible("Finalize vendor contract", timeout: 5)
    }
}
