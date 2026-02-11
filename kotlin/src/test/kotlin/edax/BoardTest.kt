package edax

import kotlin.test.*

class BoardTest {

    // MARK: - Board Initialization Tests

    @Test
    fun testBoardInit() {
        val board = Board.new()
        assertEquals(Board.INIT_PLAYER, board.player, "initial player bitboard")
        assertEquals(Board.INIT_OPPONENT, board.opponent, "initial opponent bitboard")
        assertEquals(2, board.playerDiscCount(), "player disc count")
        assertEquals(2, board.opponentDiscCount(), "opponent disc count")
        assertEquals(60, board.empties(), "empty count")
    }

    @Test
    fun testBoardPass() {
        val board = Board.new()
        val origPlayer = board.player
        val origOpponent = board.opponent
        board.pass()
        assertEquals(origOpponent, board.player, "pass swaps player")
        assertEquals(origPlayer, board.opponent, "pass swaps opponent")
    }

    @Test
    fun testBoardCountEmpties() {
        var board = Board.new()
        assertEquals(60, board.empties(), "initial has 60 empties")

        board = Board(0xFFFFFFFFFFFFFFFFuL, 0uL)
        assertEquals(0, board.empties(), "full player has 0 empties")

        board = Board(0uL, 0uL)
        assertEquals(64, board.empties(), "empty board has 64 empties")
    }

    @Test
    fun testBoardGetSquareColor() {
        val board = Board.new()
        assertEquals(0, board.getSquareColor(28), "E4 is player (0)")
        assertEquals(0, board.getSquareColor(35), "D5 is player (0)")
        assertEquals(1, board.getSquareColor(27), "D4 is opponent (1)")
        assertEquals(1, board.getSquareColor(36), "E5 is opponent (1)")
        assertEquals(2, board.getSquareColor(0), "A1 is empty (2)")
    }

    // MARK: - Get Moves Tests

    @Test
    fun testGetMovesInitial() {
        val board = Board.new()
        val moves = board.getMoves()
        assertEquals(4, moves.countOneBits(), "initial position has 4 moves")
        assertEquals(Board.INIT_MOVES, moves, "initial legal moves")
        assertNotEquals(0uL, moves and (1uL shl 19), "D3 is legal")
        assertNotEquals(0uL, moves and (1uL shl 26), "C4 is legal")
        assertNotEquals(0uL, moves and (1uL shl 37), "F5 is legal")
        assertNotEquals(0uL, moves and (1uL shl 44), "E6 is legal")
    }

    @Test
    fun testGetMovesNoMoves() {
        val board = Board(1uL, 0uL)
        val moves = board.getMoves()
        assertEquals(0uL, moves, "no opponent means no moves")
    }

    @Test
    fun testGetMovesNoHorizontalWrap() {
        val board = Board(1uL shl 31, 1uL shl 24)  // H4 and A4
        val moves = board.getMoves()
        assertEquals(0uL, moves and 0xFF000000uL, "no horizontal wrap in row 4")
    }

    @Test
    fun testGetMovesAll8Directions() {
        // Player at D4 center, opponent surrounding
        val player: ULong = 1uL shl 27
        val opponent: ULong = (1uL shl 18) or (1uL shl 19) or (1uL shl 20) or
                              (1uL shl 26) or (1uL shl 28) or
                              (1uL shl 34) or (1uL shl 35) or (1uL shl 36)
        val board = Board(player, opponent)
        val moves = board.getMoves()

        assertNotEquals(0uL, moves and (1uL shl 9), "B2 (NW) is legal")
        assertNotEquals(0uL, moves and (1uL shl 11), "D2 (N) is legal")
        assertNotEquals(0uL, moves and (1uL shl 13), "F2 (NE) is legal")
        assertNotEquals(0uL, moves and (1uL shl 25), "B4 (W) is legal")
        assertNotEquals(0uL, moves and (1uL shl 29), "F4 (E) is legal")
        assertNotEquals(0uL, moves and (1uL shl 41), "B6 (SW) is legal")
        assertNotEquals(0uL, moves and (1uL shl 43), "D6 (S) is legal")
        assertNotEquals(0uL, moves and (1uL shl 45), "F6 (SE) is legal")
    }

    // MARK: - Flip Tests

    @Test
    fun testFlipD3() {
        val board = Board.new()
        val flipped = board.flip(19)
        assertEquals(1uL shl 27, flipped, "D3 flips D4")
        assertEquals(1, flipped.countOneBits(), "D3 flips 1 disc")
    }

    @Test
    fun testFlipC4() {
        val board = Board.new()
        val flipped = board.flip(26)
        assertEquals(1uL shl 27, flipped, "C4 flips D4")
        assertEquals(1, flipped.countOneBits(), "C4 flips 1 disc")
    }

    @Test
    fun testFlipF5() {
        val board = Board.new()
        val flipped = board.flip(37)
        assertEquals(1uL shl 36, flipped, "F5 flips E5")
        assertEquals(1, flipped.countOneBits(), "F5 flips 1 disc")
    }

    @Test
    fun testFlipE6() {
        val board = Board.new()
        val flipped = board.flip(44)
        assertEquals(1uL shl 36, flipped, "E6 flips E5")
        assertEquals(1, flipped.countOneBits(), "E6 flips 1 disc")
    }

    @Test
    fun testFlipNoFlip() {
        val board = Board.new()
        val flipped = board.flip(0)
        assertEquals(0uL, flipped, "A1 is illegal, no flips")
    }

    @Test
    fun testFlipMultipleDirections() {
        // Player at E1, opponent E2-E7
        val player: ULong = 1uL shl 4
        val opponent: ULong = (1uL shl 12) or (1uL shl 20) or (1uL shl 28) or
                              (1uL shl 36) or (1uL shl 44) or (1uL shl 52)
        val board = Board(player, opponent)
        val flipped = board.flip(60)  // E8
        assertEquals(6, flipped.countOneBits(), "E8 flips 6 discs")
    }

    @Test
    fun testFlipLongDiagonal() {
        // Player at A1, opponent B2-G7
        val player: ULong = 1uL
        val opponent: ULong = (1uL shl 9) or (1uL shl 18) or (1uL shl 27) or
                              (1uL shl 36) or (1uL shl 45) or (1uL shl 54)
        val board = Board(player, opponent)
        val flipped = board.flip(63)  // H8
        assertEquals(6, flipped.countOneBits(), "H8 flips 6 discs diagonally")
    }

    // MARK: - Game State Tests

    @Test
    fun testBoardIsPass() {
        var board = Board.new()
        assertFalse(board.isPass(), "initial position is not pass")

        board = Board(1uL, 1uL shl 63)  // isolated corners
        assertTrue(board.isPass(), "player must pass with isolated corners")
    }

    @Test
    fun testBoardIsGameOver() {
        var board = Board.new()
        assertFalse(board.isGameOver(), "initial position is not game over")

        board = Board(0x00000000FFFFFFFFuL, 0xFFFFFFFF00000000uL)
        assertTrue(board.isGameOver(), "full board is game over")

        board = Board(1uL, 1uL shl 63)
        assertTrue(board.isGameOver(), "isolated corners is game over")
    }

    @Test
    fun testBoardScore() {
        var board = Board.new()
        assertEquals(0, board.score(), "initial score is 0")

        board = Board(0xFFFFFFFFFFFFFFFFuL, 0uL)
        assertEquals(64, board.score(), "all player = +64")

        board = Board(0uL, 0xFFFFFFFFFFFFFFFFuL)
        assertEquals(-64, board.score(), "all opponent = -64")

        board = Board(0x00000000FFFFFFFFuL, 0xFFFFFFFF00000000uL)
        assertEquals(0, board.score(), "equal = 0")
    }

    @Test
    fun testMobility() {
        var board = Board.new()
        assertEquals(4, board.mobility(), "initial mobility = 4")

        board.doMove(19)  // D3
        assertEquals(3, board.mobility(), "after D3, opponent has 3 moves")
    }

    // MARK: - Move Sequence Tests

    @Test
    fun testMoveSequence() {
        val board = Board.new()
        val moves = intArrayOf(19, 18, 26, 34)  // D3, C3, C4, C5

        for (sq in moves) {
            val legal = board.getMoves()
            assertNotEquals(0uL, legal and (1uL shl sq), "move ${Board.squareToString(sq)} is legal")
            board.doMove(sq)
        }

        val total = board.playerDiscCount() + board.opponentDiscCount()
        assertEquals(8, total, "8 discs after 4 moves")
    }

    @Test
    fun testDoMoveUndoMove() {
        val board = Board.new()
        val original = board.copy()
        val flipped = board.doMove(19)  // D3
        assertNotEquals(original, board, "board should change after move")

        board.undoMove(19, flipped)
        assertEquals(original, board, "board should be restored after undo")
    }

    // MARK: - Square to String Tests

    @Test
    fun testSquareToString() {
        assertEquals("A1", Board.squareToString(0))
        assertEquals("D3", Board.squareToString(19))
        assertEquals("H8", Board.squareToString(63))
    }
}
