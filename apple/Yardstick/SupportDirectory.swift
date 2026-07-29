import Foundation

/// Resolves the application-support directory holding the database and the
/// MCP token.
///
/// The product was renamed Daily -> Yardstick (spec §12 Q1), which moves that
/// directory. A launch that finds only the old one carries it over by moving
/// it, so notes written before the rename and the MCP token in every client's
/// config both survive. A move (not a copy) is deliberate: two databases would
/// diverge silently, and the next launch must find exactly one.
enum SupportDirectory {
    private static let name = "Yardstick"
    private static let legacyName = "Daily"

    /// The directory to use, created if absent. `parent` is the user's
    /// Application Support directory; it is a parameter so this is testable
    /// without touching the real one.
    static func url(in parent: URL, fileManager: FileManager = .default) -> URL {
        let current = parent.appendingPathComponent(name)
        let legacy = parent.appendingPathComponent(legacyName)

        // Only when there is nothing to lose: an existing Yardstick directory
        // is the live one and is never clobbered by the legacy copy.
        if !fileManager.fileExists(atPath: current.path),
            fileManager.fileExists(atPath: legacy.path)
        {
            try? fileManager.moveItem(at: legacy, to: current)
        }

        try? fileManager.createDirectory(at: current, withIntermediateDirectories: true)
        return current
    }

    /// The real directory under the user's Application Support.
    static func url() -> URL {
        let parent = FileManager.default.urls(
            for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return url(in: parent)
    }
}
