use crate::board::Board;

/// Count leaf nodes by expanding all legal moves to the given depth.
/// Passes count as a ply but are not leaf nodes.
pub fn perft(board: &Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = board.get_moves();

    if moves == 0 {
        // No moves for current player
        let mut passed = *board;
        passed.pass();
        if passed.get_moves() == 0 {
            // Game over (neither player can move)
            return 1;
        }
        // Pass: opponent plays next
        return perft(&passed, depth - 1);
    }

    let mut count = 0u64;
    let mut remaining = moves;
    while remaining != 0 {
        let sq = remaining.trailing_zeros() as usize;
        remaining &= remaining - 1;
        let mut child = *board;
        child.do_move(sq);
        count += perft(&child, depth - 1);
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perft_depth_0() {
        let board = Board::new();
        assert_eq!(perft(&board, 0), 1);
    }

    #[test]
    fn perft_depth_1() {
        let board = Board::new();
        assert_eq!(perft(&board, 1), 4);
    }

    #[test]
    fn perft_depth_2() {
        let board = Board::new();
        assert_eq!(perft(&board, 2), 12);
    }

    #[test]
    fn perft_depth_3() {
        let board = Board::new();
        assert_eq!(perft(&board, 3), 56);
    }

    #[test]
    fn perft_depth_4() {
        let board = Board::new();
        assert_eq!(perft(&board, 4), 244);
    }

    #[test]
    fn perft_depth_5() {
        let board = Board::new();
        assert_eq!(perft(&board, 5), 1396);
    }

    #[test]
    fn perft_depth_6() {
        let board = Board::new();
        assert_eq!(perft(&board, 6), 8200);
    }

    #[test]
    fn perft_depth_7() {
        let board = Board::new();
        assert_eq!(perft(&board, 7), 55092);
    }

    #[test]
    fn perft_depth_8() {
        let board = Board::new();
        assert_eq!(perft(&board, 8), 390216);
    }
}
