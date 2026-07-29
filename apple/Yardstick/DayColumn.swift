import App
import SwiftUI

/// Reference §5 — eyebrow 11/700/0.06em uppercase `#a0a09e`, title
/// 25/700/-0.02em, body 14/1.65 `#3a3a3c`, note text max-width 640.
///
/// Recorded deviation: the header stays fixed and the editor fills/scrolls
/// the rest (the reference scrolls header+body together; with only the note
/// in the Phase 1 column this is visually identical, and the layout gets
/// revisited when the briefing/task sections join the column in Phase 2).
struct DayColumn: View {
    let day: DayVm
    /// R1: false until the day's content has loaded (Core.dayIsEditable).
    let editable: Bool
    let onEdit: (String) -> Void

    /// Ghost visibility follows keystrokes immediately: the vm's noteText
    /// only catches up after the debounced echo round-trip (~500 ms), so
    /// this local state tracks the draft and is resynced whenever the
    /// document actually changes underneath us (editorVersion).
    @State private var draftIsEmpty = true

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("DAILY NOTE")
                .font(Theme.Typography.capsLabel)
                .tracking(0.66) // 0.06em of 11px
                .foregroundStyle(Theme.textMuted)
                .padding(.bottom, 3)
            Text(day.title)
                .font(Theme.Typography.dateTitle)
                .kerning(-0.5) // -0.02em of 25px
                .foregroundStyle(Theme.textPrimary)
                .padding(.bottom, 12)
            ZStack(alignment: .topLeading) {
                NoteEditor(text: day.noteText,
                           version: day.editorVersion,
                           isEditable: editable,
                           onEdit: { text in
                               draftIsEmpty = text.isEmpty
                               onEdit(text)
                           })
                if draftIsEmpty {
                    Text("Type to keep writing…")
                        .font(Theme.Typography.ghost)
                        .foregroundStyle(Theme.textQuiet)
                        .allowsHitTesting(false)
                }
            }
            .frame(maxWidth: Theme.Metrics.noteMaxWidth, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .padding(EdgeInsets(
            top: Theme.Metrics.contentPaddingTop,
            leading: Theme.Metrics.contentPaddingH,
            bottom: 40,
            trailing: Theme.Metrics.contentPaddingH))
        .background(Color.white)
        .onAppear { draftIsEmpty = day.noteText.isEmpty }
        .onChange(of: day.editorVersion) { draftIsEmpty = day.noteText.isEmpty }
    }
}

#Preview("With note") {
    DayColumn(
        day: DayVm(
            date: "2026-07-02",
            title: "Thursday, July 2",
            noteText: "Release Meeting\nFeature flags — check with @Ollie\nCopy changes?",
            editorVersion: 1),
        editable: true,
        onEdit: { _ in })
}

#Preview("Empty") {
    DayColumn(
        day: DayVm(
            date: "2026-07-02",
            title: "Thursday, July 2",
            noteText: "",
            editorVersion: 1),
        editable: true,
        onEdit: { _ in })
}
