package edax

import kotlin.test.*

class PerftTest {

    @Test
    fun testPerftDepth0() {
        val board = Board.new()
        assertEquals(1uL, perft(board, 0))
    }

    @Test
    fun testPerftDepth1() {
        val board = Board.new()
        assertEquals(4uL, perft(board, 1))
    }

    @Test
    fun testPerftDepth2() {
        val board = Board.new()
        assertEquals(12uL, perft(board, 2))
    }

    @Test
    fun testPerftDepth3() {
        val board = Board.new()
        assertEquals(56uL, perft(board, 3))
    }

    @Test
    fun testPerftDepth4() {
        val board = Board.new()
        assertEquals(244uL, perft(board, 4))
    }

    @Test
    fun testPerftDepth5() {
        val board = Board.new()
        assertEquals(1396uL, perft(board, 5))
    }

    @Test
    fun testPerftDepth6() {
        val board = Board.new()
        assertEquals(8200uL, perft(board, 6))
    }

    // Depth 7 and 8 are slower but verify correctness
    @Test
    fun testPerftDepth7() {
        val board = Board.new()
        assertEquals(55092uL, perft(board, 7))
    }

    @Test
    fun testPerftDepth8() {
        val board = Board.new()
        assertEquals(390216uL, perft(board, 8))
    }
}
