import AppKit
import XCTest

@testable import Yardstick

final class NoteTypographyTests: XCTestCase {
    /// Reference §5: 14px text at 1.65 line height. The total vertical
    /// rhythm (font's own line height + our spacing) must land on
    /// 14 × 1.65 = 23.1, without inflating the fragment the caret spans.
    func testLineSpacingPreservesTheReferenceRhythm() {
        let font = NSFont.systemFont(ofSize: NoteTypography.fontSize)
        let base = NSLayoutManager().defaultLineHeight(for: font)
        let total = base + NoteTypography.lineSpacing(for: font)
        XCTAssertEqual(total, 14 * 1.65, accuracy: 0.01)
    }

    func testLineSpacingIsNeverNegativeEvenForATallFont() {
        // A font whose natural line height already exceeds the target must
        // clamp to zero, not pull lines together.
        let tall = NSFont.systemFont(ofSize: 40)
        XCTAssertGreaterThanOrEqual(NoteTypography.lineSpacing(for: tall), 0)
    }
}
