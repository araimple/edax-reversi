//! Opening book for storing and retrieving known positions.
//!
//! Stores board positions with their best moves and scores.
//! Supports symmetry-aware lookups (8 symmetries of the board).

use crate::bit;
use crate::board::Board;
use std::collections::HashMap;

/// A book entry storing best move and score for a position.
#[derive(Clone, Debug)]
pub struct BookEntry {
    pub best_move: i32,
    pub score: i32,
    pub depth: i32,
    pub n_games: u32,
}

/// Opening book database.
pub struct Book {
    positions: HashMap<(u64, u64), BookEntry>,
}

impl Book {
    pub fn new() -> Self {
        Book {
            positions: HashMap::new(),
        }
    }

    /// Number of positions in the book.
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Store a position in the book (canonical form).
    pub fn store(&mut self, board: &Board, best_move: i32, score: i32, depth: i32) {
        let (canonical, _sym) = Self::canonicalize(board);
        let key = (canonical.player, canonical.opponent);

        let entry = self.positions.entry(key).or_insert(BookEntry {
            best_move: -1,
            score: 0,
            depth: 0,
            n_games: 0,
        });

        // Update if deeper or same depth
        if depth >= entry.depth {
            entry.best_move = best_move;
            entry.score = score;
            entry.depth = depth;
        }
        entry.n_games += 1;
    }

    /// Look up a position in the book.
    /// Returns the entry and the move transformed back to the original orientation.
    pub fn get(&self, board: &Board) -> Option<BookEntry> {
        let (canonical, sym_idx) = Self::canonicalize(board);
        let key = (canonical.player, canonical.opponent);

        self.positions.get(&key).map(|entry| {
            let mut e = entry.clone();
            // Transform move back from canonical to original orientation
            if e.best_move >= 0 && e.best_move < 64 {
                e.best_move = Self::inverse_transform_square(e.best_move as usize, sym_idx) as i32;
            }
            e
        })
    }

    /// Convert a board to its canonical (smallest) representation.
    /// Returns (canonical_board, symmetry_index).
    fn canonicalize(board: &Board) -> (Board, usize) {
        let mut best = *board;
        let mut best_idx = 0;

        for (i, (p, o)) in Self::all_symmetries(board.player, board.opponent)
            .into_iter()
            .enumerate()
        {
            if (p, o) < (best.player, best.opponent) {
                best = Board { player: p, opponent: o };
                best_idx = i;
            }
        }
        (best, best_idx)
    }

    /// Generate all 8 symmetries of a bitboard pair.
    fn all_symmetries(player: u64, opponent: u64) -> [(u64, u64); 8] {
        let p = player;
        let o = opponent;

        let p_h = bit::horizontal_mirror(p);
        let o_h = bit::horizontal_mirror(o);
        let p_v = bit::vertical_mirror(p);
        let o_v = bit::vertical_mirror(o);
        let p_hv = bit::vertical_mirror(p_h);
        let o_hv = bit::vertical_mirror(o_h);

        let p_t = bit::transpose(p);
        let o_t = bit::transpose(o);
        let p_th = bit::horizontal_mirror(p_t);
        let o_th = bit::horizontal_mirror(o_t);
        let p_tv = bit::vertical_mirror(p_t);
        let o_tv = bit::vertical_mirror(o_t);
        let p_thv = bit::vertical_mirror(p_th);
        let o_thv = bit::vertical_mirror(o_th);

        [
            (p, o),
            (p_h, o_h),
            (p_v, o_v),
            (p_hv, o_hv),
            (p_t, o_t),
            (p_th, o_th),
            (p_tv, o_tv),
            (p_thv, o_thv),
        ]
    }

    /// Transform a square index through the given symmetry.
    fn transform_square(sq: usize, sym: usize) -> usize {
        let row = sq / 8;
        let col = sq % 8;
        let (r, c) = match sym {
            0 => (row, col),
            1 => (row, 7 - col),
            2 => (7 - row, col),
            3 => (7 - row, 7 - col),
            4 => (col, row),
            5 => (col, 7 - row),
            6 => (7 - col, row),
            7 => (7 - col, 7 - row),
            _ => unreachable!(),
        };
        r * 8 + c
    }

    /// Inverse transform of a square (to go back from canonical to original).
    fn inverse_transform_square(sq: usize, sym: usize) -> usize {
        // Each symmetry is its own inverse (involutions) except for 4<->6 and 5<->7
        let inv = match sym {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            5 => 6,
            6 => 5,
            _ => 7,
        };
        Self::transform_square(sq, inv)
    }

    /// Save book to a simple binary format.
    pub fn save(&self, data: &mut Vec<u8>) {
        // Header: number of entries (4 bytes LE)
        let n = self.positions.len() as u32;
        data.extend_from_slice(&n.to_le_bytes());

        for (&(player, opponent), entry) in &self.positions {
            data.extend_from_slice(&player.to_le_bytes());
            data.extend_from_slice(&opponent.to_le_bytes());
            data.extend_from_slice(&(entry.best_move as i32).to_le_bytes());
            data.extend_from_slice(&entry.score.to_le_bytes());
            data.extend_from_slice(&entry.depth.to_le_bytes());
            data.extend_from_slice(&entry.n_games.to_le_bytes());
        }
    }

    /// Load book from binary data.
    pub fn load(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }

        let n = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let entry_size = 8 + 8 + 4 + 4 + 4 + 4; // 32 bytes per entry
        if data.len() < 4 + n * entry_size {
            return None;
        }

        let mut book = Book::new();
        let mut offset = 4;

        for _ in 0..n {
            let player = u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?);
            offset += 8;
            let opponent = u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?);
            offset += 8;
            let best_move = i32::from_le_bytes(data[offset..offset + 4].try_into().ok()?);
            offset += 4;
            let score = i32::from_le_bytes(data[offset..offset + 4].try_into().ok()?);
            offset += 4;
            let depth = i32::from_le_bytes(data[offset..offset + 4].try_into().ok()?);
            offset += 4;
            let n_games = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?);
            offset += 4;

            book.positions.insert(
                (player, opponent),
                BookEntry {
                    best_move,
                    score,
                    depth,
                    n_games,
                },
            );
        }

        Some(book)
    }

    /// Build a small opening book by analyzing common positions.
    pub fn build_default(search_depth: i32) -> Self {
        use crate::search::{Search, SearchOptions};

        let mut book = Book::new();
        let mut search = Search::new(SearchOptions {
            depth: search_depth,
            time_limit_ms: 60000,
            hash_size: 1 << 18,
            verbose: false,
        });

        // Analyze initial position and a few opening moves
        let board = Board::new();
        let result = search.search(&board);
        book.store(&board, result.best_move, result.score, result.depth);

        // Analyze first few moves
        let opening_moves = [19, 26, 37, 44]; // D3, C4, F5, E6
        for &first_move in &opening_moves {
            let mut b = Board::new();
            b.do_move(first_move);
            let r = search.search(&b);
            book.store(&b, r.best_move, r.score, r.depth);
        }

        book
    }
}

impl Default for Book {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_store_and_get() {
        let mut book = Book::new();
        let board = Board::new();
        book.store(&board, 19, 3, 10);

        let entry = book.get(&board);
        assert!(entry.is_some());
        let e = entry.unwrap();
        assert_eq!(e.score, 3);
        assert_eq!(e.depth, 10);
    }

    #[test]
    fn book_empty_lookup() {
        let book = Book::new();
        let board = Board::new();
        assert!(book.get(&board).is_none());
    }

    #[test]
    fn book_symmetry_lookup() {
        let mut book = Book::new();
        let board = Board::new();
        book.store(&board, 19, 5, 8);

        // Lookup with horizontally mirrored board should find same entry
        let mirrored = Board {
            player: bit::horizontal_mirror(board.player),
            opponent: bit::horizontal_mirror(board.opponent),
        };
        let entry = book.get(&mirrored);
        assert!(entry.is_some(), "symmetric lookup should work");
    }

    #[test]
    fn book_save_load_roundtrip() {
        let mut book = Book::new();
        let board = Board::new();
        book.store(&board, 19, 3, 10);

        let mut data = Vec::new();
        book.save(&mut data);

        let loaded = Book::load(&data);
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.len(), 1);

        let entry = loaded.get(&board);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().score, 3);
    }

    #[test]
    fn book_update_deeper_search() {
        let mut book = Book::new();
        let board = Board::new();

        book.store(&board, 19, 3, 5);
        book.store(&board, 26, 7, 10); // deeper search

        let entry = book.get(&board).unwrap();
        assert_eq!(entry.score, 7); // should use deeper result
        assert_eq!(entry.depth, 10);
    }

    #[test]
    fn book_len() {
        let mut book = Book::new();
        assert_eq!(book.len(), 0);
        assert!(book.is_empty());

        let board = Board::new();
        book.store(&board, 19, 0, 5);
        assert_eq!(book.len(), 1);
        assert!(!book.is_empty());
    }

    #[test]
    fn canonicalize_is_deterministic() {
        let board = Board::new();
        let (c1, s1) = Book::canonicalize(&board);
        let (c2, s2) = Book::canonicalize(&board);
        assert_eq!(c1, c2);
        assert_eq!(s1, s2);
    }

    #[test]
    fn canonicalize_same_for_symmetric_boards() {
        let board = Board::new();
        let mirrored = Board {
            player: bit::horizontal_mirror(board.player),
            opponent: bit::horizontal_mirror(board.opponent),
        };
        let (c1, _) = Book::canonicalize(&board);
        let (c2, _) = Book::canonicalize(&mirrored);
        assert_eq!(c1, c2);
    }

    #[test]
    fn build_default_book_has_entries() {
        let book = Book::build_default(4);
        assert!(book.len() >= 1, "default book should have at least 1 entry");
    }
}
