//! Evaluation function for Othello positions.
//!
//! Implements a pattern-based evaluation using disc placement heuristics.
//! Since the original eval.dat weight file is not available, this uses
//! a hand-crafted evaluation based on positional strategy:
//! - Corner control
//! - Edge stability
//! - Mobility
//! - Disc difference (weighted by game phase)
//! - Potential mobility

use crate::board::Board;

/// Positional weights for each square (static evaluation).
/// Corners are highly valuable, X-squares (diagonally adjacent to corners) are bad.
#[rustfmt::skip]
const SQUARE_WEIGHT: [i32; 64] = [
    120, -20,  20,   5,   5,  20, -20, 120,
    -20, -40,  -5,  -5,  -5,  -5, -40, -20,
     20,  -5,  15,   3,   3,  15,  -5,  20,
      5,  -5,   3,   3,   3,   3,  -5,   5,
      5,  -5,   3,   3,   3,   3,  -5,   5,
     20,  -5,  15,   3,   3,  15,  -5,  20,
    -20, -40,  -5,  -5,  -5,  -5, -40, -20,
    120, -20,  20,   5,   5,  20, -20, 120,
];

/// Evaluate a board position from the current player's perspective.
/// Returns a score in the range roughly -64..+64.
pub fn evaluate(board: &Board) -> i32 {
    let n_empties = board.empties() as i32;

    // Endgame: exact disc difference
    if n_empties == 0 {
        return board.score();
    }

    // Near endgame (<=8 empties): blend toward disc difference
    if n_empties <= 8 {
        let positional = positional_score(board);
        let disc_score = board.score() * 4;
        return (positional + disc_score) / 5;
    }

    positional_score(board)
}

/// Compute positional evaluation score.
fn positional_score(board: &Board) -> i32 {
    let n_empties = board.empties() as i32;

    // 1. Weighted piece square table
    let mut piece_score = 0i32;
    let mut bits = board.player;
    while bits != 0 {
        let sq = bits.trailing_zeros() as usize;
        piece_score += SQUARE_WEIGHT[sq];
        bits &= bits - 1;
    }
    let mut bits = board.opponent;
    while bits != 0 {
        let sq = bits.trailing_zeros() as usize;
        piece_score -= SQUARE_WEIGHT[sq];
        bits &= bits - 1;
    }

    // 2. Mobility (current player moves - opponent moves)
    let my_moves = board.get_moves().count_ones() as i32;
    let mut opp_board = *board;
    opp_board.pass();
    let opp_moves = opp_board.get_moves().count_ones() as i32;
    let mobility_score = (my_moves - opp_moves) * 10;

    // 3. Corner occupancy
    let corners: u64 = (1u64 << 0) | (1u64 << 7) | (1u64 << 56) | (1u64 << 63);
    let my_corners = (board.player & corners).count_ones() as i32;
    let opp_corners = (board.opponent & corners).count_ones() as i32;
    let corner_score = (my_corners - opp_corners) * 25;

    // 4. Stability estimate: corners + stable edges
    let stability_score = stability_estimate(board);

    // 5. Frontier discs (discs adjacent to empty squares — fewer is better)
    let empty = !(board.player | board.opponent);
    let frontier = frontier_discs(board.player, empty);
    let opp_frontier = frontier_discs(board.opponent, empty);
    let frontier_score = (opp_frontier as i32 - frontier as i32) * 3;

    // Weight components by game phase
    let phase_weight = if n_empties > 40 {
        // Opening: mobility and position matter most
        (piece_score / 4) + mobility_score * 3 + corner_score + stability_score + frontier_score
    } else if n_empties > 20 {
        // Midgame: balance everything
        (piece_score / 3) + mobility_score * 2 + corner_score * 2 + stability_score * 2 + frontier_score
    } else {
        // Late midgame: stability and disc count matter more
        piece_score / 2 + mobility_score + corner_score * 3 + stability_score * 3 + frontier_score
    };

    // Normalize to approximately -64..+64 range
    phase_weight.clamp(-63, 63)
}

/// Count frontier discs (player discs adjacent to at least one empty square).
fn frontier_discs(player: u64, empty: u64) -> u32 {
    let shifted = (empty << 1) | (empty >> 1) | (empty << 8) | (empty >> 8)
        | (empty << 7) | (empty >> 7) | (empty << 9) | (empty >> 9);
    (player & shifted).count_ones()
}

/// Estimate stability from corner control.
fn stability_estimate(board: &Board) -> i32 {
    let mut stable_player = 0i32;
    let mut stable_opponent = 0i32;

    // Check each corner and count stable discs along edges
    let corner_squares = [0u64, 7, 56, 63];
    for &c in &corner_squares {
        if board.player & (1u64 << c) != 0 {
            stable_player += 1 + count_stable_edge(board.player, c);
        } else if board.opponent & (1u64 << c) != 0 {
            stable_opponent += 1 + count_stable_edge(board.opponent, c);
        }
    }

    (stable_player - stable_opponent) * 8
}

/// Count stable discs along edges from a corner.
fn count_stable_edge(discs: u64, corner: u64) -> i32 {
    let mut count = 0i32;
    // Edges from each corner
    let edges: &[(i32, &[u64])] = match corner {
        0 => &[
            (1, &[1, 2, 3, 4, 5, 6, 7]),
            (8, &[8, 16, 24, 32, 40, 48, 56]),
        ],
        7 => &[
            (-1, &[6, 5, 4, 3, 2, 1, 0]),
            (8, &[15, 23, 31, 39, 47, 55, 63]),
        ],
        56 => &[
            (1, &[57, 58, 59, 60, 61, 62, 63]),
            (-8, &[48, 40, 32, 24, 16, 8, 0]),
        ],
        63 => &[
            (-1, &[62, 61, 60, 59, 58, 57, 56]),
            (-8, &[55, 47, 39, 31, 23, 15, 7]),
        ],
        _ => return 0,
    };

    for &(_dir, squares) in edges {
        for &sq in squares {
            if discs & (1u64 << sq) != 0 {
                count += 1;
            } else {
                break;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_initial_position_near_zero() {
        let board = Board::new();
        let score = evaluate(&board);
        // Initial position should be close to 0 (symmetric)
        assert!(score.abs() <= 10, "initial eval {} not near zero", score);
    }

    #[test]
    fn eval_respects_score_range() {
        let board = Board::new();
        let score = evaluate(&board);
        assert!(score >= -64 && score <= 64, "score {} out of range", score);
    }

    #[test]
    fn eval_corner_advantage_is_positive() {
        // Player has A1 corner, opponent doesn't
        let board = Board {
            player: (1u64 << 0) | (1u64 << 1) | (1u64 << 8),
            opponent: (1u64 << 18) | (1u64 << 19) | (1u64 << 20),
        };
        let score = evaluate(&board);
        assert!(score > 0, "corner advantage should be positive, got {}", score);
    }

    #[test]
    fn eval_empty_board_is_zero() {
        // All discs same count, no empties
        let board = Board {
            player: 0x00000000FFFFFFFF,
            opponent: 0xFFFFFFFF00000000,
        };
        let score = evaluate(&board);
        // Full board: exact disc difference = 0
        assert_eq!(score, 0);
    }

    #[test]
    fn eval_winning_endgame_positive() {
        // Player has 40 discs, opponent has 24
        let board = Board {
            player: 0x00000000FFFFFFFF | (0xFFu64 << 32), // 40 bits
            opponent: 0xFFFFFF0000000000u64,                // 24 bits
        };
        let score = evaluate(&board);
        assert!(score > 0, "winning position should be positive, got {}", score);
    }

    #[test]
    fn eval_symmetry_negation() {
        // Swapping player/opponent should negate score
        let board = Board::new();
        let mut board2 = board;
        board2.do_move(19); // D3
        let score1 = evaluate(&board2);
        let mut flipped = board2;
        std::mem::swap(&mut flipped.player, &mut flipped.opponent);
        let score2 = evaluate(&flipped);
        // Scores should be negated (approximately, due to mobility differences)
        assert!((score1 + score2).abs() <= 5,
            "symmetry: {} + {} = {} (should be ~0)", score1, score2, score1 + score2);
    }

    #[test]
    fn square_weights_are_symmetric() {
        // Board is symmetric under 4 rotations
        assert_eq!(SQUARE_WEIGHT[0], SQUARE_WEIGHT[7]);   // A1 == H1
        assert_eq!(SQUARE_WEIGHT[0], SQUARE_WEIGHT[56]);  // A1 == A8
        assert_eq!(SQUARE_WEIGHT[0], SQUARE_WEIGHT[63]);  // A1 == H8
        assert_eq!(SQUARE_WEIGHT[1], SQUARE_WEIGHT[6]);   // B1 == G1
        assert_eq!(SQUARE_WEIGHT[9], SQUARE_WEIGHT[14]);  // B2 == G2
    }
}
