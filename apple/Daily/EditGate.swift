/// R1 — the editability gate, as a pure state machine (unit-tested in
/// EditGateTests; Core owns the only instance).
///
/// Why it exists: `EditDay` replaces the WHOLE day core-side, so typing into
/// a day whose content hasn't loaded yet would save only the typed text over
/// unseen DB content. The editor therefore stays non-editable (selectable,
/// visually normal) until the current date's content has actually arrived —
/// observable in the shell as `editor_version` moving past the baseline
/// recorded when the date-changing event was sent.
///
/// The version arithmetic mirrors shared/src/app.rs exactly:
/// - `Startup` issues GetDay with NO immediate bump; the first `DayLoaded`
///   bumps (+1). Baseline = version at send time.
/// - `NavigateToDay`/`GoToToday` run `select_date`, which bumps once
///   immediately (the clear-the-editor render) before `DayLoaded` bumps
///   again (+2 total). Baseline = version at send time + 1.
/// `max()` keeps the baseline monotone when navigations stack up faster
/// than their clear renders arrive (renders may coalesce/reorder — see the
/// render-refetch-idempotency comment in Core.swift).
struct EditGate {
    private var baseline: UInt64 = 0

    /// True once the day's content has loaded: `editor_version` has moved
    /// past the baseline of the last date-changing send.
    func isOpen(atVersion version: UInt64) -> Bool { version > baseline }

    /// Call before sending `Startup` (initial launch or a future wake path).
    mutating func closeForStartup(currentVersion: UInt64) {
        baseline = max(baseline, currentVersion)
    }

    /// Call before sending `NavigateToDay` or `GoToToday`.
    mutating func closeForNavigation(currentVersion: UInt64) {
        baseline = max(baseline, currentVersion) + 1
    }
}
