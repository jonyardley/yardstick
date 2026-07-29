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
}

/// Reference §7.2 — 17px checkbox, 14px title, optional priority badge and
/// status pill, then the fixed 70px right-aligned meta column (present even
/// when empty, so titles stay aligned down the list).
struct TaskRow: View {
    let row: TaskRowVm
    let onToggleDone: () -> Void
    let onOpenTriage: () -> Void
    let onSetStatus: (Status) -> Void

    @State private var isHovered = false

    var body: some View {
        HStack(spacing: Theme.Metrics.taskRowGap) {
            Button(action: onToggleDone) { checkbox }
                .buttonStyle(.plain)
                .accessibilityLabel(row.isDone ? "Mark not done" : "Mark done")

            Text(row.title)
                .font(Theme.Typography.body)
                .foregroundStyle(row.isDone ? Theme.textTertiary : Theme.textPrimary)
                .strikethrough(row.isDone)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)

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
                .foregroundStyle(row.isDone ? Theme.textTertiary : Theme.textQuiet)
                .frame(width: Theme.Metrics.metaColumnWidth, alignment: .trailing)
        }
        .padding(.vertical, Theme.Metrics.taskRowVPadding)
        .padding(.horizontal, Theme.Metrics.taskRowHPadding)
        .background(isHovered ? Theme.hoverBg : .clear)
        .clipShape(RoundedRectangle(cornerRadius: Theme.Metrics.rowRadius))
        .opacity(row.isDone ? 0.55 : 1)
        .onHover { isHovered = $0 }
        .contextMenu {
            Button("Triage…", action: onOpenTriage)
            StatusMenuItems(current: row.statusKind, onSelect: onSetStatus)
        }
        .overlay(alignment: .bottomLeading) {
            if !row.blockedReason.isEmpty {
                Text(row.blockedReason)
                    .font(Theme.Typography.meta)
                    .foregroundStyle(Theme.statusBlocked)
                    .padding(.leading, 45)
            }
        }
    }

    @ViewBuilder
    private var checkbox: some View {
        let size = Theme.Metrics.checkboxSize
        switch RowStyle.checkbox(row.checkbox) {
        case .ring:
            Circle().strokeBorder(Theme.checkboxRing, lineWidth: 1.5)
                .frame(width: size, height: size)
        case .ringWithSoftCentre:
            Circle().strokeBorder(Theme.accent, lineWidth: 1.5)
                .frame(width: size, height: size)
                .overlay(Circle().fill(Theme.accent).opacity(0.25).padding(3))
        case .filledCheck:
            Circle().fill(Theme.statusDone)
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
