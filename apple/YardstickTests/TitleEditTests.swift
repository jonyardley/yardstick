import XCTest

@testable import Yardstick

final class TitleEditTests: XCTestCase {
    func testTrimsAndReturnsAChangedTitle() {
        XCTAssertEqual(TitleEdit.commit(draft: "  New title ", original: "Old"), "New title")
    }

    func testDropsAnEmptyOrWhitespaceDraft() {
        XCTAssertNil(TitleEdit.commit(draft: "   ", original: "Old"))
        XCTAssertNil(TitleEdit.commit(draft: "", original: "Old"))
    }

    func testDropsAnUnchangedTitleIncludingWhitespaceOnlyChanges() {
        XCTAssertNil(TitleEdit.commit(draft: "Old", original: "Old"))
        XCTAssertNil(TitleEdit.commit(draft: "  Old  ", original: "Old"))
    }
}
