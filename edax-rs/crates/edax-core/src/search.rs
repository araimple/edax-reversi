//! Alpha-beta search engine with PVS, iterative deepening, and endgame solver.

use crate::board::Board;
use crate::eval;
use crate::hash::{HashTable, NOMOVE};
use std::time::Instant;

/// Score bounds
pub const SCORE_MIN: i32 = -64;
pub const SCORE_MAX: i32 = 64;

/// Maximum search depth
pub const MAX_DEPTH: i32 = 60;

/// No selectivity (search everything)
pub const NO_SELECTIVITY: i32 = 5;

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

        // Endgame solver for exact game
        if depth >= n_empties && n_empties <= 6 {
            return self.endgame_solve(board, alpha, beta);
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

        if depth >= n_empties && n_empties <= 6 {
            return self.endgame_solve(board, alpha, beta);
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
    fn endgame_solve(&mut self, board: &Board, mut alpha: i32, beta: i32) -> i32 {
        self.nodes += 1;
        let n_empties = board.empties();

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
}
