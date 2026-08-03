import App
import SwiftUI

struct BulkPayload: Equatable {
    let ids: [String]
    let bucket: Bucket?
    let priority: UInt8?
    let status: Status?
}

/// One selection + one intent = one event (plan decision #10).
enum BulkEdit {
    static func payload(for intent: TriageIntent, ids: [String]) -> BulkPayload {
        switch intent {
        case .bucket(let bucket):
            return BulkPayload(ids: ids, bucket: bucket, priority: nil, status: nil)
        case .priority(let priority):
            return BulkPayload(ids: ids, bucket: nil, priority: priority, status: nil)
        }
    }
}

/// Every task in the space, in one editable list. Supersedes the handoff's
/// "All tasks · by status" board (spec §6): status grouping is one option
/// here rather than a separate screen.
struct AllActionsView: View {
    let list: TaskListVm
    let onToggleDone: (String) -> Void
    let onOpenTriage: (String) -> Void
    let onSetStatus: (String, Status) -> Void
    let onEditTitle: (String, String) -> Void
    let onBulk: (BulkPayload) -> Void
    let onGroupBy: (String) -> Void
    let onFilter: (String, String) -> Void

    @State private var selection = Set<String>()
    @State private var editingID: String?
    @State private var draftTitle = ""

    /// Selection in the order the list draws it, so a bulk edit reads the
    /// same way it looks (the `Set` has no order of its own).
    private var orderedSelection: [String] {
        list.groups.flatMap(\.rows).map(\.id).filter { selection.contains($0) }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            controls
            List(selection: $selection) {
                ForEach(Array(list.groups.enumerated()), id: \.offset) { _, group in
                    Section {
                        // Criterion 8: an empty group says so rather than
                        // vanishing, so the grouping stays legible.
                        if group.rows.isEmpty {
                            Text("Nothing here")
                                .font(Theme.Typography.meta)
                                .foregroundStyle(Theme.countEmpty)
                        }
                        ForEach(group.rows, id: \.id) { row in
                            rowView(row)
                                .tag(row.id)
                        }
                    } header: {
                        if !group.label.isEmpty {
                            HStack(spacing: 8) {
                                Text(group.label)
                                Text("\(group.count)").foregroundStyle(Theme.textMuted)
                            }
                        }
                    }
                }
                if !list.collapsed.isEmpty {
                    Section {
                        HStack(spacing: 14) {
                            ForEach(Array(list.collapsed.enumerated()), id: \.offset) { _, group in
                                Text("\(group.label) · \(group.count)")
                                    .font(Theme.Typography.meta)
                                    .foregroundStyle(Theme.textQuiet)
                            }
                        }
                    }
                }
            }
            .listStyle(.inset)
            .onExitCommand {
                selection.removeAll()
            }
            .onKeyPress { press in
                guard !selection.isEmpty,
                      let character = press.characters.first,
                      let intent = TriageKey.intent(for: character) else { return .ignored }
                onBulk(BulkEdit.payload(for: intent, ids: orderedSelection))
                return .handled
            }
        }
        .background(Color.white)
    }

    @ViewBuilder
    private func rowView(_ row: TaskRowVm) -> some View {
        if editingID == row.id {
            TaskRow(row: row,
                    onToggleDone: { onToggleDone(row.id) },
                    onOpenTriage: { onOpenTriage(row.id) },
                    onSetStatus: { onSetStatus(row.id, $0) },
                    titleEditing: TaskRow.TitleEditing(
                        draft: $draftTitle,
                        onSubmit: {
                            if let title = TitleEdit.commit(draft: draftTitle, original: row.title) {
                                onEditTitle(row.id, title)
                            }
                            editingID = nil
                        },
                        onCancel: { editingID = nil }))
        } else {
            TaskRow(row: row,
                    onToggleDone: { onToggleDone(row.id) },
                    onOpenTriage: { onOpenTriage(row.id) },
                    onSetStatus: { onSetStatus(row.id, $0) })
                // simultaneousGesture, not onTapGesture: a plain double-tap
                // recognizer swallows single clicks over the row content, so
                // ⌘-click selection only worked on the padding around it.
                .simultaneousGesture(TapGesture(count: 2).onEnded {
                    draftTitle = row.title
                    editingID = row.id
                })
        }
    }

    private var controls: some View {
        HStack(spacing: 14) {
            Picker("Group by", selection: Binding(
                get: { list.groupBy },
                set: { onGroupBy($0) })) {
                    Text("Status").tag("status")
                    Text("Bucket").tag("bucket")
                    Text("None").tag("none")
                }
                .pickerStyle(.segmented)
                .frame(width: 220)

            Picker("Bucket", selection: Binding(
                get: { list.filterBucket },
                set: { onFilter($0, list.filterStatus) })) {
                    Text("Any bucket").tag("")
                    Text("Inbox").tag("inbox")
                    Text("Now").tag("now")
                    Text("Next").tag("next")
                    Text("Later").tag("later")
                }
                .frame(width: 130)

            Picker("Status", selection: Binding(
                get: { list.filterStatus },
                set: { onFilter(list.filterBucket, $0) })) {
                    Text("Any status").tag("")
                    ForEach(StatusOption.all) { option in
                        Text(option.label).tag(option.key)
                    }
                }
                .frame(width: 140)

            if !list.filterBucket.isEmpty || !list.filterStatus.isEmpty {
                Button("Clear filters") { onFilter("", "") }
                    .buttonStyle(.link)
            }

            Spacer()

            if !selection.isEmpty {
                Text("\(selection.count) selected")
                    .font(Theme.Typography.meta)
                    .foregroundStyle(Theme.textSecondary)
                StatusMenuItems(current: "") { status in
                    onBulk(BulkPayload(ids: orderedSelection, bucket: nil,
                                       priority: nil, status: status))
                }
            }
        }
        .padding(EdgeInsets(top: 14, leading: Theme.Metrics.contentPaddingH,
                            bottom: 10, trailing: Theme.Metrics.contentPaddingH))
    }
}
