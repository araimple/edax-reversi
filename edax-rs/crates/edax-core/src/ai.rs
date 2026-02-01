use crate::board::Board;
use crate::flip;
use crate::search::{Search, SearchOptions};

/// Choose a move for the current player.
pub trait Player {
    fn choose_move(&mut self, board: &Board) -> usize;
    fn name(&self) -> &str;
}

/// Picks a random legal move.
pub struct RandomPlayer {
    rng: u64,
}

impl RandomPlayer {
    pub fn new(seed: u64) -> Self {
        RandomPlayer { rng: seed }
    }

    fn next_rand(&mut self) -> u64 {
        // xorshift64
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }
}

impl Player for RandomPlayer {
    fn choose_move(&mut self, board: &Board) -> usize {
        let moves = board.get_moves();
        let n = moves.count_ones() as u64;
        let idx = self.next_rand() % n;
        let mut remaining = moves;
        for _ in 0..idx {
            remaining &= remaining - 1;
        }
        remaining.trailing_zeros() as usize
    }

    fn name(&self) -> &str {
        "Random"
    }
}

/// Greedy: picks the move that maximizes player's disc count.
pub struct GreedyPlayer;

impl Player for GreedyPlayer {
    fn choose_move(&mut self, board: &Board) -> usize {
        let moves = board.get_moves();
        let mut best_sq = 64;
        let mut best_count = -1i32;
        let mut remaining = moves;
        while remaining != 0 {
            let sq = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;
            let flipped = flip::flip(sq, board.player, board.opponent);
            let count = flipped.count_ones() as i32;
            if count > best_count {
                best_count = count;
                best_sq = sq;
            }
        }
        best_sq
    }

    fn name(&self) -> &str {
        "Greedy"
    }
}

/// Mobility: picks the move that minimizes opponent's mobility.
pub struct MobilityPlayer;

impl Player for MobilityPlayer {
    fn choose_move(&mut self, board: &Board) -> usize {
        let moves = board.get_moves();
        let mut best_sq = 64;
        let mut best_opp_moves = 65u32;
        let mut remaining = moves;
        while remaining != 0 {
            let sq = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;
            let mut child = *board;
            child.do_move(sq);
            let opp_moves = child.get_moves().count_ones();
            if opp_moves < best_opp_moves {
                best_opp_moves = opp_moves;
                best_sq = sq;
            }
        }
        best_sq
    }

    fn name(&self) -> &str {
        "Mobility"
    }
}

/// Alpha-beta search player using the full search engine.
pub struct SearchPlayer {
    search: Search,
}

impl SearchPlayer {
    pub fn new(depth: i32) -> Self {
        SearchPlayer {
            search: Search::new(SearchOptions {
                depth,
                time_limit_ms: 30000,
                hash_size: 1 << 16,
                verbose: false,
            }),
        }
    }
}

impl Player for SearchPlayer {
    fn choose_move(&mut self, board: &Board) -> usize {
        let result = self.search.search(board);
        if result.best_move >= 0 && result.best_move < 64 {
            result.best_move as usize
        } else {
            // Fallback to first legal move
            let moves = board.get_moves();
            moves.trailing_zeros() as usize
        }
    }

    fn name(&self) -> &str {
        "Search"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_player_returns_legal_move() {
        let board = Board::new();
        let moves = board.get_moves();
        let mut player = RandomPlayer::new(42);
        let sq = player.choose_move(&board);
        assert!(moves & (1u64 << sq) != 0, "must be a legal move");
    }

    #[test]
    fn greedy_player_returns_legal_move() {
        let board = Board::new();
        let moves = board.get_moves();
        let mut player = GreedyPlayer;
        let sq = player.choose_move(&board);
        assert!(moves & (1u64 << sq) != 0, "must be a legal move");
    }

    #[test]
    fn mobility_player_returns_legal_move() {
        let board = Board::new();
        let moves = board.get_moves();
        let mut player = MobilityPlayer;
        let sq = player.choose_move(&board);
        assert!(moves & (1u64 << sq) != 0, "must be a legal move");
    }

    #[test]
    fn search_player_returns_legal_move() {
        let board = Board::new();
        let moves = board.get_moves();
        let mut player = SearchPlayer::new(4);
        let sq = player.choose_move(&board);
        assert!(moves & (1u64 << sq) != 0, "must be a legal move");
    }

    #[test]
    fn random_player_different_seeds_give_variety() {
        let board = Board::new();
        let mut results = std::collections::HashSet::new();
        for seed in 1..100 {
            let mut p = RandomPlayer::new(seed);
            results.insert(p.choose_move(&board));
        }
        assert!(results.len() > 1, "should pick different moves with different seeds");
    }
}
