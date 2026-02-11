import XCTest
@testable import EdaxCore

final class PerftTests: XCTestCase {

    func testPerftDepth0() {
        let board = Board()
        XCTAssertEqual(perft(board, depth: 0), 1)
    }

    func testPerftDepth1() {
        let board = Board()
        XCTAssertEqual(perft(board, depth: 1), 4)
    }

    func testPerftDepth2() {
        let board = Board()
        XCTAssertEqual(perft(board, depth: 2), 12)
    }

    func testPerftDepth3() {
        let board = Board()
        XCTAssertEqual(perft(board, depth: 3), 56)
    }

    func testPerftDepth4() {
        let board = Board()
        XCTAssertEqual(perft(board, depth: 4), 244)
    }

    func testPerftDepth5() {
        let board = Board()
        XCTAssertEqual(perft(board, depth: 5), 1396)
    }

    func testPerftDepth6() {
        let board = Board()
        XCTAssertEqual(perft(board, depth: 6), 8200)
    }

    // Depth 7 and 8 are slower but verify correctness
    func testPerftDepth7() {
        let board = Board()
        XCTAssertEqual(perft(board, depth: 7), 55092)
    }

    func testPerftDepth8() {
        let board = Board()
        XCTAssertEqual(perft(board, depth: 8), 390216)
    }
}
