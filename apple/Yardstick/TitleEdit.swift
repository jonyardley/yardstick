import Foundation

/// The rule for committing an inline title edit, kept out of the view so it
/// is testable without a host app: trim, drop empties, drop no-ops.
enum TitleEdit {
    static func commit(draft: String, original: String) -> String? {
        let trimmed = draft.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty, trimmed != original else { return nil }
        return trimmed
    }
}
