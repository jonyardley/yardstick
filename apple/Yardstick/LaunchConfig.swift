import Foundation

/// Environment overrides for launching the app under test. Absent or invalid
/// variables mean production behaviour; parsing is pure so it is unit-tested
/// (LaunchConfigTests) without launching anything.
struct LaunchConfig: Equatable {
    /// YARDSTICK_SUPPORT_DIR — absolute path replacing the real Application
    /// Support directory (database + MCP token live inside it).
    var supportDir: String?
    /// YARDSTICK_DISABLE_MCP=1 — do not start the embedded MCP server, so a
    /// test-launched app never fights a dev instance for port 52111.
    var mcpDisabled: Bool
    /// YARDSTICK_TODAY — fixed 'YYYY-MM-DD' handed to the clock-free core,
    /// so date-derived copy ("entered today", due weekdays) is deterministic.
    var today: String?

    static func from(_ environment: [String: String]) -> LaunchConfig {
        LaunchConfig(
            supportDir: environment["YARDSTICK_SUPPORT_DIR"]
                .flatMap { $0.hasPrefix("/") ? $0 : nil },
            mcpDisabled: environment["YARDSTICK_DISABLE_MCP"] == "1",
            today: environment["YARDSTICK_TODAY"].flatMap { isoDate($0) ? $0 : nil })
    }

    private static func isoDate(_ candidate: String) -> Bool {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.date(from: candidate) != nil
    }
}
