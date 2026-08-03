import App
import SwiftUI

enum CheckboxStyle: Equatable {
    case ring                 // open: 1.5px #c4c3c0 ring
    case ringWithSoftCentre   // in progress: blue ring + 25% blue centre
    case filledCheck          // done: green fill + white check
}

/// Pure mapping from the core's strings to styling. Kept out of the view so
/// it can be tested without a host app (TaskRowFormattingTests).
enum RowStyle {
    static func checkbox(_ key: String) -> CheckboxStyle {
        switch key {
        case "in_progress": return .ringWithSoftCentre
        case "done": return .filledCheck
        default: return .ring
        }
    }

    static func pill(_ kind: String) -> (fg: Color, bg: Color)? {
        switch kind {
        case "in_progress": return (Theme.accentTextDark, Theme.pillTint)
        case "blocked": return (Theme.statusBlocked, Theme.statusBlockedBg)
        case "waiting": return (Theme.statusWaiting, Theme.statusWaitingBg)
        case "binned": return (Theme.textSecondary, Theme.chipBg)
        default: return nil
        }
    }

    static func priorityColour(_ priority: UInt8) -> Color? {
        switch priority {
        case 1: return Theme.priority1
        case 2: return Theme.priority2
        case 3: return Theme.priority3
        default: return nil
        }
    }

    /// A blocked reason is domain data that now survives a Done round trip
    /// (Task 10a), but the done row (reference §7.2 row 4) never displays
    /// it — the strikethrough is the whole story once a task is complete.
    static func showsBlockedReason(isDone: Bool, blockedReason: String) -> Bool {
        !isDone && !blockedReason.isEmpty
    }
}

/// Reference §7.2 — 17px checkbox, 14px title, optional priority badge and
/// status pill, then the fixed 70px right-aligned meta column (present even
/// when empty, so titles stay aligned down the list).
struct TaskRow: View {
    /// When set, the title renders as a focused text field in place — same
    /// anatomy, same height, checkbox still visible and live — instead of
    /// swapping the whole row for a bare field (the gate-walk layout jump).
    struct TitleEditing {
        let draft: Binding<String>
        let onSubmit: () -> Void
        let onCancel: () -> Void
    }

    let row: TaskRowVm
    let onToggleDone: () -> Void
    let onOpenTriage: () -> Void
    let onSetStatus: (Status) -> Void
    var titleEditing: TitleEditing? = nil
    /// Double-click-to-rename, attached to the TITLE only.
    ///
    /// It must not go on the whole row: the row is one opaque hit target
    /// (`.contentShape(Rectangle())` below, needed for hover and the context
    /// menu), so a row-wide tap gesture makes SwiftUI own every click in the
    /// row and a `List(selection:)` never sees one — rows stopped selecting at
    /// all, by click or ⌘-click. Scoped to the title, the rest of the row
    /// stays free for selection.
    var onRequestTitleEdit: (() -> Void)? = nil
    /// Whether this row's list keeps done rows visible (Now, All actions) —
    /// they restyle in place; vanishing lists get the §7.2 grace instead.
    var retainsDoneRows: Bool = true

    @State private var isHovered = false
    @State private var pendingDone = false
    @State private var graceTask: Task<Void, Never>?
    @FocusState private var titleFieldFocused: Bool

    /// Done styling applies while parked in the grace window too.
    private var showsDone: Bool { row.isDone || pendingDone }

    var body: some View {
        HStack(spacing: Theme.Metrics.taskRowGap) {
            Button(action: handleToggle) {
                checkbox
                    .contentShape(
                        RoundedRectangle(cornerRadius: Theme.Metrics.checkboxRadius)
                            .inset(by: -4))
            }
            .buttonStyle(.plain)
            .accessibilityLabel(row.isDone ? "Mark not done" : "Mark done")

            if let editing = titleEditing {
                TextField("", text: editing.draft)
                    .textFieldStyle(.plain)
                    .font(Theme.Typography.body)
                    .foregroundStyle(Theme.textPrimary)
                    .focused($titleFieldFocused)
                    .onSubmit(editing.onSubmit)
                    .onExitCommand(perform: editing.onCancel)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .onAppear { titleFieldFocused = true }
            } else {
                Text(row.title)
                    .font(Theme.Typography.body)
                    .foregroundStyle(showsDone ? Theme.textTertiary : Theme.textPrimary)
                    .strikethrough(showsDone)
                    .lineLimit(1)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    // simultaneousGesture, not onTapGesture: a single click on
                    // the title must still reach the list underneath so the
                    // row selects (PR #39's lesson, now scoped to the title).
                    .simultaneousGesture(
                        TapGesture(count: 2).onEnded { onRequestTitleEdit?() })
            }

            if let colour = RowStyle.priorityColour(row.priority) {
                Text("\(row.priority)")
                    .font(.system(size: 11, weight: .bold))
                    .foregroundStyle(.white)
                    .frame(width: Theme.Metrics.priorityBadgeSize,
                           height: Theme.Metrics.priorityBadgeSize)
                    .background(colour)
                    .clipShape(RoundedRectangle(cornerRadius: Theme.Metrics.priorityBadgeRadius))
            }

            if let tint = RowStyle.pill(row.statusKind), !row.statusPill.isEmpty {
                HStack(spacing: 5) {
                    Circle().fill(tint.fg).frame(width: 6, height: 6)
                    Text(row.statusPill)
                }
                .font(Theme.Typography.meta)
                .foregroundStyle(tint.fg)
                .padding(.horizontal, 9)
                .padding(.vertical, 3)
                .background(tint.bg)
                .clipShape(Capsule())
            }

            // Chips are empty until Phase 3 (carve-out 1); the loop renders
            // nothing today and needs no change when pages arrive.
            ForEach(row.chips, id: \.self) { chip in
                Text(chip)
                    .font(Theme.Typography.meta)
                    .foregroundStyle(Theme.textSecondary)
                    .padding(.horizontal, 9)
                    .padding(.vertical, 3)
                    .background(Theme.chipBg)
                    .clipShape(Capsule())
            }

            Text(row.meta)
                .font(Theme.Typography.meta)
                .foregroundStyle(showsDone ? Theme.textTertiary : Theme.textQuiet)
                .frame(width: Theme.Metrics.metaColumnWidth, alignment: .trailing)
        }
        .padding(.vertical, Theme.Metrics.taskRowVPadding)
        .padding(.horizontal, Theme.Metrics.taskRowHPadding)
        .contentShape(Rectangle())
        .background(isHovered ? Theme.hoverBg : .clear)
        .clipShape(RoundedRectangle(cornerRadius: Theme.Metrics.rowRadius))
        .opacity(showsDone ? 0.55 : 1)
        .onHover { isHovered = $0 }
        .contextMenu {
            Button("Triage…", action: onOpenTriage)
            StatusMenuItems(current: row.statusKind, onSelect: onSetStatus)
        }
        .overlay(alignment: .bottomLeading) {
            if RowStyle.showsBlockedReason(isDone: showsDone, blockedReason: row.blockedReason) {
                Text(row.blockedReason)
                    .font(Theme.Typography.meta)
                    .foregroundStyle(Theme.statusBlocked)
                    .padding(.leading, 45)
            }
        }
    }

    private func handleToggle() {
        switch TickGrace.decide(isDone: row.isDone,
                                graceActive: pendingDone,
                                listRetainsDoneRows: retainsDoneRows) {
        case .toggleNow:
            onToggleDone()
        case .beginGrace:
            pendingDone = true
            graceTask = Task {
                try? await Task.sleep(for: .seconds(TickGrace.holdSeconds))
                guard !Task.isCancelled else { return }
                pendingDone = false
                onToggleDone()
            }
        case .cancelGrace:
            graceTask?.cancel()
            graceTask = nil
            pendingDone = false
        }
    }

    @ViewBuilder
    private var checkbox: some View {
        // §7.2 as amended 2026-08-03: a Things-style rounded square.
        let size = Theme.Metrics.checkboxSize
        let shape = RoundedRectangle(cornerRadius: Theme.Metrics.checkboxRadius)
        switch RowStyle.checkbox(pendingDone ? "done" : row.checkbox) {
        case .ring:
            shape.strokeBorder(Theme.checkboxRing, lineWidth: 1.5)
                .frame(width: size, height: size)
        case .ringWithSoftCentre:
            shape.strokeBorder(Theme.accent, lineWidth: 1.5)
                .frame(width: size, height: size)
                .overlay(shape.fill(Theme.accent).opacity(0.25).padding(3))
        case .filledCheck:
            shape.fill(Theme.statusDone)
                .frame(width: size, height: size)
                .overlay(
                    Image(systemName: "checkmark")
                        .font(.system(size: 9, weight: .semibold))
                        .foregroundStyle(.white))
        }
    }
}

#Preview("Open — no priority, no pill (§7.2 row 1 baseline)") {
    TaskRow(
        row: TaskRowVm(
            id: "1", title: "Draft the onboarding doc", checkbox: "open",
            priority: 0, statusPill: "", statusKind: "", chips: [],
            meta: "2 days old", isDone: false, blockedReason: "",
            bucket: .now, due: ""),
        onToggleDone: {}, onOpenTriage: {}, onSetStatus: { _ in })
    .padding()
}

#Preview("In progress — priority badge + pill (§7.2 row 2)") {
    TaskRow(
        row: TaskRowVm(
            id: "2", title: "Chase COAST support docs response", checkbox: "in_progress",
            priority: 1, statusPill: "In progress", statusKind: "in_progress", chips: [],
            meta: "from Slack", isDone: false, blockedReason: "",
            bucket: .now, due: ""),
        onToggleDone: {}, onOpenTriage: {}, onSetStatus: { _ in })
    .padding()
}

#Preview("Now, entered today — empty meta spacer (§7.2 row 3)") {
    TaskRow(
        row: TaskRowVm(
            id: "3", title: "Book dentist", checkbox: "open",
            priority: 0, statusPill: "", statusKind: "", chips: [],
            meta: "", isDone: false, blockedReason: "",
            bucket: .now, due: ""),
        onToggleDone: {}, onOpenTriage: {}, onSetStatus: { _ in })
    .padding()
}

#Preview("Done — struck through, dimmed (§7.2 row 4)") {
    TaskRow(
        row: TaskRowVm(
            id: "4", title: "Reply to landlord", checkbox: "done",
            priority: 0, statusPill: "", statusKind: "", chips: [],
            meta: "", isDone: true, blockedReason: "",
            bucket: .now, due: ""),
        onToggleDone: {}, onOpenTriage: {}, onSetStatus: { _ in })
    .padding()
}
