import Foundation

/// §7.2 amendment (2026-08-03): ticking a row that would leave its list
/// holds the done styling for a grace window, then the row animates out.
/// Pure decision table — the view owns the timer, this owns the choices.
enum TickGrace {
    static let holdSeconds: TimeInterval = 1.2

    enum Decision: Equatable {
        case toggleNow    // dispatch ToggleDone immediately
        case beginGrace   // show done styling now, dispatch after the hold
        case cancelGrace  // second click during the hold: revert, dispatch nothing
    }

    static func decide(isDone: Bool, graceActive: Bool, listRetainsDoneRows: Bool) -> Decision {
        if graceActive { return .cancelGrace }
        if isDone || listRetainsDoneRows { return .toggleNow }
        return .beginGrace
    }
}
