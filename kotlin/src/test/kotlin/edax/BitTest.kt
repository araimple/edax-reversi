package edax

import kotlin.test.*

class BitTest {

    // MARK: - Bit Count Tests

    @Test
    fun testBitCount() {
        assertEquals(0, 0uL.countOneBits(), "popcount(0) = 0")
        assertEquals(1, 1uL.countOneBits(), "popcount(1) = 1")
        assertEquals(64, 0xFFFFFFFFFFFFFFFFuL.countOneBits(), "popcount(all 1s) = 64")
        assertEquals(32, 0xAAAAAAAAAAAAAAAAuL.countOneBits(), "popcount(alternating) = 32")
        assertEquals(4, (Board.INIT_PLAYER or Board.INIT_OPPONENT).countOneBits(), "initial discs = 4")
    }

    // MARK: - First Bit Tests

    @Test
    fun testFirstBit() {
        assertEquals(0, firstBit(1uL), "first_bit(1) = 0")
        assertEquals(63, firstBit(0x8000000000000000uL), "first_bit(high) = 63")
        assertEquals(4, firstBit(0x10uL), "first_bit(0x10) = 4")
        assertEquals(0, firstBit(0xFFFFFFFFFFFFFFFFuL), "first_bit(all) = 0")
        assertEquals(19, firstBit(Board.INIT_MOVES), "first_bit(INIT_MOVES) = 19")
    }

    // MARK: - Transpose Tests

    @Test
    fun testTranspose() {
        assertEquals(1uL, transpose(1uL), "transpose A1 = A1")
        assertEquals(1uL shl 56, transpose(1uL shl 7), "transpose H1 = A8")
        assertEquals(1uL shl 7, transpose(1uL shl 56), "transpose A8 = H1")
        assertEquals(1uL shl 63, transpose(1uL shl 63), "transpose H8 = H8")

        // Double transpose is identity
        val test = Board.INIT_PLAYER
        assertEquals(test, transpose(transpose(test)), "double transpose = identity")
    }

    // MARK: - Horizontal Mirror Tests

    @Test
    fun testHorizontalMirror() {
        assertEquals(1uL shl 7, horizontalMirror(1uL), "H-mirror A1 = H1")
        assertEquals(1uL, horizontalMirror(1uL shl 7), "H-mirror H1 = A1")

        // Double mirror is identity
        val test = Board.INIT_PLAYER
        assertEquals(test, horizontalMirror(horizontalMirror(test)), "double H-mirror = identity")

        // Initial player H-mirrored equals opponent
        val board = Board.new()
        assertEquals(board.opponent, horizontalMirror(board.player), "H-mirror player = opponent")
    }

    // MARK: - Vertical Mirror Tests

    @Test
    fun testVerticalMirror() {
        assertEquals(1uL shl 56, verticalMirror(1uL), "V-mirror A1 = A8")
        assertEquals(1uL shl 63, verticalMirror(1uL shl 7), "V-mirror H1 = H8")

        // Double mirror is identity
        val test = Board.INIT_PLAYER
        assertEquals(test, verticalMirror(verticalMirror(test)), "double V-mirror = identity")
    }
}
