import XCTest
@testable import EdaxCore

final class BitTests: XCTestCase {

    // MARK: - Bit Count Tests

    func testBitCount() {
        XCTAssertEqual(UInt64(0).nonzeroBitCount, 0, "popcount(0) = 0")
        XCTAssertEqual(UInt64(1).nonzeroBitCount, 1, "popcount(1) = 1")
        XCTAssertEqual(UInt64(0xFFFFFFFFFFFFFFFF).nonzeroBitCount, 64, "popcount(all 1s) = 64")
        XCTAssertEqual(UInt64(0xAAAAAAAAAAAAAAAA).nonzeroBitCount, 32, "popcount(alternating) = 32")
        XCTAssertEqual((Board.initPlayer | Board.initOpponent).nonzeroBitCount, 4, "initial discs = 4")
    }

    // MARK: - First Bit Tests

    func testFirstBit() {
        XCTAssertEqual(firstBit(1), 0, "first_bit(1) = 0")
        XCTAssertEqual(firstBit(0x8000000000000000), 63, "first_bit(high) = 63")
        XCTAssertEqual(firstBit(0x10), 4, "first_bit(0x10) = 4")
        XCTAssertEqual(firstBit(0xFFFFFFFFFFFFFFFF), 0, "first_bit(all) = 0")
        XCTAssertEqual(firstBit(Board.initMoves), 19, "first_bit(INIT_MOVES) = 19")
    }

    // MARK: - Transpose Tests

    func testTranspose() {
        XCTAssertEqual(transpose(1), 1, "transpose A1 = A1")
        XCTAssertEqual(transpose(1 << 7), UInt64(1) << 56, "transpose H1 = A8")
        XCTAssertEqual(transpose(1 << 56), UInt64(1) << 7, "transpose A8 = H1")
        XCTAssertEqual(transpose(1 << 63), UInt64(1) << 63, "transpose H8 = H8")

        // Double transpose is identity
        let test = Board.initPlayer
        XCTAssertEqual(transpose(transpose(test)), test, "double transpose = identity")
    }

    // MARK: - Horizontal Mirror Tests

    func testHorizontalMirror() {
        XCTAssertEqual(horizontalMirror(1), UInt64(1) << 7, "H-mirror A1 = H1")
        XCTAssertEqual(horizontalMirror(1 << 7), 1, "H-mirror H1 = A1")

        // Double mirror is identity
        let test = Board.initPlayer
        XCTAssertEqual(horizontalMirror(horizontalMirror(test)), test, "double H-mirror = identity")

        // Initial player H-mirrored equals opponent
        let board = Board()
        XCTAssertEqual(horizontalMirror(board.player), board.opponent, "H-mirror player = opponent")
    }

    // MARK: - Vertical Mirror Tests

    func testVerticalMirror() {
        XCTAssertEqual(verticalMirror(1), UInt64(1) << 56, "V-mirror A1 = A8")
        XCTAssertEqual(verticalMirror(1 << 7), UInt64(1) << 63, "V-mirror H1 = H8")

        // Double mirror is identity
        let test = Board.initPlayer
        XCTAssertEqual(verticalMirror(verticalMirror(test)), test, "double V-mirror = identity")
    }
}
