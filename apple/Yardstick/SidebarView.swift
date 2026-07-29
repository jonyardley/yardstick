import App
import SwiftUI

/// Reference §2 — `#edecea`, 12/10 padding, right hairline; §2.1 space row;
/// §2.2 active Today row 30px accent radius-7; §2.4 section headers
/// 11/700/0.06em uppercase `#9a9a98`; §§2.5–2.6 view rows 29px with per-kind
/// dots and right-aligned counts; §§2.7–2.9 sections render only with rows
/// — empty in Phase 1 (Global Constraints carve-out: Projects/People/Pages
/// are data-driven and simply absent, not faked, while empty).
///
/// Layout decision, recorded: a plain `ScrollView` + `VStack` over
/// `Theme.sidebarBg`, not `List(.sidebar)` + system material as spec §6
/// sketches — the reference's exact metrics (29px rows, radius-7, precise
/// paddings) are the acceptance criteria and `List` fights all of them.
/// (spec §6 delta: sidebar is custom-layout over a flat tint in P1; revisit
/// material look at a polish pass.)
struct SidebarView: View {
    let sidebar: SidebarVm
    let calendar: CalendarVm
    let onGoToToday: () -> Void
    let onSelectDate: (String) -> Void
    let onShiftMonth: (Int32) -> Void
    let mcpStatus: String

    var body: some View {
        VStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    spaceRow
                    todayRow
                    CalendarCard(calendar: calendar,
                                 onSelect: onSelectDate,
                                 onShift: onShiftMonth)
                    sectionHeader("Views", topPadding: 4)
                    ForEach(Array(sidebar.views.enumerated()), id: \.offset) { _, row in
                        viewRow(row)
                    }
                    // Projects / People / Pages: data-driven; empty in
                    // Phase 1 ⇒ absent, not dead (Global Constraints).
                    entrySection("Projects", sidebar.projects)
                    entrySection("People", sidebar.people)
                    entrySection("Pages", sidebar.pages)
                }
                .padding(.horizontal, 10)
                .padding(.top, 12)
            }
            // Dev-useful, honest status until the Phase 5 Settings UI.
            Text(mcpStatus)
                .font(.system(size: 10))
                .foregroundStyle(Theme.textMuted)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(10)
        }
        .frame(width: Theme.Metrics.sidebarWidth)
        .background(Theme.sidebarBg)
        .overlay(alignment: .trailing) {
            Theme.hairline09.frame(width: 0.5)
        }
    }

    private var spaceRow: some View {
        HStack(spacing: 8) {
            Text(sidebar.spaceInitials)
                .font(.system(size: 10, weight: .bold))
                .foregroundStyle(.white)
                .frame(width: 22, height: 22)
                .background(Theme.spaceBadge)
                .clipShape(RoundedRectangle(cornerRadius: 6))
            Text(sidebar.spaceName)
                .font(Theme.Typography.spaceName)
            Spacer()
            Text("⌄")
                .font(.system(size: 10))
                .foregroundStyle(Theme.textQuaternary)
        }
        .padding(EdgeInsets(top: 5, leading: 8, bottom: 12, trailing: 8))
    }

    private var todayRow: some View {
        Button(action: onGoToToday) {
            HStack(spacing: 9) {
                RoundedRectangle(cornerRadius: 2)
                    .fill(.white)
                    .frame(width: 9, height: 9)
                Text("Today")
                    .font(Theme.Typography.sidebarRowActive)
                Spacer()
                Text(sidebar.todayLabel)
                    .font(.system(size: 11))
                    .opacity(0.85)
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 9)
            .frame(height: Theme.Metrics.sidebarActiveRowHeight)
            .background(Theme.accent)
            .clipShape(RoundedRectangle(cornerRadius: Theme.Metrics.sidebarRowRadius))
        }
        .buttonStyle(.plain)
    }

    private func sectionHeader(_ title: String, topPadding: CGFloat = 12) -> some View {
        Text(title.uppercased())
            .font(Theme.Typography.capsLabel)
            .tracking(0.66) // 0.06em of 11px
            .foregroundStyle(Theme.textQuiet)
            .padding(EdgeInsets(top: topPadding, leading: 8, bottom: 6, trailing: 8))
    }

    private func viewRow(_ row: ViewRowVm) -> some View {
        HStack(spacing: 9) {
            viewIcon(kind: row.kind)
            Text(row.label)
                .font(Theme.Typography.sidebarRow)
                .foregroundStyle(Theme.textPrimary)
            Spacer()
            Text("\(row.count)")
                .font(Theme.Typography.count)
                .foregroundStyle(row.count == 0 ? Theme.countEmpty : Theme.textMuted)
        }
        .padding(.horizontal, 9)
        .frame(height: Theme.Metrics.sidebarRowHeight)
    }

    /// Reference §2.6: Now/Next = filled dots, Later = grey dot, Waiting on
    /// = hollow amber ring, Inbox = tray outline.
    @ViewBuilder
    private func viewIcon(kind: String) -> some View {
        switch kind {
        case "now":
            Circle().fill(Theme.accent).frame(width: 8, height: 8)
        case "next":
            Circle().fill(Theme.amberDot).frame(width: 8, height: 8)
        case "later":
            Circle().fill(Theme.textDisabled).frame(width: 8, height: 8)
        case "waiting":
            Circle().strokeBorder(Theme.amberDot, lineWidth: 1.5)
                .frame(width: 8, height: 8)
        default: // inbox tray
            UnevenRoundedRectangle(
                bottomLeadingRadius: 2, bottomTrailingRadius: 2)
                .strokeBorder(Theme.textMuted, lineWidth: 1.5)
                .frame(width: 11, height: 8)
        }
    }

    @ViewBuilder
    private func entrySection(_ title: String, _ entries: [SidebarEntryVm]) -> some View {
        if !entries.isEmpty {
            sectionHeader(title)
            ForEach(Array(entries.enumerated()), id: \.offset) { _, entry in
                HStack(spacing: 9) {
                    Text(entry.label)
                        .font(Theme.Typography.sidebarRow)
                    Spacer()
                    if entry.count > 0 {
                        Text("\(entry.count)")
                            .font(Theme.Typography.count)
                            .foregroundStyle(Theme.textMuted)
                    }
                }
                .padding(.horizontal, 9)
                .frame(height: Theme.Metrics.sidebarRowHeight)
            }
        }
    }
}

#Preview {
    SidebarView(
        sidebar: SidebarVm(
            spaceName: "Red Badger",
            spaceInitials: "RB",
            todayLabel: "Jul 2",
            views: [
                ViewRowVm(kind: "now", label: "Now", count: 3),
                ViewRowVm(kind: "next", label: "Next · This week", count: 4),
                ViewRowVm(kind: "later", label: "Later", count: 11),
                ViewRowVm(kind: "waiting", label: "Waiting on", count: 3),
                ViewRowVm(kind: "inbox", label: "Inbox", count: 0),
            ],
            projects: [],
            people: [],
            pages: []),
        calendar: CalendarVm(
            monthLabel: "July 2026",
            cells: [
                CalendarCellVm(day: 2, date: "2026-07-02", isToday: true, isSelected: true, isWeekend: false),
            ]),
        onGoToToday: {},
        onSelectDate: { _ in },
        onShiftMonth: { _ in },
        mcpStatus: "MCP · 127.0.0.1:52111")
}
