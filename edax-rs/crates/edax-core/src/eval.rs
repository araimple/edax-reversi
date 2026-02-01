//! Evaluation function for Othello positions.
//!
//! Implements the edax-compatible 47-feature pattern evaluation.
//! Loads weights from eval.dat for each game phase (ply).
//! Falls back to a simple heuristic if eval.dat is not available.

use crate::board::Board;
use std::path::Path;
use std::sync::OnceLock;

/// Number of evaluation features.
const EVAL_N_FEATURE: usize = 47;

/// Number of unpacked weights per ply.
const EVAL_N_WEIGHT: usize = 226315;

/// Number of plies (game phases, 0..60 inclusive).
const EVAL_N_PLY: usize = 61;

/// Number of packed weights per ply in eval.dat.
const EVAL_N_PACKED: usize = 114364;

/// Feature group sizes (unpacked, one entry per group type).
const EVAL_SIZE: [usize; 13] = [
    19683, 59049, 59049, 59049, 6561, 6561, 6561, 6561, 2187, 729, 243, 81, 1,
];

/// Packed feature group sizes (symmetry-reduced).
const EVAL_PACKED_SIZE: [usize; 13] = [
    10206, 29889, 29646, 29646, 3321, 3321, 3321, 3321, 1134, 378, 135, 45, 1,
];

/// Feature offsets into the flat weight array, one per feature.
#[rustfmt::skip]
const EVAL_OFFSET: [usize; EVAL_N_FEATURE] = [
         0,      0,      0,      0,
     19683,  19683,  19683,  19683,
     78732,  78732,  78732,  78732,
    137781, 137781, 137781, 137781,
    196830, 196830, 196830, 196830,
    203391, 203391, 203391, 203391,
    209952, 209952, 209952, 209952,
    216513, 216513,
    223074, 223074, 223074, 223074,
    225261, 225261, 225261, 225261,
    225990, 225990, 225990, 225990,
    226233, 226233, 226233, 226233,
    226314,
];

/// Number of squares for each feature.
#[rustfmt::skip]
const EVAL_N_SQUARE: [usize; EVAL_N_FEATURE] = [
    9, 9, 9, 9,
    10, 10, 10, 10,
    10, 10, 10, 10,
    10, 10, 10, 10,
    8, 8, 8, 8,
    8, 8, 8, 8,
    8, 8, 8, 8,
    8, 8,
    7, 7, 7, 7,
    6, 6, 6, 6,
    5, 5, 5, 5,
    4, 4, 4, 4,
    0,
];

/// Feature-to-coordinate mapping: square indices for each of the 47 features.
/// Unused slots are filled with -1. The square numbering matches the C version:
/// A1=0, B1=1, ..., H1=7, A2=8, ..., H8=63.
#[rustfmt::skip]
const EVAL_F2X: [[i32; 10]; EVAL_N_FEATURE] = [
    // Corner 3x3 patterns (features 0-3)
    [ 0,  1,  8,  9,  2, 16, 10, 17, 18, -1],
    [ 7,  6, 15, 14,  5, 23, 13, 22, 21, -1],
    [56, 48, 57, 49, 40, 58, 41, 50, 42, -1],
    [63, 55, 62, 54, 47, 61, 46, 53, 45, -1],
    // Edge+2X patterns (features 4-7, angle+X, C10 symmetry)
    [32, 24, 16,  8,  0,  9,  1,  2,  3,  4],
    [39, 31, 23, 15,  7, 14,  6,  5,  4,  3],
    [24, 32, 40, 48, 56, 49, 57, 58, 59, 60],
    [31, 39, 47, 55, 63, 54, 62, 61, 60, 59],
    // Edge+2X patterns (features 8-11, edge+X, S10 symmetry)
    [ 9,  0,  1,  2,  3,  4,  5,  6,  7, 14],
    [49, 56, 57, 58, 59, 60, 61, 62, 63, 54],
    [ 9,  0,  8, 16, 24, 32, 40, 48, 56, 49],
    [14,  7, 15, 23, 31, 39, 47, 55, 63, 54],
    // Edge+2X patterns (features 12-15, S10 symmetry)
    [ 0,  2,  3, 10, 11, 12, 13,  4,  5,  7],
    [56, 58, 59, 50, 51, 52, 53, 60, 61, 63],
    [ 0, 16, 24, 17, 25, 33, 41, 32, 40, 56],
    [ 7, 23, 31, 22, 30, 38, 46, 39, 47, 63],
    // Row/column 8-square patterns (features 16-19, S8 symmetry)
    [ 8,  9, 10, 11, 12, 13, 14, 15, -1, -1],
    [48, 49, 50, 51, 52, 53, 54, 55, -1, -1],
    [ 1,  9, 17, 25, 33, 41, 49, 57, -1, -1],
    [ 6, 14, 22, 30, 38, 46, 54, 62, -1, -1],
    // Row/column 8-square patterns (features 20-23, S8 symmetry)
    [16, 17, 18, 19, 20, 21, 22, 23, -1, -1],
    [40, 41, 42, 43, 44, 45, 46, 47, -1, -1],
    [ 2, 10, 18, 26, 34, 42, 50, 58, -1, -1],
    [ 5, 13, 21, 29, 37, 45, 53, 61, -1, -1],
    // Row/column 8-square patterns (features 24-27, S8 symmetry)
    [24, 25, 26, 27, 28, 29, 30, 31, -1, -1],
    [32, 33, 34, 35, 36, 37, 38, 39, -1, -1],
    [ 3, 11, 19, 27, 35, 43, 51, 59, -1, -1],
    [ 4, 12, 20, 28, 36, 44, 52, 60, -1, -1],
    // Main diagonal 8-square patterns (features 28-29, S8 symmetry)
    [ 0,  9, 18, 27, 36, 45, 54, 63, -1, -1],
    [56, 49, 42, 35, 28, 21, 14,  7, -1, -1],
    // 7-diagonals (features 30-33, S7 symmetry)
    [ 1, 10, 19, 28, 37, 46, 55, -1, -1, -1],
    [15, 22, 29, 36, 43, 50, 57, -1, -1, -1],
    [ 8, 17, 26, 35, 44, 53, 62, -1, -1, -1],
    [ 6, 13, 20, 27, 34, 41, 48, -1, -1, -1],
    // 6-diagonals (features 34-37, S6 symmetry)
    [ 2, 11, 20, 29, 38, 47, -1, -1, -1, -1],
    [16, 25, 34, 43, 52, 61, -1, -1, -1, -1],
    [ 5, 12, 19, 26, 33, 40, -1, -1, -1, -1],
    [23, 30, 37, 44, 51, 58, -1, -1, -1, -1],
    // 5-diagonals (features 38-41, S5 symmetry)
    [ 3, 12, 21, 30, 39, -1, -1, -1, -1, -1],
    [24, 33, 42, 51, 60, -1, -1, -1, -1, -1],
    [ 4, 11, 18, 25, 32, -1, -1, -1, -1, -1],
    [31, 38, 45, 52, 59, -1, -1, -1, -1, -1],
    // 4-diagonals (features 42-45, S4 symmetry)
    [ 3, 10, 17, 24, -1, -1, -1, -1, -1, -1],
    [32, 41, 50, 59, -1, -1, -1, -1, -1, -1],
    [ 4, 13, 22, 31, -1, -1, -1, -1, -1, -1],
    [39, 46, 53, 60, -1, -1, -1, -1, -1, -1],
    // Scalar (feature 46, always index 0)
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
];

// Magic numbers for eval.dat file header
const EDAX_HEADER: u32 = 0x45444158;
const EVAL_HEADER_MAGIC: u32 = 0x4556414C;
const XADE_HEADER: u32 = 0x58414445;

/// Loaded evaluation weights.
struct EvalWeights {
    /// Flat array indexed as weights[ply * EVAL_N_WEIGHT + index].
    weights: Vec<i16>,
}

/// Global evaluation weights, loaded once on first use.
static EVAL_WEIGHTS: OnceLock<Option<EvalWeights>> = OnceLock::new();

/// Compute the opponent feature: swap player (0) and opponent (1) in base-3 encoding.
/// Empty (2) stays the same.
fn opponent_feature(l: usize, d: usize) -> usize {
    let o = [1usize, 0, 2];
    let f = o[l % 3];
    if d > 1 {
        f + opponent_feature(l / 3, d - 1) * 3
    } else {
        f
    }
}

/// Build symmetry table for corner-9 patterns (C9).
/// Returns mapping from full index to packed index.
fn build_eval_c9() -> Vec<usize> {
    let mut table = vec![0usize; 19683];
    let mut n = 0usize;
    for l in 0..19683usize {
        let k = ((l / 6561) % 3) * 6561
            + ((l / 729) % 3) * 2187
            + ((l / 2187) % 3) * 729
            + ((l / 243) % 3) * 243
            + ((l / 27) % 3) * 81
            + ((l / 81) % 3) * 27
            + ((l / 3) % 3) * 9
            + ((l / 9) % 3) * 3
            + (l % 3);
        if k < l {
            table[l] = table[k];
        } else {
            table[l] = n;
            n += 1;
        }
    }
    table
}

/// Build symmetry table for angle+X 10-square patterns (C10).
fn build_eval_c10() -> Vec<usize> {
    let mut table = vec![0usize; 59049];
    let mut n = 0usize;
    for l in 0..59049usize {
        let k = ((l / 19683) % 3)
            + ((l / 6561) % 3) * 3
            + ((l / 2187) % 3) * 9
            + ((l / 729) % 3) * 27
            + ((l / 243) % 3) * 243
            + ((l / 81) % 3) * 81
            + ((l / 27) % 3) * 729
            + ((l / 9) % 3) * 2187
            + ((l / 3) % 3) * 6561
            + (l % 3) * 19683;
        if k < l {
            table[l] = table[k];
        } else {
            table[l] = n;
            n += 1;
        }
    }
    table
}

/// Build symmetry table for edge+X 10-square patterns (S10).
fn build_eval_s10() -> Vec<usize> {
    let mut table = vec![0usize; 59049];
    let mut n = 0usize;
    for l in 0..59049usize {
        let k = ((l / 19683) % 3)
            + ((l / 6561) % 3) * 3
            + ((l / 2187) % 3) * 9
            + ((l / 729) % 3) * 27
            + ((l / 243) % 3) * 81
            + ((l / 81) % 3) * 243
            + ((l / 27) % 3) * 729
            + ((l / 9) % 3) * 2187
            + ((l / 3) % 3) * 6561
            + (l % 3) * 19683;
        if k < l {
            table[l] = table[k];
        } else {
            table[l] = n;
            n += 1;
        }
    }
    table
}

/// Build symmetry table for 8-square symmetric patterns (S8).
fn build_eval_s8() -> Vec<usize> {
    let mut table = vec![0usize; 6561];
    let mut n = 0usize;
    for l in 0..6561usize {
        let k = ((l / 2187) % 3)
            + ((l / 729) % 3) * 3
            + ((l / 243) % 3) * 9
            + ((l / 81) % 3) * 27
            + ((l / 27) % 3) * 81
            + ((l / 9) % 3) * 243
            + ((l / 3) % 3) * 729
            + (l % 3) * 2187;
        if k < l {
            table[l] = table[k];
        } else {
            table[l] = n;
            n += 1;
        }
    }
    table
}

/// Build symmetry table for 7-square patterns (S7).
fn build_eval_s7() -> Vec<usize> {
    let mut table = vec![0usize; 2187];
    let mut n = 0usize;
    for l in 0..2187usize {
        let k = ((l / 729) % 3)
            + ((l / 243) % 3) * 3
            + ((l / 81) % 3) * 9
            + ((l / 27) % 3) * 27
            + ((l / 9) % 3) * 81
            + ((l / 3) % 3) * 243
            + (l % 3) * 729;
        if k < l {
            table[l] = table[k];
        } else {
            table[l] = n;
            n += 1;
        }
    }
    table
}

/// Build symmetry table for 6-square patterns (S6).
fn build_eval_s6() -> Vec<usize> {
    let mut table = vec![0usize; 729];
    let mut n = 0usize;
    for l in 0..729usize {
        let k = ((l / 243) % 3)
            + ((l / 81) % 3) * 3
            + ((l / 27) % 3) * 9
            + ((l / 9) % 3) * 27
            + ((l / 3) % 3) * 81
            + (l % 3) * 243;
        if k < l {
            table[l] = table[k];
        } else {
            table[l] = n;
            n += 1;
        }
    }
    table
}

/// Build symmetry table for 5-square patterns (S5).
fn build_eval_s5() -> Vec<usize> {
    let mut table = vec![0usize; 243];
    let mut n = 0usize;
    for l in 0..243usize {
        let k = ((l / 81) % 3)
            + ((l / 27) % 3) * 3
            + ((l / 9) % 3) * 9
            + ((l / 3) % 3) * 27
            + (l % 3) * 81;
        if k < l {
            table[l] = table[k];
        } else {
            table[l] = n;
            n += 1;
        }
    }
    table
}

/// Build symmetry table for 4-square patterns (S4).
fn build_eval_s4() -> Vec<usize> {
    let mut table = vec![0usize; 81];
    let mut n = 0usize;
    for l in 0..81usize {
        let k = ((l / 27) % 3) + ((l / 9) % 3) * 3 + ((l / 3) % 3) * 9 + (l % 3) * 27;
        if k < l {
            table[l] = table[k];
        } else {
            table[l] = n;
            n += 1;
        }
    }
    table
}

/// Try to find eval.dat in common locations.
fn find_eval_dat() -> Option<std::path::PathBuf> {
    // Check environment variable first
    if let Ok(path) = std::env::var("EDAX_EVAL") {
        if Path::new(&path).exists() {
            return Some(std::path::PathBuf::from(path));
        }
    }

    // Search common locations relative to the executable and working directory
    let candidates = [
        "eval.dat",
        "data/eval.dat",
        "../data/eval.dat",
        "../../data/eval.dat",
        "../../../data/eval.dat",
    ];

    for candidate in &candidates {
        let path = Path::new(candidate);
        if path.exists() {
            return Some(path.to_path_buf());
        }
    }

    None
}

/// Load evaluation weights from eval.dat.
fn load_eval(path: &Path) -> Result<EvalWeights, String> {
    use std::io::Read;

    let mut f = std::fs::File::open(path).map_err(|e| format!("Cannot open {:?}: {}", path, e))?;

    // Read header (28 bytes: 2 u32 magic, 3 u32 version info, 1 f64 date)
    let mut header_buf = [0u8; 28];
    f.read_exact(&mut header_buf)
        .map_err(|e| format!("Cannot read header: {}", e))?;

    let edax_magic = u32::from_le_bytes([header_buf[0], header_buf[1], header_buf[2], header_buf[3]]);
    let eval_magic = u32::from_le_bytes([header_buf[4], header_buf[5], header_buf[6], header_buf[7]]);

    let need_bswap = if (edax_magic == EDAX_HEADER && eval_magic == EVAL_HEADER_MAGIC)
        || (edax_magic == XADE_HEADER)
    {
        edax_magic == XADE_HEADER
    } else {
        return Err(format!(
            "Not a valid edax eval file (magic: {:08x} {:08x})",
            edax_magic, eval_magic
        ));
    };

    // Build symmetry tables for unpacking
    let sym_c9 = build_eval_c9();
    let sym_c10 = build_eval_c10();
    let sym_s10 = build_eval_s10();
    let sym_s8 = build_eval_s8();
    let sym_s7 = build_eval_s7();
    let sym_s6 = build_eval_s6();
    let sym_s5 = build_eval_s5();
    let sym_s4 = build_eval_s4();

    // Allocate weight storage
    let mut weights = vec![0i16; EVAL_N_PLY * EVAL_N_WEIGHT];

    // Read packed weights for each ply
    let mut packed_buf = vec![0u8; EVAL_N_PACKED * 2]; // each weight is an i16 (2 bytes)

    for ply in 0..EVAL_N_PLY {
        f.read_exact(&mut packed_buf)
            .map_err(|e| format!("Cannot read weights for ply {}: {}", ply, e))?;

        // Parse packed weights as i16 (little-endian)
        let mut w = vec![0i16; EVAL_N_PACKED];
        for i in 0..EVAL_N_PACKED {
            let lo = packed_buf[i * 2] as u16;
            let hi = packed_buf[i * 2 + 1] as u16;
            w[i] = if need_bswap {
                // Big-endian: swap bytes
                ((lo << 8) | hi) as i16
            } else {
                // Little-endian (normal)
                (lo | (hi << 8)) as i16
            };
        }

        // Unpack using symmetry tables
        // The unpacking order matches eval_open() in the C code exactly:
        // C9, C10, S10, S10, S8, S8, S8, S8, S7, S6, S5, S4, scalar

        let base = ply * EVAL_N_WEIGHT;
        let mut j = 0usize;
        let mut offset = 0usize;

        // Group 0: corner 3x3 (C9), size 19683, packed 10206
        for k in 0..EVAL_SIZE[0] {
            weights[base + j] = w[sym_c9[k] + offset];
            j += 1;
        }
        offset += EVAL_PACKED_SIZE[0];

        // Group 1: angle+X (C10), size 59049, packed 29889
        for k in 0..EVAL_SIZE[1] {
            weights[base + j] = w[sym_c10[k] + offset];
            j += 1;
        }
        offset += EVAL_PACKED_SIZE[1];

        // Group 2: edge+X (S10), size 59049, packed 29646
        for k in 0..EVAL_SIZE[2] {
            weights[base + j] = w[sym_s10[k] + offset];
            j += 1;
        }
        offset += EVAL_PACKED_SIZE[2];

        // Group 3: edge+X (S10), size 59049, packed 29646
        for k in 0..EVAL_SIZE[3] {
            weights[base + j] = w[sym_s10[k] + offset];
            j += 1;
        }
        offset += EVAL_PACKED_SIZE[3];

        // Groups 4-7: rows/cols/diags (S8), size 6561, packed 3321 each
        for _group in 4..8 {
            for k in 0..EVAL_SIZE[4] {
                weights[base + j] = w[sym_s8[k] + offset];
                j += 1;
            }
            offset += EVAL_PACKED_SIZE[4];
        }

        // Group 8: 7-diag (S7), size 2187, packed 1134
        for k in 0..EVAL_SIZE[8] {
            weights[base + j] = w[sym_s7[k] + offset];
            j += 1;
        }
        offset += EVAL_PACKED_SIZE[8];

        // Group 9: 6-diag (S6), size 729, packed 378
        for k in 0..EVAL_SIZE[9] {
            weights[base + j] = w[sym_s6[k] + offset];
            j += 1;
        }
        offset += EVAL_PACKED_SIZE[9];

        // Group 10: 5-diag (S5), size 243, packed 135
        for k in 0..EVAL_SIZE[10] {
            weights[base + j] = w[sym_s5[k] + offset];
            j += 1;
        }
        offset += EVAL_PACKED_SIZE[10];

        // Group 11: 4-diag (S4), size 81, packed 45
        for k in 0..EVAL_SIZE[11] {
            weights[base + j] = w[sym_s4[k] + offset];
            j += 1;
        }
        offset += EVAL_PACKED_SIZE[11];

        // Group 12: scalar, size 1, packed 1
        weights[base + j] = w[offset];
    }

    Ok(EvalWeights { weights })
}

/// Initialize evaluation weights (call early to avoid lazy-load overhead).
pub fn eval_open() {
    EVAL_WEIGHTS.get_or_init(|| {
        if let Some(path) = find_eval_dat() {
            match load_eval(&path) {
                Ok(w) => {
                    eprintln!("<Evaluation weights loaded from {:?}>", path);
                    Some(w)
                }
                Err(e) => {
                    eprintln!("Warning: {}", e);
                    None
                }
            }
        } else {
            None
        }
    });
}

/// Initialize evaluation weights from a specific file path.
pub fn eval_open_file(path: &str) {
    EVAL_WEIGHTS.get_or_init(|| {
        let p = Path::new(path);
        match load_eval(p) {
            Ok(w) => {
                eprintln!("<Evaluation weights loaded from {:?}>", p);
                Some(w)
            }
            Err(e) => {
                eprintln!("Warning: {}", e);
                None
            }
        }
    });
}

/// Get the square color from the board (matches C version's board_get_square_color).
/// Returns: 0 = current player, 1 = opponent, 2 = empty.
#[inline]
fn board_get_square_color(board: &Board, x: usize) -> usize {
    (2 - 2 * ((board.player >> x) & 1) - ((board.opponent >> x) & 1)) as usize
}

/// Compute the 47 feature indices from a board position.
/// Each feature index includes the EVAL_OFFSET so it can directly index the weight array.
#[inline]
fn eval_set(board: &Board) -> [usize; EVAL_N_FEATURE] {
    let mut features = [0usize; EVAL_N_FEATURE];
    for i in 0..EVAL_N_FEATURE {
        let n = EVAL_N_SQUARE[i];
        let mut f = 0usize;
        for j in 0..n {
            let sq = EVAL_F2X[i][j] as usize;
            let c = board_get_square_color(board, sq);
            f = f * 3 + c;
        }
        features[i] = f + EVAL_OFFSET[i];
    }
    features
}

/// Evaluate using the loaded pattern weights (C-compatible search_eval_0).
fn eval_pattern(board: &Board, eval_weights: &EvalWeights) -> i32 {
    let n_empties = board.empties();
    let ply = (60u32.saturating_sub(n_empties)) as usize;
    let ply = ply.min(EVAL_N_PLY - 1);

    let features = eval_set(board);
    let w = &eval_weights.weights[ply * EVAL_N_WEIGHT..];

    let mut score: i32 = 0;
    for i in 0..EVAL_N_FEATURE {
        score += w[features[i]] as i32;
    }

    if score > 0 {
        score += 64;
    } else {
        score -= 64;
    }
    score /= 128;

    score.clamp(-63, 63)
}

/// Evaluate a board position from the current player's perspective.
/// Returns a score in the range -64..+64.
///
/// Uses the edax-compatible pattern evaluation if weights are loaded,
/// otherwise falls back to a simple heuristic.
pub fn evaluate(board: &Board) -> i32 {
    let n_empties = board.empties() as i32;

    // Endgame: exact disc difference
    if n_empties == 0 {
        return board.score();
    }

    // Try pattern evaluation
    let weights = EVAL_WEIGHTS.get_or_init(|| {
        if let Some(path) = find_eval_dat() {
            match load_eval(&path) {
                Ok(w) => {
                    eprintln!("<Evaluation weights loaded from {:?}>", path);
                    Some(w)
                }
                Err(e) => {
                    eprintln!("Warning: {}", e);
                    None
                }
            }
        } else {
            None
        }
    });

    if let Some(w) = weights {
        return eval_pattern(board, w);
    }

    // Fallback: simple heuristic
    eval_heuristic(board)
}

/// Check whether pattern weights have been loaded.
pub fn is_loaded() -> bool {
    EVAL_WEIGHTS
        .get()
        .map_or(false, |opt| opt.is_some())
}

// ============================================================
// Fallback heuristic evaluation (used when eval.dat is unavailable)
// ============================================================

/// Positional weights for each square.
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

fn eval_heuristic(board: &Board) -> i32 {
    let n_empties = board.empties() as i32;

    if n_empties <= 8 {
        let positional = positional_score(board);
        let disc_score = board.score() * 4;
        return (positional + disc_score) / 5;
    }

    positional_score(board)
}

fn positional_score(board: &Board) -> i32 {
    let n_empties = board.empties() as i32;

    let mut piece_score = 0i32;
    let mut bits = board.player;
    while bits != 0 {
        let sq = bits.trailing_zeros() as usize;
        piece_score += SQUARE_WEIGHT[sq];
        bits &= bits - 1;
    }
    bits = board.opponent;
    while bits != 0 {
        let sq = bits.trailing_zeros() as usize;
        piece_score -= SQUARE_WEIGHT[sq];
        bits &= bits - 1;
    }

    let my_moves = board.get_moves().count_ones() as i32;
    let mut opp_board = *board;
    opp_board.pass();
    let opp_moves = opp_board.get_moves().count_ones() as i32;
    let mobility_score = (my_moves - opp_moves) * 10;

    let corners: u64 = (1u64 << 0) | (1u64 << 7) | (1u64 << 56) | (1u64 << 63);
    let my_corners = (board.player & corners).count_ones() as i32;
    let opp_corners = (board.opponent & corners).count_ones() as i32;
    let corner_score = (my_corners - opp_corners) * 25;

    let empty = !(board.player | board.opponent);
    let frontier = frontier_discs(board.player, empty);
    let opp_frontier = frontier_discs(board.opponent, empty);
    let frontier_score = (opp_frontier as i32 - frontier as i32) * 3;

    let phase_weight = if n_empties > 40 {
        (piece_score / 4) + mobility_score * 3 + corner_score + frontier_score
    } else if n_empties > 20 {
        (piece_score / 3) + mobility_score * 2 + corner_score * 2 + frontier_score
    } else {
        piece_score / 2 + mobility_score + corner_score * 3 + frontier_score
    };

    phase_weight.clamp(-63, 63)
}

fn frontier_discs(player: u64, empty: u64) -> u32 {
    let shifted = (empty << 1)
        | (empty >> 1)
        | (empty << 8)
        | (empty >> 8)
        | (empty << 7)
        | (empty >> 7)
        | (empty << 9)
        | (empty >> 9);
    (player & shifted).count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_initial_position_in_range() {
        let board = Board::new();
        let score = evaluate(&board);
        assert!(
            score >= -64 && score <= 64,
            "score {} out of range",
            score
        );
    }

    #[test]
    fn eval_full_board_is_disc_diff() {
        // Full board: exact disc difference
        let board = Board {
            player: 0x00000000FFFFFFFF,
            opponent: 0xFFFFFFFF00000000,
        };
        let score = evaluate(&board);
        assert_eq!(score, 0);
    }

    #[test]
    fn eval_winning_endgame_positive() {
        // Player has 40 discs, opponent has 24
        let board = Board {
            player: 0x00000000FFFFFFFF | (0xFFu64 << 32),
            opponent: 0xFFFFFF0000000000u64,
        };
        let score = evaluate(&board);
        assert!(score > 0, "winning position should be positive, got {}", score);
    }

    #[test]
    fn eval_symmetry_negation() {
        let board = Board::new();
        let mut board2 = board;
        board2.do_move(19); // D3
        let score1 = evaluate(&board2);
        let mut flipped = board2;
        std::mem::swap(&mut flipped.player, &mut flipped.opponent);
        let score2 = evaluate(&flipped);
        // With pattern eval, the +/-64 rounding and learned weight
        // asymmetry mean scores aren't perfectly anti-symmetric.
        assert!(
            (score1 + score2).abs() <= 10,
            "symmetry: {} + {} = {} (should be ~0)",
            score1,
            score2,
            score1 + score2
        );
    }

    #[test]
    fn board_get_square_color_values() {
        let board = Board::new();
        // Initial position: E4,D5 = player(0), D4,E5 = opponent(1), rest = empty(2)
        // D4 = square 27, E4 = square 28, D5 = square 35, E5 = square 36
        assert_eq!(board_get_square_color(&board, 28), 0); // E4 = player
        assert_eq!(board_get_square_color(&board, 35), 0); // D5 = player
        assert_eq!(board_get_square_color(&board, 27), 1); // D4 = opponent
        assert_eq!(board_get_square_color(&board, 36), 1); // E5 = opponent
        assert_eq!(board_get_square_color(&board, 0), 2); // A1 = empty
    }

    #[test]
    fn eval_set_feature_indices_in_range() {
        let board = Board::new();
        let features = eval_set(&board);
        for i in 0..EVAL_N_FEATURE {
            assert!(
                features[i] < EVAL_N_WEIGHT,
                "feature {} index {} out of range",
                i,
                features[i]
            );
        }
    }

    #[test]
    fn eval_set_all_empty_feature_is_max() {
        // All squares empty (no discs)
        let board = Board {
            player: 0,
            opponent: 0,
        };
        let features = eval_set(&board);
        // For a feature of size n, all-empty = 2*3^(n-1) + 2*3^(n-2) + ... + 2 = 3^n - 1
        // Feature 0 (9 squares): 3^9 - 1 = 19682 + offset 0 = 19682
        assert_eq!(features[0], 19682);
        // Feature 46 (scalar, 0 squares): 0 + offset 226314 = 226314
        assert_eq!(features[46], 226314);
    }

    #[test]
    fn symmetry_table_sizes() {
        let c9 = build_eval_c9();
        assert_eq!(c9.len(), 19683);
        // Max packed index should be EVAL_PACKED_SIZE[0] - 1 = 10205
        assert_eq!(*c9.iter().max().unwrap(), 10205);

        let c10 = build_eval_c10();
        assert_eq!(c10.len(), 59049);
        assert_eq!(*c10.iter().max().unwrap(), 29888);

        let s10 = build_eval_s10();
        assert_eq!(s10.len(), 59049);
        assert_eq!(*s10.iter().max().unwrap(), 29645);

        let s8 = build_eval_s8();
        assert_eq!(s8.len(), 6561);
        assert_eq!(*s8.iter().max().unwrap(), 3320);

        let s7 = build_eval_s7();
        assert_eq!(s7.len(), 2187);
        assert_eq!(*s7.iter().max().unwrap(), 1133);

        let s6 = build_eval_s6();
        assert_eq!(s6.len(), 729);
        assert_eq!(*s6.iter().max().unwrap(), 377);

        let s5 = build_eval_s5();
        assert_eq!(s5.len(), 243);
        assert_eq!(*s5.iter().max().unwrap(), 134);

        let s4 = build_eval_s4();
        assert_eq!(s4.len(), 81);
        assert_eq!(*s4.iter().max().unwrap(), 44);
    }

    #[test]
    fn opponent_feature_swap() {
        // Single digit: 0 -> 1, 1 -> 0, 2 -> 2
        assert_eq!(opponent_feature(0, 1), 1);
        assert_eq!(opponent_feature(1, 1), 0);
        assert_eq!(opponent_feature(2, 1), 2);

        // Two digits: (0,0)=0 -> (1,1)=4, (1,0)=1 -> (0,1)=3
        assert_eq!(opponent_feature(0, 2), 4); // 1*3 + 1
        assert_eq!(opponent_feature(1, 2), 3); // 1*3 + 0
    }

    #[test]
    fn eval_dat_loading() {
        // Try to load eval.dat if available
        if let Some(path) = find_eval_dat() {
            let weights = load_eval(&path);
            assert!(weights.is_ok(), "Failed to load eval.dat: {:?}", weights.err());
            let w = weights.unwrap();
            assert_eq!(w.weights.len(), EVAL_N_PLY * EVAL_N_WEIGHT);

            // Verify some weights are non-zero
            let mut non_zero = 0;
            for &v in &w.weights[..EVAL_N_WEIGHT] {
                if v != 0 {
                    non_zero += 1;
                }
            }
            assert!(non_zero > 0, "All weights for ply 0 are zero");
        }
    }

    #[test]
    fn eval_pattern_if_loaded() {
        if !is_loaded() {
            eval_open();
        }
        if is_loaded() {
            let board = Board::new();
            let score = evaluate(&board);
            // Initial position should evaluate close to 0
            assert!(
                score.abs() <= 10,
                "pattern eval of initial pos = {}, expected near 0",
                score
            );

            // After D3 (a common opening move), score should be in valid range
            let mut board2 = board;
            board2.do_move(19);
            let score2 = evaluate(&board2);
            assert!(
                score2 >= -63 && score2 <= 63,
                "pattern eval after D3 = {}, out of range",
                score2
            );
        }
    }
}
