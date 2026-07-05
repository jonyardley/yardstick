import App
import SwiftUI

/// Reference §5 — eyebrow 11/700/0.06em uppercase `#a0a09e`, title
/// 25/700/-0.02em, body 14/1.65 `#3a3a3c`, note text max-width 640;
/// read-only `Text` this task, T8 swaps in the editor.
struct DayColumn: View {
    let day: DayVm

    var body: some View {
        ScrollView {
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
                if day.noteText.isEmpty {
                    Text("Type to keep writing…")
                        .font(Theme.Typography.body)
                        .foregroundStyle(Theme.textQuiet)
                } else {
                    Text(day.noteText)
                        .font(Theme.Typography.body)
                        .lineSpacing(14 * 0.65)
                        .foregroundStyle(Theme.textBody)
                }
            }
            .frame(maxWidth: Theme.Metrics.noteMaxWidth, alignment: .leading)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(EdgeInsets(
                top: Theme.Metrics.contentPaddingTop,
                leading: Theme.Metrics.contentPaddingH,
                bottom: 40,
                trailing: Theme.Metrics.contentPaddingH))
        }
        .background(Color.white)
    }
}

#Preview("With note") {
    DayColumn(day: DayVm(
        date: "2026-07-02",
        title: "Thursday, July 2",
        noteText: "Release Meeting\nFeature flags — check with @Ollie\nCopy changes?",
        editorVersion: 0))
}

#Preview("Empty") {
    DayColumn(day: DayVm(
        date: "2026-07-02",
        title: "Thursday, July 2",
        noteText: "",
        editorVersion: 0))
}
