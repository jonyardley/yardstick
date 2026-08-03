import XCTest

/// TEMPORARY probe (removed before this PR is reviewed): which interaction
/// actually selects a row in All actions? Each candidate is tried in turn and
/// the report says which produced the "N selected" control.
final class ZZSelectionProbe: UITestCase {
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
            ("outlineRow.click", { [self] in
                app.outlineRows.element(boundBy: 1).click()
            }),
        ]
        for (name, action) in candidates {
            action()
            let state = selectedText()
            log.append("\(name) -> \(state)")
            if state != "none" { break }
        }

        XCTFail(log.joined(separator: "\n"))
    }
}
