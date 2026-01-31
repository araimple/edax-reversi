/// Transpose a bitboard along the A1-H8 diagonal.
pub fn transpose(mut b: u64) -> u64 {
    let mut t;
    t = (b ^ (b >> 7)) & 0x00aa00aa00aa00aa;
    b = b ^ t ^ (t << 7);
    t = (b ^ (b >> 14)) & 0x0000cccc0000cccc;
    b = b ^ t ^ (t << 14);
    t = (b ^ (b >> 28)) & 0x00000000f0f0f0f0;
    b = b ^ t ^ (t << 28);
    b
}

/// Mirror vertically (swap rows 1<->8, 2<->7, etc.).
pub fn vertical_mirror(b: u64) -> u64 {
    b.swap_bytes()
}

/// Mirror horizontally (swap columns A<->H, B<->G, etc.).
pub fn horizontal_mirror(mut b: u64) -> u64 {
    b = ((b >> 1) & 0x5555555555555555) | ((b << 1) & 0xaaaaaaaaaaaaaaaa);
    b = ((b >> 2) & 0x3333333333333333) | ((b << 2) & 0xcccccccccccccccc);
    b = ((b >> 4) & 0x0f0f0f0f0f0f0f0f) | ((b << 4) & 0xf0f0f0f0f0f0f0f0);
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- transpose ---
    #[test]
    fn transpose_identity_for_symmetric() {
        // A diagonal-symmetric pattern should be unchanged
        // Main diagonal bits: A1(0), B2(9), C3(18), D4(27)
        let diag = (1u64 << 0) | (1u64 << 9) | (1u64 << 18) | (1u64 << 27);
        assert_eq!(transpose(diag), diag);
    }

    #[test]
    fn transpose_swaps_b1_and_a2() {
        // B1 = bit 1, A2 = bit 8. Transposing should swap them.
        assert_eq!(transpose(1u64 << 1), 1u64 << 8);
        assert_eq!(transpose(1u64 << 8), 1u64 << 1);
    }

    #[test]
    fn transpose_h1_becomes_a8() {
        // H1 = bit 7, A8 = bit 56
        assert_eq!(transpose(1u64 << 7), 1u64 << 56);
    }

    #[test]
    fn transpose_involution() {
        // transpose(transpose(x)) == x for any x
        let x = 0xDEADBEEFCAFEBABEu64;
        assert_eq!(transpose(transpose(x)), x);
    }

    // --- vertical_mirror ---
    #[test]
    fn vertical_mirror_swaps_row1_and_row8() {
        // Row 1 = bits 0..7, Row 8 = bits 56..63
        assert_eq!(vertical_mirror(0xFF), 0xFF << 56);
        assert_eq!(vertical_mirror(0xFF << 56), 0xFF);
    }

    #[test]
    fn vertical_mirror_is_byte_swap() {
        let x = 0x0102030405060708u64;
        assert_eq!(vertical_mirror(x), x.swap_bytes());
    }

    #[test]
    fn vertical_mirror_involution() {
        let x = 0xDEADBEEFCAFEBABEu64;
        assert_eq!(vertical_mirror(vertical_mirror(x)), x);
    }

    // --- horizontal_mirror ---
    #[test]
    fn horizontal_mirror_swaps_a_and_h_columns() {
        // A1 = bit 0, H1 = bit 7
        assert_eq!(horizontal_mirror(1u64 << 0), 1u64 << 7);
        assert_eq!(horizontal_mirror(1u64 << 7), 1u64 << 0);
    }

    #[test]
    fn horizontal_mirror_involution() {
        let x = 0xDEADBEEFCAFEBABEu64;
        assert_eq!(horizontal_mirror(horizontal_mirror(x)), x);
    }

    #[test]
    fn horizontal_mirror_full_row_unchanged() {
        // A full row (0xFF) should remain 0xFF
        assert_eq!(horizontal_mirror(0xFF), 0xFF);
    }
}
