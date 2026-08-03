import XCTest
@testable import Yardstick

/// The UI-test launch seam: three environment variables redirect the app to a
/// throwaway support directory, silence the embedded MCP server, and freeze
/// "today". Parsing is pure, so the contract Tasks 2–4 depend on is proved
/// here without launching anything. Absent or invalid values must mean
/// production behaviour — a typo in CI must never quietly point the app at a
/// relative path or an unparseable date.
final class LaunchConfigTests: XCTestCase {
    func testAbsentVariablesMeanProductionBehaviour() {
        XCTAssertEqual(
            LaunchConfig.from([:]),
            LaunchConfig(supportDir: nil, mcpDisabled: false, today: nil))
    }

    func testSupportDirMustBeAbsolute() {
        XCTAssertEqual(
            LaunchConfig.from(["YARDSTICK_SUPPORT_DIR": "/tmp/yardstick-test"]).supportDir,
            "/tmp/yardstick-test")
        XCTAssertNil(
            LaunchConfig.from(["YARDSTICK_SUPPORT_DIR": "relative/path"]).supportDir)
        XCTAssertNil(LaunchConfig.from(["YARDSTICK_SUPPORT_DIR": ""]).supportDir)
    }

    func testDisableMcpIsExactlyTheStringOne() {
        XCTAssertTrue(LaunchConfig.from(["YARDSTICK_DISABLE_MCP": "1"]).mcpDisabled)
        XCTAssertFalse(LaunchConfig.from(["YARDSTICK_DISABLE_MCP": "true"]).mcpDisabled)
        XCTAssertFalse(LaunchConfig.from(["YARDSTICK_DISABLE_MCP": "0"]).mcpDisabled)
    }

    func testTodayMustParseAsIsoDate() {
        XCTAssertEqual(LaunchConfig.from(["YARDSTICK_TODAY": "2026-01-14"]).today, "2026-01-14")
        XCTAssertNil(LaunchConfig.from(["YARDSTICK_TODAY": "14/01/2026"]).today)
        XCTAssertNil(LaunchConfig.from(["YARDSTICK_TODAY": "not-a-date"]).today)
    }
}
