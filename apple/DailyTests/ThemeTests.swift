import XCTest
@testable import Daily
import SwiftUI

final class ThemeTests: XCTestCase {
    private func srgb(_ color: Color) -> (r: CGFloat, g: CGFloat, b: CGFloat) {
        let ns = NSColor(color).usingColorSpace(.sRGB)!
        return (ns.redComponent, ns.greenComponent, ns.blueComponent)
    }

    func testOklchAchromaticEndpoints() {
        // oklch(1 0 h) is white and oklch(0 0 h) is black for any hue.
        let white = srgb(Color(oklch: 1.0, 0.0, 123))
        XCTAssertEqual(white.r, 1.0, accuracy: 0.01)
        XCTAssertEqual(white.g, 1.0, accuracy: 0.01)
        XCTAssertEqual(white.b, 1.0, accuracy: 0.01)

        let black = srgb(Color(oklch: 0.0, 0.0, 0))
        XCTAssertEqual(black.r, 0.0, accuracy: 0.01)
        XCTAssertEqual(black.g, 0.0, accuracy: 0.01)
        XCTAssertEqual(black.b, 0.0, accuracy: 0.01)
    }

    func testAccentBlueIsActuallyBlue() {
        // oklch(0.62 0.13 250): blue-dominant, red-recessive — ordering is a
        // robust invariant without golden values.
        let c = srgb(Theme.accent)
        XCTAssertGreaterThan(c.b, c.g)
        XCTAssertGreaterThan(c.g, c.r)
    }

    func testPriority1RedIsActuallyRed() {
        let c = srgb(Theme.priority1)  // oklch(0.6 0.16 25)
        XCTAssertGreaterThan(c.r, c.g)
        XCTAssertGreaterThan(c.r, c.b)
    }

    func testHexTokensDecodeExactly() {
        let sidebar = srgb(Theme.sidebarBg)  // #edecea
        XCTAssertEqual(sidebar.r, 0xED / 255.0, accuracy: 0.005)
        XCTAssertEqual(sidebar.g, 0xEC / 255.0, accuracy: 0.005)
        XCTAssertEqual(sidebar.b, 0xEA / 255.0, accuracy: 0.005)
    }

    func testMetricsMatchTheReference() {
        XCTAssertEqual(Theme.Metrics.sidebarWidth, 238)
        XCTAssertEqual(Theme.Metrics.contentMaxWidth, 760)
        XCTAssertEqual(Theme.Metrics.noteMaxWidth, 640)
    }
}
