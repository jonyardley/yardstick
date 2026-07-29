import SwiftUI

struct ContentView: View {
    @Environment(Core.self) private var core
    @State private var showQuickAdd = false

    var body: some View {
        HStack(spacing: 0) {
            SidebarView(
                sidebar: core.view.sidebar,
                calendar: core.view.calendar,
                onGoToToday: { core.goToToday() },
                onSelectDate: { core.navigate(to: $0) },
                onShiftMonth: { core.shiftMonth($0) },
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
                DayColumn(day: core.view.day,
                          editable: core.dayIsEditable,
                          onEdit: { core.noteEdited($0) })
            }
        }
        .navigationTitle("Today")
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
                    QuickAddView { core.send(.captureTask(title: $0, source: "quick_add")) }
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
