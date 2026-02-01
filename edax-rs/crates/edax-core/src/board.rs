use crate::flip;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Board {
    pub player: u64,
    pub opponent: u64,
}

impl Board {
    pub fn new() -> Self {
        // Black = E4, D5; White = D4, E5. Black moves first.
        Board {
            player: (1u64 << 28) | (1u64 << 35),
            opponent: (1u64 << 27) | (1u64 << 36),
        }
    }

    pub fn pass(&mut self) {
        std::mem::swap(&mut self.player, &mut self.opponent);
    }

    /// Return bitmask of all legal moves for the current player.
    pub fn get_moves(&self) -> u64 {
        let empty = !(self.player | self.opponent);
        let mut moves = 0u64;
        let mut empties = empty;
        while empties != 0 {
            let sq = empties.trailing_zeros() as usize;
            if flip::flip(sq, self.player, self.opponent) != 0 {
                moves |= 1u64 << sq;
            }
            empties &= empties - 1; // clear LSB
        }
        moves
    }

    /// Play a move at square `sq`. Flips opponent discs, swaps turn.
    /// Returns the flipped bitmask (for undo).
    pub fn do_move(&mut self, sq: usize) -> u64 {
        let flipped = flip::flip(sq, self.player, self.opponent);
        self.player |= flipped | (1u64 << sq);
        self.opponent ^= flipped;
        std::mem::swap(&mut self.player, &mut self.opponent);
        flipped
    }

    /// Undo a move. Caller must provide the same sq and flipped from do_move.
    pub fn undo_move(&mut self, sq: usize, flipped: u64) {
        std::mem::swap(&mut self.player, &mut self.opponent);
        self.player ^= flipped | (1u64 << sq);
        self.opponent ^= flipped;
    }

    /// Convert square index (0-63) to algebraic notation ("A1".."H8").
    pub fn square_to_string(sq: usize) -> String {
        let col = (b'A' + (sq % 8) as u8) as char;
        let row = (b'1' + (sq / 8) as u8) as char;
        format!("{}{}", col, row)
    }

    /// Render the board as a string.
    /// `player_is_black`: if true, player='X'(black), opponent='O'(white).
    pub fn to_board_string(&self, player_is_black: bool) -> String {
        let (black, white) = if player_is_black {
            (self.player, self.opponent)
        } else {
            (self.opponent, self.player)
        };
        let mut s = String::from("  A B C D E F G H\n");
        for row in 0..8 {
            s.push_str(&format!("{} ", row + 1));
            for col in 0..8 {
                let bit = 1u64 << (row * 8 + col);
                if black & bit != 0 {
                    s.push('X');
                } else if white & bit != 0 {
                    s.push('O');
                } else {
                    s.push('-');
                }
                if col < 7 {
                    s.push(' ');
                }
            }
            s.push('\n');
        }
        s
    }

    pub fn count_player_discs(&self) -> u32 {
        self.player.count_ones()
    }

    pub fn count_opponent_discs(&self) -> u32 {
        self.opponent.count_ones()
    }

    pub fn empties(&self) -> u32 {
        64 - (self.player | self.opponent).count_ones()
    }

    /// Final score = player_discs - opponent_discs.
    /// Empty squares are awarded to the player with more discs (standard Othello rule).
    /// Compute the number of stable discs for `player` against `opponent`.
    /// A disc is stable if it can never be flipped in any future game state.
    /// This is a lower bound (conservative estimate) used for stability cutoff.
    pub fn get_stability(player: u64, opponent: u64) -> i32 {
        let disc = player | opponent;
        let central_mask = player & 0x007e7e7e7e7e7e00u64;

        // Full lines in each direction
        let full_h = Self::get_full_lines(disc, 1);
        let full_v = Self::get_full_lines(disc, 8);
        let full_d7 = Self::get_full_lines(disc, 7);
        let full_d9 = Self::get_full_lines(disc, 9);

        // Stable edges (corner-anchored runs)
        let mut new_stable = Self::get_stable_edge(player, opponent);

        // Add squares that are on full lines in ALL 4 directions
        new_stable |= full_h & full_v & full_d7 & full_d9 & central_mask;

        // Propagate: a disc is stable if in each direction, either
        // the line is full OR an adjacent disc is stable
        let mut stable = 0u64;
        while (new_stable & !stable) != 0 {
            stable |= new_stable;
            let stable_h = (stable >> 1) | (stable << 1) | full_h;
            let stable_v = (stable >> 8) | (stable << 8) | full_v;
            let stable_d7 = (stable >> 7) | (stable << 7) | full_d7;
            let stable_d9 = (stable >> 9) | (stable << 9) | full_d9;
            new_stable = stable_h & stable_v & stable_d7 & stable_d9 & central_mask;
        }

        stable.count_ones() as i32
    }

    /// Get full lines in the given direction.
    /// Returns a mask of squares that are on a completely filled line in direction `dir`.
    fn get_full_lines(line: u64, dir: u32) -> u64 {
        let edge: u64 = line & 0xff818181818181ffu64;
        let mut full = line & (((line >> dir) & (line << dir)) | edge);
        full &= ((full >> dir) & (full << dir)) | edge;
        full &= ((full >> dir) & (full << dir)) | edge;
        full &= ((full >> dir) & (full << dir)) | edge;
        full &= ((full >> dir) & (full << dir)) | edge;
        (full >> dir) & (full << dir)
    }

    /// Compute stable edge discs using corner-anchored runs.
    /// For each of the 4 edges, find continuous runs of player discs from corners.
    fn get_stable_edge(player: u64, opponent: u64) -> u64 {
        let mut stable = 0u64;

        // Top edge (row 1, bits 0-7)
        let p_top = (player & 0xFF) as u8;
        let o_top = (opponent & 0xFF) as u8;
        stable |= Self::edge_stable_line(p_top, o_top) as u64;

        // Bottom edge (row 8, bits 56-63)
        let p_bot = (player >> 56) as u8;
        let o_bot = (opponent >> 56) as u8;
        stable |= (Self::edge_stable_line(p_bot, o_bot) as u64) << 56;

        // Left edge (column A, bits 0,8,16,...,56)
        let p_left = Self::pack_col_a(player);
        let o_left = Self::pack_col_a(opponent);
        let s_left = Self::edge_stable_line(p_left, o_left);
        stable |= Self::unpack_col_a(s_left);

        // Right edge (column H, bits 7,15,23,...,63)
        let p_right = Self::pack_col_h(player);
        let o_right = Self::pack_col_h(opponent);
        let s_right = Self::edge_stable_line(p_right, o_right);
        stable |= Self::unpack_col_h(s_right);

        stable & player
    }

    /// Compute stable discs on a single edge (8-bit line).
    /// Returns corner-anchored stable player discs.
    fn edge_stable_line(p: u8, o: u8) -> u8 {
        let full = p | o;
        // If the edge is completely full, all player discs are stable
        if full == 0xFF {
            return p;
        }
        let mut stable = 0u8;
        // From left: consecutive P discs from bit 0
        for i in 0..8u8 {
            if (p >> i) & 1 == 1 {
                stable |= 1 << i;
            } else {
                break;
            }
        }
        // From right: consecutive P discs from bit 7
        for i in (0..8u8).rev() {
            if (p >> i) & 1 == 1 {
                stable |= 1 << i;
            } else {
                break;
            }
        }
        stable
    }

    /// Pack column A (bits 0,8,16,...,56) into 8 bits.
    #[inline]
    fn pack_col_a(bb: u64) -> u8 {
        ((bb & 0x0101010101010101u64).wrapping_mul(0x0102040810204080u64) >> 56) as u8
    }

    /// Pack column H (bits 7,15,23,...,63) into 8 bits.
    #[inline]
    fn pack_col_h(bb: u64) -> u8 {
        ((bb & 0x8080808080808080u64).wrapping_mul(0x0002040810204081u64) >> 56) as u8
    }

    /// Unpack 8 bits to column A (bits 0,8,16,...,56).
    fn unpack_col_a(val: u8) -> u64 {
        let mut result = 0u64;
        for i in 0..8 {
            if (val >> i) & 1 != 0 {
                result |= 1u64 << (i * 8);
            }
        }
        result
    }

    /// Unpack 8 bits to column H (bits 7,15,23,...,63).
    fn unpack_col_h(val: u8) -> u64 {
        let mut result = 0u64;
        for i in 0..8 {
            if (val >> i) & 1 != 0 {
                result |= 1u64 << (i * 8 + 7);
            }
        }
        result
    }

    pub fn score(&self) -> i32 {
        let p = self.count_player_discs() as i32;
        let o = self.count_opponent_discs() as i32;
        let e = self.empties() as i32;
        if p > o {
            p - o + e
        } else if p < o {
            p - o - e
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_board_has_two_discs_each() {
        let board = Board::new();
        assert_eq!(board.count_player_discs(), 2);
        assert_eq!(board.count_opponent_discs(), 2);
    }

    #[test]
    fn score_on_initial_board_is_zero() {
        let board = Board::new();
        assert_eq!(board.score(), 0);
    }

    #[test]
    fn empties_on_initial_board_is_60() {
        let board = Board::new();
        assert_eq!(board.empties(), 60);
    }

    #[test]
    fn pass_swaps_player_and_opponent() {
        let mut board = Board::new();
        let original = board;
        board.pass();
        assert_eq!(board.player, original.opponent);
        assert_eq!(board.opponent, original.player);
    }

    #[test]
    fn get_moves_initial_position_has_four_moves() {
        let board = Board::new();
        let moves = board.get_moves();
        // D3(19), C4(26), F5(37), E6(44)
        let expected = (1u64 << 19) | (1u64 << 26) | (1u64 << 37) | (1u64 << 44);
        assert_eq!(moves, expected);
    }

    #[test]
    fn get_moves_returns_four_legal_moves() {
        let board = Board::new();
        assert_eq!(board.get_moves().count_ones(), 4);
    }

    #[test]
    fn do_move_d3_updates_board_correctly() {
        let mut board = Board::new();
        let flipped = board.do_move(19); // D3
        // Should have flipped D4(27)
        assert_eq!(flipped, 1u64 << 27);
        // After move: opponent is now "player" (turn swapped)
        // New opponent (was black, now has: E4(28)+D5(35)+D3(19)+D4(27))
        // New player (was white, now has: E5(36), lost D4)
        assert_eq!(board.player, (1u64 << 36)); // only E5
        let expected_opp = (1u64 << 19) | (1u64 << 27) | (1u64 << 28) | (1u64 << 35);
        assert_eq!(board.opponent, expected_opp);
    }

    #[test]
    fn undo_move_restores_board() {
        let mut board = Board::new();
        let original = board;
        let flipped = board.do_move(19); // D3
        board.undo_move(19, flipped);
        assert_eq!(board, original);
    }

    #[test]
    fn do_move_then_opponent_moves() {
        let mut board = Board::new();
        board.do_move(19); // Black plays D3
        // Now it's white's turn
        let moves = board.get_moves();
        assert!(moves != 0, "white should have moves after D3");
        // White should be able to play C3(18), E3(20), C5(34)
        assert!(moves & (1u64 << 18) != 0, "C3 should be legal for white");
    }

    #[test]
    fn display_initial_board() {
        let board = Board::new();
        let s = board.to_board_string(true); // true = player is black
        assert!(s.contains("  A B C D E F G H"));
        assert!(s.contains("X"), "should contain X for black");
        assert!(s.contains("O"), "should contain O for white");
        assert!(s.contains("-"), "should contain - for empty");
        // Row 4 should have . . . O X . . .
        // Row 5 should have . . . X O . . .
    }

    #[test]
    fn square_to_string_d3() {
        assert_eq!(Board::square_to_string(19), "D3");
    }

    #[test]
    fn square_to_string_a1() {
        assert_eq!(Board::square_to_string(0), "A1");
    }

    #[test]
    fn square_to_string_h8() {
        assert_eq!(Board::square_to_string(63), "H8");
    }

    #[test]
    fn stability_corners_are_stable() {
        // All 4 corners filled by player
        let player = (1u64 << 0) | (1u64 << 7) | (1u64 << 56) | (1u64 << 63);
        let opponent = 0u64;
        let stable = Board::get_stability(player, opponent);
        assert_eq!(stable, 4, "4 corner discs should be stable");
    }

    #[test]
    fn stability_full_edge() {
        // Full top edge: player has first 4, opponent has last 4
        let player = 0x0Fu64; // A1-D1
        let opponent = 0xF0u64; // E1-H1
        let stable = Board::get_stability(player, opponent);
        assert!(stable >= 4, "full edge player discs should be stable, got {}", stable);
    }

    #[test]
    fn stability_empty_board_zero() {
        // Nearly empty board
        let player = (1u64 << 28) | (1u64 << 35); // E4, D5
        let opponent = (1u64 << 27) | (1u64 << 36); // D4, E5
        let stable = Board::get_stability(player, opponent);
        assert_eq!(stable, 0, "no discs should be stable on initial board");
    }

    #[test]
    fn stability_full_board() {
        // Full board, player has bottom half
        let player = 0x00000000FFFFFFFFu64;
        let opponent = 0xFFFFFFFF00000000u64;
        let stable = Board::get_stability(player, opponent);
        // All discs are stable on a full board
        assert_eq!(stable, 32, "all player discs should be stable on full board");
    }

    #[test]
    fn stability_corner_run() {
        // Player has A1, B1, C1 (corner run on top edge)
        let player = 0x07u64;
        let opponent = 0x08u64; // D1 is opponent
        let stable = Board::get_stability(player, opponent);
        assert!(stable >= 3, "A1-C1 corner run should be stable, got {}", stable);
    }

    #[test]
    fn new_board_has_initial_position() {
        let board = Board::new();
        // Standard Othello starting position:
        //   D4=white, E4=black, D5=black, E5=white
        //   Black plays first, so black = player
        //
        // D4 = bit 27, E4 = bit 28, D5 = bit 35, E5 = bit 36
        let black = (1u64 << 28) | (1u64 << 35); // E4, D5
        let white = (1u64 << 27) | (1u64 << 36); // D4, E5
        assert_eq!(board.player, black, "player (black) mismatch");
        assert_eq!(board.opponent, white, "opponent (white) mismatch");
    }
}
