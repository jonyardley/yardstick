import App
import SwiftUI

/// Reference §7.1 — four 16×5px pips, done ones green, the rest #dedcd8.
/// The pip count follows the row count (capped so a 40-task day does not
/// draw 40 pips); the label carries the exact numbers.
struct MomentumPips: View {
    let done: UInt64
    let remaining: UInt64

    private var total: Int { min(Int(done + remaining), 4) }
    private var filled: Int {
        guard done + remaining > 0 else { return 0 }
        return Int((Double(done) / Double(done + remaining) * Double(total)).rounded())
    }

    var body: some View {
        HStack(spacing: 3) {
            ForEach(0..<total, id: \.self) { index in
                RoundedRectangle(cornerRadius: Theme.Metrics.pipRadius)
                    .fill(index < filled ? Theme.statusDone : Theme.segmentRemaining)
                    .frame(width: Theme.Metrics.pipWidth, height: Theme.Metrics.pipHeight)
            }
        }
    }
}

/// One task surface: the section header (title + subtitle + momentum cue),
/// then grouped rows. Used by the Today column's Now section, every sidebar
/// bucket view, and the All-actions view.
struct TaskListView: View {
    let list: TaskListVm
    var showsHeader = true
    /// Now keeps done rows in place; Inbox and the bucket views drop them,
    /// which is what triggers the §7.2 done-grace before the row leaves.
    var retainsDoneRows: Bool = true
    let onToggleDone: (String) -> Void
    let onOpenTriage: (String) -> Void
    let onSetStatus: (String, Status) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if showsHeader { header }
            ForEach(Array(list.groups.enumerated()), id: \.offset) { _, group in
                if !group.label.isEmpty {
                    HStack(spacing: 8) {
                        Text(group.label)
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(Theme.textPrimary)
                        Text("\(group.count)")
                            .font(Theme.Typography.count)
                            .foregroundStyle(Theme.textMuted)
                    }
                    .padding(.top, 14)
                    .padding(.bottom, 2)
                }
                if group.rows.isEmpty && !group.label.isEmpty {
                    Text("Nothing here")
                        .font(Theme.Typography.meta)
                        .foregroundStyle(Theme.countEmpty)
                        .padding(.vertical, 6)
                        .padding(.horizontal, Theme.Metrics.taskRowHPadding)
                }
                ForEach(group.rows, id: \.id) { row in
                    TaskRow(row: row,
                            onToggleDone: { onToggleDone(row.id) },
                            onOpenTriage: { onOpenTriage(row.id) },
                            onSetStatus: { onSetStatus(row.id, $0) },
                            retainsDoneRows: retainsDoneRows)
                        .transition(.opacity)
                }
            }
            ForEach(Array(list.collapsed.enumerated()), id: \.offset) { _, group in
                Text("\(group.label) · \(group.count)")
                    .font(Theme.Typography.meta)
                    .foregroundStyle(Theme.textQuiet)
                    .padding(.top, 10)
                    .padding(.horizontal, Theme.Metrics.taskRowHPadding)
            }
        }
        .frame(maxWidth: Theme.Metrics.contentMaxWidth, alignment: .leading)
        // §7.2 amendment: rows leave with a short fade once the grace ends.
        .animation(.easeOut(duration: 0.25),
                   value: list.groups.flatMap(\.rows).map(\.id))
    }

    private var header: some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Text(list.title)
                .font(Theme.Typography.sectionHeader)
                .foregroundStyle(Theme.textPrimary)
            if !list.subtitle.isEmpty {
                Text(list.subtitle)
                    .font(Theme.Typography.sidebarRow)
                    .foregroundStyle(Theme.textTertiary)
            }
            Spacer()
            if let momentum = list.momentum {
                HStack(spacing: 8) {
                    MomentumPips(done: momentum.done, remaining: momentum.remaining)
                    Text(momentum.label)
                        .font(.system(size: 12))
                        .foregroundStyle(Theme.textTertiary)
                }
            }
        }
        .padding(.bottom, 4)
        .overlay(alignment: .bottom) { Theme.hairline08.frame(height: 0.5) }
        .padding(.bottom, 4)
    }
}

#Preview("Now, empty") {
    TaskListView(
        list: TaskListVm(
            title: "Now", subtitle: "Today", groups: [], momentum: MomentumVm(done: 0, remaining: 0, label: "0 done · 0 to go"),
            collapsed: [], groupBy: "status", filterBucket: "", filterStatus: ""),
        onToggleDone: { _ in }, onOpenTriage: { _ in }, onSetStatus: { _, _ in })
    .padding()
}
