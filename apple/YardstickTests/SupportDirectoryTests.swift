import XCTest
@testable import Yardstick

/// The product was renamed Daily -> Yardstick (spec §12 Q1), which moves the
/// application-support directory holding `daily.db` and `mcp-token`. A launch
/// that finds only the old directory must carry it over, or the rename would
/// silently orphan every note written before it.
final class SupportDirectoryTests: XCTestCase {
    private var parent: URL!

    override func setUpWithError() throws {
        parent = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("SupportDirectoryTests-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: parent)
    }

    private func write(_ contents: String, to url: URL) throws {
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try contents.write(to: url, atomically: true, encoding: .utf8)
    }

    func testLegacyDirectoryIsCarriedOverWithItsContents() throws {
        try write("notes", to: parent.appendingPathComponent("Daily/daily.db"))
        try write("secret", to: parent.appendingPathComponent("Daily/mcp-token"))

        let url = SupportDirectory.url(in: parent)

        XCTAssertEqual(url.path, parent.appendingPathComponent("Yardstick").path)
        XCTAssertEqual(
            try String(contentsOf: url.appendingPathComponent("daily.db"), encoding: .utf8),
            "notes", "the database must arrive under the new name")
        XCTAssertEqual(
            try String(contentsOf: url.appendingPathComponent("mcp-token"), encoding: .utf8),
            "secret", "the MCP token must survive, or every client config breaks")
        XCTAssertFalse(
            FileManager.default.fileExists(atPath: parent.appendingPathComponent("Daily").path),
            "the legacy directory is moved, not copied — two databases would diverge")
    }

    func testExistingYardstickDirectoryWins() throws {
        try write("old", to: parent.appendingPathComponent("Daily/daily.db"))
        try write("current", to: parent.appendingPathComponent("Yardstick/daily.db"))

        let url = SupportDirectory.url(in: parent)

        XCTAssertEqual(
            try String(contentsOf: url.appendingPathComponent("daily.db"), encoding: .utf8),
            "current", "a live database is never clobbered by the legacy one")
        XCTAssertTrue(
            FileManager.default.fileExists(atPath: parent.appendingPathComponent("Daily").path),
            "the legacy directory is left in place for the user to inspect or delete")
    }

    func testFreshInstallCreatesTheDirectory() throws {
        let url = SupportDirectory.url(in: parent)

        XCTAssertEqual(url.path, parent.appendingPathComponent("Yardstick").path)
        var isDirectory: ObjCBool = false
        XCTAssertTrue(
            FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory),
            "a fresh install still gets its directory")
        XCTAssertTrue(isDirectory.boolValue)
    }
}
