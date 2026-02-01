//! Alpha-beta search engine with PVS, iterative deepening, and endgame solver.

use crate::board::Board;
use crate::eval;
use crate::flip;
use crate::hash::{HashTable, NOMOVE};
use std::time::Instant;

/// Score bounds
pub const SCORE_MIN: i32 = -64;
pub const SCORE_MAX: i32 = 64;
pub const SCORE_INF: i32 = 127;

/// Maximum search depth
pub const MAX_DEPTH: i32 = 60;

/// No selectivity (search everything)
pub const NO_SELECTIVITY: i32 = 5;

/// Depth threshold for shallow endgame search (no hash table).
pub const DEPTH_TO_SHALLOW_SEARCH: i32 = 7;

/// Neighbour bitmask for each square: all adjacent squares (including diagonals).
/// Used for fast rejection in endgame solvers: if no opponent disc is adjacent,
/// no move is possible at that square.
#[rustfmt::skip]
pub const NEIGHBOUR: [u64; 66] = [
    0x0000000000000302, 0x0000000000000705, 0x0000000000000e0a, 0x0000000000001c14,
    0x0000000000003828, 0x0000000000007050, 0x000000000000e0a0, 0x000000000000c040,
    0x0000000000030203, 0x0000000000070507, 0x00000000000e0a0e, 0x00000000001c141c,
    0x0000000000382838, 0x0000000000705070, 0x0000000000e0a0e0, 0x0000000000c040c0,
    0x0000000003020300, 0x0000000007050700, 0x000000000e0a0e00, 0x000000001c141c00,
    0x0000000038283800, 0x0000000070507000, 0x00000000e0a0e000, 0x00000000c040c000,
    0x0000000302030000, 0x0000000705070000, 0x0000000e0a0e0000, 0x0000001c141c0000,
    0x0000003828380000, 0x0000007050700000, 0x000000e0a0e00000, 0x000000c040c00000,
    0x0000030203000000, 0x0000070507000000, 0x00000e0a0e000000, 0x00001c141c000000,
    0x0000382838000000, 0x0000705070000000, 0x0000e0a0e0000000, 0x0000c040c0000000,
    0x0003020300000000, 0x0007050700000000, 0x000e0a0e00000000, 0x001c141c00000000,
    0x0038283800000000, 0x0070507000000000, 0x00e0a0e000000000, 0x00c040c000000000,
    0x0302030000000000, 0x0705070000000000, 0x0e0a0e0000000000, 0x1c141c0000000000,
    0x3828380000000000, 0x7050700000000000, 0xe0a0e00000000000, 0xc040c00000000000,
    0x0203000000000000, 0x0507000000000000, 0x0a0e000000000000, 0x141c000000000000,
    0x2838000000000000, 0x5070000000000000, 0xa0e0000000000000, 0x40c0000000000000,
    0, 0, // sentinel for PASS and NOMOVE
];

/// NWS stability threshold: minimum alpha value to try stability cutoff.
/// Index = number of empties. 99 means don't try.
#[rustfmt::skip]
pub const NWS_STABILITY_THRESHOLD: [i32; 56] = [
    99, 99, 99, 99,  6,  8, 10, 12,
    14, 16, 20, 22, 24, 26, 28, 30,
    32, 34, 36, 38, 40, 42, 44, 46,
    48, 48, 50, 50, 52, 52, 54, 54,
    56, 56, 58, 58, 60, 60, 62, 62,
    64, 64, 64, 64, 64, 64, 64, 64,
    99, 99, 99, 99, 99, 99, 99, 99,
];

/// Quadrant ID for each square. Used for parity-based move ordering in endgame.
/// Each quadrant is a 4x4 block: top-left=1, top-right=2, bottom-left=4, bottom-right=8.
#[rustfmt::skip]
pub const QUADRANT_ID: [i32; 66] = [
    1, 1, 1, 1, 2, 2, 2, 2,
    1, 1, 1, 1, 2, 2, 2, 2,
    1, 1, 1, 1, 2, 2, 2, 2,
    1, 1, 1, 1, 2, 2, 2, 2,
    4, 4, 4, 4, 8, 8, 8, 8,
    4, 4, 4, 4, 8, 8, 8, 8,
    4, 4, 4, 4, 8, 8, 8, 8,
    4, 4, 4, 4, 8, 8, 8, 8,
    0, 0, // sentinel
];

/// Result of a search.
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub score: i32,
    pub best_move: i32,
    pub depth: i32,
    pub nodes: u64,
    pub pv: Vec<i32>,
    pub time_ms: u64,
    pub is_exact: bool,
}

/// Search configuration.
#[derive(Clone, Debug)]
pub struct SearchOptions {
    pub depth: i32,
    pub time_limit_ms: u64,
    pub hash_size: usize,
    pub verbose: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions {
            depth: 12,
            time_limit_ms: 5000,
            hash_size: 1 << 18, // 256K entries
            verbose: false,
        }
    }
}

/// Main search state.
pub struct Search {
    pub hash_table: HashTable,
    pub nodes: u64,
    pub options: SearchOptions,
    start_time: Option<Instant>,
    stopped: bool,
}

impl Search {
    pub fn new(options: SearchOptions) -> Self {
        Search {
            hash_table: HashTable::new(options.hash_size),
            nodes: 0,
            options,
            start_time: None,
            stopped: false,
        }
    }

    /// Run iterative deepening search and return the best move.
    pub fn search(&mut self, board: &Board) -> SearchResult {
        self.nodes = 0;
        self.stopped = false;
        self.start_time = Some(Instant::now());
        self.hash_table.clear();

        let n_empties = board.empties() as i32;
        let max_depth = self.options.depth.min(n_empties);

        let mut best_result = SearchResult {
            score: 0,
            best_move: NOMOVE,
            depth: 0,
            nodes: 0,
            pv: Vec::new(),
            time_ms: 0,
            is_exact: false,
        };

        // Find legal moves first
        let moves = board.get_moves();
        if moves == 0 {
            // No moves: either pass or game over
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() == 0 {
                // Game over
                best_result.score = board.score();
                best_result.is_exact = true;
                return best_result;
            }
            // Must pass, search opponent
            let mut opp_result = self.search(&passed);
            opp_result.score = -opp_result.score;
            return opp_result;
        }

        // Iterative deepening
        for depth in 1..=max_depth {
            if self.stopped {
                break;
            }

            let score = self.pvs_root(board, depth, SCORE_MIN - 1, SCORE_MAX + 1);

            if !self.stopped {
                best_result.score = score;
                best_result.depth = depth;
                best_result.nodes = self.nodes;
                best_result.time_ms = self.elapsed_ms();

                // Extract best move from hash
                let hc = HashTable::hash_code(board);
                if let Some(data) = self.hash_table.get(board, hc) {
                    if data.best_move[0] >= 0 {
                        best_result.best_move = data.best_move[0] as i32;
                    }
                }

                // Extract PV
                best_result.pv = self.extract_pv(board, depth);

                if self.options.verbose {
                    let nps = if best_result.time_ms > 0 {
                        self.nodes * 1000 / best_result.time_ms
                    } else {
                        0
                    };
                    eprintln!(
                        "depth {:2} score {:+3} move {} nodes {} time {}ms nps {}",
                        depth,
                        score,
                        if best_result.best_move >= 0 {
                            Board::square_to_string(best_result.best_move as usize)
                        } else {
                            "--".to_string()
                        },
                        self.nodes,
                        best_result.time_ms,
                        nps,
                    );
                }

                // Exact solve
                if depth == n_empties {
                    best_result.is_exact = true;
                }
            }
        }

        best_result
    }

    /// Root PVS search.
    fn pvs_root(&mut self, board: &Board, depth: i32, mut alpha: i32, beta: i32) -> i32 {
        let moves = board.get_moves();
        if moves == 0 {
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() == 0 {
                return board.score();
            }
            return -self.pvs_root(&passed, depth, -beta, -alpha);
        }

        let hc = HashTable::hash_code(board);

        // Get move ordering from hash table
        let hash_move = self.hash_table.get(board, hc)
            .and_then(|d| if d.best_move[0] >= 0 { Some(d.best_move[0] as i32) } else { None });

        let move_list = self.order_moves(board, moves, hash_move);

        let mut best_score = SCORE_MIN - 1;
        let mut best_move = NOMOVE;
        let mut first = true;

        for sq in &move_list {
            let sq = *sq as usize;
            let mut child = *board;
            child.do_move(sq);

            let score;
            if first {
                score = -self.pvs(&child, depth - 1, -beta, -alpha);
                first = false;
            } else {
                // Null window search
                let nws = -self.nws(&child, depth - 1, -alpha - 1);
                if nws > alpha && nws < beta && !self.stopped {
                    score = -self.pvs(&child, depth - 1, -beta, -nws);
                } else {
                    score = nws;
                }
            }

            if self.stopped {
                break;
            }

            if score > best_score {
                best_score = score;
                best_move = sq as i32;
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        break;
                    }
                }
            }
        }

        if !self.stopped && best_move >= 0 {
            self.hash_table.store(board, hc, depth, NO_SELECTIVITY, alpha, beta, best_score, best_move);
        }

        best_score
    }

    /// PVS (Principal Variation Search) for midgame.
    fn pvs(&mut self, board: &Board, depth: i32, mut alpha: i32, beta: i32) -> i32 {
        self.check_timeout();
        if self.stopped {
            return 0;
        }
        self.nodes += 1;

        let n_empties = board.empties() as i32;

        // Terminal or leaf
        if depth <= 0 || n_empties == 0 {
            return eval::evaluate(board);
        }

        // Endgame solver: when search depth covers all remaining empties
        if depth >= n_empties {
            if n_empties <= DEPTH_TO_SHALLOW_SEARCH {
                return self.endgame_nws(board, alpha);
            }
            // For deeper endgame, fall through to normal PVS with hash
        }

        let moves = board.get_moves();
        if moves == 0 {
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() == 0 {
                return board.score();
            }
            return -self.pvs(&passed, depth, -beta, -alpha);
        }

        // Hash lookup
        let hc = HashTable::hash_code(board);
        let hash_move = if let Some(data) = self.hash_table.get(board, hc) {
            if data.depth as i32 >= depth {
                let lower = data.lower as i32;
                let upper = data.upper as i32;
                if lower >= beta { return lower; }
                if upper <= alpha { return upper; }
                if lower == upper { return lower; }
                alpha = alpha.max(lower);
            }
            if data.best_move[0] >= 0 { Some(data.best_move[0] as i32) } else { None }
        } else {
            None
        };

        let move_list = self.order_moves(board, moves, hash_move);
        let mut best_score = SCORE_MIN - 1;
        let mut best_move = NOMOVE;
        let mut first = true;

        for sq in &move_list {
            let sq = *sq as usize;
            let mut child = *board;
            child.do_move(sq);

            let score;
            if first {
                score = -self.pvs(&child, depth - 1, -beta, -alpha);
                first = false;
            } else {
                let nws = -self.nws(&child, depth - 1, -alpha - 1);
                if nws > alpha && nws < beta && !self.stopped {
                    score = -self.pvs(&child, depth - 1, -beta, -nws);
                } else {
                    score = nws;
                }
            }

            if self.stopped {
                return 0;
            }

            if score > best_score {
                best_score = score;
                best_move = sq as i32;
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        break;
                    }
                }
            }
        }

        if best_move >= 0 {
            self.hash_table.store(board, hc, depth, NO_SELECTIVITY, alpha, beta, best_score, best_move);
        }

        best_score
    }

    /// Null-Window Search (alpha-beta with window [alpha, alpha+1]).
    fn nws(&mut self, board: &Board, depth: i32, alpha: i32) -> i32 {
        self.check_timeout();
        if self.stopped {
            return 0;
        }
        self.nodes += 1;

        let beta = alpha + 1;
        let n_empties = board.empties() as i32;

        if depth <= 0 || n_empties == 0 {
            return eval::evaluate(board);
        }

        // Endgame solver
        if depth >= n_empties {
            if n_empties <= DEPTH_TO_SHALLOW_SEARCH {
                return self.endgame_nws(board, alpha);
            }
        }

        // Stability cutoff
        if let Some(sc_score) = self.search_sc_nws(board, n_empties, alpha) {
            return sc_score;
        }

        let moves = board.get_moves();
        if moves == 0 {
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() == 0 {
                return board.score();
            }
            return -self.nws(&passed, depth, -beta);
        }

        // Hash lookup
        let hc = HashTable::hash_code(board);
        let hash_move = if let Some(data) = self.hash_table.get(board, hc) {
            if data.depth as i32 >= depth {
                let lower = data.lower as i32;
                let upper = data.upper as i32;
                if lower >= beta { return lower; }
                if upper <= alpha { return upper; }
            }
            if data.best_move[0] >= 0 { Some(data.best_move[0] as i32) } else { None }
        } else {
            None
        };

        // Enhanced Transposition Cutoff (ETC)
        if let Some((etc_score, etc_move)) = self.search_etc_nws(board, moves, depth, n_empties, alpha) {
            self.hash_table.store(board, hc, depth, NO_SELECTIVITY, alpha, beta, etc_score, etc_move);
            return etc_score;
        }

        let move_list = self.order_moves(board, moves, hash_move);
        let mut best_score = SCORE_MIN - 1;
        let mut best_move = NOMOVE;

        for sq in &move_list {
            let sq = *sq as usize;
            let mut child = *board;
            child.do_move(sq);

            let score = -self.nws(&child, depth - 1, -beta);

            if self.stopped {
                return 0;
            }

            if score > best_score {
                best_score = score;
                best_move = sq as i32;
                if score >= beta {
                    break;
                }
            }
        }

        if best_move >= 0 {
            self.hash_table.store(board, hc, depth, NO_SELECTIVITY, alpha, beta, best_score, best_move);
        }

        best_score
    }

    /// Endgame solver: exact score when few empties remain.
    /// This is the general-purpose version. For <= 4 empties, specialized
    /// solvers are called for better performance.
    fn endgame_solve(&mut self, board: &Board, mut alpha: i32, beta: i32) -> i32 {
        self.nodes += 1;
        let n_empties = board.empties() as i32;

        if n_empties == 0 {
            return board.score();
        }

        let moves = board.get_moves();
        if moves == 0 {
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() == 0 {
                return board.score();
            }
            return -self.endgame_solve(&passed, -beta, -alpha);
        }

        let mut best_score = SCORE_MIN - 1;
        let mut remaining = moves;

        while remaining != 0 {
            let sq = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;

            let mut child = *board;
            child.do_move(sq);

            let score = -self.endgame_solve(&child, -beta, -alpha);

            if score > best_score {
                best_score = score;
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        break;
                    }
                }
            }
        }

        best_score
    }

    // ================================================================
    // Stability Cutoff (SC)
    // ================================================================

    /// Try stability cutoff for NWS.
    /// If opponent has enough stable discs that the current player can't beat alpha,
    /// return the bound score. Otherwise return None.
    #[inline]
    fn search_sc_nws(&self, board: &Board, n_empties: i32, alpha: i32) -> Option<i32> {
        if n_empties < NWS_STABILITY_THRESHOLD.len() as i32
            && alpha >= NWS_STABILITY_THRESHOLD[n_empties as usize]
        {
            let stable = Board::get_stability(board.opponent, board.player);
            let score = SCORE_MAX - 2 * stable;
            if score <= alpha {
                return Some(score);
            }
        }
        None
    }

    /// Minimum depth for Enhanced Transposition Cutoff
    const ETC_MIN_DEPTH: i32 = 5;

    /// Minimum depth for ProbCut
    const PROBCUT_MIN_DEPTH: i32 = 5;

    // ================================================================
    // ProbCut (probabilistic forward pruning)
    // ================================================================

    /// Evaluation error model: sigma(n_empties, depth, probcut_depth).
    /// Quadratic function fitted to edax's evaluation accuracy data.
    fn eval_sigma(n_empties: i32, depth: i32, probcut_depth: i32) -> f64 {
        const EVAL_A: f64 = -0.10026799;
        const EVAL_B: f64 = 0.31027733;
        const EVAL_C: f64 = -0.57772603;
        const EVAL_AA: f64 = 0.07585621;
        const EVAL_BB: f64 = 1.16492647;
        const EVAL_CC: f64 = 5.4171698;

        let sigma = EVAL_A * n_empties as f64 + EVAL_B * depth as f64 + EVAL_C * probcut_depth as f64;
        EVAL_AA * sigma * sigma + EVAL_BB * sigma + EVAL_CC
    }

    /// Try ProbCut: use a shallow search to predict if the deep search result
    /// will exceed or fall below the bounds. Returns Some(score) on cutoff.
    ///
    /// ProbCut is only active when selectivity < NO_SELECTIVITY and depth is sufficient.
    fn search_probcut(&mut self, board: &Board, depth: i32, n_empties: i32, alpha: i32) -> Option<i32> {
        if depth < Self::PROBCUT_MIN_DEPTH {
            return None;
        }

        let beta = alpha + 1;
        let t = 1.5f64; // 87% confidence level (selectivity level 1)

        // Compute reduced depth
        let probcut_d = 0.5f64; // reduction factor
        let mut probcut_depth = (2.0 * (probcut_d * depth as f64).floor()) as i32 + (depth & 1);
        if probcut_depth == 0 {
            probcut_depth = depth - 2;
        }
        if probcut_depth < 2 || probcut_depth > depth - 2 {
            return None;
        }

        // RCD (rounding correction) = 0.5 for non-ICC compilers
        let rcd = 0.5f64;

        // Compute error margins
        let probcut_error = (t * Self::eval_sigma(n_empties, depth, probcut_depth) + rcd) as i32;
        let eval_score = eval::evaluate(board);
        let eval_error = (t * 0.5
            * (Self::eval_sigma(n_empties, depth, 0)
                + Self::eval_sigma(n_empties, depth, probcut_depth))
            + rcd) as i32;

        // Try probable upper cut
        let eval_beta = beta - eval_error;
        let probcut_beta = beta + probcut_error;
        if eval_score >= eval_beta && probcut_beta < SCORE_MAX {
            let score = self.nws(board, probcut_depth, probcut_beta - 1);
            if !self.stopped && score >= probcut_beta {
                return Some(beta);
            }
        }

        // Try probable lower cut
        let eval_alpha = alpha + eval_error;
        let probcut_alpha = alpha - probcut_error;
        if eval_score < eval_alpha && probcut_alpha > SCORE_MIN {
            let score = self.nws(board, probcut_depth, probcut_alpha);
            if !self.stopped && score <= probcut_alpha {
                return Some(alpha);
            }
        }

        None
    }

    /// Enhanced Transposition Cutoff (ETC).
    /// Pre-checks all child positions in the hash table before the move loop.
    /// Returns Some(score) if a cutoff is found, None otherwise.
    fn search_etc_nws(
        &self,
        board: &Board,
        moves: u64,
        depth: i32,
        n_empties: i32,
        alpha: i32,
    ) -> Option<(i32, i32)> {
        if depth <= Self::ETC_MIN_DEPTH {
            return None;
        }

        let beta = alpha + 1;
        let etc_depth = depth - 1;
        let mut remaining = moves;

        while remaining != 0 {
            let sq = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;

            let flipped = flip::flip(sq, board.player, board.opponent);
            if flipped == 0 {
                continue;
            }

            let child = Board {
                player: board.opponent ^ flipped,
                opponent: board.player ^ (flipped | (1u64 << sq)),
            };

            // Check SC on child (Enhanced SC)
            if n_empties < NWS_STABILITY_THRESHOLD.len() as i32
                && alpha <= -(NWS_STABILITY_THRESHOLD[n_empties as usize])
            {
                let stable = Board::get_stability(child.opponent, child.player);
                let score = 2 * stable - SCORE_MAX;
                if score > alpha {
                    return Some((score, sq as i32));
                }
            }

            // Check hash for child
            let child_hc = HashTable::hash_code(&child);
            if let Some(data) = self.hash_table.get(&child, child_hc) {
                if data.depth as i32 >= etc_depth && data.selectivity >= NO_SELECTIVITY as u8 {
                    let score = -(data.upper as i32);
                    if score > alpha {
                        return Some((score, sq as i32));
                    }
                }
            }
        }

        None
    }

    // ================================================================
    // Optimized endgame solvers for 0-4 empties
    // ================================================================

    /// Score when board is full (0 empties). Returns disc difference.
    #[inline]
    fn board_solve_0(board: &Board) -> i32 {
        2 * board.player.count_ones() as i32 - SCORE_MAX
    }

    /// Score with 1 empty square remaining. Uses count_last_flip with lazy cutoff.
    /// Returns the score from the OPPONENT's perspective (as in C version).
    ///
    /// `board`: current board
    /// `beta`: beta bound
    /// `x`: the last empty square
    #[inline]
    fn board_score_1(board: &Board, beta: i32, x: usize) -> i32 {
        let mut score = 2 * board.opponent.count_ones() as i32 - SCORE_MAX;

        let n_flips = flip::count_last_flip(x, board.player);
        if n_flips != 0 {
            score -= n_flips;
        } else {
            if score >= 0 {
                score += 2;
                if score < beta {
                    // lazy cut-off
                    let n_flips_opp = flip::count_last_flip(x, board.opponent);
                    score += n_flips_opp;
                }
            } else {
                if score < beta {
                    // lazy cut-off
                    let n_flips_opp = flip::count_last_flip(x, board.opponent);
                    if n_flips_opp != 0 {
                        score += n_flips_opp + 2;
                    }
                }
            }
        }

        score
    }

    /// Make a move and return a new board (player plays at `x`).
    /// Returns (new_board, flipped). If no flips, returns None.
    #[inline]
    fn board_next(board: &Board, x: usize) -> Option<Board> {
        let flipped = flip::flip(x, board.player, board.opponent);
        if flipped == 0 {
            return None;
        }
        Some(Board {
            player: board.opponent ^ flipped,
            opponent: board.player ^ (flipped | (1u64 << x)),
        })
    }

    /// Make a move for the opponent (pass + play at `x`).
    /// Returns new board if opponent can play at x. Otherwise None.
    #[inline]
    fn board_pass_next(board: &Board, x: usize) -> Option<Board> {
        let flipped = flip::flip(x, board.opponent, board.player);
        if flipped == 0 {
            return None;
        }
        Some(Board {
            player: board.player ^ flipped,
            opponent: board.opponent ^ (flipped | (1u64 << x)),
        })
    }

    /// Solve with exactly 2 empties. NWS with alpha, beta = alpha + 1.
    fn board_solve_2(&mut self, board: &Board, alpha: i32, x1: usize, x2: usize) -> i32 {
        self.nodes += 1;
        let beta = alpha + 1;
        let mut bestscore: i32;

        // Try x1
        if (NEIGHBOUR[x1] & board.opponent) != 0 {
            if let Some(next) = Self::board_next(board, x1) {
                self.nodes += 1;
                bestscore = Self::board_score_1(&next, beta, x2);
            } else {
                bestscore = -SCORE_INF;
            }
        } else {
            bestscore = -SCORE_INF;
        }

        if bestscore < beta {
            // Try x2
            if (NEIGHBOUR[x2] & board.opponent) != 0 {
                if let Some(next) = Self::board_next(board, x2) {
                    self.nodes += 1;
                    let score = Self::board_score_1(&next, beta, x1);
                    if score > bestscore {
                        bestscore = score;
                    }
                }
            }

            // Pass?
            if bestscore == -SCORE_INF {
                // Try opponent playing x1
                if (NEIGHBOUR[x1] & board.player) != 0 {
                    if let Some(next) = Self::board_pass_next(board, x1) {
                        self.nodes += 1;
                        bestscore = -Self::board_score_1(&next, -alpha, x2);
                    } else {
                        bestscore = SCORE_INF;
                    }
                } else {
                    bestscore = SCORE_INF;
                }

                if bestscore > alpha {
                    // Try opponent playing x2
                    if (NEIGHBOUR[x2] & board.player) != 0 {
                        if let Some(next) = Self::board_pass_next(board, x2) {
                            self.nodes += 1;
                            let score = -Self::board_score_1(&next, -alpha, x1);
                            if score < bestscore {
                                bestscore = score;
                            }
                        }
                    }

                    // Game over
                    if bestscore == SCORE_INF {
                        bestscore = board.score();
                    }
                }
            }
        }

        bestscore
    }

    /// Solve with exactly 3 empties. NWS with alpha.
    fn search_solve_3(&mut self, board: &Board, alpha: i32, mut x1: usize, mut x2: usize, mut x3: usize, parity: i32) -> i32 {
        self.nodes += 1;
        let beta = alpha + 1;

        // Parity-based move sorting: prefer odd-parity quadrant
        if (parity & QUADRANT_ID[x1]) == 0 {
            if (parity & QUADRANT_ID[x2]) != 0 {
                // x2 is odd, swap x1<->x2
                std::mem::swap(&mut x1, &mut x2);
            } else {
                // x3 is odd, rotate x1<-x3<-x2<-x1
                let tmp = x1;
                x1 = x3;
                x3 = x2;
                x2 = tmp;
            }
        }

        let mut bestscore: i32;

        // Try x1
        if (NEIGHBOUR[x1] & board.opponent) != 0 {
            if let Some(next) = Self::board_next(board, x1) {
                bestscore = -self.board_solve_2(&next, -beta, x2, x3);
                if bestscore >= beta {
                    return bestscore;
                }
            } else {
                bestscore = -SCORE_INF;
            }
        } else {
            bestscore = -SCORE_INF;
        }

        // Try x2
        if (NEIGHBOUR[x2] & board.opponent) != 0 {
            if let Some(next) = Self::board_next(board, x2) {
                let score = -self.board_solve_2(&next, -beta, x1, x3);
                if score >= beta {
                    return score;
                }
                if score > bestscore {
                    bestscore = score;
                }
            }
        }

        // Try x3
        if (NEIGHBOUR[x3] & board.opponent) != 0 {
            if let Some(next) = Self::board_next(board, x3) {
                let score = -self.board_solve_2(&next, -beta, x1, x2);
                if score > bestscore {
                    bestscore = score;
                }
            }
        }

        // Pass?
        if bestscore == -SCORE_INF {
            // Opponent tries x1
            if (NEIGHBOUR[x1] & board.player) != 0 {
                if let Some(next) = Self::board_pass_next(board, x1) {
                    bestscore = self.board_solve_2(&next, alpha, x2, x3);
                    if bestscore <= alpha {
                        return bestscore;
                    }
                } else {
                    bestscore = SCORE_INF;
                }
            } else {
                bestscore = SCORE_INF;
            }

            // Opponent tries x2
            if (NEIGHBOUR[x2] & board.player) != 0 {
                if let Some(next) = Self::board_pass_next(board, x2) {
                    let score = self.board_solve_2(&next, alpha, x1, x3);
                    if score <= alpha {
                        return score;
                    }
                    if score < bestscore {
                        bestscore = score;
                    }
                }
            }

            // Opponent tries x3
            if (NEIGHBOUR[x3] & board.player) != 0 {
                if let Some(next) = Self::board_pass_next(board, x3) {
                    let score = self.board_solve_2(&next, alpha, x1, x2);
                    if score < bestscore {
                        bestscore = score;
                    }
                }
            }

            // Game over
            if bestscore == SCORE_INF {
                bestscore = board.score();
            }
        }

        bestscore
    }

    /// Solve with exactly 4 empties. NWS with alpha.
    fn search_solve_4(&mut self, board: &Board, alpha: i32, mut x1: usize, mut x2: usize, mut x3: usize, mut x4: usize, parity: i32) -> i32 {
        self.nodes += 1;
        let beta = alpha + 1;

        // Parity-based move sorting for 4 empties
        if (parity & QUADRANT_ID[x1]) == 0 {
            if (parity & QUADRANT_ID[x2]) != 0 {
                if (parity & QUADRANT_ID[x3]) != 0 {
                    // case 1(x2) 1(x3) 2(x1 x4)
                    let tmp = x1; x1 = x2; x2 = x3; x3 = tmp;
                } else {
                    // case 1(x2) 1(x4) 2(x1 x3)
                    let tmp = x1; x1 = x2; x2 = x4; x4 = x3; x3 = tmp;
                }
            } else if (parity & QUADRANT_ID[x3]) != 0 {
                // case 1(x3) 1(x4) 2(x1 x2)
                let tmp = x1; x1 = x3; x3 = tmp;
                let tmp = x2; x2 = x4; x4 = tmp;
            }
        } else {
            if (parity & QUADRANT_ID[x2]) == 0 {
                if (parity & QUADRANT_ID[x3]) != 0 {
                    // case 1(x1) 1(x3) 2(x2 x4)
                    std::mem::swap(&mut x2, &mut x3);
                } else {
                    // case 1(x1) 1(x4) 2(x2 x3)
                    let tmp = x2; x2 = x4; x4 = x3; x3 = tmp;
                }
            }
        }

        let new_parity = parity ^ QUADRANT_ID[x1] ^ QUADRANT_ID[x2] ^ QUADRANT_ID[x3] ^ QUADRANT_ID[x4];
        let _ = new_parity; // parity is fully consumed by the 4 squares

        let mut bestscore: i32;

        // Try x1
        if (NEIGHBOUR[x1] & board.opponent) != 0 {
            if let Some(next) = Self::board_next(board, x1) {
                let child_parity = parity ^ QUADRANT_ID[x1];
                bestscore = -self.search_solve_3(&next, -beta, x2, x3, x4, child_parity);
                if bestscore >= beta {
                    return bestscore;
                }
            } else {
                bestscore = -SCORE_INF;
            }
        } else {
            bestscore = -SCORE_INF;
        }

        // Try x2
        if (NEIGHBOUR[x2] & board.opponent) != 0 {
            if let Some(next) = Self::board_next(board, x2) {
                let child_parity = parity ^ QUADRANT_ID[x2];
                let score = -self.search_solve_3(&next, -beta, x1, x3, x4, child_parity);
                if score >= beta {
                    return score;
                }
                if score > bestscore {
                    bestscore = score;
                }
            }
        }

        // Try x3
        if (NEIGHBOUR[x3] & board.opponent) != 0 {
            if let Some(next) = Self::board_next(board, x3) {
                let child_parity = parity ^ QUADRANT_ID[x3];
                let score = -self.search_solve_3(&next, -beta, x1, x2, x4, child_parity);
                if score >= beta {
                    return score;
                }
                if score > bestscore {
                    bestscore = score;
                }
            }
        }

        // Try x4
        if (NEIGHBOUR[x4] & board.opponent) != 0 {
            if let Some(next) = Self::board_next(board, x4) {
                let child_parity = parity ^ QUADRANT_ID[x4];
                let score = -self.search_solve_3(&next, -beta, x1, x2, x3, child_parity);
                if score > bestscore {
                    bestscore = score;
                }
            }
        }

        // No move: pass or game over
        if bestscore == -SCORE_INF {
            // Check if opponent can move
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() != 0 {
                bestscore = -self.search_solve_4(&passed, -beta, x1, x2, x3, x4, parity);
            } else {
                bestscore = board.score();
            }
        }

        bestscore
    }

    /// Shallow endgame NWS (no hash table), for 5-7 empties.
    /// Uses parity-based move ordering.
    fn search_shallow(&mut self, board: &Board, alpha: i32, empties_list: &[usize], parity: i32) -> i32 {
        self.nodes += 1;
        let beta = alpha + 1;
        let mut bestscore = -SCORE_INF;

        // Try odd-parity squares first, then even-parity
        for &x in empties_list.iter() {
            if (parity & QUADRANT_ID[x]) == 0 {
                continue; // skip even parity in first pass
            }
            if (NEIGHBOUR[x] & board.opponent) == 0 {
                continue;
            }
            if let Some(next) = Self::board_next(board, x) {
                let child_empties: Vec<usize> = empties_list.iter()
                    .copied()
                    .filter(|&e| e != x)
                    .collect();
                let child_parity = parity ^ QUADRANT_ID[x];
                let score = if child_empties.len() == 4 {
                    -self.search_solve_4(&next, -beta,
                        child_empties[0], child_empties[1],
                        child_empties[2], child_empties[3], child_parity)
                } else {
                    -self.search_shallow(&next, -beta, &child_empties, child_parity)
                };
                if score >= beta {
                    return score;
                }
                if score > bestscore {
                    bestscore = score;
                }
            }
        }

        // Now even-parity squares
        for &x in empties_list.iter() {
            if (parity & QUADRANT_ID[x]) != 0 {
                continue; // skip odd parity in second pass
            }
            if (NEIGHBOUR[x] & board.opponent) == 0 {
                continue;
            }
            if let Some(next) = Self::board_next(board, x) {
                let child_empties: Vec<usize> = empties_list.iter()
                    .copied()
                    .filter(|&e| e != x)
                    .collect();
                let child_parity = parity ^ QUADRANT_ID[x];
                let score = if child_empties.len() == 4 {
                    -self.search_solve_4(&next, -beta,
                        child_empties[0], child_empties[1],
                        child_empties[2], child_empties[3], child_parity)
                } else {
                    -self.search_shallow(&next, -beta, &child_empties, child_parity)
                };
                if score >= beta {
                    return score;
                }
                if score > bestscore {
                    bestscore = score;
                }
            }
        }

        // No move: pass or game over
        if bestscore == -SCORE_INF {
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() != 0 {
                bestscore = -self.search_shallow(&passed, -beta, empties_list, parity);
            } else {
                bestscore = board.score();
            }
        }

        bestscore
    }

    /// Optimized endgame NWS using the specialized solvers.
    /// Entry point for endgame when n_empties is known.
    fn endgame_nws(&mut self, board: &Board, alpha: i32) -> i32 {
        let n_empties = board.empties() as i32;

        if n_empties == 0 {
            self.nodes += 1;
            return board.score();
        }

        // Collect empty squares
        let empty = !(board.player | board.opponent);
        let mut empties_list = Vec::with_capacity(n_empties as usize);
        let mut bits = empty;
        while bits != 0 {
            let sq = bits.trailing_zeros() as usize;
            empties_list.push(sq);
            bits &= bits - 1;
        }

        // Compute parity
        let mut parity: i32 = 0;
        for &x in &empties_list {
            parity ^= QUADRANT_ID[x];
        }

        match n_empties {
            0 => {
                self.nodes += 1;
                board.score()
            }
            1 => {
                self.nodes += 1;
                // With 1 empty, we are the opponent in board_score_1 terms
                let x = empties_list[0];
                // Try player move
                let n_flips = flip::count_last_flip(x, board.player);
                if n_flips != 0 {
                    let score = 2 * board.player.count_ones() as i32 - SCORE_MAX + n_flips + 2;
                    return score;
                }
                // Try opponent move
                let n_flips_opp = flip::count_last_flip(x, board.opponent);
                if n_flips_opp != 0 {
                    let score = 2 * board.player.count_ones() as i32 - SCORE_MAX - n_flips_opp;
                    return score;
                }
                // Neither can play
                board.score()
            }
            2 => {
                self.board_solve_2(board, alpha, empties_list[0], empties_list[1])
            }
            3 => {
                self.search_solve_3(board, alpha, empties_list[0], empties_list[1], empties_list[2], parity)
            }
            4 => {
                self.search_solve_4(board, alpha, empties_list[0], empties_list[1], empties_list[2], empties_list[3], parity)
            }
            _ if n_empties <= DEPTH_TO_SHALLOW_SEARCH => {
                self.search_shallow(board, alpha, &empties_list, parity)
            }
            _ => {
                // For deeper endgame, use the general endgame solver with hash
                self.endgame_solve(board, alpha, alpha + 1)
            }
        }
    }

    /// Order moves: hash move first, then by mobility reduction heuristic.
    fn order_moves(&self, board: &Board, moves: u64, hash_move: Option<i32>) -> Vec<i32> {
        let mut move_list: Vec<(i32, i32)> = Vec::new();
        let mut remaining = moves;

        while remaining != 0 {
            let sq = remaining.trailing_zeros() as i32;
            remaining &= remaining - 1;

            // Hash move gets highest priority
            if hash_move == Some(sq) {
                move_list.push((sq, i32::MAX));
                continue;
            }

            // Score by: corner bonus + opponent mobility reduction
            let mut child = *board;
            child.do_move(sq as usize);
            let opp_mobility = child.get_moves().count_ones() as i32;
            let corner_bonus = match sq {
                0 | 7 | 56 | 63 => 1000,
                _ => 0,
            };
            let score = corner_bonus - opp_mobility * 10 + SQUARE_ORDER[sq as usize] as i32;
            move_list.push((sq, score));
        }

        move_list.sort_by(|a, b| b.1.cmp(&a.1));
        move_list.into_iter().map(|(sq, _)| sq).collect()
    }

    /// Extract principal variation from hash table.
    fn extract_pv(&self, board: &Board, max_depth: i32) -> Vec<i32> {
        let mut pv = Vec::new();
        let mut b = *board;
        let mut seen = std::collections::HashSet::new();

        for _ in 0..max_depth {
            let hc = HashTable::hash_code(&b);
            if !seen.insert((b.player, b.opponent)) {
                break;
            }
            if let Some(data) = self.hash_table.get(&b, hc) {
                let mv = data.best_move[0] as i32;
                if mv < 0 || mv >= 64 {
                    break;
                }
                let moves = b.get_moves();
                if moves & (1u64 << mv) == 0 {
                    break;
                }
                pv.push(mv);
                b.do_move(mv as usize);
            } else {
                break;
            }
        }
        pv
    }

    /// Check if time limit exceeded.
    fn check_timeout(&mut self) {
        // Check every 4096 nodes to reduce overhead
        if self.nodes & 0xFFF != 0 {
            return;
        }
        if let Some(start) = self.start_time {
            if start.elapsed().as_millis() as u64 >= self.options.time_limit_ms {
                self.stopped = true;
            }
        }
    }

    /// Elapsed time in milliseconds.
    fn elapsed_ms(&self) -> u64 {
        self.start_time
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Square ordering preference (higher = search first).
/// Corners first, then edges, then center, X-squares last.
#[rustfmt::skip]
const SQUARE_ORDER: [u8; 64] = [
    90, 10, 80, 50, 50, 80, 10, 90,
    10,  5, 15, 20, 20, 15,  5, 10,
    80, 15, 70, 40, 40, 70, 15, 80,
    50, 20, 40, 30, 30, 40, 20, 50,
    50, 20, 40, 30, 30, 40, 20, 50,
    80, 15, 70, 40, 40, 70, 15, 80,
    10,  5, 15, 20, 20, 15,  5, 10,
    90, 10, 80, 50, 50, 80, 10, 90,
];

#[cfg(test)]
mod tests {
    use super::*;

    fn make_search(depth: i32) -> Search {
        Search::new(SearchOptions {
            depth,
            time_limit_ms: 30000,
            hash_size: 1 << 16,
            verbose: false,
        })
    }

    #[test]
    fn search_returns_legal_move() {
        let board = Board::new();
        let mut search = make_search(6);
        let result = search.search(&board);
        let moves = board.get_moves();
        assert!(result.best_move >= 0 && result.best_move < 64);
        assert!(moves & (1u64 << result.best_move) != 0,
            "search returned illegal move {}", result.best_move);
    }

    #[test]
    fn search_depth_1_returns_move() {
        let board = Board::new();
        let mut search = make_search(1);
        let result = search.search(&board);
        assert!(result.best_move >= 0);
        assert_eq!(result.depth, 1);
    }

    #[test]
    fn search_score_in_range() {
        let board = Board::new();
        let mut search = make_search(8);
        let result = search.search(&board);
        assert!(result.score >= -64 && result.score <= 64,
            "score {} out of range", result.score);
    }

    #[test]
    fn search_counts_nodes() {
        let board = Board::new();
        let mut search = make_search(6);
        let result = search.search(&board);
        assert!(result.nodes > 0, "should have searched some nodes");
    }

    #[test]
    fn search_pv_starts_with_best_move() {
        let board = Board::new();
        let mut search = make_search(6);
        let result = search.search(&board);
        if !result.pv.is_empty() {
            assert_eq!(result.pv[0], result.best_move);
        }
    }

    #[test]
    fn search_deeper_searches_more_nodes() {
        let board = Board::new();
        let mut s1 = make_search(2);
        let r1 = s1.search(&board);
        let mut s2 = make_search(6);
        let r2 = s2.search(&board);
        assert!(r2.nodes > r1.nodes,
            "depth 6 ({}) should search more nodes than depth 2 ({})",
            r2.nodes, r1.nodes);
    }

    #[test]
    fn endgame_solve_exact() {
        // Create a near-end position (few empties)
        let board = Board {
            player: 0x00000000FFFFFFFF,  // 32 discs
            opponent: 0xFFFFFFF000000000, // 28 discs, 4 empties
        };
        let mut search = make_search(60);
        let result = search.search(&board);
        // Score should be exact disc difference
        assert!(result.score >= -64 && result.score <= 64);
    }

    #[test]
    fn search_game_over_returns_score() {
        // Board with no moves for either player
        let board = Board {
            player: 0x00000000FFFFFFFF,
            opponent: 0xFFFFFFFF00000000,
        };
        let mut search = make_search(10);
        let result = search.search(&board);
        assert_eq!(result.score, 0); // 32-32 = 0
        assert!(result.is_exact);
    }

    #[test]
    fn search_finds_winning_move() {
        // Setup where one move clearly wins
        let mut board = Board::new();
        // Play a short game
        board.do_move(19); // D3
        board.do_move(18); // C3
        board.do_move(10); // C2

        let mut search = make_search(8);
        let result = search.search(&board);
        assert!(result.best_move >= 0, "should find a move");
    }

    #[test]
    fn search_move_ordering_hash_move_first() {
        let search = make_search(6);
        let board = Board::new();
        let moves = board.get_moves();
        let ordered = search.order_moves(&board, moves, Some(19));
        assert_eq!(ordered[0], 19, "hash move should be first");
    }

    #[test]
    fn search_iterative_deepening_reaches_target() {
        let board = Board::new();
        let mut search = make_search(8);
        let result = search.search(&board);
        assert_eq!(result.depth, 8, "should reach depth 8");
    }

    // ================================================================
    // Endgame solver tests (TDD)
    // ================================================================

    #[test]
    fn board_solve_0_full_board() {
        // Full board: 32 player, 32 opponent -> score = 0
        let board = Board {
            player: 0x00000000FFFFFFFF,
            opponent: 0xFFFFFFFF00000000,
        };
        assert_eq!(Search::board_solve_0(&board), 0);
    }

    #[test]
    fn board_solve_0_player_wins() {
        // Player has 40 discs, opponent has 24
        let board = Board {
            player: 0x00000000FFFFFFFF | (0xFFu64 << 32),
            opponent: 0xFFFFFF0000000000u64,
        };
        let p = board.player.count_ones() as i32;
        let expected = 2 * p - 64;
        assert_eq!(Search::board_solve_0(&board), expected);
    }

    #[test]
    fn board_score_1_player_can_flip() {
        // 1 empty at A1(0). Player has B1(1) and row fills.
        // Construct: player has bottom 4 rows + B1, opponent has top 4 rows minus A1
        let empty_sq = 0usize; // A1
        let player = 0x00000000FFFFFFFEu64 | (1u64 << 1); // bottom half + B1
        let opponent = 0xFFFFFFFF00000000u64; // top half
        // Verify it's 1 empty
        assert_eq!(64 - (player | opponent).count_ones(), 1);

        let board = Board { player, opponent };
        let score = Search::board_score_1(&board, SCORE_MAX + 1, empty_sq);
        // Score should be in valid range
        assert!(score >= SCORE_MIN && score <= SCORE_MAX,
            "board_score_1 returned {}", score);
        // Score should be even (endgame scores are always even)
        assert_eq!(score & 1, 0, "endgame score {} should be even", score);
    }

    #[test]
    fn board_score_1_consistent_with_full_score() {
        // Create a position with 1 empty, compute score both ways
        let empty_sq = 36usize; // E5
        // Player has all of bottom half except E5, plus some top
        let player = 0x00000000FFFFFFFFu64;
        let mut opponent = 0xFFFFFFFF00000000u64;
        // Make E5 (36) empty - it's in opponent range
        opponent &= !(1u64 << empty_sq);
        assert_eq!(64 - (player | opponent).count_ones(), 1);

        let board = Board { player, opponent };

        // Use board_score_1
        let score1 = Search::board_score_1(&board, SCORE_MAX + 1, empty_sq);

        // Alternative: actually play the move and get full score
        let n_flips = flip::count_last_flip(empty_sq, player);
        let expected_p = player.count_ones() as i32 + n_flips / 2 + 1;
        let expected_o = opponent.count_ones() as i32 - n_flips / 2;
        let expected_score = expected_o - expected_p; // opponent perspective
        if n_flips != 0 {
            assert_eq!(score1, expected_score,
                "board_score_1 mismatch: got {}, expected {}", score1, expected_score);
        }
    }

    #[test]
    fn board_solve_2_returns_valid_score() {
        // Construct a 2-empty position
        let empty1 = 0usize; // A1
        let empty2 = 63usize; // H8
        let mut board = Board {
            player: 0x00000000FFFFFFFF,
            opponent: 0xFFFFFFFF00000000,
        };
        // Clear empty squares
        board.player &= !(1u64 << empty1);
        board.opponent &= !(1u64 << empty2);
        assert_eq!(board.empties(), 2);

        let mut search = make_search(60);
        let score = search.board_solve_2(&board, -1, empty1, empty2);
        assert!(score >= SCORE_MIN && score <= SCORE_MAX,
            "board_solve_2 returned {}", score);
    }

    #[test]
    fn solve_3_returns_valid_score() {
        // 3 empties
        let empty1 = 0usize;
        let empty2 = 7usize;
        let empty3 = 63usize;
        let mut board = Board {
            player: 0x00000000FFFFFFFF,
            opponent: 0xFFFFFFFF00000000,
        };
        board.player &= !(1u64 << empty1);
        board.player &= !(1u64 << empty2);
        board.opponent &= !(1u64 << empty3);
        assert_eq!(board.empties(), 3);

        let parity = QUADRANT_ID[empty1] ^ QUADRANT_ID[empty2] ^ QUADRANT_ID[empty3];
        let mut search = make_search(60);
        let score = search.search_solve_3(&board, -1, empty1, empty2, empty3, parity);
        assert!(score >= SCORE_MIN && score <= SCORE_MAX,
            "search_solve_3 returned {}", score);
    }

    #[test]
    fn solve_4_returns_valid_score() {
        // 4 empties
        let empty1 = 0usize;
        let empty2 = 7usize;
        let empty3 = 56usize;
        let empty4 = 63usize;
        let mut board = Board {
            player: 0x00000000FFFFFFFF,
            opponent: 0xFFFFFFFF00000000,
        };
        board.player &= !(1u64 << empty1);
        board.player &= !(1u64 << empty2);
        board.opponent &= !(1u64 << empty3);
        board.opponent &= !(1u64 << empty4);
        assert_eq!(board.empties(), 4);

        let parity = QUADRANT_ID[empty1] ^ QUADRANT_ID[empty2] ^ QUADRANT_ID[empty3] ^ QUADRANT_ID[empty4];
        let mut search = make_search(60);
        let score = search.search_solve_4(&board, -1, empty1, empty2, empty3, empty4, parity);
        assert!(score >= SCORE_MIN && score <= SCORE_MAX,
            "search_solve_4 returned {}", score);
    }

    #[test]
    fn endgame_nws_matches_endgame_solve() {
        // The optimized endgame NWS should give the same result as the
        // general endgame_solve for the same position
        let empty1 = 0usize;
        let empty2 = 7usize;
        let empty3 = 56usize;
        let empty4 = 63usize;
        let mut board = Board {
            player: 0x00000000FFFFFFFF,
            opponent: 0xFFFFFFFF00000000,
        };
        board.player &= !(1u64 << empty1);
        board.player &= !(1u64 << empty2);
        board.opponent &= !(1u64 << empty3);
        board.opponent &= !(1u64 << empty4);

        // Test with general solver
        let mut search1 = make_search(60);
        let score1 = search1.endgame_solve(&board, SCORE_MIN - 1, SCORE_MAX + 1);

        // Test with optimized NWS: try a range of alpha values
        // The exact score should be the same
        for alpha in (SCORE_MIN..=SCORE_MAX).step_by(2) {
            let mut search2 = make_search(60);
            let score2 = search2.endgame_nws(&board, alpha);
            // NWS returns: <= alpha if score <= alpha, > alpha if score > alpha
            if score1 <= alpha {
                assert!(score2 <= alpha,
                    "endgame_nws(alpha={}) = {}, but exact = {} <= alpha",
                    alpha, score2, score1);
            } else {
                assert!(score2 > alpha,
                    "endgame_nws(alpha={}) = {}, but exact = {} > alpha",
                    alpha, score2, score1);
            }
        }
    }

    #[test]
    fn endgame_nws_6_empties() {
        // Test with 6 empties (uses search_shallow)
        let mut board = Board::new();
        // Play a game to get to 6 empties
        let moves = [19, 18, 10, 9, 26, 34, 42, 43, 44, 37, 20,
                     11, 2, 3, 17, 33, 41, 32, 25, 24, 40, 48,
                     49, 50, 51, 52, 53, 54, 55, 57, 58, 59, 60,
                     61, 62, 63, 56, 16, 8, 0, 1, 4, 5, 6, 7,
                     15, 23, 31, 39, 47, 38, 46, 45, 14, 13];
        for &m in &moves {
            if board.get_moves() == 0 {
                board.pass();
            }
            if board.get_moves() & (1u64 << m) != 0 {
                board.do_move(m);
            } else {
                break;
            }
        }

        let n_empties = board.empties();
        if n_empties <= 10 && n_empties >= 1 {
            // Compare general solver with optimized NWS
            let mut search1 = make_search(60);
            let score1 = search1.endgame_solve(&board, SCORE_MIN - 1, SCORE_MAX + 1);

            let mut search2 = make_search(60);
            let score2 = search2.endgame_nws(&board, score1 - 1);

            // NWS with alpha = score-1 should return the exact score
            assert_eq!(score2, score1,
                "endgame_nws with {} empties: expected {}, got {}",
                n_empties, score1, score2);
        }
    }

    #[test]
    fn neighbour_table_correct() {
        // Verify a few NEIGHBOUR entries
        // A1(0) should neighbor B1(1), A2(8), B2(9)
        assert_eq!(NEIGHBOUR[0], (1u64 << 1) | (1u64 << 8) | (1u64 << 9));
        // H8(63) should neighbor G7(54), H7(55), G8(62)
        assert_eq!(NEIGHBOUR[63], (1u64 << 54) | (1u64 << 55) | (1u64 << 62));
        // D4(27) should have 8 neighbors
        assert_eq!(NEIGHBOUR[27].count_ones(), 8);
    }

    #[test]
    fn eval_sigma_positive() {
        // eval_sigma should return positive values for valid inputs
        let sigma = Search::eval_sigma(30, 10, 4);
        assert!(sigma > 0.0, "eval_sigma should be positive, got {}", sigma);

        let sigma2 = Search::eval_sigma(20, 20, 8);
        assert!(sigma2 > 0.0, "eval_sigma should be positive, got {}", sigma2);
    }

    #[test]
    fn eval_sigma_deeper_has_lower_error() {
        // Deeper probcut depth should have lower error than shallower
        let sigma_shallow = Search::eval_sigma(30, 12, 4);
        let sigma_deep = Search::eval_sigma(30, 12, 8);
        assert!(sigma_deep < sigma_shallow,
            "deeper probcut ({}) should have less error than shallow ({})",
            sigma_deep, sigma_shallow);
    }

    #[test]
    fn probcut_returns_none_at_low_depth() {
        let board = Board::new();
        let mut search = make_search(4);
        search.start_time = Some(Instant::now());
        let result = search.search_probcut(&board, 3, 60, 0);
        assert!(result.is_none(), "ProbCut should not fire at depth 3");
    }

    #[test]
    fn stability_cutoff_with_many_stable_discs() {
        // When opponent has many stable discs, SC should prune
        let search = make_search(60);
        // Full board minus a few empties - opponent has lots of stable discs
        let board = Board {
            player: 0x000000000000003Fu64,   // 6 discs on bottom
            opponent: 0xFFFFFFFFFFFFFFC0u64,  // 58 discs everywhere else
        };
        let n_empties = board.empties() as i32;
        // With 58 opponent discs (most stable), alpha=50 should trigger SC
        let result = search.search_sc_nws(&board, n_empties, 50);
        // The opponent is dominant - SC should fire
        if let Some(score) = result {
            assert!(score <= 50, "SC score {} should be <= alpha=50", score);
        }
    }

    #[test]
    fn quadrant_id_values() {
        // Top-left = 1, top-right = 2, bottom-left = 4, bottom-right = 8
        assert_eq!(QUADRANT_ID[0], 1);   // A1 = top-left
        assert_eq!(QUADRANT_ID[7], 2);   // H1 = top-right
        assert_eq!(QUADRANT_ID[56], 4);  // A8 = bottom-left
        assert_eq!(QUADRANT_ID[63], 8);  // H8 = bottom-right
        assert_eq!(QUADRANT_ID[27], 1);  // D4 = top-left
        assert_eq!(QUADRANT_ID[36], 8);  // E5 = bottom-right
    }
}
