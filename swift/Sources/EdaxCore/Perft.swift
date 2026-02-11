/// Performance test - count leaf nodes by expanding all legal moves to a given depth.
public func perft(_ board: Board, depth: Int) -> UInt64 {
    if depth == 0 { return 1 }

    let moves = board.getMoves()

    if moves == 0 {
        var passed = board
        passed.pass()
        if passed.getMoves() == 0 {
            // Game over
            return 1
        }
        return perft(passed, depth: depth - 1)
    }

    var count: UInt64 = 0
    var remaining = moves

    while remaining != 0 {
        let sq = remaining.trailingZeroBitCount
        remaining &= remaining - 1

        var child = board
        child.doMove(sq)
        count += perft(child, depth: depth - 1)
    }

    return count
}

/// Expected perft results for the initial position.
public let perftExpected: [UInt64] = [
    1, 4, 12, 56, 244, 1396, 8200, 55092, 390216
]
