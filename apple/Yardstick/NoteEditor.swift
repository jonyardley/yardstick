import AppKit
import SwiftUI

/// ONE plain-text NSTextView (TextKit 2) for the daily note — spec §6.
/// Phase 1 scope: plain paragraphs only. Phase 3 adds mention chips and
/// pickers to THIS view; do not replace it with TextEditor.
///
/// The version contract (the editor-side twin of Core.swift's
/// render-refetch-idempotency comment): `updateNSView` pushes `text` into
/// the view ONLY when `version` differs from the last version it applied.
/// Renders caused by the user's own typing echo (version unchanged) never
/// touch the view — no caret jumps, no fighting the typist. Day switches
/// and loads bump the version core-side and thus replace the text.
struct NoteEditor: NSViewRepresentable {
    let text: String
    let version: UInt64
    /// R1 gate: false until the day's content has loaded (Core.dayIsEditable).
    /// The view stays selectable and visually normal, just not editable.
    let isEditable: Bool
    let onEdit: (String) -> Void

    /// Reference §5: 14px / 1.65 line height / #3a3a3c. Applied as
    /// typingAttributes AND re-asserted over replaced content: a plain-text
    /// NSTextView keeps typing attributes per-view, but wholesale `.string`
    /// replacement does not reliably restyle the new characters.
    private static let noteAttributes: [NSAttributedString.Key: Any] = {
        let paragraph = NSMutableParagraphStyle()
        paragraph.lineHeightMultiple = 1.65
        return [
            .font: NSFont.systemFont(ofSize: 14),
            .foregroundColor: NSColor(Theme.textBody),
            .paragraphStyle: paragraph,
        ]
    }()

    func makeCoordinator() -> Coordinator { Coordinator(onEdit: onEdit) }

    func makeNSView(context: Context) -> NSScrollView {
        let textView = NSTextView(usingTextLayoutManager: true) // TextKit 2 opt-in
        textView.delegate = context.coordinator
        textView.isRichText = false
        textView.allowsUndo = true
        textView.isSelectable = true
        textView.isEditable = isEditable
        textView.drawsBackground = false

        // Manual NSScrollView assembly (init(usingTextLayoutManager:) gives a
        // bare, zero-frame view): wrap-to-width, grow-down-forever.
        textView.isVerticallyResizable = true
        textView.isHorizontallyResizable = false
        textView.autoresizingMask = [.width]
        textView.minSize = .zero
        textView.maxSize = NSSize(
            width: CGFloat.greatestFiniteMagnitude,
            height: CGFloat.greatestFiniteMagnitude)
        textView.textContainer?.widthTracksTextView = true
        // Default lineFragmentPadding is 5pt; zero it so the note text
        // left-aligns with the column header and the ghost overlay.
        textView.textContainer?.lineFragmentPadding = 0
        textView.textContainerInset = .zero

        textView.defaultParagraphStyle =
            Self.noteAttributes[.paragraphStyle] as? NSParagraphStyle
        textView.typingAttributes = Self.noteAttributes
        textView.font = .systemFont(ofSize: 14)
        textView.textColor = NSColor(Theme.textBody)

        let scroll = NSScrollView()
        scroll.documentView = textView
        scroll.hasVerticalScroller = true
        scroll.hasHorizontalScroller = false
        scroll.autohidesScrollers = true
        scroll.drawsBackground = false
        context.coordinator.textView = textView
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        context.coordinator.onEdit = onEdit
        guard let textView = scroll.documentView as? NSTextView else { return }
        textView.isEditable = isEditable
        // The version contract: only push core text into the view when the
        // document changed underneath us (day switch, load). Never on the
        // render echo of the user's own typing — that would fight the caret.
        if context.coordinator.appliedVersion != version {
            context.coordinator.appliedVersion = version
            if textView.string != text {
                textView.string = text
                if let storage = textView.textStorage, storage.length > 0 {
                    storage.setAttributes(
                        Self.noteAttributes,
                        range: NSRange(location: 0, length: storage.length))
                }
                textView.typingAttributes = Self.noteAttributes
            }
            // A version bump means a different document (day switch or
            // load): undo is per day-editing-session, never across it.
            textView.undoManager?.removeAllActions()
        }
    }

    @MainActor
    final class Coordinator: NSObject, NSTextViewDelegate {
        var onEdit: (String) -> Void
        var appliedVersion: UInt64 = 0
        weak var textView: NSTextView?

        init(onEdit: @escaping (String) -> Void) { self.onEdit = onEdit }

        func textDidChange(_ notification: Notification) {
            guard let textView else { return }
            onEdit(textView.string)
        }
    }
}
