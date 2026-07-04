import SwiftUI

struct ContentView: View {
    @Environment(Core.self) private var core
    @State private var draft = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                TextField("New task", text: $draft)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit(create)
                Button("Add", action: create)
                    .disabled(draft.trimmingCharacters(in: .whitespaces).isEmpty)
            }
            if let error = core.view.error {
                Text(error).foregroundStyle(.red)
            }
            List(core.view.tasks, id: \.id) { task in
                TaskRow(task: task)
            }
            Text(footer)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(16)
        .frame(minWidth: 420, minHeight: 480)
    }

    private var footer: String {
        let mcp = core.mcpPort == 0
            ? "MCP failed to start"
            : "MCP on 127.0.0.1:\(core.mcpPort)"
        return "\(core.view.count) tasks · \(mcp)"
    }

    private func create() {
        let title = draft.trimmingCharacters(in: .whitespaces)
        guard !title.isEmpty else { return }
        core.send(.createTask(title: title))
        draft = ""
    }
}

struct TaskRow: View {
    let task: DailyTask

    var body: some View {
        Text(task.title)
    }
}
