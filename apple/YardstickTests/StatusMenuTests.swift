import XCTest
@testable import Yardstick

/// Journey 5A ships six statuses with verbatim one-line hints, in a fixed
/// order, each with its dot colour. These are product copy: a test is the
/// only thing that stops them drifting.
final class StatusMenuTests: XCTestCase {

    func testAllSixStatusesInTheDesignedOrder() {
        XCTAssertEqual(
            StatusOption.all.map(\.label),
            ["Backlog", "In progress", "Blocked", "Waiting", "Done", "Binned"])
    }

    func testHintsAreVerbatimFromTheReference() {
        XCTAssertEqual(
            StatusOption.all.map(\.hint),
            [
                "Someday / unstarted",
                "Actively on it",
                "Can't proceed",
                "On someone else",
                "Complete",
                "Dropped",
            ])
    }

    func testDotColoursComeFromTheStatusTokens() {
        let byLabel = Dictionary(uniqueKeysWithValues: StatusOption.all.map { ($0.label, $0.colour) })
        XCTAssertEqual(byLabel["Backlog"], Theme.statusBacklog)
        XCTAssertEqual(byLabel["In progress"], Theme.statusInProgress)
        XCTAssertEqual(byLabel["Blocked"], Theme.statusBlocked)
        XCTAssertEqual(byLabel["Waiting"], Theme.statusWaiting)
        XCTAssertEqual(byLabel["Done"], Theme.statusDone)
        XCTAssertEqual(byLabel["Binned"], Theme.statusBinned)
    }

    func testStatusKeysMatchTheCoresStringsSoTheCheckmarkLandsOnTheRightRow() {
        XCTAssertEqual(
            StatusOption.all.map(\.key),
            ["backlog", "in_progress", "blocked", "waiting", "done", "binned"])
    }

    func testOnlyBlockedNeedsAReasonPrompt() {
        XCTAssertTrue(StatusOption.needsReason(.blocked))
        XCTAssertFalse(StatusOption.needsReason(.waiting))
        XCTAssertFalse(StatusOption.needsReason(.done))
        XCTAssertFalse(StatusOption.needsReason(.backlog))
    }
}
