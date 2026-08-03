import XCTest

/// TEMPORARY probe (removed before this PR is reviewed): which interaction
/// actually selects a row in All actions? Each candidate is tried in turn and
/// the report says which produced the "N selected" control.
final class ZZSelectionProbe: UITestCase {
    override func setUpWithError() throws {
        try super.setUpWithError()
        // The base fixture stops at the first failure, which killed this probe
        // before it could report; a probe must survive every candidate.
        continueAfterFailure = true
    }

    func testWhichInteractionSelectsARow() {
        var log: [String] = []
        capture("Alpha")
        capture("Bravo")
        openSidebarView("all")
        app.radioButtons["None"].click()

        func selectedText() -> String {
            let hits = app.staticTexts.matching(
                NSPredicate(format: "value CONTAINS 'selected' OR label CONTAINS 'selected'"))
            return hits.count == 0 ? "none" : (hits.firstMatch.value as? String ?? "?")
        }

        let cell = app.cells.containing(.staticText, identifier: "Alpha").firstMatch
        let text = app.staticTexts["Alpha"]
        log.append("cells=\(app.cells.count) cellExists=\(cell.exists) "
            + "cellHittable=\(cell.isHittable) textHittable=\(text.isHittable)")
        log.append("cellFrame=\(cell.frame) textFrame=\(text.frame)")

        let candidates: [(String, () -> Void)] = [
            ("cell.click", { cell.click() }),
            ("cell dx=0.05", {
                cell.coordinate(withNormalizedOffset: CGVector(dx: 0.05, dy: 0.5)).click()
            }),
            ("cell dx=0.95", {
                cell.coordinate(withNormalizedOffset: CGVector(dx: 0.95, dy: 0.5)).click()
            }),
            ("title text.click", { text.click() }),
            ("outlineRow.coordinate", { [self] in
                // OutlineRow itself reported "Not hittable", so go by
                // coordinate rather than asking XCUITest to hit the element.
                app.outlineRows.element(boundBy: 1)
                    .coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
            }),
        ]
        for (name, action) in candidates {
            action()
            let state = selectedText()
            let line = "PROBE \(name) -> \(state)"
            print(line) // survives even if a later candidate blows up
            log.append(line)
            if state != "none" { break }
        }

        XCTFail("PROBE REPORT\n" + log.joined(separator: "\n"))
    }
}
