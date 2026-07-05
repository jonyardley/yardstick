import App
import SwiftUI

/// Reference §2.3: card `rgba(255,255,255,0.55)` radius 9, header 12.5/600
/// with `‹ ›` arrows, Monday-first weekday row 9.5/600 `#b0b0ae`, 22px cells,
/// today = 21px accent circle w/ white 11.5/600 numeral, weekends `#c0c0be`.
///
/// Interpretation recorded (brief step 2): the mock's outlined circle on
/// "yesterday" marks **today when it is not the selected day** — filled
/// circle always marks the selected day.
struct CalendarCard: View {
    let calendar: CalendarVm
    let onSelect: (String) -> Void
    let onShift: (Int32) -> Void

    private let columns = Array(
        repeating: GridItem(.flexible(), spacing: 1), count: 7)
    private let weekdays = ["M", "T", "W", "T", "F", "S", "S"]

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(calendar.monthLabel)
                    .font(Theme.Typography.calendarHeader)
                    .foregroundStyle(Theme.textPrimary)
                Spacer()
                HStack(spacing: 11) {
                    Button { onShift(-1) } label: { Text("‹") }
                    Button { onShift(1) } label: { Text("›") }
                }
                .buttonStyle(.plain)
                .font(.system(size: 11))
                .foregroundStyle(Theme.textQuaternary)
            }
            .padding(.horizontal, 2)
            .padding(.bottom, 7)

            LazyVGrid(columns: columns, spacing: 1) {
                ForEach(Array(weekdays.enumerated()), id: \.offset) { _, d in
                    Text(d)
                        .font(Theme.Typography.calendarWeekday)
                        .foregroundStyle(Theme.textQuaternary)
                        .padding(.bottom, 3)
                }
                ForEach(Array(calendar.cells.enumerated()), id: \.offset) { _, cell in
                    CalendarCellView(cell: cell, onSelect: onSelect)
                }
            }
        }
        .padding(9)
        .background(Theme.calendarCardBg)
        .clipShape(RoundedRectangle(cornerRadius: Theme.Metrics.calendarCardRadius))
        .padding(.horizontal, 4)
        .padding(.top, 10)
        .padding(.bottom, 12)
    }
}

private struct CalendarCellView: View {
    let cell: CalendarCellVm
    let onSelect: (String) -> Void

    var body: some View {
        Group {
            if cell.day == 0 {
                Color.clear
            } else {
                Button { onSelect(cell.date) } label: {
                    ZStack {
                        if cell.isSelected {
                            Circle().fill(Theme.accent)
                                .frame(width: Theme.Metrics.calendarDayCircle,
                                       height: Theme.Metrics.calendarDayCircle)
                        } else if cell.isToday {
                            Circle().strokeBorder(Theme.calendarOutline, lineWidth: 1)
                                .frame(width: Theme.Metrics.calendarDayCircle,
                                       height: Theme.Metrics.calendarDayCircle)
                        }
                        Text("\(cell.day)")
                            .font(cell.isSelected
                                  ? .system(size: 11.5, weight: .semibold)
                                  : Theme.Typography.calendarDay)
                            .foregroundStyle(
                                cell.isSelected ? .white
                                : cell.isWeekend ? Theme.textDisabled
                                : Theme.textBody)
                    }
                }
                .buttonStyle(.plain)
            }
        }
        .frame(height: Theme.Metrics.calendarCellHeight)
    }
}

#Preview {
    CalendarCard(
        calendar: CalendarVm(
            monthLabel: "July 2026",
            cells: [
                CalendarCellVm(day: 0, date: "", isToday: false, isSelected: false, isWeekend: false),
                CalendarCellVm(day: 0, date: "", isToday: false, isSelected: false, isWeekend: false),
                CalendarCellVm(day: 1, date: "2026-07-01", isToday: false, isSelected: false, isWeekend: false),
                CalendarCellVm(day: 2, date: "2026-07-02", isToday: true, isSelected: true, isWeekend: false),
                CalendarCellVm(day: 3, date: "2026-07-03", isToday: false, isSelected: false, isWeekend: false),
                CalendarCellVm(day: 4, date: "2026-07-04", isToday: false, isSelected: false, isWeekend: true),
                CalendarCellVm(day: 5, date: "2026-07-05", isToday: false, isSelected: false, isWeekend: true),
            ]),
        onSelect: { _ in },
        onShift: { _ in })
    .padding()
    .background(Theme.sidebarBg)
}
