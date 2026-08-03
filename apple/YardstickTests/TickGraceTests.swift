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
