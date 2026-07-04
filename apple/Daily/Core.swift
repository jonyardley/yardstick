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
    private(set) var view = ViewModel(tasks: [], count: 0, error: nil)
    /// Bound port of the embedded MCP server; 0 means it failed to start
    /// (surfaced in the UI footer, never fatal).
    private(set) var mcpPort: UInt16 = 0

    private let ffi: CoreFFI
    private let shell: ShellHandler

    init() {
        let dbURL = Self.appSupportURL().appendingPathComponent("daily.db")
        let shell = ShellHandler()
        self.shell = shell
        self.ffi = CoreFFI(dbPath: dbURL.path, shell: shell)
        // No effects can arrive before the first `update`, so wiring the
        // callback after `CoreFFI` construction is race-free (and avoids
        // capturing `self` before initialization completes).
        shell.onEffects = { [weak self] bytes in
            // The callback arrives on an arbitrary Rust thread (generated
            // CruxShell doc warning) — hop to the main actor before touching
            // observable state.
            _Concurrency.Task { @MainActor in
                self?.processEffects(bytes)
            }
        }
        mcpPort = ffi.startMcp(port: 52111, token: Self.loadOrCreateToken())
        send(.startup)
    }

    func send(_ event: Event) {
        // Serializing our own generated types cannot fail; a failure here is
        // a typegen bug and mirrors the Rust side's panic-on-mismatch contract.
        ffi.update(data: Data(try! event.bincodeSerialize()))
    }

    private func processEffects(_ bytes: Data) {
        let requests = try! Requests.bincodeDeserialize(input: [UInt8](bytes))
        for request in requests.value {
            switch request.effect {
            case .render:
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

/// Bridges the BoltFFI `CruxShell` protocol (invoked from arbitrary Rust
/// threads) to a Swift closure. The closure owns the main-actor hop.
final class ShellHandler: CruxShell {
    var onEffects: ((Data) -> Void)?
    func processEffects(bytes: Data) { onEffects?(bytes) }
}
