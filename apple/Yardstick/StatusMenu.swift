import App
import SwiftUI

/// The six status choices (core-journeys Journey 5). Task 4 ships the bare
/// buttons only, wired into `TaskRow`'s context menu; Task 8 adds the
/// blocked-reason prompt and the checkmark/description styling here.
struct StatusMenuItems: View {
    let current: String
    let onSelect: (Status) -> Void

    var body: some View {
        Button("Backlog") { onSelect(.backlog) }
        Button("In progress") { onSelect(.inProgress) }
        Button("Blocked") { onSelect(.blocked) }
        Button("Waiting") { onSelect(.waiting) }
        Button("Done") { onSelect(.done) }
        Button("Binned") { onSelect(.binned) }
    }
}
