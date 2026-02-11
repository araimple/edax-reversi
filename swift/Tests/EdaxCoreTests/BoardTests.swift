import XCTest
@testable import EdaxCore

final class BoardTests: XCTestCase {

    // MARK: - Board Initialization Tests

    func testBoardInit() {
        let board = Board()
        XCTAssertEqual(board.player, Board.initPlayer, "initial player bitboard")
        XCTAssertEqual(board.opponent, Board.initOpponent, "initial opponent bitboard")
        XCTAssertEqual(board.playerDiscCount(), 2, "player disc count")
        XCTAssertEqual(board.opponentDiscCount(), 2, "opponent disc count")
        XCTAssertEqual(board.empties(), 60, "empty count")
    }

    func testBoardPass() {
        var board = Board()
        let origPlayer = board.player
        let origOpponent = board.opponent
        board.pass()
        XCTAssertEqual(board.player, origOpponent, "pass swaps player")
        XCTAssertEqual(board.opponent, origPlayer, "pass swaps opponent")
    }

    func testBoardCountEmpties() {
        var board = Board()
        XCTAssertEqual(board.empties(), 60, "initial has 60 empties")

        board = Board(player: 0xFFFFFFFFFFFFFFFF, opponent: 0)
        XCTAssertEqual(board.empties(), 0, "full player has 0 empties")

        board = Board(player: 0, opponent: 0)
        XCTAssertEqual(board.empties(), 64, "empty board has 64 empties")
    }

    func testBoardGetSquareColor() {
        let board = Board()
        XCTAssertEqual(board.getSquareColor(28), 0, "E4 is player (0)")
        XCTAssertEqual(board.getSquareColor(35), 0, "D5 is player (0)")
        XCTAssertEqual(board.getSquareColor(27), 1, "D4 is opponent (1)")
        XCTAssertEqual(board.getSquareColor(36), 1, "E5 is opponent (1)")
        XCTAssertEqual(board.getSquareColor(0), 2, "A1 is empty (2)")
    }

    // MARK: - Get Moves Tests

    func testGetMovesInitial() {
        let board = Board()
        let moves = board.getMoves()
        XCTAssertEqual(moves.nonzeroBitCount, 4, "initial position has 4 moves")
        XCTAssertEqual(moves, Board.initMoves, "initial legal moves")
        XCTAssertNotEqual(moves & (1 << 19), 0, "D3 is legal")
        XCTAssertNotEqual(moves & (1 << 26), 0, "C4 is legal")
        XCTAssertNotEqual(moves & (1 << 37), 0, "F5 is legal")
        XCTAssertNotEqual(moves & (1 << 44), 0, "E6 is legal")
    }

    func testGetMovesNoMoves() {
        let board = Board(player: 1, opponent: 0)
        let moves = board.getMoves()
        XCTAssertEqual(moves, 0, "no opponent means no moves")
    }

    func testGetMovesNoHorizontalWrap() {
        let board = Board(player: 1 << 31, opponent: 1 << 24)  // H4 and A4
        let moves = board.getMoves()
        XCTAssertEqual(moves & 0xFF000000, 0, "no horizontal wrap in row 4")
    }

    func testGetMovesAll8Directions() {
        // Player at D4 center, opponent surrounding
        let player: UInt64 = 1 << 27
        let opponent: UInt64 = (1 << 18) | (1 << 19) | (1 << 20) |
                               (1 << 26) | (1 << 28) |
                               (1 << 34) | (1 << 35) | (1 << 36)
        let board = Board(player: player, opponent: opponent)
        let moves = board.getMoves()

        XCTAssertNotEqual(moves & (1 << 9), 0, "B2 (NW) is legal")
        XCTAssertNotEqual(moves & (1 << 11), 0, "D2 (N) is legal")
        XCTAssertNotEqual(moves & (1 << 13), 0, "F2 (NE) is legal")
        XCTAssertNotEqual(moves & (1 << 25), 0, "B4 (W) is legal")
        XCTAssertNotEqual(moves & (1 << 29), 0, "F4 (E) is legal")
        XCTAssertNotEqual(moves & (1 << 41), 0, "B6 (SW) is legal")
        XCTAssertNotEqual(moves & (1 << 43), 0, "D6 (S) is legal")
        XCTAssertNotEqual(moves & (1 << 45), 0, "F6 (SE) is legal")
    }

    // MARK: - Flip Tests

    func testFlipD3() {
        let board = Board()
        let flipped = board.flip(19)
        XCTAssertEqual(flipped, 1 << 27, "D3 flips D4")
        XCTAssertEqual(flipped.nonzeroBitCount, 1, "D3 flips 1 disc")
    }

    func testFlipC4() {
        let board = Board()
        let flipped = board.flip(26)
        XCTAssertEqual(flipped, 1 << 27, "C4 flips D4")
        XCTAssertEqual(flipped.nonzeroBitCount, 1, "C4 flips 1 disc")
    }

    func testFlipF5() {
        let board = Board()
        let flipped = board.flip(37)
        XCTAssertEqual(flipped, 1 << 36, "F5 flips E5")
        XCTAssertEqual(flipped.nonzeroBitCount, 1, "F5 flips 1 disc")
    }

    func testFlipE6() {
        let board = Board()
        let flipped = board.flip(44)
        XCTAssertEqual(flipped, 1 << 36, "E6 flips E5")
        XCTAssertEqual(flipped.nonzeroBitCount, 1, "E6 flips 1 disc")
    }

    func testFlipNoFlip() {
        let board = Board()
        let flipped = board.flip(0)
        XCTAssertEqual(flipped, 0, "A1 is illegal, no flips")
    }

    func testFlipMultipleDirections() {
        // Player at E1, opponent E2-E7
        let player: UInt64 = 1 << 4
        let opponent: UInt64 = (1 << 12) | (1 << 20) | (1 << 28) |
                               (1 << 36) | (1 << 44) | (1 << 52)
        let board = Board(player: player, opponent: opponent)
        let flipped = board.flip(60)  // E8
        XCTAssertEqual(flipped.nonzeroBitCount, 6, "E8 flips 6 discs")
    }

    func testFlipLongDiagonal() {
        // Player at A1, opponent B2-G7
        let player: UInt64 = 1
        let opponent: UInt64 = (1 << 9) | (1 << 18) | (1 << 27) |
                               (1 << 36) | (1 << 45) | (1 << 54)
        let board = Board(player: player, opponent: opponent)
        let flipped = board.flip(63)  // H8
        XCTAssertEqual(flipped.nonzeroBitCount, 6, "H8 flips 6 discs diagonally")
    }

    // MARK: - Game State Tests

    func testBoardIsPass() {
        var board = Board()
        XCTAssertFalse(board.isPass(), "initial position is not pass")

        board = Board(player: 1, opponent: 1 << 63)  // isolated corners
        XCTAssertTrue(board.isPass(), "player must pass with isolated corners")
    }

    func testBoardIsGameOver() {
        var board = Board()
        XCTAssertFalse(board.isGameOver(), "initial position is not game over")

        board = Board(player: 0x00000000FFFFFFFF, opponent: 0xFFFFFFFF00000000)
        XCTAssertTrue(board.isGameOver(), "full board is game over")

        board = Board(player: 1, opponent: 1 << 63)
        XCTAssertTrue(board.isGameOver(), "isolated corners is game over")
    }

    func testBoardScore() {
        var board = Board()
        XCTAssertEqual(board.score(), 0, "initial score is 0")

        board = Board(player: 0xFFFFFFFFFFFFFFFF, opponent: 0)
        XCTAssertEqual(board.score(), 64, "all player = +64")

        board = Board(player: 0, opponent: 0xFFFFFFFFFFFFFFFF)
        XCTAssertEqual(board.score(), -64, "all opponent = -64")

        board = Board(player: 0x00000000FFFFFFFF, opponent: 0xFFFFFFFF00000000)
        XCTAssertEqual(board.score(), 0, "equal = 0")
    }

    func testMobility() {
        var board = Board()
        XCTAssertEqual(board.mobility(), 4, "initial mobility = 4")

        board.doMove(19)  // D3
        XCTAssertEqual(board.mobility(), 3, "after D3, opponent has 3 moves")
    }

    // MARK: - Move Sequence Tests

    func testMoveSequence() {
        var board = Board()
        let moves = [19, 18, 26, 34]  // D3, C3, C4, C5

        for sq in moves {
            let legal = board.getMoves()
            XCTAssertNotEqual(legal & (1 << sq), 0, "move \(Board.squareToString(sq)) is legal")
            board.doMove(sq)
        }

        let total = board.playerDiscCount() + board.opponentDiscCount()
        XCTAssertEqual(total, 8, "8 discs after 4 moves")
    }

    func testDoMoveUndoMove() {
        var board = Board()
        let original = board
        let flipped = board.doMove(19)  // D3
        XCTAssertNotEqual(board, original, "board should change after move")

        board.undoMove(19, flipped: flipped)
        XCTAssertEqual(board, original, "board should be restored after undo")
    }

    // MARK: - Square to String Tests

    func testSquareToString() {
        XCTAssertEqual(Board.squareToString(0), "A1")
        XCTAssertEqual(Board.squareToString(19), "D3")
        XCTAssertEqual(Board.squareToString(63), "H8")
    }
}
