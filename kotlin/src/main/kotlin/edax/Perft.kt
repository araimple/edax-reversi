package edax

/** Performance test - count leaf nodes by expanding all legal moves to a given depth. */
fun perft(board: Board, depth: Int): ULong {
    if (depth == 0) return 1uL

    val moves = board.getMoves()

    if (moves == 0uL) {
        val passed = board.copy()
        passed.pass()
        if (passed.getMoves() == 0uL) {
            // Game over
            return 1uL
        }
        return perft(passed, depth - 1)
    }

    var count: ULong = 0uL
    var remaining = moves

    while (remaining != 0uL) {
        val sq = remaining.countTrailingZeroBits()
        remaining = remaining and (remaining - 1uL)

        val child = board.copy()
        child.doMove(sq)
        count += perft(child, depth - 1)
    }

    return count
}

/** Expected perft results for the initial position. */
val PERFT_EXPECTED = ulongArrayOf(
    1uL, 4uL, 12uL, 56uL, 244uL, 1396uL, 8200uL, 55092uL, 390216uL
)
