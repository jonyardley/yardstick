import SwiftUI

/// The toolbar `+` popover — keeps the Phase 0 task-creation path reachable;
/// Inbox count updates live.
struct QuickAddView: View {
    let onSubmit: (String) -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var title = ""

    var body: some View {
        HStack {
            TextField("New task", text: $title)
                .textFieldStyle(.roundedBorder)
                .frame(width: 260)
                .onSubmit(submit)
            Button("Add", action: submit)
                .disabled(title.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .padding(12)
    }

    private func submit() {
        let trimmed = title.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        onSubmit(trimmed)
        title = ""
        dismiss()
    }
}

#Preview {
    QuickAddView(onSubmit: { _ in })
}
