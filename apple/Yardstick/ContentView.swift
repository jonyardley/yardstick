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
        .navigationTitle("Today")
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
