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
