use crate::ai::Player;
use crate::board::Board;

/// Result of a completed game.
pub struct GameResult {
    pub black_discs: u32,
    pub white_discs: u32,
    pub moves: Vec<usize>,
}

impl GameResult {
    pub fn winner(&self) -> &str {
        if self.black_discs > self.white_discs {
            "Black"
        } else if self.white_discs > self.black_discs {
            "White"
        } else {
            "Draw"
        }
    }
}

/// Play a complete game between two AI players.
/// `black` moves first, `white` moves second.
/// Returns the game result.
pub fn play_game(
    black: &mut dyn Player,
    white: &mut dyn Player,
    verbose: bool,
) -> GameResult {
    let mut board = Board::new();
    let mut is_black_turn = true;
    let mut moves = Vec::new();
    let mut consecutive_passes = 0;

    if verbose {
        println!("=== {} (X) vs {} (O) ===\n", black.name(), white.name());
        println!("{}", board.to_board_string(true));
    }

    loop {
        let legal = board.get_moves();

        if legal == 0 {
            consecutive_passes += 1;
            if consecutive_passes >= 2 {
                break; // game over
            }
            if verbose {
                let who = if is_black_turn { "Black" } else { "White" };
                println!("{} passes.\n", who);
            }
            board.pass();
            is_black_turn = !is_black_turn;
            continue;
        }

        consecutive_passes = 0;

        let sq = if is_black_turn {
            black.choose_move(&board)
        } else {
            white.choose_move(&board)
        };

        moves.push(sq);
        board.do_move(sq);

        if verbose {
            let who = if is_black_turn { "Black" } else { "White" };
            println!(
                "#{:2}: {} plays {}\n{}",
                moves.len(),
                who,
                Board::square_to_string(sq),
                board.to_board_string(!is_black_turn) // after do_move, player/opponent swapped
            );
        }

        is_black_turn = !is_black_turn;
    }

    // Count final discs. Board's player/opponent depends on whose turn it is.
    let (black_discs, white_discs) = if is_black_turn {
        (board.count_player_discs(), board.count_opponent_discs())
    } else {
        (board.count_opponent_discs(), board.count_player_discs())
    };

    if verbose {
        println!("=== Game Over ===");
        println!("Black(X): {}  White(O): {}", black_discs, white_discs);
        println!("Winner: {}\n", if black_discs > white_discs {
            "Black"
        } else if white_discs > black_discs {
            "White"
        } else {
            "Draw"
        });
    }

    GameResult {
        black_discs,
        white_discs,
        moves,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{GreedyPlayer, MobilityPlayer, RandomPlayer};

    #[test]
    fn game_completes_with_valid_disc_count() {
        let mut black = RandomPlayer::new(1);
        let mut white = RandomPlayer::new(2);
        let result = play_game(&mut black, &mut white, false);
        let total = result.black_discs + result.white_discs;
        // Total discs must be between 4 (minimum if all flipped away) and 64
        assert!(total <= 64, "total discs {} > 64", total);
        assert!(total >= 2, "total discs {} < 2", total);
    }

    #[test]
    fn game_has_at_least_one_move() {
        let mut black = RandomPlayer::new(42);
        let mut white = RandomPlayer::new(99);
        let result = play_game(&mut black, &mut white, false);
        assert!(!result.moves.is_empty(), "game should have at least one move");
    }

    #[test]
    fn game_discs_fill_board_or_end_early() {
        let mut black = GreedyPlayer;
        let mut white = GreedyPlayer;
        let result = play_game(&mut black, &mut white, false);
        // Discs + empties = 64
        let total = result.black_discs + result.white_discs;
        assert!(total <= 64);
    }

    #[test]
    fn winner_returns_correct_string() {
        let r = GameResult { black_discs: 40, white_discs: 24, moves: vec![] };
        assert_eq!(r.winner(), "Black");
        let r = GameResult { black_discs: 20, white_discs: 44, moves: vec![] };
        assert_eq!(r.winner(), "White");
        let r = GameResult { black_discs: 32, white_discs: 32, moves: vec![] };
        assert_eq!(r.winner(), "Draw");
    }

    #[test]
    fn mobility_vs_greedy_completes() {
        let mut black = MobilityPlayer;
        let mut white = GreedyPlayer;
        let result = play_game(&mut black, &mut white, false);
        assert!(result.black_discs + result.white_discs <= 64);
    }

    // --- Match / tournament tests ---

    #[test]
    fn deterministic_game_produces_same_result() {
        // Same seeds should produce identical games
        let mut b1 = RandomPlayer::new(100);
        let mut w1 = RandomPlayer::new(200);
        let r1 = play_game(&mut b1, &mut w1, false);

        let mut b2 = RandomPlayer::new(100);
        let mut w2 = RandomPlayer::new(200);
        let r2 = play_game(&mut b2, &mut w2, false);

        assert_eq!(r1.black_discs, r2.black_discs);
        assert_eq!(r1.white_discs, r2.white_discs);
        assert_eq!(r1.moves, r2.moves);
    }

    #[test]
    fn different_seeds_produce_different_games() {
        let mut b1 = RandomPlayer::new(1);
        let mut w1 = RandomPlayer::new(2);
        let r1 = play_game(&mut b1, &mut w1, false);

        let mut b2 = RandomPlayer::new(999);
        let mut w2 = RandomPlayer::new(1000);
        let r2 = play_game(&mut b2, &mut w2, false);

        // Extremely unlikely to produce identical move sequences
        assert_ne!(r1.moves, r2.moves, "different seeds should produce different games");
    }

    #[test]
    fn random_vs_random_100_games_all_valid() {
        for i in 0..100 {
            let mut b = RandomPlayer::new(i * 31 + 7);
            let mut w = RandomPlayer::new(i * 17 + 3);
            let r = play_game(&mut b, &mut w, false);
            let total = r.black_discs + r.white_discs;
            assert!(total >= 2 && total <= 64,
                "game {} invalid disc count: {}", i, total);
            assert!(!r.moves.is_empty(),
                "game {} had no moves", i);
        }
    }

    #[test]
    fn all_moves_are_on_board() {
        let mut b = RandomPlayer::new(42);
        let mut w = RandomPlayer::new(99);
        let r = play_game(&mut b, &mut w, false);
        for &sq in &r.moves {
            assert!(sq < 64, "move square {} out of range", sq);
        }
    }

    #[test]
    fn greedy_vs_greedy_is_deterministic() {
        // Deterministic players should always produce the same game
        let r1 = play_game(&mut GreedyPlayer, &mut GreedyPlayer, false);
        let r2 = play_game(&mut GreedyPlayer, &mut GreedyPlayer, false);
        assert_eq!(r1.moves, r2.moves);
        assert_eq!(r1.black_discs, r2.black_discs);
        assert_eq!(r1.white_discs, r2.white_discs);
    }

    #[test]
    fn mobility_vs_mobility_is_deterministic() {
        let r1 = play_game(&mut MobilityPlayer, &mut MobilityPlayer, false);
        let r2 = play_game(&mut MobilityPlayer, &mut MobilityPlayer, false);
        assert_eq!(r1.moves, r2.moves);
        assert_eq!(r1.black_discs, r2.black_discs);
        assert_eq!(r1.white_discs, r2.white_discs);
    }

    #[test]
    fn greedy_beats_random_majority() {
        // Greedy should win more than 50% against Random
        let mut greedy_wins = 0;
        for i in 0..50 {
            let mut b = GreedyPlayer;
            let mut w = RandomPlayer::new(i * 37 + 5);
            let r = play_game(&mut b, &mut w, false);
            if r.winner() == "Black" {
                greedy_wins += 1;
            }
        }
        assert!(greedy_wins > 25,
            "Greedy should beat Random >50%, but only won {}/50", greedy_wins);
    }

    #[test]
    fn mobility_beats_random_majority() {
        // Mobility should win more than 50% against Random
        let mut mobility_wins = 0;
        for i in 0..50 {
            let mut b = MobilityPlayer;
            let mut w = RandomPlayer::new(i * 13 + 1);
            let r = play_game(&mut b, &mut w, false);
            if r.winner() == "Black" {
                mobility_wins += 1;
            }
        }
        assert!(mobility_wins > 25,
            "Mobility should beat Random >50%, but only won {}/50", mobility_wins);
    }

    #[test]
    fn winner_consistent_with_disc_counts() {
        // Run several games and verify winner() matches disc comparison
        for i in 0..20 {
            let mut b = RandomPlayer::new(i * 7 + 1);
            let mut w = RandomPlayer::new(i * 11 + 3);
            let r = play_game(&mut b, &mut w, false);
            match r.winner() {
                "Black" => assert!(r.black_discs > r.white_discs,
                    "game {}: winner=Black but black={} white={}", i, r.black_discs, r.white_discs),
                "White" => assert!(r.white_discs > r.black_discs,
                    "game {}: winner=White but black={} white={}", i, r.black_discs, r.white_discs),
                "Draw" => assert_eq!(r.black_discs, r.white_discs,
                    "game {}: winner=Draw but black={} white={}", i, r.black_discs, r.white_discs),
                other => panic!("game {}: unexpected winner '{}'", i, other),
            }
        }
    }

    #[test]
    fn verbose_mode_does_not_affect_result() {
        // Same game with verbose=true and verbose=false should produce identical results
        let mut b1 = RandomPlayer::new(77);
        let mut w1 = RandomPlayer::new(88);
        let r1 = play_game(&mut b1, &mut w1, false);

        let mut b2 = RandomPlayer::new(77);
        let mut w2 = RandomPlayer::new(88);
        let r2 = play_game(&mut b2, &mut w2, true);

        assert_eq!(r1.moves, r2.moves);
        assert_eq!(r1.black_discs, r2.black_discs);
        assert_eq!(r1.white_discs, r2.white_discs);
    }

    #[test]
    fn all_ai_combinations_complete() {
        // Every combination of AI types should complete a game without panicking
        let run = |mut b: Box<dyn Player>, mut w: Box<dyn Player>| {
            let r = play_game(b.as_mut(), w.as_mut(), false);
            assert!(r.black_discs + r.white_discs <= 64);
            assert!(!r.moves.is_empty());
        };

        run(Box::new(RandomPlayer::new(1)), Box::new(RandomPlayer::new(2)));
        run(Box::new(RandomPlayer::new(3)), Box::new(GreedyPlayer));
        run(Box::new(RandomPlayer::new(4)), Box::new(MobilityPlayer));
        run(Box::new(GreedyPlayer), Box::new(RandomPlayer::new(5)));
        run(Box::new(GreedyPlayer), Box::new(GreedyPlayer));
        run(Box::new(GreedyPlayer), Box::new(MobilityPlayer));
        run(Box::new(MobilityPlayer), Box::new(RandomPlayer::new(6)));
        run(Box::new(MobilityPlayer), Box::new(GreedyPlayer));
        run(Box::new(MobilityPlayer), Box::new(MobilityPlayer));
    }

    #[test]
    fn game_move_count_in_reasonable_range() {
        // Othello games typically have 40-60 moves, but edge cases go lower
        for i in 0..30 {
            let mut b = RandomPlayer::new(i * 41 + 9);
            let mut w = RandomPlayer::new(i * 23 + 7);
            let r = play_game(&mut b, &mut w, false);
            assert!(r.moves.len() >= 1, "game {} had 0 moves", i);
            assert!(r.moves.len() <= 60, "game {} had {} moves (>60)", i, r.moves.len());
        }
    }
}
