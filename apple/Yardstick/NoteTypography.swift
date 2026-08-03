import AppKit

/// Reference §5's type rhythm (14px / 1.65) expressed as inter-line spacing
/// rather than lineHeightMultiple: the multiple inflates every line fragment
/// and AppKit draws the insertion point the full fragment height, so the
/// caret towered over the text. Spacing keeps the fragment (and caret) at
/// the font's natural height and puts the extra leading between lines.
enum NoteTypography {
    static let fontSize: CGFloat = 14
    static let lineHeightMultiple: CGFloat = 1.65

    static func lineSpacing(for font: NSFont) -> CGFloat {
        let base = NSLayoutManager().defaultLineHeight(for: font)
        return max(0, fontSize * lineHeightMultiple - base)
    }
}
