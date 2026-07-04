import SwiftUI

@main
struct DailyApp: App {
    @State private var core = Core()

    var body: some Scene {
        WindowGroup("Daily") {
            ContentView().environment(core)
        }
    }
}
