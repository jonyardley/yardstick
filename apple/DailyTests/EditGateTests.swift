import XCTest
@testable import Daily

/// R1 — the editability gate. Typing into a not-yet-loaded day would save
/// ONLY the typed text over unseen DB content (EditDay replaces the whole
/// day core-side), so the editor stays non-editable until the current
/// date's content has actually arrived — i.e. until editor_version has
/// moved past the baseline recorded when the date-changing event was sent.
///
/// Version arithmetic mirrors shared/src/app.rs:
/// - Startup issues GetDay with NO bump; the first DayLoaded bumps (+1).
/// - select_date (NavigateToDay / GoToToday) bumps once immediately (the
///   clear-the-editor render), then DayLoaded bumps again (+2 total).
final class EditGateTests: XCTestCase {

    func testFreshGateClosedUntilFirstDayLoaded() {
        var gate = EditGate()
        gate.closeForStartup(currentVersion: 0)
        XCTAssertFalse(gate.isOpen(atVersion: 0), "before any DayLoaded: closed")
        XCTAssertTrue(gate.isOpen(atVersion: 1), "first DayLoaded bump opens it")
    }

    func testNavigationClosedThroughClearRenderOpenOnLoad() {
        var gate = EditGate()
        // Day loaded long ago; version has reached 5 and the gate is open.
        gate.closeForStartup(currentVersion: 0)
        XCTAssertTrue(gate.isOpen(atVersion: 5))
        // User navigates: select_date will bump to 6 (clear), DayLoaded to 7.
        gate.closeForNavigation(currentVersion: 5)
        XCTAssertFalse(gate.isOpen(atVersion: 5), "pre-navigation echo: closed")
        XCTAssertFalse(gate.isOpen(atVersion: 6), "clear render is NOT loaded content")
        XCTAssertTrue(gate.isOpen(atVersion: 7), "DayLoaded opens it")
    }

    func testStackedNavigationsBeforeClearRenderArrives() {
        var gate = EditGate()
        // Loaded at 5. Two navigations land before the first clear render is
        // observed (renders may coalesce/reorder — Core.swift's render
        // comment), so both close calls see version 5.
        gate.closeForNavigation(currentVersion: 5) // core will go 5 -> 6 -> (7)
        gate.closeForNavigation(currentVersion: 5) // core will go 6 -> 7 -> (8)
        XCTAssertFalse(gate.isOpen(atVersion: 6), "first clear render: closed")
        XCTAssertFalse(gate.isOpen(atVersion: 7), "second clear render: closed")
        XCTAssertTrue(gate.isOpen(atVersion: 8), "second day's DayLoaded opens it")
    }

    func testWakeStartupReclosesUntilFreshLoad() {
        var gate = EditGate()
        gate.closeForStartup(currentVersion: 0)
        // Day loaded; version 3; gate open. Wake across midnight re-sends
        // Startup{new date}: no immediate bump, fresh DayLoaded will bump to 4.
        XCTAssertTrue(gate.isOpen(atVersion: 3))
        gate.closeForStartup(currentVersion: 3)
        XCTAssertFalse(gate.isOpen(atVersion: 3), "stale text visible but not editable (R2)")
        XCTAssertTrue(gate.isOpen(atVersion: 4), "fresh DayLoaded re-opens")
    }

    func testBaselineIsMonotone() {
        var gate = EditGate()
        gate.closeForNavigation(currentVersion: 5) // baseline 6
        // A startup close observing an older version must not lower the bar.
        gate.closeForStartup(currentVersion: 4)
        XCTAssertFalse(gate.isOpen(atVersion: 6))
        XCTAssertTrue(gate.isOpen(atVersion: 7))
    }
}
