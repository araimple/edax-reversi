//! Transposition table for caching search results.
//!
//! Uses a fixed-size hash table with 4-way set-associative buckets.
//! Each entry stores the full board position (no collisions from hash aliasing).

use crate::board::Board;

/// Score constants
pub const SCORE_INF: i32 = 127;

/// No move
pub const NOMOVE: i32 = -1;

/// Hash entry data (search result).
#[derive(Clone, Copy, Debug)]
pub struct HashData {
    pub depth: u8,
    pub selectivity: u8,
    pub cost: u8,
    pub date: u8,
    pub lower: i8,
    pub upper: i8,
    pub best_move: [i8; 2],
}

impl HashData {
    pub const INIT: Self = HashData {
        depth: 0,
        selectivity: 0,
        cost: 0,
        date: 0,
        lower: -SCORE_INF as i8,
        upper: SCORE_INF as i8,
        best_move: [NOMOVE as i8, NOMOVE as i8],
    };
}

/// A single hash table entry: board + search data.
#[derive(Clone, Copy, Debug)]
struct HashEntry {
    board: Board,
    data: HashData,
}

impl HashEntry {
    const EMPTY: Self = HashEntry {
        board: Board { player: 0, opponent: 0 },
        data: HashData::INIT,
    };
}

/// Number of entries per bucket (set-associativity).
const HASH_N_WAY: usize = 4;

/// Transposition hash table.
pub struct HashTable {
    buckets: Vec<[HashEntry; HASH_N_WAY]>,
    mask: usize,
    date: u8,
}

impl HashTable {
    /// Create a new hash table with approximately `size` entries.
    /// Actual size is rounded down to power of 2.
    pub fn new(size: usize) -> Self {
        let n_buckets = (size / HASH_N_WAY).next_power_of_two().max(1);
        HashTable {
            buckets: vec![[HashEntry::EMPTY; HASH_N_WAY]; n_buckets],
            mask: n_buckets - 1,
            date: 0,
        }
    }

    /// Clear the table by incrementing the date (lazy clearing).
    pub fn clear(&mut self) {
        self.date = self.date.wrapping_add(1);
        if self.date == 0 {
            // Full clear on wrap
            for bucket in &mut self.buckets {
                *bucket = [HashEntry::EMPTY; HASH_N_WAY];
            }
            self.date = 1;
        }
    }

    /// Compute hash code for a board position.
    pub fn hash_code(board: &Board) -> u64 {
        // Use a simple but effective hash combining player and opponent
        let mut h = board.player;
        h = h.wrapping_mul(0x517cc1b727220a95);
        h ^= board.opponent;
        h = h.wrapping_mul(0x6c62272e07bb0142);
        h ^= h >> 32;
        h
    }

    /// Get bucket index from hash code.
    fn bucket_index(&self, hash_code: u64) -> usize {
        (hash_code as usize) & self.mask
    }

    /// Look up a board position in the hash table.
    /// Returns Some(data) if found, None otherwise.
    pub fn get(&self, board: &Board, hash_code: u64) -> Option<HashData> {
        let idx = self.bucket_index(hash_code);
        let bucket = &self.buckets[idx];
        for entry in bucket {
            if entry.board == *board && entry.data.date == self.date {
                return Some(entry.data);
            }
        }
        None
    }

    /// Store a search result in the hash table.
    pub fn store(
        &mut self,
        board: &Board,
        hash_code: u64,
        depth: i32,
        selectivity: i32,
        alpha: i32,
        beta: i32,
        score: i32,
        best_move: i32,
    ) {
        let idx = self.bucket_index(hash_code);
        let bucket = &mut self.buckets[idx];

        // Look for existing entry or worst entry to replace
        let mut replace_idx = 0;
        let mut worst_priority = i32::MAX;

        for i in 0..HASH_N_WAY {
            let entry = &bucket[i];

            // Found same position: update
            if entry.board == *board && entry.data.date == self.date {
                let data = &mut bucket[i].data;
                // Update bounds
                if score >= beta {
                    if score > data.lower as i32 {
                        data.lower = score as i8;
                    }
                } else if score <= alpha {
                    if score < data.upper as i32 {
                        data.upper = score as i8;
                    }
                } else {
                    data.lower = score as i8;
                    data.upper = score as i8;
                }
                // Update move and depth if deeper
                if depth >= data.depth as i32 {
                    data.depth = depth as u8;
                    data.selectivity = selectivity as u8;
                    if best_move >= 0 {
                        if data.best_move[0] < 0 || data.best_move[0] == best_move as i8 {
                            data.best_move[0] = best_move as i8;
                        } else {
                            data.best_move[1] = data.best_move[0];
                            data.best_move[0] = best_move as i8;
                        }
                    }
                }
                return;
            }

            // Compute replacement priority (lower = more replaceable)
            let priority = if entry.data.date != self.date {
                -1 // Old entries are most replaceable
            } else {
                entry.data.depth as i32 * 2 + entry.data.cost as i32
            };
            if priority < worst_priority {
                worst_priority = priority;
                replace_idx = i;
            }
        }

        // Replace worst entry
        let entry = &mut bucket[replace_idx];
        entry.board = *board;
        entry.data = HashData {
            depth: depth as u8,
            selectivity: selectivity as u8,
            cost: 0,
            date: self.date,
            lower: if score >= beta { score as i8 } else { -SCORE_INF as i8 },
            upper: if score <= alpha { score as i8 } else { SCORE_INF as i8 },
            best_move: if best_move >= 0 {
                [best_move as i8, NOMOVE as i8]
            } else {
                [NOMOVE as i8, NOMOVE as i8]
            },
        };
        if score > alpha && score < beta {
            entry.data.lower = score as i8;
            entry.data.upper = score as i8;
        }
    }

    /// Get the number of buckets.
    pub fn len(&self) -> usize {
        self.buckets.len() * HASH_N_WAY
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.buckets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_table_creation() {
        let ht = HashTable::new(1024);
        assert!(ht.len() >= 1024);
    }

    #[test]
    fn hash_table_store_and_get() {
        let mut ht = HashTable::new(1024);
        let board = Board::new();
        let hc = HashTable::hash_code(&board);

        ht.store(&board, hc, 10, 5, -64, 64, 5, 19);

        let data = ht.get(&board, hc);
        assert!(data.is_some());
        let d = data.unwrap();
        assert_eq!(d.depth, 10);
        assert_eq!(d.lower, 5);
        assert_eq!(d.upper, 5);
        assert_eq!(d.best_move[0], 19);
    }

    #[test]
    fn hash_table_miss() {
        let ht = HashTable::new(1024);
        let board = Board::new();
        let hc = HashTable::hash_code(&board);
        assert!(ht.get(&board, hc).is_none());
    }

    #[test]
    fn hash_table_clear_invalidates() {
        let mut ht = HashTable::new(1024);
        let board = Board::new();
        let hc = HashTable::hash_code(&board);

        ht.store(&board, hc, 10, 5, -64, 64, 5, 19);
        assert!(ht.get(&board, hc).is_some());

        ht.clear();
        assert!(ht.get(&board, hc).is_none());
    }

    #[test]
    fn hash_table_update_bounds() {
        let mut ht = HashTable::new(1024);
        let board = Board::new();
        let hc = HashTable::hash_code(&board);

        // Store a fail-high (score >= beta)
        ht.store(&board, hc, 10, 5, -10, 5, 8, 19);
        let d = ht.get(&board, hc).unwrap();
        assert_eq!(d.lower, 8); // lower bound updated

        // Store a fail-low (score <= alpha)
        ht.store(&board, hc, 10, 5, 10, 20, 3, 19);
        let d = ht.get(&board, hc).unwrap();
        assert_eq!(d.upper, 3); // upper bound updated
    }

    #[test]
    fn hash_code_differs_for_different_boards() {
        let b1 = Board::new();
        let mut b2 = Board::new();
        b2.do_move(19);

        let h1 = HashTable::hash_code(&b1);
        let h2 = HashTable::hash_code(&b2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_table_multiple_entries() {
        let mut ht = HashTable::new(1024);
        let mut boards = Vec::new();

        // Store 20 different positions
        let mut board = Board::new();
        for sq in [19, 18, 26, 37, 44] {
            let moves = board.get_moves();
            if moves & (1u64 << sq) != 0 {
                boards.push(board);
                let hc = HashTable::hash_code(&board);
                ht.store(&board, hc, 5, 5, -64, 64, 0, sq as i32);
                board.do_move(sq);
            }
        }

        // All should be retrievable
        for b in &boards {
            let hc = HashTable::hash_code(b);
            assert!(ht.get(b, hc).is_some(), "board should be in hash table");
        }
    }

    #[test]
    fn hash_data_init_has_wide_bounds() {
        let d = HashData::INIT;
        assert_eq!(d.lower, -SCORE_INF as i8);
        assert_eq!(d.upper, SCORE_INF as i8);
        assert_eq!(d.best_move[0], NOMOVE as i8);
    }
}
