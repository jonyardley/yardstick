import App
import SwiftUI

/// Journey 5A's six statuses: label, verbatim hint, dot colour, and the
/// core's own key string so the current-selection checkmark can match.
struct StatusOption: Identifiable {
    let status: Status
    let label: String
    let hint: String
    let key: String
    let colour: Color

    var id: String { key }

    static let all: [StatusOption] = [
        .init(status: .backlog, label: "Backlog", hint: "Someday / unstarted",
              key: "backlog", colour: Theme.statusBacklog),
        .init(status: .inProgress, label: "In progress", hint: "Actively on it",
              key: "in_progress", colour: Theme.statusInProgress),
        .init(status: .blocked, label: "Blocked", hint: "Can't proceed",
              key: "blocked", colour: Theme.statusBlocked),
        .init(status: .waiting, label: "Waiting", hint: "On someone else",
              key: "waiting", colour: Theme.statusWaiting),
        .init(status: .done, label: "Done", hint: "Complete",
              key: "done", colour: Theme.statusDone),
        .init(status: .binned, label: "Binned", hint: "Dropped",
              key: "binned", colour: Theme.statusBinned),
    ]

    /// Spec §7: setting Blocked prompts for an optional one-line reason.
    /// Nothing else does — Waiting's "who" comes from person links (Phase 3).
    static func needsReason(_ status: Status) -> Bool { status == .blocked }
}

/// The six menu rows, used from a row's context menu and the All-actions
/// view's bulk menu. A `Menu` in the caller wraps these.
struct StatusMenuItems: View {
    let current: String
    let onSelect: (Status) -> Void

    var body: some View {
        Menu("Set status") {
            ForEach(StatusOption.all) { option in
                Button {
                    onSelect(option.status)
                } label: {
                    // Menu rows cannot draw a coloured dot on macOS, so the
                    // checkmark carries the current state and the hint gives
                    // the one-line description from the reference.
                    if option.key == current {
                        Label("\(option.label) — \(option.hint)", systemImage: "checkmark")
                    } else {
                        Text("\(option.label) — \(option.hint)")
                    }
                }
            }
        }
    }
}
