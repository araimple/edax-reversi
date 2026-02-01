/// Positive directions (left-shift): right, down, down-right, down-left
const POS_DIRS: [(u32, u64); 4] = [
    (1, 0xfefefefefefefefe), // right
    (8, 0xffffffffffffff00), // down
    (9, 0xfefefefefefefe00), // down-right
    (7, 0x7f7f7f7f7f7f7f00), // down-left
];

/// Negative directions (right-shift): left, up, up-right, up-left
const NEG_DIRS: [(u32, u64); 4] = [
    (1, 0x7f7f7f7f7f7f7f7f), // left
    (8, 0x00ffffffffffffff), // up
    (7, 0x00fefefefefefefe), // up-right
    (9, 0x007f7f7f7f7f7f7f), // up-left
];

/// Compute flipped discs when `player` places a disc on square `sq` (0-63)
/// against `opponent`. Returns a bitmask of flipped opponent discs.
pub fn flip(sq: usize, player: u64, opponent: u64) -> u64 {
    let mut flipped = 0u64;
    let bit = 1u64 << sq;
    for &(shift, mask) in &POS_DIRS {
        flipped |= scan_and_flip::<true>(bit, shift, mask, player, opponent);
    }
    for &(shift, mask) in &NEG_DIRS {
        flipped |= scan_and_flip::<false>(bit, shift, mask, player, opponent);
    }
    flipped
}

/// Walk along a direction from `start`. Collect opponent discs until we hit
/// a player disc (return collected) or empty/edge (return 0).
/// POSITIVE=true means left-shift, false means right-shift.
#[inline]
fn scan_and_flip<const POSITIVE: bool>(
    start: u64,
    shift: u32,
    mask: u64,
    player: u64,
    opponent: u64,
) -> u64 {
    let mut cursor = start;
    let mut flips = 0u64;
    loop {
        cursor = if POSITIVE {
            (cursor << shift) & mask
        } else {
            (cursor >> shift) & mask
        };
        if cursor == 0 {
            return 0;
        }
        if cursor & opponent != 0 {
            flips |= cursor;
        } else if cursor & player != 0 {
            return flips;
        } else {
            return 0;
        }
    }
}

/// Count the number of flipped discs for the last move (1-empty board).
///
/// Returns **twice** the number of flipped discs (matching C convention for
/// efficient disc-difference computation in endgame solvers).
///
/// `x`: square index (0-63) of the last empty square.
/// `player`: bitboard of the player about to move.
///
/// In the 1-empty case, the opponent occupies all squares except `x` and
/// the player's squares, so we don't need an explicit opponent bitboard.
pub fn count_last_flip(x: usize, player: u64) -> i32 {
    debug_assert!(x < 64);
    // Opponent = all occupied squares that aren't player's (the only empty is x)
    let opponent = !player & !(1u64 << x);
    let flipped = flip(x, player, opponent);
    2 * flipped.count_ones() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    // Initial position: Black=E4(28)+D5(35), White=D4(27)+E5(36)
    const BLACK: u64 = (1 << 28) | (1 << 35);
    const WHITE: u64 = (1 << 27) | (1 << 36);

    #[test]
    fn flip_d3_from_initial_flips_d4() {
        assert_eq!(flip(19, BLACK, WHITE), 1u64 << 27);
    }

    #[test]
    fn flip_c4_from_initial_flips_d4() {
        assert_eq!(flip(26, BLACK, WHITE), 1u64 << 27);
    }

    #[test]
    fn flip_f5_from_initial_flips_e5() {
        assert_eq!(flip(37, BLACK, WHITE), 1u64 << 36);
    }

    #[test]
    fn flip_e6_from_initial_flips_e5() {
        assert_eq!(flip(44, BLACK, WHITE), 1u64 << 36);
    }

    #[test]
    fn flip_a1_from_initial_flips_nothing() {
        assert_eq!(flip(0, BLACK, WHITE), 0);
    }

    #[test]
    fn flip_c5_from_initial_flips_nothing() {
        assert_eq!(flip(34, BLACK, WHITE), 0);
    }

    #[test]
    fn flip_after_d3_white_plays_c3() {
        let black2 = BLACK | (1 << 19) | (1 << 27);
        let white2 = WHITE ^ (1 << 27);
        assert_eq!(flip(18, white2, black2), 1u64 << 27);
    }

    #[test]
    fn flip_chain_two_discs() {
        // Player at D1(3), opponent at B1(1)+C1(2). Player plays A1(0).
        let player = 1u64 << 3;
        let opponent = (1u64 << 1) | (1u64 << 2);
        assert_eq!(flip(0, player, opponent), (1u64 << 1) | (1u64 << 2));
    }

    #[test]
    fn flip_no_bracket_returns_zero() {
        // Opponent fills B1-H1, no player disc to bracket
        let player = 0u64;
        let opponent = 0xFE;
        assert_eq!(flip(0, player, opponent), 0);
    }

    #[test]
    fn flip_diagonal() {
        // Player at C3(18), opponent at B2(9). Player plays A1(0).
        let player = 1u64 << 18;
        let opponent = 1u64 << 9;
        assert_eq!(flip(0, player, opponent), 1u64 << 9);
    }

    #[test]
    fn count_last_flip_matches_flip_function() {
        // Property test: count_last_flip(x, player) should equal
        // 2 * flip(x, player, opponent).count_ones()
        // where opponent = !player & !(1<<x)  (1-empty board)

        // Case 1: All player except column A, empty at A1(0)
        let player = 0xFEFEFEFEFEFEFEFEu64;
        let x = 0usize;
        let opponent = !player & !(1u64 << x);
        let expected = 2 * flip(x, player, opponent).count_ones() as i32;
        assert_eq!(count_last_flip(x, player), expected);
    }

    #[test]
    fn count_last_flip_center_square() {
        // Empty at D4(27), player has all edges and some center
        let x = 27usize;
        let player = 0xFF818181818181FFu64 & !(1u64 << x); // all edges
        let opponent = !player & !(1u64 << x);
        let expected = 2 * flip(x, player, opponent).count_ones() as i32;
        assert_eq!(count_last_flip(x, player), expected);
    }

    #[test]
    fn count_last_flip_no_flips() {
        // Empty at H8(63), player has nothing adjacent to bracket
        let x = 63usize;
        // Player has only A1(0) - way too far to bracket anything
        let player = 1u64;
        let opponent = !player & !(1u64 << x);
        let expected = 2 * flip(x, player, opponent).count_ones() as i32;
        assert_eq!(count_last_flip(x, player), expected);
    }

    #[test]
    fn count_last_flip_all_directions() {
        // Empty at E5(36), player surrounds opponent discs in multiple directions
        let x = 36usize;
        // Player has the ring around a 3x3 center, opponent has the inner ring
        // This creates flips in all 8 directions
        let player = 0xFFFF_FFFF_FFFF_FFFFu64
            & !(1u64 << 27) & !(1u64 << 28) & !(1u64 << 29)
            & !(1u64 << 35) & !(1u64 << 36) & !(1u64 << 37)
            & !(1u64 << 43) & !(1u64 << 44) & !(1u64 << 45);
        // Opponent = all the inner ring except x
        let opponent = !player & !(1u64 << x);
        let expected = 2 * flip(x, player, opponent).count_ones() as i32;
        assert_eq!(count_last_flip(x, player), expected);
    }

    #[test]
    fn count_last_flip_every_square() {
        // Exhaustive: for a fixed player pattern, verify all 64 squares
        let player = 0xAA55AA55AA55AA55u64; // checkerboard pattern
        for x in 0..64 {
            if player & (1u64 << x) != 0 {
                continue; // skip squares that are player
            }
            let opponent = !player & !(1u64 << x);
            let expected = 2 * flip(x, player, opponent).count_ones() as i32;
            assert_eq!(
                count_last_flip(x, player), expected,
                "mismatch at square {} ({})",
                x, super::super::board::Board::square_to_string(x)
            );
        }
    }
}
