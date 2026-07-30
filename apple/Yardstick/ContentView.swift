import SwiftUI

struct ContentView: View {
    @Environment(Core.self) private var core
    @State private var showQuickAdd = false

    var body: some View {
        @Bindable var core = core
        HStack(spacing: 0) {
            SidebarView(
                sidebar: core.view.sidebar,
                calendar: core.view.calendar,
                route: core.view.route,
                onGoToToday: { core.goToToday() },
                onSelectDate: { core.navigate(to: $0) },
                onShiftMonth: { core.shiftMonth($0) },
                onSelectView: { core.selectView($0) },
                mcpStatus: core.mcpPort == 0
                    ? "MCP not running"
                    : "MCP · 127.0.0.1:\(core.mcpPort)")
            VStack(spacing: 0) {
                if let error = core.view.error {
                    Text(error)
                        .font(.system(size: 12))
                        .foregroundStyle(Theme.textSecondary)
                        .frame(maxWidth: .infinity)
                        .padding(6)
                        .background(Theme.blockBg)
                }
                switch core.view.route {
                case "today":
                    DayColumn(day: core.view.day,
                              editable: core.dayIsEditable,
                              onEdit: { core.noteEdited($0) },
                              list: core.view.list,
                              onToggleDone: { core.toggleDone(id: $0) },
                              onOpenTriage: { _ in },
                              onSetStatus: { core.send(.setStatus(id: $0, status: $1, reason: "")) })
                case "inbox":
                    InboxView(list: core.view.list,
                              onToggleDone: { core.toggleDone(id: $0) },
                              onOpenTriage: { core.triageTarget = $0 },
                              onSetStatus: { core.setStatus(id: $0, status: $1) })
                case "all":
                    AllActionsView(list: core.view.list,
                                   onToggleDone: { core.toggleDone(id: $0) },
                                   onOpenTriage: { core.triageTarget = $0 },
                                   onSetStatus: { core.setStatus(id: $0, status: $1) },
                                   onEditTitle: { core.editTitle(id: $0, title: $1) },
                                   onBulk: { core.bulk($0) },
                                   onGroupBy: { core.setGrouping($0) },
                                   onFilter: { core.setFilter(bucket: $0, status: $1) })
                default:
                    // now / next / later / waiting: the same list surface.
                    ScrollView {
                        TaskListView(list: core.view.list,
                                     onToggleDone: { core.toggleDone(id: $0) },
                                     onOpenTriage: { core.triageTarget = $0 },
                                     onSetStatus: { core.setStatus(id: $0, status: $1) })
                            .padding(EdgeInsets(top: Theme.Metrics.contentPaddingTop,
                                                 leading: Theme.Metrics.contentPaddingH,
                                                 bottom: 40,
                                                 trailing: Theme.Metrics.contentPaddingH))
                    }
                    .background(Color.white)
                }
            }
        }
        .sheet(isPresented: Binding(
            get: { core.triageTarget != nil },
            set: { if !$0 { core.triageTarget = nil } }
        )) {
            if let id = core.triageTarget {
                let row = core.row(id: id)
                TriageSheet(
                    title: "Triage · \(row?.title ?? "")",
                    initial: TriageDraft(bucket: row?.bucket ?? .now,
                                         priority: row?.priority ?? 0,
                                         due: row?.due ?? ""),
                    onCommit: { core.triage(id: id, draft: $0) },
                    onCancel: { core.triageTarget = nil })
            }
        }
        .navigationTitle(core.view.route == "today" ? "Today" : core.view.list.title)
        .sheet(isPresented: Binding(
            get: { core.blockedReasonTarget != nil },
            set: { if !$0 { core.blockedReasonTarget = nil } }
        )) {
            if let id = core.blockedReasonTarget {
                BlockedReasonPrompt(
                    onCommit: { core.commitBlockedReason(id: id, reason: $0) },
                    onCancel: { core.blockedReasonTarget = nil })
            }
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button { showQuickAdd = true } label: {
                    Image(systemName: "plus")
                        .foregroundStyle(.white)
                        .frame(width: Theme.Metrics.plusButtonSize,
                               height: Theme.Metrics.plusButtonSize)
                        .background(Theme.accent)
                        .clipShape(RoundedRectangle(cornerRadius: 7))
                }
                .buttonStyle(.plain)
                .popover(isPresented: $showQuickAdd) {
                    QuickAddView { core.capture($0, source: "quick_add") }
                }
            }
        }
    }
}

/// Journey 5A: setting Blocked can carry an optional one-line reason.
/// Return commits (an empty reason is allowed), Escape cancels — the
/// status change is only sent to the core on commit, so cancelling
/// leaves it untouched.
private struct BlockedReasonPrompt: View {
    @State private var reason = ""
    let onCommit: (String) -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Blocked")
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(Theme.textPrimary)
            TextField("Reason (optional)", text: $reason)
                .textFieldStyle(.roundedBorder)
                .onSubmit { onCommit(reason) }
            HStack {
                Spacer()
                Button("Cancel", role: .cancel, action: onCancel)
                    .keyboardShortcut(.cancelAction)
                Button("Blocked") { onCommit(reason) }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
        .frame(width: 280)
    }
}
