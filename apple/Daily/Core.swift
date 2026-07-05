import App
import Foundation
import Shared

/// The one place the `App.Task` / `_Concurrency.Task` name collision is
/// resolved (task-5 report §6): everywhere in the shell the domain type is
/// spelled `DailyTask`; bare `Task` is never used for the domain type.
typealias DailyTask = App.Task

/// Owns the Rust core: sends bincode-serialized events in, receives effect
/// batches via the `CruxShell` push callback, and exposes the latest
/// `ViewModel` to SwiftUI.
@Observable @MainActor
final class Core {
    private(set) var view = ViewModel(
        sidebar: SidebarVm(
            spaceName: "", spaceInitials: "", todayLabel: "",
            views: [], projects: [], people: [], pages: []),
        calendar: CalendarVm(monthLabel: "", cells: []),
        day: DayVm(date: "", title: "", noteText: "", editorVersion: 0),
        error: nil
    )
    /// Bound port of the embedded MCP server; 0 means it failed to start
    /// (surfaced in the UI footer, never fatal).
    private(set) var mcpPort: UInt16 = 0
    /// Non-nil when the Rust core failed to open/migrate its database at
    /// startup. The app shows a message + Quit and sends no events.
    private(set) var startupError: String?

    private let ffi: CoreFFI
    private let shell: ShellHandler

    init() {
        let dbURL = Self.appSupportURL().appendingPathComponent("daily.db")
        let relay = EffectRelay()
        let shell = ShellHandler { bytes in
            // The callback arrives on an arbitrary Rust thread — hop to the
            // main actor before touching observable state.
            _Concurrency.Task { @MainActor in
                relay.target?.processEffects(bytes)
            }
        }
        self.shell = shell
        self.ffi = CoreFFI(dbPath: dbURL.path, shell: shell)

        let initError = ffi.initError()
        guard initError.isEmpty else {
            startupError = initError
            return // inert core: no relay target, no MCP, no startup event
        }

        relay.target = self
        mcpPort = ffi.startMcp(port: 52111, token: Self.loadOrCreateToken())
        send(.startup(today: Self.todayString()))
    }

    func send(_ event: Event) {
        // Serializing our own generated types cannot fail; a failure here is
        // a typegen bug and mirrors the Rust side's panic-on-mismatch contract.
        ffi.update(data: Data(try! event.bincodeSerialize()))
    }

    // Navigation entry points. Thin today; Task 8 gives them flush-pending-
    // edit semantics, so ALL UI navigation must route through these, never
    // send(.navigateToDay) directly.
    func navigate(to date: String) { send(.navigateToDay(date: date)) }
    func goToToday() { send(.goToToday) }
    func shiftMonth(_ delta: Int32) { send(.shiftMonth(delta: delta)) }

    private func processEffects(_ bytes: Data) {
        let requests = try! Requests.bincodeDeserialize(input: [UInt8](bytes))
        for request in requests.value {
            switch request.effect {
            case .render:
                // CONSTRAINT: Render carries no payload — its contract is
                // "re-fetch the whole view model", and that refetch is
                // idempotent. That idempotency is exactly what makes the
                // unstructured `_Concurrency.Task` hop from the Rust callback
                // thread safe: if renders coalesce, reorder, or double-fire we
                // still converge on the latest view. This breaks the day Render
                // ever carries a diff — restructure the hop before doing that.
                view = try! ViewModel.bincodeDeserialize(input: [UInt8](ffi.view()))
            case .storage:
                // Storage is consumed Rust-side by the EffectRouter; the case
                // only exists because typegen still emits it (task-5 §3).
                assertionFailure("storage effect reached the shell: \(request)")
            }
        }
    }

    private static func appSupportURL() -> URL {
        let url = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Daily")
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    /// The core is clock-free (decision #6): the shell supplies today's
    /// date, in the user's current timezone, as 'YYYY-MM-DD'.
    private static func todayString() -> String {
        let fmt = DateFormatter()
        fmt.calendar = Calendar(identifier: .gregorian)
        fmt.locale = Locale(identifier: "en_US_POSIX")
        fmt.timeZone = .current
        fmt.dateFormat = "yyyy-MM-dd"
        return fmt.string(from: Date())
    }

    private static func loadOrCreateToken() -> String {
        let url = appSupportURL().appendingPathComponent("mcp-token")
        if let token = try? String(contentsOf: url, encoding: .utf8)
            .trimmingCharacters(in: .whitespacesAndNewlines),
            !token.isEmpty
        {
            return token
        }
        let token = (0..<32).map { _ in String(format: "%02x", UInt8.random(in: 0...255)) }.joined()
        try? token.write(to: url, atomically: true, encoding: .utf8)
        try? FileManager.default.setAttributes(
            [.posixPermissions: 0o600], ofItemAtPath: url.path)
        return token
    }
}

/// Breaks the init-order cycle: `ShellHandler` wants its closure at
/// construction time (immutable — Swift 6 friendly), but the closure's
/// target (`Core`) doesn't exist until after `CoreFFI` is built. No effects
/// can arrive before the first `update`, so wiring `target` after
/// construction is race-free (same argument as Phase 0's late closure).
@MainActor
final class EffectRelay {
    weak var target: Core?
}

/// Bridges the BoltFFI `CruxShell` protocol (invoked from arbitrary Rust
/// threads) to an immutable Swift closure. The closure owns the
/// main-actor hop.
final class ShellHandler: CruxShell {
    private let onEffects: @Sendable (Data) -> Void
    init(_ onEffects: @escaping @Sendable (Data) -> Void) { self.onEffects = onEffects }
    func processEffects(bytes: Data) { onEffects(bytes) }
}
