import App
import SwiftUI

/// Journey 1A — "Captured today · unsorted": no metadata, no ordering
/// promises, one source tag per row, and a Triage button on the selected row.
struct InboxView: View {
    let list: TaskListVm
    let onToggleDone: (String) -> Void
    let onOpenTriage: (String) -> Void
    let onSetStatus: (String, Status) -> Void

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                TaskListView(list: list,
                             onToggleDone: onToggleDone,
                             onOpenTriage: onOpenTriage,
                             onSetStatus: onSetStatus)
                if list.groups.allSatisfy({ $0.rows.isEmpty }) {
                    Text("Nothing to sort.")
                        .font(Theme.Typography.body)
                        .foregroundStyle(Theme.textQuiet)
                        .padding(.top, 18)
                }
            }
            .padding(EdgeInsets(top: Theme.Metrics.contentPaddingTop,
                                leading: Theme.Metrics.contentPaddingH,
                                bottom: 40,
                                trailing: Theme.Metrics.contentPaddingH))
            .frame(maxWidth: .infinity, alignment: .topLeading)
        }
        .background(Color.white)
    }
}

#Preview("With rows") {
    InboxView(
        list: TaskListVm(
            title: "Inbox", subtitle: "Captured today · unsorted",
            groups: [
                TaskGroupVm(label: "", kind: "", count: 2, rows: [
                    TaskRowVm(id: "1", title: "Book dentist", checkbox: "open",
                              priority: 0, statusPill: "", statusKind: "", chips: [],
                              meta: "quick add", isDone: false, blockedReason: "",
                              bucket: .inbox, due: ""),
                    TaskRowVm(id: "2", title: "Chase COAST support docs response", checkbox: "open",
                              priority: 0, statusPill: "", statusKind: "", chips: [],
                              meta: "from an agent", isDone: false, blockedReason: "",
                              bucket: .inbox, due: ""),
                ]),
            ],
            momentum: nil, collapsed: [], groupBy: "status", filterBucket: "", filterStatus: ""),
        onToggleDone: { _ in }, onOpenTriage: { _ in }, onSetStatus: { _, _ in })
}

#Preview("Empty") {
    InboxView(
        list: TaskListVm(
            title: "Inbox", subtitle: "Captured today · unsorted",
            groups: [TaskGroupVm(label: "", kind: "", count: 0, rows: [])],
            momentum: nil, collapsed: [], groupBy: "status", filterBucket: "", filterStatus: ""),
        onToggleDone: { _ in }, onOpenTriage: { _ in }, onSetStatus: { _, _ in })
}
