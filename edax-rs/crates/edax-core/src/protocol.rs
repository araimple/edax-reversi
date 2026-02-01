//! Edax protocol interface - command-line interactive Othello engine.
//!
//! Supports core Edax commands: init, go, play, undo, setboard, hint, etc.

use crate::board::Board;
use crate::book::Book;
use crate::search::{Search, SearchOptions, SearchResult};

/// Player color.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Black = 0,
    White = 1,
}

impl Color {
    pub fn opponent(self) -> Color {
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }
}

/// Game state for the Edax protocol engine.
pub struct EdaxEngine {
    pub board: Board,
    pub current_player: Color,
    pub search: Search,
    pub book: Book,
    pub history: Vec<(Board, Color, i32)>, // (board, player, move)
    pub level: i32,
}

impl EdaxEngine {
    pub fn new(options: SearchOptions) -> Self {
        let level = options.depth;
        EdaxEngine {
            board: Board::new(),
            current_player: Color::Black,
            search: Search::new(options),
            book: Book::new(),
            history: Vec::new(),
            level,
        }
    }

    /// Initialize a new game.
    pub fn init(&mut self) {
        self.board = Board::new();
        self.current_player = Color::Black;
        self.history.clear();
    }

    /// Set board from a string representation.
    /// Format: 64 chars (X=black, O=white, -=empty) + " " + color_to_move (X or O)
    pub fn set_board(&mut self, s: &str) -> Result<(), String> {
        let s = s.trim();
        if s.len() < 66 {
            return Err("Board string too short (need 64 chars + space + color)".to_string());
        }

        let board_str = &s[..64];
        let color_char = s.chars().nth(65).unwrap_or('X');

        let mut black = 0u64;
        let mut white = 0u64;

        for (i, c) in board_str.chars().enumerate() {
            match c {
                'X' | 'x' | '*' => black |= 1u64 << i,
                'O' | 'o' | '0' => white |= 1u64 << i,
                '-' | '.' => {}
                _ => return Err(format!("Invalid character '{}' at position {}", c, i)),
            }
        }

        let (player, opponent, color) = match color_char {
            'X' | 'x' | '*' => (black, white, Color::Black),
            'O' | 'o' | '0' => (white, black, Color::White),
            _ => return Err(format!("Invalid color '{}'", color_char)),
        };

        self.board = Board { player, opponent };
        self.current_player = color;
        self.history.clear();
        Ok(())
    }

    /// Parse a move string like "D3" and return the square index.
    pub fn parse_move(s: &str) -> Result<usize, String> {
        let s = s.trim().to_uppercase();
        if s == "PA" || s == "PASS" || s == "--" {
            return Ok(64); // Pass
        }
        if s.len() != 2 {
            return Err(format!("Invalid move '{}' (expected 2 chars like D3)", s));
        }
        let col = s.as_bytes()[0];
        let row = s.as_bytes()[1];
        if !(b'A'..=b'H').contains(&col) || !(b'1'..=b'8').contains(&row) {
            return Err(format!("Invalid move '{}'", s));
        }
        let sq = (row - b'1') as usize * 8 + (col - b'A') as usize;
        Ok(sq)
    }

    /// Play a move (square index 0-63, or 64 for pass).
    pub fn play_move(&mut self, sq: usize) -> Result<(), String> {
        if sq == 64 {
            // Pass
            let moves = self.board.get_moves();
            if moves != 0 {
                return Err("Cannot pass when legal moves exist".to_string());
            }
            self.history.push((self.board, self.current_player, 64));
            self.board.pass();
            self.current_player = self.current_player.opponent();
            return Ok(());
        }

        if sq >= 64 {
            return Err(format!("Invalid square {}", sq));
        }

        let moves = self.board.get_moves();
        if moves & (1u64 << sq) == 0 {
            return Err(format!(
                "{} is not a legal move",
                Board::square_to_string(sq)
            ));
        }

        self.history.push((self.board, self.current_player, sq as i32));
        self.board.do_move(sq);
        self.current_player = self.current_player.opponent();
        Ok(())
    }

    /// Play a sequence of moves (e.g., "D3C3C4").
    pub fn play_moves(&mut self, moves_str: &str) -> Result<(), String> {
        let s = moves_str.trim().to_uppercase();
        let mut i = 0;
        while i + 1 < s.len() {
            let mv_str = &s[i..i + 2];
            let sq = Self::parse_move(mv_str)?;
            self.play_move(sq)?;
            i += 2;
        }
        Ok(())
    }

    /// Undo the last move.
    pub fn undo(&mut self) -> Result<(), String> {
        if let Some((board, player, _mv)) = self.history.pop() {
            self.board = board;
            self.current_player = player;
            Ok(())
        } else {
            Err("No moves to undo".to_string())
        }
    }

    /// Ask the engine to find and return the best move.
    pub fn go(&mut self) -> SearchResult {
        // Check book first
        if let Some(entry) = self.book.get(&self.board) {
            if entry.best_move >= 0 && entry.best_move < 64 {
                let moves = self.board.get_moves();
                if moves & (1u64 << entry.best_move) != 0 {
                    return SearchResult {
                        score: entry.score,
                        best_move: entry.best_move,
                        depth: entry.depth,
                        nodes: 0,
                        pv: vec![entry.best_move],
                        time_ms: 0,
                        is_exact: false,
                    };
                }
            }
        }

        self.search.search(&self.board)
    }

    /// Get hints (top N moves with scores).
    pub fn hint(&mut self, n: usize) -> Vec<(i32, i32)> {
        let moves = self.board.get_moves();
        if moves == 0 {
            return Vec::new();
        }

        let mut results = Vec::new();
        let mut remaining = moves;
        while remaining != 0 {
            let sq = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;

            let mut child = self.board;
            child.do_move(sq);
            let mut child_search = Search::new(SearchOptions {
                depth: self.level.min(self.board.empties() as i32),
                time_limit_ms: self.search.options.time_limit_ms / 4,
                hash_size: self.search.options.hash_size,
                verbose: false,
            });
            let r = child_search.search(&child);
            results.push((sq as i32, -r.score));
        }

        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.truncate(n);
        results
    }

    /// Process a single command string. Returns output to display.
    pub fn process_command(&mut self, cmd: &str) -> String {
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
        if parts.is_empty() {
            return String::new();
        }

        match parts[0].to_lowercase().as_str() {
            "init" | "i" => {
                self.init();
                "New game started.".to_string()
            }
            "go" | "g" => {
                let result = self.go();
                if result.best_move >= 0 && result.best_move < 64 {
                    let mv_str = Board::square_to_string(result.best_move as usize);
                    let _ = self.play_move(result.best_move as usize);
                    format!(
                        "Engine plays {} (score: {:+}, depth: {}, nodes: {})",
                        mv_str, result.score, result.depth, result.nodes
                    )
                } else {
                    "No legal moves.".to_string()
                }
            }
            "play" | "p" => {
                if parts.len() < 2 {
                    return "Usage: play <move>".to_string();
                }
                match Self::parse_move(parts[1]) {
                    Ok(sq) => match self.play_move(sq) {
                        Ok(_) => format!("Played {}", Board::square_to_string(sq)),
                        Err(e) => e,
                    },
                    Err(e) => e,
                }
            }
            "undo" | "u" => match self.undo() {
                Ok(_) => "Move undone.".to_string(),
                Err(e) => e,
            },
            "hint" | "h" => {
                let n = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(4);
                let hints = self.hint(n);
                let mut s = String::new();
                for (i, (mv, score)) in hints.iter().enumerate() {
                    s.push_str(&format!(
                        "{}. {} ({:+})\n",
                        i + 1,
                        Board::square_to_string(*mv as usize),
                        score
                    ));
                }
                s
            }
            "setboard" | "sb" => {
                if parts.len() < 2 {
                    return "Usage: setboard <board_string>".to_string();
                }
                let board_str = parts[1..].join(" ");
                match self.set_board(&board_str) {
                    Ok(_) => "Board set.".to_string(),
                    Err(e) => e,
                }
            }
            "show" | "s" => {
                let player_is_black = self.current_player == Color::Black;
                let mut s = self.board.to_board_string(player_is_black);
                s.push_str(&format!(
                    "\n{} to move. Empties: {}\n",
                    if self.current_player == Color::Black { "Black" } else { "White" },
                    self.board.empties()
                ));
                let moves = self.board.get_moves();
                s.push_str(&format!("Legal moves: {}\n", moves.count_ones()));
                s
            }
            "level" | "l" => {
                if let Some(lv) = parts.get(1).and_then(|s| s.parse().ok()) {
                    self.level = lv;
                    self.search.options.depth = lv;
                    format!("Level set to {}", lv)
                } else {
                    format!("Current level: {}", self.level)
                }
            }
            "quit" | "q" | "exit" => "quit".to_string(),
            "help" | "?" => {
                "Commands:\n\
                 init      - New game\n\
                 go        - Engine plays\n\
                 play <mv> - Play move (e.g., D3)\n\
                 undo      - Undo last move\n\
                 hint [n]  - Show top n moves\n\
                 show      - Display board\n\
                 setboard  - Set position\n\
                 level [n] - Set/show search depth\n\
                 quit      - Exit\n"
                    .to_string()
            }
            _ => format!("Unknown command '{}'. Type 'help' for commands.", parts[0]),
        }
    }

    /// Is the game over?
    pub fn is_game_over(&self) -> bool {
        let moves = self.board.get_moves();
        if moves != 0 {
            return false;
        }
        let mut passed = self.board;
        passed.pass();
        passed.get_moves() == 0
    }

    /// Get the final score (from Black's perspective).
    pub fn final_score(&self) -> i32 {
        let board = if self.current_player == Color::Black {
            self.board
        } else {
            Board {
                player: self.board.opponent,
                opponent: self.board.player,
            }
        };
        board.score()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> EdaxEngine {
        EdaxEngine::new(SearchOptions {
            depth: 6,
            time_limit_ms: 5000,
            hash_size: 1 << 16,
            verbose: false,
        })
    }

    #[test]
    fn engine_init() {
        let engine = make_engine();
        assert_eq!(engine.current_player, Color::Black);
        assert_eq!(engine.board, Board::new());
    }

    #[test]
    fn engine_play_move() {
        let mut engine = make_engine();
        let result = engine.play_move(19); // D3
        assert!(result.is_ok());
        assert_eq!(engine.current_player, Color::White);
    }

    #[test]
    fn engine_play_illegal_move() {
        let mut engine = make_engine();
        let result = engine.play_move(0); // A1 is not legal at start
        assert!(result.is_err());
    }

    #[test]
    fn engine_undo() {
        let mut engine = make_engine();
        let original = engine.board;
        engine.play_move(19).unwrap();
        engine.undo().unwrap();
        assert_eq!(engine.board, original);
        assert_eq!(engine.current_player, Color::Black);
    }

    #[test]
    fn engine_undo_empty() {
        let mut engine = make_engine();
        assert!(engine.undo().is_err());
    }

    #[test]
    fn engine_go() {
        let mut engine = make_engine();
        let result = engine.go();
        assert!(result.best_move >= 0 && result.best_move < 64);
    }

    #[test]
    fn parse_move_d3() {
        assert_eq!(EdaxEngine::parse_move("D3").unwrap(), 19);
    }

    #[test]
    fn parse_move_a1() {
        assert_eq!(EdaxEngine::parse_move("A1").unwrap(), 0);
    }

    #[test]
    fn parse_move_h8() {
        assert_eq!(EdaxEngine::parse_move("H8").unwrap(), 63);
    }

    #[test]
    fn parse_move_pass() {
        assert_eq!(EdaxEngine::parse_move("PA").unwrap(), 64);
    }

    #[test]
    fn parse_move_invalid() {
        assert!(EdaxEngine::parse_move("Z9").is_err());
    }

    #[test]
    fn engine_play_moves_sequence() {
        let mut engine = make_engine();
        let result = engine.play_moves("D3C3");
        assert!(result.is_ok());
        // After D3 (black) and C3 (white), should be black's turn
        assert_eq!(engine.current_player, Color::Black);
    }

    #[test]
    fn engine_show_command() {
        let mut engine = make_engine();
        let output = engine.process_command("show");
        assert!(output.contains("Black to move"));
        assert!(output.contains("A B C D E F G H"));
    }

    #[test]
    fn engine_init_command() {
        let mut engine = make_engine();
        engine.play_move(19).unwrap();
        engine.process_command("init");
        assert_eq!(engine.board, Board::new());
    }

    #[test]
    fn engine_go_command() {
        let mut engine = make_engine();
        let output = engine.process_command("go");
        assert!(output.contains("Engine plays"));
    }

    #[test]
    fn engine_help_command() {
        let mut engine = make_engine();
        let output = engine.process_command("help");
        assert!(output.contains("Commands:"));
    }

    #[test]
    fn engine_unknown_command() {
        let mut engine = make_engine();
        let output = engine.process_command("foobar");
        assert!(output.contains("Unknown command"));
    }

    #[test]
    fn engine_is_game_over_initial() {
        let engine = make_engine();
        assert!(!engine.is_game_over());
    }

    #[test]
    fn engine_game_over_full_board() {
        let mut engine = make_engine();
        engine.board = Board {
            player: 0x00000000FFFFFFFF,
            opponent: 0xFFFFFFFF00000000,
        };
        assert!(engine.is_game_over());
    }

    #[test]
    fn color_opponent() {
        assert_eq!(Color::Black.opponent(), Color::White);
        assert_eq!(Color::White.opponent(), Color::Black);
    }

    #[test]
    fn engine_level_command() {
        let mut engine = make_engine();
        let output = engine.process_command("level 10");
        assert!(output.contains("10"));
        assert_eq!(engine.level, 10);
    }
}
