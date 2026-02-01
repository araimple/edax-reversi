//! Benchmarking utilities for measuring search performance.

use crate::board::Board;
use crate::search::{Search, SearchOptions, SearchResult};
use std::time::Instant;

/// Benchmark result for a single search.
#[derive(Clone, Debug)]
pub struct BenchResult {
    pub depth: i32,
    pub score: i32,
    pub best_move: i32,
    pub nodes: u64,
    pub time_ms: f64,
    pub nps: u64,
}

/// Run a benchmark at the given depth from initial position.
pub fn bench_search(depth: i32) -> BenchResult {
    bench_search_from(&Board::new(), depth)
}

/// Run a benchmark at the given depth from a specific position.
pub fn bench_search_from(board: &Board, depth: i32) -> BenchResult {
    let mut search = Search::new(SearchOptions {
        depth,
        time_limit_ms: 300_000,
        hash_size: 1 << 18,
        verbose: false,
    });

    let start = Instant::now();
    let result = search.search(board);
    let elapsed = start.elapsed();
    let time_ms = elapsed.as_secs_f64() * 1000.0;
    let nps = if elapsed.as_millis() > 0 {
        result.nodes * 1000 / elapsed.as_millis() as u64
    } else {
        0
    };

    BenchResult {
        depth: result.depth,
        score: result.score,
        best_move: result.best_move,
        nodes: result.nodes,
        time_ms,
        nps,
    }
}

/// Run perft benchmark (move generation speed).
pub fn bench_perft(depth: u32) -> (u64, f64) {
    use crate::perft;
    let board = Board::new();
    let start = Instant::now();
    let count = perft::perft(&board, depth);
    let time_ms = start.elapsed().as_secs_f64() * 1000.0;
    (count, time_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bench_search_depth_4() {
        let r = bench_search(4);
        assert!(r.nodes > 0);
        assert!(r.best_move >= 0 && r.best_move < 64);
        assert!(r.score >= -64 && r.score <= 64);
    }

    #[test]
    fn bench_search_depth_8() {
        let r = bench_search(8);
        assert!(r.nodes > 0);
        assert!(r.depth == 8);
    }

    #[test]
    fn bench_perft_depth_6() {
        let (count, time_ms) = bench_perft(6);
        assert_eq!(count, 8200);
        assert!(time_ms >= 0.0);
    }

    #[test]
    fn bench_from_midgame_position() {
        let mut board = Board::new();
        board.do_move(19); // D3
        board.do_move(18); // C3
        board.do_move(10); // C2
        let r = bench_search_from(&board, 6);
        assert!(r.best_move >= 0);
    }
}
