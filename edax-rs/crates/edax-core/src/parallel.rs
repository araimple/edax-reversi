//! Parallel search using Young Brothers Wait Concept (YBWC).
//!
//! After the first (principal) move is searched sequentially,
//! remaining sibling moves are distributed across threads.

use crate::board::Board;
use crate::eval;
use crate::hash::{HashTable, NOMOVE};
use crate::search::{SearchOptions, SCORE_MAX, SCORE_MIN, NO_SELECTIVITY};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Parallel search engine.
pub struct ParallelSearch {
    pub options: SearchOptions,
    pub n_threads: usize,
    nodes: Arc<AtomicU64>,
    stopped: Arc<AtomicBool>,
    start_time: Option<Instant>,
}

/// Shared search state for parallel workers.
struct SharedState {
    hash_table: Mutex<HashTable>,
    nodes: Arc<AtomicU64>,
    stopped: Arc<AtomicBool>,
    time_limit_ms: u64,
    start_time: Instant,
}

impl SharedState {
    fn check_timeout(&self) {
        let n = self.nodes.load(Ordering::Relaxed);
        if n & 0xFFF == 0 && self.start_time.elapsed().as_millis() as u64 >= self.time_limit_ms {
            self.stopped.store(true, Ordering::Relaxed);
        }
    }

    fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }
}

impl ParallelSearch {
    pub fn new(options: SearchOptions, n_threads: usize) -> Self {
        ParallelSearch {
            options,
            n_threads: n_threads.max(1),
            nodes: Arc::new(AtomicU64::new(0)),
            stopped: Arc::new(AtomicBool::new(false)),
            start_time: None,
        }
    }

    /// Run parallel search with iterative deepening.
    pub fn search(&mut self, board: &Board) -> crate::search::SearchResult {
        self.nodes.store(0, Ordering::Relaxed);
        self.stopped.store(false, Ordering::Relaxed);
        let start = Instant::now();
        self.start_time = Some(start);

        let state = Arc::new(SharedState {
            hash_table: Mutex::new(HashTable::new(self.options.hash_size)),
            nodes: self.nodes.clone(),
            stopped: self.stopped.clone(),
            time_limit_ms: self.options.time_limit_ms,
            start_time: start,
        });

        let n_empties = board.empties() as i32;
        let max_depth = self.options.depth.min(n_empties);

        let mut best_result = crate::search::SearchResult {
            score: 0,
            best_move: NOMOVE,
            depth: 0,
            nodes: 0,
            pv: Vec::new(),
            time_ms: 0,
            is_exact: false,
        };

        let moves = board.get_moves();
        if moves == 0 {
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() == 0 {
                best_result.score = board.score();
                best_result.is_exact = true;
                return best_result;
            }
            let mut result = self.search(&passed);
            result.score = -result.score;
            return result;
        }

        // Iterative deepening
        for depth in 1..=max_depth {
            if state.is_stopped() {
                break;
            }

            let score = self.pvs_root_parallel(board, depth, &state);

            if !state.is_stopped() {
                best_result.score = score;
                best_result.depth = depth;
                best_result.nodes = self.nodes.load(Ordering::Relaxed);
                best_result.time_ms = start.elapsed().as_millis() as u64;

                // Extract best move
                let hc = HashTable::hash_code(board);
                let ht = state.hash_table.lock().unwrap();
                if let Some(data) = ht.get(board, hc) {
                    if data.best_move[0] >= 0 {
                        best_result.best_move = data.best_move[0] as i32;
                    }
                }
                drop(ht);

                if self.options.verbose {
                    eprintln!(
                        "depth {:2} score {:+3} move {} nodes {} time {}ms",
                        depth,
                        score,
                        if best_result.best_move >= 0 {
                            Board::square_to_string(best_result.best_move as usize)
                        } else {
                            "--".to_string()
                        },
                        best_result.nodes,
                        best_result.time_ms,
                    );
                }

                if depth == n_empties {
                    best_result.is_exact = true;
                }
            }
        }

        best_result
    }

    /// Root-level PVS with YBWC parallelism.
    fn pvs_root_parallel(&self, board: &Board, depth: i32, state: &Arc<SharedState>) -> i32 {
        let moves = board.get_moves();
        if moves == 0 {
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() == 0 {
                return board.score();
            }
            return -self.pvs_root_parallel(&passed, depth, state);
        }

        let move_list = self.order_moves_parallel(board, moves, state);
        let mut alpha = SCORE_MIN - 1;
        let beta = SCORE_MAX + 1;
        let mut best_score = SCORE_MIN - 1;
        let mut best_move = NOMOVE;

        // First move: search sequentially (Young Brother Wait)
        if let Some(&first_sq) = move_list.first() {
            let mut child = *board;
            child.do_move(first_sq as usize);
            let score = -self.pvs_worker(&child, depth - 1, -beta, -alpha, state);

            if !state.is_stopped() {
                best_score = score;
                best_move = first_sq;
                if score > alpha {
                    alpha = score;
                }
            }
        }

        // Remaining moves: parallel NWS then sequential re-search if needed
        if move_list.len() > 1 && !state.is_stopped() && alpha < beta {
            let alpha_shared = Arc::new(AtomicI32::new(alpha));
            let best_score_shared = Arc::new(AtomicI32::new(best_score));
            let best_move_shared = Arc::new(AtomicI32::new(best_move));

            let siblings: Vec<i32> = move_list[1..].to_vec();

            // Use scoped threads for parallel NWS
            std::thread::scope(|s| {
                let chunks = Self::chunk_moves(&siblings, self.n_threads);
                let handles: Vec<_> = chunks
                    .into_iter()
                    .map(|chunk| {
                        let state = state.clone();
                        let alpha_shared = alpha_shared.clone();
                        let best_score_shared = best_score_shared.clone();
                        let best_move_shared = best_move_shared.clone();
                        let board = *board;

                        s.spawn(move || {
                            for &sq in &chunk {
                                if state.is_stopped() {
                                    break;
                                }

                                let current_alpha = alpha_shared.load(Ordering::Relaxed);
                                let mut child = board;
                                child.do_move(sq as usize);

                                let score = -Self::nws_worker_static(
                                    &child,
                                    depth - 1,
                                    -current_alpha - 1,
                                    &state,
                                );

                                if !state.is_stopped() && score > current_alpha {
                                    // Re-search needed; record candidate
                                    let cur_best = best_score_shared.load(Ordering::Relaxed);
                                    if score > cur_best {
                                        best_score_shared.store(score, Ordering::Relaxed);
                                        best_move_shared.store(sq, Ordering::Relaxed);
                                        alpha_shared.store(
                                            alpha_shared.load(Ordering::Relaxed).max(score),
                                            Ordering::Relaxed,
                                        );
                                    }
                                }
                            }
                        })
                    })
                    .collect();

                for h in handles {
                    let _ = h.join();
                }
            });

            best_score = best_score_shared.load(Ordering::Relaxed);
            best_move = best_move_shared.load(Ordering::Relaxed);
            alpha = alpha_shared.load(Ordering::Relaxed);
        }

        // Store in hash table
        if !state.is_stopped() && best_move >= 0 {
            let hc = HashTable::hash_code(board);
            let mut ht = state.hash_table.lock().unwrap();
            ht.store(board, hc, depth, NO_SELECTIVITY, alpha, beta, best_score, best_move);
        }

        best_score
    }

    /// Sequential PVS worker.
    fn pvs_worker(
        &self,
        board: &Board,
        depth: i32,
        mut alpha: i32,
        beta: i32,
        state: &Arc<SharedState>,
    ) -> i32 {
        state.check_timeout();
        if state.is_stopped() {
            return 0;
        }
        state.nodes.fetch_add(1, Ordering::Relaxed);

        let n_empties = board.empties() as i32;
        if depth <= 0 || n_empties == 0 {
            return eval::evaluate(board);
        }

        if depth >= n_empties && n_empties <= 6 {
            return Self::endgame_solve_static(board, alpha, beta, state);
        }

        let moves = board.get_moves();
        if moves == 0 {
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() == 0 {
                return board.score();
            }
            return -self.pvs_worker(&passed, depth, -beta, -alpha, state);
        }

        // Hash lookup
        let hc = HashTable::hash_code(board);
        let hash_move = {
            let ht = state.hash_table.lock().unwrap();
            if let Some(data) = ht.get(board, hc) {
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
            }
        };

        let move_list = Self::order_moves_static(board, moves, hash_move);
        let mut best_score = SCORE_MIN - 1;
        let mut best_move = NOMOVE;
        let mut first = true;

        for sq in &move_list {
            let sq = *sq as usize;
            let mut child = *board;
            child.do_move(sq);

            let score;
            if first {
                score = -self.pvs_worker(&child, depth - 1, -beta, -alpha, state);
                first = false;
            } else {
                let nws = -Self::nws_worker_static(&child, depth - 1, -alpha - 1, state);
                if nws > alpha && nws < beta && !state.is_stopped() {
                    score = -self.pvs_worker(&child, depth - 1, -beta, -nws, state);
                } else {
                    score = nws;
                }
            }

            if state.is_stopped() {
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
            let mut ht = state.hash_table.lock().unwrap();
            ht.store(board, hc, depth, NO_SELECTIVITY, alpha, beta, best_score, best_move);
        }

        best_score
    }

    /// Static NWS worker (no &self, for use in threads).
    fn nws_worker_static(
        board: &Board,
        depth: i32,
        alpha: i32,
        state: &Arc<SharedState>,
    ) -> i32 {
        state.check_timeout();
        if state.is_stopped() {
            return 0;
        }
        state.nodes.fetch_add(1, Ordering::Relaxed);

        let beta = alpha + 1;
        let n_empties = board.empties() as i32;

        if depth <= 0 || n_empties == 0 {
            return eval::evaluate(board);
        }

        if depth >= n_empties && n_empties <= 6 {
            return Self::endgame_solve_static(board, alpha, beta, state);
        }

        let moves = board.get_moves();
        if moves == 0 {
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() == 0 {
                return board.score();
            }
            return -Self::nws_worker_static(&passed, depth, -beta, state);
        }

        let hc = HashTable::hash_code(board);
        let hash_move = {
            let ht = state.hash_table.lock().unwrap();
            if let Some(data) = ht.get(board, hc) {
                if data.depth as i32 >= depth {
                    let lower = data.lower as i32;
                    let upper = data.upper as i32;
                    if lower >= beta { return lower; }
                    if upper <= alpha { return upper; }
                }
                if data.best_move[0] >= 0 { Some(data.best_move[0] as i32) } else { None }
            } else {
                None
            }
        };

        let move_list = Self::order_moves_static(board, moves, hash_move);
        let mut best_score = SCORE_MIN - 1;
        let mut best_move = NOMOVE;

        for sq in &move_list {
            let sq = *sq as usize;
            let mut child = *board;
            child.do_move(sq);

            let score = -Self::nws_worker_static(&child, depth - 1, -beta, state);

            if state.is_stopped() {
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
            let mut ht = state.hash_table.lock().unwrap();
            ht.store(board, hc, depth, NO_SELECTIVITY, alpha, beta, best_score, best_move);
        }

        best_score
    }

    /// Static endgame solver.
    fn endgame_solve_static(
        board: &Board,
        mut alpha: i32,
        beta: i32,
        state: &Arc<SharedState>,
    ) -> i32 {
        state.nodes.fetch_add(1, Ordering::Relaxed);
        if board.empties() == 0 {
            return board.score();
        }

        let moves = board.get_moves();
        if moves == 0 {
            let mut passed = *board;
            passed.pass();
            if passed.get_moves() == 0 {
                return board.score();
            }
            return -Self::endgame_solve_static(&passed, -beta, -alpha, state);
        }

        let mut best_score = SCORE_MIN - 1;
        let mut remaining = moves;
        while remaining != 0 {
            let sq = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;
            let mut child = *board;
            child.do_move(sq);
            let score = -Self::endgame_solve_static(&child, -beta, -alpha, state);
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

    fn order_moves_parallel(&self, board: &Board, moves: u64, state: &Arc<SharedState>) -> Vec<i32> {
        let hc = HashTable::hash_code(board);
        let hash_move = {
            let ht = state.hash_table.lock().unwrap();
            ht.get(board, hc)
                .and_then(|d| if d.best_move[0] >= 0 { Some(d.best_move[0] as i32) } else { None })
        };
        Self::order_moves_static(board, moves, hash_move)
    }

    fn order_moves_static(board: &Board, moves: u64, hash_move: Option<i32>) -> Vec<i32> {
        let mut move_list: Vec<(i32, i32)> = Vec::new();
        let mut remaining = moves;
        while remaining != 0 {
            let sq = remaining.trailing_zeros() as i32;
            remaining &= remaining - 1;
            if hash_move == Some(sq) {
                move_list.push((sq, i32::MAX));
                continue;
            }
            let mut child = *board;
            child.do_move(sq as usize);
            let opp_mobility = child.get_moves().count_ones() as i32;
            let corner_bonus = match sq { 0 | 7 | 56 | 63 => 1000, _ => 0 };
            move_list.push((sq, corner_bonus - opp_mobility * 10));
        }
        move_list.sort_by(|a, b| b.1.cmp(&a.1));
        move_list.into_iter().map(|(sq, _)| sq).collect()
    }

    fn chunk_moves(moves: &[i32], n_threads: usize) -> Vec<Vec<i32>> {
        let chunk_size = (moves.len() + n_threads - 1) / n_threads;
        moves.chunks(chunk_size.max(1)).map(|c| c.to_vec()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_parallel_search(depth: i32, threads: usize) -> ParallelSearch {
        ParallelSearch::new(
            SearchOptions {
                depth,
                time_limit_ms: 30000,
                hash_size: 1 << 16,
                verbose: false,
            },
            threads,
        )
    }

    #[test]
    fn parallel_search_returns_legal_move() {
        let board = Board::new();
        let mut search = make_parallel_search(6, 2);
        let result = search.search(&board);
        let moves = board.get_moves();
        assert!(result.best_move >= 0 && result.best_move < 64);
        assert!(moves & (1u64 << result.best_move) != 0);
    }

    #[test]
    fn parallel_search_single_thread() {
        let board = Board::new();
        let mut search = make_parallel_search(4, 1);
        let result = search.search(&board);
        assert!(result.best_move >= 0);
    }

    #[test]
    fn parallel_search_multi_thread() {
        let board = Board::new();
        let mut search = make_parallel_search(6, 4);
        let result = search.search(&board);
        assert!(result.best_move >= 0);
        assert!(result.nodes > 0);
    }

    #[test]
    fn parallel_search_score_reasonable() {
        let board = Board::new();
        let mut search = make_parallel_search(8, 2);
        let result = search.search(&board);
        assert!(result.score >= -64 && result.score <= 64);
    }

    #[test]
    fn parallel_game_over_returns_score() {
        let board = Board {
            player: 0x00000000FFFFFFFF,
            opponent: 0xFFFFFFFF00000000,
        };
        let mut search = make_parallel_search(10, 2);
        let result = search.search(&board);
        assert_eq!(result.score, 0);
    }
}
