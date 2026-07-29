import XCTest
@testable import Yardstick

/// Journey 1B's keyboard contract: N/E/L for when, 1/2/3 for priority.
/// `E` for Next (not `N`, which Now already owns) is the reference's own
/// choice and the thing most likely to be "corrected" by mistake later.
final class TriageKeyboardTests: XCTestCase {

    func testWhenKeys() {
        XCTAssertEqual(TriageKey.intent(for: "n"), .bucket(.now))
        XCTAssertEqual(TriageKey.intent(for: "e"), .bucket(.next))
        XCTAssertEqual(TriageKey.intent(for: "l"), .bucket(.later))
    }

    func testWhenKeysAreCaseInsensitive() {
        XCTAssertEqual(TriageKey.intent(for: "N"), .bucket(.now))
        XCTAssertEqual(TriageKey.intent(for: "E"), .bucket(.next))
        XCTAssertEqual(TriageKey.intent(for: "L"), .bucket(.later))
    }

    func testPriorityKeys() {
        XCTAssertEqual(TriageKey.intent(for: "1"), .priority(1))
        XCTAssertEqual(TriageKey.intent(for: "2"), .priority(2))
        XCTAssertEqual(TriageKey.intent(for: "3"), .priority(3))
    }

    func testUnboundKeysDoNothing() {
        // `#` opens the project/person linker in the reference, which needs
        // pages (Phase 3). Binding it now would be a key that lies.
        XCTAssertNil(TriageKey.intent(for: "#"))
        XCTAssertNil(TriageKey.intent(for: "0"))
        XCTAssertNil(TriageKey.intent(for: "4"))
        XCTAssertNil(TriageKey.intent(for: "f"), "F is focus — Phase 4")
        XCTAssertNil(TriageKey.intent(for: " "))
    }

    func testApplyingAnIntentLeavesEverythingElseAlone() {
        var draft = TriageDraft(bucket: .inbox, priority: 0, due: "2026-07-31")
        draft.apply(.bucket(.later))
        XCTAssertEqual(draft.bucket, .later)
        XCTAssertEqual(draft.priority, 0, "a when key must not clear priority")
        XCTAssertEqual(draft.due, "2026-07-31", "nor the due date")

        draft.apply(.priority(2))
        XCTAssertEqual(draft.priority, 2)
        XCTAssertEqual(draft.bucket, .later)
    }

    func testPressingTheSamePriorityAgainClearsIt() {
        // Priority is optional (handoff §Task), so the toggle needs a way
        // back to "none" without reaching for the mouse.
        var draft = TriageDraft(bucket: .now, priority: 2, due: "")
        draft.apply(.priority(2))
        XCTAssertEqual(draft.priority, 0)
    }
}
