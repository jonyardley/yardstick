import XCTest
@testable import Yardstick

/// A bulk edit must be exactly one event carrying every selected id, and it
/// must leave unmentioned fields alone — the core reads `nil` as "don't
/// touch", so a wrong `nil` here silently rewrites 30 tasks.
final class BulkEditTests: XCTestCase {

    func testBucketIntentSetsOnlyTheBucket() {
        let payload = BulkEdit.payload(for: .bucket(.later), ids: ["a", "b"])
        XCTAssertEqual(payload.ids, ["a", "b"])
        XCTAssertEqual(payload.bucket, .later)
        XCTAssertNil(payload.priority)
        XCTAssertNil(payload.status)
    }

    func testPriorityIntentSetsOnlyThePriority() {
        let payload = BulkEdit.payload(for: .priority(1), ids: ["a"])
        XCTAssertEqual(payload.priority, 1)
        XCTAssertNil(payload.bucket)
        XCTAssertNil(payload.status)
    }

    func testAnEmptySelectionProducesNoIds() {
        // The caller must be able to check this and skip the send entirely.
        XCTAssertTrue(BulkEdit.payload(for: .priority(2), ids: []).ids.isEmpty)
    }

    func testSelectionOrderIsPreservedForPredictableUndoTalk() {
        let payload = BulkEdit.payload(for: .bucket(.now), ids: ["c", "a", "b"])
        XCTAssertEqual(payload.ids, ["c", "a", "b"])
    }
}
