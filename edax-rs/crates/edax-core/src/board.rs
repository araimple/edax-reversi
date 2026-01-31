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
