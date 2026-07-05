import SwiftUI

@main
struct DailyApp: App {
    @State private var core = Core()

    var body: some Scene {
        // Daily is a one-window app by design (spec §6): a `WindowGroup`
        // grants File > New Window (⌘N), and two windows would share one
        // Core but run independent NoteEditor coordinators, so each
        // debounced whole-day save could silently wipe the other window's
        // typing. `Window` with a fixed id has no New Window command.
        Window("Daily", id: "main") {
            if let message = core.startupError {
                StartupFailureView(message: message)
            } else {
                ContentView()
                    .environment(core)
                    .frame(minWidth: 900, minHeight: 600)
            }
        }
    }
}

/// Calm failure screen for an unopenable database (decision #5): explain,
/// offer Quit. No auto-retry, no crash-loop, no red styling.
struct StartupFailureView: View {
    let message: String

    var body: some View {
        VStack(spacing: 12) {
            Text("Daily can't open its database")
                .font(.headline)
            Text(message)
                .font(.caption)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
                .frame(maxWidth: 380)
            Text("Your data has not been touched. This usually means the database was created by a newer version of Daily.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(maxWidth: 380)
            Button("Quit Daily") { NSApplication.shared.terminate(nil) }
                .keyboardShortcut(.defaultAction)
        }
        .padding(32)
        .frame(minWidth: 460, minHeight: 220)
    }
}
