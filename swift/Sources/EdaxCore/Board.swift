/// Othello/Reversi board representation using bitboards.
/// Each square is represented as a bit in a 64-bit integer.
/// Square 0 = A1, Square 7 = H1, Square 56 = A8, Square 63 = H8.
public struct Board: Equatable, Hashable {
    public var player: UInt64
    public var opponent: UInt64

    /// Initial position constants
    public static let initPlayer: UInt64 = 0x0000000810000000    // E4, D5
    public static let initOpponent: UInt64 = 0x0000001008000000  // D4, E5
    public static let initMoves: UInt64 = 0x0000102004080000    // D3, C4, F5, E6

    /// Create a new board with the standard initial position.
    public init() {
        self.player = Board.initPlayer
        self.opponent = Board.initOpponent
    }

    /// Create a board with specified player and opponent bitboards.
    public init(player: UInt64, opponent: UInt64) {
        self.player = player
        self.opponent = opponent
    }

    /// Pass the turn (swap player and opponent).
    public mutating func pass() {
        swap(&player, &opponent)
    }

    /// Return the number of empty squares.
    public func empties() -> Int {
        return 64 - (player | opponent).nonzeroBitCount
    }

    /// Return the number of player discs.
    public func playerDiscCount() -> Int {
        return player.nonzeroBitCount
    }

    /// Return the number of opponent discs.
    public func opponentDiscCount() -> Int {
        return opponent.nonzeroBitCount
    }

    /// Return the score (player discs - opponent discs).
    /// Empty squares are awarded to the leader.
    public func score() -> Int {
        let p = playerDiscCount()
        let o = opponentDiscCount()
        let e = empties()
        if p > o {
            return p - o + e
        } else if p < o {
            return p - o - e
        } else {
            return 0
        }
    }

    /// Get the color at a specific square.
    /// Returns: 0 = player, 1 = opponent, 2 = empty
    public func getSquareColor(_ square: Int) -> Int {
        let mask: UInt64 = 1 << square
        if (player & mask) != 0 { return 0 }
        if (opponent & mask) != 0 { return 1 }
        return 2
    }

    /// Get all legal moves for the current player.
    public func getMoves() -> UInt64 {
        return getMoves(player: player, opponent: opponent)
    }

    /// Get legal moves using the parallel prefix algorithm.
    public func getMoves(player P: UInt64, opponent O: UInt64) -> UInt64 {
        let mask = O & 0x7E7E7E7E7E7E7E7E
        var moves: UInt64 = 0
        let empty = ~(P | O)

        // Right direction (+1)
        var flip = mask & (P << 1)
        flip |= mask & (flip << 1)
        flip |= mask & (flip << 1)
        flip |= mask & (flip << 1)
        flip |= mask & (flip << 1)
        flip |= mask & (flip << 1)
        moves |= flip << 1

        // Left direction (-1)
        flip = mask & (P >> 1)
        flip |= mask & (flip >> 1)
        flip |= mask & (flip >> 1)
        flip |= mask & (flip >> 1)
        flip |= mask & (flip >> 1)
        flip |= mask & (flip >> 1)
        moves |= flip >> 1

        // Down direction (+8)
        flip = O & (P << 8)
        flip |= O & (flip << 8)
        flip |= O & (flip << 8)
        flip |= O & (flip << 8)
        flip |= O & (flip << 8)
        flip |= O & (flip << 8)
        moves |= flip << 8

        // Up direction (-8)
        flip = O & (P >> 8)
        flip |= O & (flip >> 8)
        flip |= O & (flip >> 8)
        flip |= O & (flip >> 8)
        flip |= O & (flip >> 8)
        flip |= O & (flip >> 8)
        moves |= flip >> 8

        // Down-right direction (+9)
        flip = mask & (P << 9)
        flip |= mask & (flip << 9)
        flip |= mask & (flip << 9)
        flip |= mask & (flip << 9)
        flip |= mask & (flip << 9)
        flip |= mask & (flip << 9)
        moves |= flip << 9

        // Up-left direction (-9)
        flip = mask & (P >> 9)
        flip |= mask & (flip >> 9)
        flip |= mask & (flip >> 9)
        flip |= mask & (flip >> 9)
        flip |= mask & (flip >> 9)
        flip |= mask & (flip >> 9)
        moves |= flip >> 9

        // Down-left direction (+7)
        flip = mask & (P << 7)
        flip |= mask & (flip << 7)
        flip |= mask & (flip << 7)
        flip |= mask & (flip << 7)
        flip |= mask & (flip << 7)
        flip |= mask & (flip << 7)
        moves |= flip << 7

        // Up-right direction (-7)
        flip = mask & (P >> 7)
        flip |= mask & (flip >> 7)
        flip |= mask & (flip >> 7)
        flip |= mask & (flip >> 7)
        flip |= mask & (flip >> 7)
        flip |= mask & (flip >> 7)
        moves |= flip >> 7

        return moves & empty
    }

    /// Calculate the discs that would be flipped by placing at `square`.
    public func flip(_ square: Int) -> UInt64 {
        return flipDisc(square, player: player, opponent: opponent)
    }

    /// Flip calculation implementation.
    public func flipDisc(_ square: Int, player P: UInt64, opponent O: UInt64) -> UInt64 {
        let move: UInt64 = 1 << square
        var totalFlip: UInt64 = 0

        let directions = [1, -1, 8, -8, 9, -9, 7, -7]
        let masks: [UInt64] = [
            0x7E7E7E7E7E7E7E7E, 0x7E7E7E7E7E7E7E7E,
            0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF,
            0x7E7E7E7E7E7E7E7E, 0x7E7E7E7E7E7E7E7E,
            0x7E7E7E7E7E7E7E7E, 0x7E7E7E7E7E7E7E7E
        ]

        for i in 0..<8 {
            let m = masks[i] & O
            let d = directions[i]
            var f: UInt64 = 0
            var x = move

            if d > 0 {
                x = x << d
                while (x & m) != 0 {
                    f |= x
                    x = x << d
                }
                if (x & P) != 0 { totalFlip |= f }
            } else {
                let nd = -d
                x = x >> nd
                while (x & m) != 0 {
                    f |= x
                    x = x >> nd
                }
                if (x & P) != 0 { totalFlip |= f }
            }
        }

        return totalFlip
    }

    /// Play a move at the given square.
    /// Returns the flipped discs for undo.
    @discardableResult
    public mutating func doMove(_ square: Int) -> UInt64 {
        let flipped = flip(square)
        player |= flipped | (1 << square)
        opponent ^= flipped
        swap(&player, &opponent)
        return flipped
    }

    /// Undo a move.
    public mutating func undoMove(_ square: Int, flipped: UInt64) {
        swap(&player, &opponent)
        player ^= flipped | (1 << square)
        opponent ^= flipped
    }

    /// Check if the game is over (neither player can move).
    public func isGameOver() -> Bool {
        if getMoves() != 0 { return false }
        var passed = self
        passed.pass()
        return passed.getMoves() == 0
    }

    /// Check if the current player must pass.
    public func isPass() -> Bool {
        return getMoves() == 0
    }

    /// Get mobility (number of legal moves).
    public func mobility() -> Int {
        return getMoves().nonzeroBitCount
    }

    /// Convert square index to algebraic notation.
    public static func squareToString(_ square: Int) -> String {
        let col = Character(UnicodeScalar(UInt8(65 + square % 8)))
        let row = (square / 8) + 1
        return "\(col)\(row)"
    }
}
