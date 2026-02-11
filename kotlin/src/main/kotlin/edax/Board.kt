package edax

/**
 * Othello/Reversi board representation using bitboards.
 * Each square is represented as a bit in a 64-bit integer.
 * Square 0 = A1, Square 7 = H1, Square 56 = A8, Square 63 = H8.
 */
data class Board(
    var player: ULong,
    var opponent: ULong
) {
    companion object {
        const val INIT_PLAYER: ULong = 0x0000000810000000uL   // E4, D5
        const val INIT_OPPONENT: ULong = 0x0000001008000000uL // D4, E5
        const val INIT_MOVES: ULong = 0x0000102004080000uL    // D3, C4, F5, E6

        /** Create a new board with the standard initial position. */
        fun new(): Board = Board(INIT_PLAYER, INIT_OPPONENT)

        /** Convert square index to algebraic notation. */
        fun squareToString(square: Int): String {
            val col = ('A' + (square % 8))
            val row = (square / 8) + 1
            return "$col$row"
        }
    }

    /** Pass the turn (swap player and opponent). */
    fun pass() {
        val tmp = player
        player = opponent
        opponent = tmp
    }

    /** Return the number of empty squares. */
    fun empties(): Int = 64 - (player or opponent).countOneBits()

    /** Return the number of player discs. */
    fun playerDiscCount(): Int = player.countOneBits()

    /** Return the number of opponent discs. */
    fun opponentDiscCount(): Int = opponent.countOneBits()

    /** Return the score (player discs - opponent discs). */
    fun score(): Int {
        val p = playerDiscCount()
        val o = opponentDiscCount()
        val e = empties()
        return when {
            p > o -> p - o + e
            p < o -> p - o - e
            else -> 0
        }
    }

    /**
     * Get the color at a specific square.
     * Returns: 0 = player, 1 = opponent, 2 = empty
     */
    fun getSquareColor(square: Int): Int {
        val mask = 1uL shl square
        return when {
            (player and mask) != 0uL -> 0
            (opponent and mask) != 0uL -> 1
            else -> 2
        }
    }

    /** Get all legal moves for the current player. */
    fun getMoves(): ULong = getMoves(player, opponent)

    /** Get legal moves using the parallel prefix algorithm. */
    fun getMoves(P: ULong, O: ULong): ULong {
        val mask = O and 0x7E7E7E7E7E7E7E7EuL
        var moves: ULong = 0uL
        val empty = (P or O).inv()

        // Right direction (+1)
        var flip = mask and (P shl 1)
        flip = flip or (mask and (flip shl 1))
        flip = flip or (mask and (flip shl 1))
        flip = flip or (mask and (flip shl 1))
        flip = flip or (mask and (flip shl 1))
        flip = flip or (mask and (flip shl 1))
        moves = moves or (flip shl 1)

        // Left direction (-1)
        flip = mask and (P shr 1)
        flip = flip or (mask and (flip shr 1))
        flip = flip or (mask and (flip shr 1))
        flip = flip or (mask and (flip shr 1))
        flip = flip or (mask and (flip shr 1))
        flip = flip or (mask and (flip shr 1))
        moves = moves or (flip shr 1)

        // Down direction (+8)
        flip = O and (P shl 8)
        flip = flip or (O and (flip shl 8))
        flip = flip or (O and (flip shl 8))
        flip = flip or (O and (flip shl 8))
        flip = flip or (O and (flip shl 8))
        flip = flip or (O and (flip shl 8))
        moves = moves or (flip shl 8)

        // Up direction (-8)
        flip = O and (P shr 8)
        flip = flip or (O and (flip shr 8))
        flip = flip or (O and (flip shr 8))
        flip = flip or (O and (flip shr 8))
        flip = flip or (O and (flip shr 8))
        flip = flip or (O and (flip shr 8))
        moves = moves or (flip shr 8)

        // Down-right direction (+9)
        flip = mask and (P shl 9)
        flip = flip or (mask and (flip shl 9))
        flip = flip or (mask and (flip shl 9))
        flip = flip or (mask and (flip shl 9))
        flip = flip or (mask and (flip shl 9))
        flip = flip or (mask and (flip shl 9))
        moves = moves or (flip shl 9)

        // Up-left direction (-9)
        flip = mask and (P shr 9)
        flip = flip or (mask and (flip shr 9))
        flip = flip or (mask and (flip shr 9))
        flip = flip or (mask and (flip shr 9))
        flip = flip or (mask and (flip shr 9))
        flip = flip or (mask and (flip shr 9))
        moves = moves or (flip shr 9)

        // Down-left direction (+7)
        flip = mask and (P shl 7)
        flip = flip or (mask and (flip shl 7))
        flip = flip or (mask and (flip shl 7))
        flip = flip or (mask and (flip shl 7))
        flip = flip or (mask and (flip shl 7))
        flip = flip or (mask and (flip shl 7))
        moves = moves or (flip shl 7)

        // Up-right direction (-7)
        flip = mask and (P shr 7)
        flip = flip or (mask and (flip shr 7))
        flip = flip or (mask and (flip shr 7))
        flip = flip or (mask and (flip shr 7))
        flip = flip or (mask and (flip shr 7))
        flip = flip or (mask and (flip shr 7))
        moves = moves or (flip shr 7)

        return moves and empty
    }

    /** Calculate the discs that would be flipped by placing at `square`. */
    fun flip(square: Int): ULong = flipDisc(square, player, opponent)

    /** Flip calculation implementation. */
    fun flipDisc(square: Int, P: ULong, O: ULong): ULong {
        val move = 1uL shl square
        var totalFlip: ULong = 0uL

        val directions = intArrayOf(1, -1, 8, -8, 9, -9, 7, -7)
        val masks = ulongArrayOf(
            0x7E7E7E7E7E7E7E7EuL, 0x7E7E7E7E7E7E7E7EuL,
            0xFFFFFFFFFFFFFFFFuL, 0xFFFFFFFFFFFFFFFFuL,
            0x7E7E7E7E7E7E7E7EuL, 0x7E7E7E7E7E7E7E7EuL,
            0x7E7E7E7E7E7E7E7EuL, 0x7E7E7E7E7E7E7E7EuL
        )

        for (i in 0 until 8) {
            val m = masks[i] and O
            val d = directions[i]
            var f: ULong = 0uL
            var x = move

            if (d > 0) {
                x = x shl d
                while ((x and m) != 0uL) {
                    f = f or x
                    x = x shl d
                }
                if ((x and P) != 0uL) totalFlip = totalFlip or f
            } else {
                val nd = -d
                x = x shr nd
                while ((x and m) != 0uL) {
                    f = f or x
                    x = x shr nd
                }
                if ((x and P) != 0uL) totalFlip = totalFlip or f
            }
        }

        return totalFlip
    }

    /** Play a move at the given square. Returns the flipped discs for undo. */
    fun doMove(square: Int): ULong {
        val flipped = flip(square)
        player = player or flipped or (1uL shl square)
        opponent = opponent xor flipped
        pass()
        return flipped
    }

    /** Undo a move. */
    fun undoMove(square: Int, flipped: ULong) {
        pass()
        player = player xor flipped xor (1uL shl square)
        opponent = opponent xor flipped
    }

    /** Check if the game is over (neither player can move). */
    fun isGameOver(): Boolean {
        if (getMoves() != 0uL) return false
        val passed = copy()
        passed.pass()
        return passed.getMoves() == 0uL
    }

    /** Check if the current player must pass. */
    fun isPass(): Boolean = getMoves() == 0uL

    /** Get mobility (number of legal moves). */
    fun mobility(): Int = getMoves().countOneBits()
}
