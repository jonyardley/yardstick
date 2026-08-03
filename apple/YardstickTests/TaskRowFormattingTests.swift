import SwiftUI
import XCTest
@testable import Yardstick

/// The row's pure mapping from core strings to styling. Pixel fidelity is
/// checked by eye against reference §7.2; these tests pin the parts that
/// silently rot: which state maps to which shape, and that an unknown key
/// degrades to the neutral open state rather than crashing or vanishing.
final class TaskRowFormattingTests: XCTestCase {

    func testCheckboxStatesMapToTheReferenceShapes() {
        XCTAssertEqual(RowStyle.checkbox("open"), .ring)
        XCTAssertEqual(RowStyle.checkbox("in_progress"), .ringWithSoftCentre)
        XCTAssertEqual(RowStyle.checkbox("done"), .filledCheck)
    }

    func testUnknownCheckboxKeyFallsBackToOpen() {
        // A core that grows a new state must not produce an invisible row.
        XCTAssertEqual(RowStyle.checkbox("something_new"), .ring)
        XCTAssertEqual(RowStyle.checkbox(""), .ring)
    }

    func testOnlyNotableStatusesGetAPillTint() {
        XCTAssertNil(RowStyle.pill(""), "an ordinary task has no pill")
        XCTAssertNotNil(RowStyle.pill("in_progress"))
        XCTAssertNotNil(RowStyle.pill("blocked"))
        XCTAssertNotNil(RowStyle.pill("waiting"))
        XCTAssertNotNil(RowStyle.pill("binned"))
    }

    func testPillTintsComeFromTheStatusTokensNotAdHocColours() {
        XCTAssertEqual(RowStyle.pill("blocked")?.bg, Theme.statusBlockedBg)
        XCTAssertEqual(RowStyle.pill("blocked")?.fg, Theme.statusBlocked)
        XCTAssertEqual(RowStyle.pill("waiting")?.bg, Theme.statusWaitingBg)
        XCTAssertEqual(RowStyle.pill("in_progress")?.bg, Theme.pillTint)
    }

    func testPriorityBadgeColoursFollowTheTokenScale() {
        XCTAssertEqual(RowStyle.priorityColour(1), Theme.priority1)
        XCTAssertEqual(RowStyle.priorityColour(2), Theme.priority2)
        XCTAssertEqual(RowStyle.priorityColour(3), Theme.priority3)
        XCTAssertNil(RowStyle.priorityColour(0), "priority is optional — no badge")
        XCTAssertNil(RowStyle.priorityColour(9), "out of range renders nothing")
    }

    func testDoneRowsNeverShowABlockedReasonEvenIfOneSurvivesInData() {
        // Task 10a made blocked_reason survive a Done round trip (it used to
        // be destroyed). That is correct for data, but reference §7.2 row 4
        // (the done state) shows no such text under the strikethrough — the
        // row must gate display on isDone, not on the reason being empty.
        XCTAssertFalse(RowStyle.showsBlockedReason(isDone: true, blockedReason: "Legal review"))
    }

    func testOpenRowsShowTheBlockedReasonWhenPresent() {
        XCTAssertTrue(RowStyle.showsBlockedReason(isDone: false, blockedReason: "Legal review"))
    }

    func testNoBlockedReasonNeverShowsRegardlessOfDoneState() {
        XCTAssertFalse(RowStyle.showsBlockedReason(isDone: false, blockedReason: ""))
        XCTAssertFalse(RowStyle.showsBlockedReason(isDone: true, blockedReason: ""))
    }
}
