import App
import SwiftUI

enum TriageIntent: Equatable {
    case bucket(Bucket)
    case priority(UInt8)
}

/// Journey 1B: N/E/L and 1/2/3. Kept pure so the mapping is testable
/// without a window (TriageKeyboardTests).
enum TriageKey {
    static func intent(for character: Character) -> TriageIntent? {
        switch Character(character.lowercased()) {
        case "n": return .bucket(.now)
        case "e": return .bucket(.next)
        case "l": return .bucket(.later)
        case "1": return .priority(1)
        case "2": return .priority(2)
        case "3": return .priority(3)
        default: return nil
        }
    }
}

struct TriageDraft: Equatable {
    var bucket: Bucket
    /// 0 = none.
    var priority: UInt8
    /// 'YYYY-MM-DD' or "".
    var due: String

    mutating func apply(_ intent: TriageIntent) {
        switch intent {
        case .bucket(let bucket):
            self.bucket = bucket
        case .priority(let priority):
            // Pressing the same digit again clears it — priority is optional.
            self.priority = self.priority == priority ? 0 : priority
        }
    }
}

/// Journey 1B — one lightweight sheet, three fields, keyboard-first.
/// The reference's fourth field (PROJECT / PERSON, opened with `#`) needs the
/// pages table and lands in Phase 3 (carve-out 1).
struct TriageSheet: View {
    let title: String
    let onCommit: (TriageDraft) -> Void
    let onCancel: () -> Void

    @State private var draft: TriageDraft
    @State private var hasDue: Bool
    @State private var dueDate: Date
    @FocusState private var keyboardFocus: Bool

    init(title: String,
         initial: TriageDraft,
         onCommit: @escaping (TriageDraft) -> Void,
         onCancel: @escaping () -> Void) {
        self.title = title
        self.onCommit = onCommit
        self.onCancel = onCancel
        _draft = State(initialValue: initial)
        _hasDue = State(initialValue: !initial.due.isEmpty)
        _dueDate = State(initialValue: Self.parse(initial.due) ?? Date())
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(title)
                .font(.system(size: 17, weight: .semibold))
                .foregroundStyle(Theme.textPrimary)

            field("WHEN") {
                Picker("", selection: $draft.bucket) {
                    Text("Now").tag(Bucket.now)
                    Text("Next").tag(Bucket.next)
                    Text("Later").tag(Bucket.later)
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .frame(width: 260)
            }

            field("PRIORITY") {
                HStack(spacing: 8) {
                    ForEach(UInt8(1)...UInt8(3), id: \.self) { value in
                        Button {
                            draft.apply(.priority(value))
                        } label: {
                            Text("\(value)")
                                .font(.system(size: 12, weight: .bold))
                                .foregroundStyle(draft.priority == value ? .white : Theme.textSecondary)
                                .frame(width: 26, height: 26)
                                .background(draft.priority == value
                                    ? (RowStyle.priorityColour(value) ?? Theme.priority3)
                                    : Theme.chipBg)
                                .clipShape(RoundedRectangle(cornerRadius: 6))
                        }
                        .buttonStyle(.plain)
                    }
                }
            }

            field("DUE") {
                HStack(spacing: 10) {
                    Toggle("", isOn: $hasDue).labelsHidden()
                    DatePicker("", selection: $dueDate, displayedComponents: .date)
                        .labelsHidden()
                        .disabled(!hasDue)
                        .opacity(hasDue ? 1 : 0.4)
                }
            }

            HStack {
                Spacer()
                Button("Cancel", action: onCancel).keyboardShortcut(.cancelAction)
                Button("Triage") {
                    draft.due = hasDue ? Self.iso(dueDate) : ""
                    onCommit(draft)
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
        .frame(width: 380)
        // Keyboard-first: the sheet takes key events itself, so N/E/L and
        // 1/2/3 work without tabbing to a control. The focus is keyboard
        // plumbing, not a control the user tabbed to — suppress the ring
        // macOS would otherwise draw around the whole sheet body.
        .focusable()
        .focusEffectDisabled()
        .focused($keyboardFocus)
        .onAppear { keyboardFocus = true }
        .onKeyPress { press in
            guard let character = press.characters.first,
                  let intent = TriageKey.intent(for: character) else { return .ignored }
            draft.apply(intent)
            return .handled
        }
    }

    @ViewBuilder
    private func field(_ label: String, @ViewBuilder content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label)
                .font(Theme.Typography.capsLabel)
                .tracking(0.66)
                .foregroundStyle(Theme.textMuted)
            content()
        }
    }

    private static let formatter: DateFormatter = {
        let f = DateFormatter()
        f.calendar = Calendar(identifier: .gregorian)
        f.locale = Locale(identifier: "en_US_POSIX")
        f.timeZone = .current
        f.dateFormat = "yyyy-MM-dd"
        return f
    }()

    static func iso(_ date: Date) -> String { formatter.string(from: date) }
    static func parse(_ iso: String) -> Date? { formatter.date(from: iso) }
}
