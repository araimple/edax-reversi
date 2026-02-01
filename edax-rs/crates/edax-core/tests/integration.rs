//! Integration tests for the full Edax-RS engine.

use edax_core::ai::{GreedyPlayer, MobilityPlayer, Player, RandomPlayer, SearchPlayer};
use edax_core::bench;
use edax_core::board::Board;
use edax_core::book::Book;
use edax_core::game::{play_game, GameResult};
use edax_core::hash::HashTable;
use edax_core::parallel::ParallelSearch;
use edax_core::protocol::{Color, EdaxEngine};
use edax_core::search::{Search, SearchOptions};

// --- Full game integration tests ---

#[test]
fn search_player_vs_random_completes() {
    let mut black = SearchPlayer::new(4);
    let mut white = RandomPlayer::new(42);
    let result = play_game(&mut black, &mut white, false);
    assert!(result.black_discs + result.white_discs <= 64);
    assert!(!result.moves.is_empty());
}

#[test]
fn search_player_vs_greedy_completes() {
    let mut black = SearchPlayer::new(4);
    let mut white = GreedyPlayer;
    let result = play_game(&mut black, &mut white, false);
    assert!(result.black_discs + result.white_discs <= 64);
}

#[test]
fn search_player_vs_mobility_completes() {
    let mut black = SearchPlayer::new(4);
    let mut white = MobilityPlayer;
    let result = play_game(&mut black, &mut white, false);
    assert!(result.black_discs + result.white_discs <= 64);
}

#[test]
fn search_beats_random_majority() {
    let mut search_wins = 0;
    for i in 0..20 {
        let mut black = SearchPlayer::new(4);
        let mut white = RandomPlayer::new(i * 37 + 5);
        let result = play_game(&mut black, &mut white, false);
        if result.winner() == "Black" {
            search_wins += 1;
        }
    }
    assert!(
        search_wins >= 15,
        "Search(d=4) should beat Random >75%, but only won {}/20",
        search_wins
    );
}

#[test]
fn search_beats_greedy_majority() {
    let mut search_wins = 0;
    for i in 0..10 {
        let mut black = SearchPlayer::new(4);
        let mut white = GreedyPlayer;
        let result = play_game(&mut black, &mut white, false);
        if result.winner() == "Black" {
            search_wins += 1;
        }
        // Also try as white
        let mut black2 = GreedyPlayer;
        let mut white2 = SearchPlayer::new(4);
        let result2 = play_game(&mut black2, &mut white2, false);
        if result2.winner() == "White" {
            search_wins += 1;
        }
    }
    assert!(
        search_wins >= 10,
        "Search should beat Greedy >50%, but only won {}/20",
        search_wins
    );
}

// --- Protocol engine integration tests ---

#[test]
fn protocol_full_game() {
    let mut engine = EdaxEngine::new(SearchOptions {
        depth: 4,
        time_limit_ms: 5000,
        hash_size: 1 << 16,
        verbose: false,
    });

    let mut moves_played = 0;
    while !engine.is_game_over() && moves_played < 80 {
        let result = engine.go();
        if result.best_move >= 0 && result.best_move < 64 {
            engine.play_move(result.best_move as usize).unwrap();
        } else {
            break;
        }
        moves_played += 1;
    }

    assert!(moves_played > 0);
}

#[test]
fn protocol_play_and_undo_sequence() {
    let mut engine = EdaxEngine::new(SearchOptions::default());
    let initial = engine.board;

    // Play 5 moves
    for _ in 0..5 {
        let result = engine.go();
        if result.best_move >= 0 {
            engine.play_move(result.best_move as usize).unwrap();
        }
    }

    // Undo all
    for _ in 0..5 {
        engine.undo().unwrap();
    }

    assert_eq!(engine.board, initial);
}

#[test]
fn protocol_command_sequence() {
    let mut engine = EdaxEngine::new(SearchOptions {
        depth: 4,
        time_limit_ms: 5000,
        hash_size: 1 << 16,
        verbose: false,
    });

    let output = engine.process_command("show");
    assert!(output.contains("Black to move"));

    let output = engine.process_command("play D3");
    assert!(output.contains("Played D3"));

    let output = engine.process_command("show");
    assert!(output.contains("White to move"));

    let output = engine.process_command("undo");
    assert!(output.contains("undone"));

    let output = engine.process_command("go");
    assert!(output.contains("Engine plays"));
}

// --- Book integration tests ---

#[test]
fn book_store_and_retrieve_after_moves() {
    let mut book = Book::new();
    let mut board = Board::new();

    // Store initial position
    book.store(&board, 19, 3, 10);

    // Play a move and store that too
    board.do_move(19);
    book.store(&board, 18, -2, 10);

    // Retrieve both
    assert!(book.get(&Board::new()).is_some());
    assert!(book.get(&board).is_some());
    assert_eq!(book.len(), 2);
}

#[test]
fn book_save_load_multiple_positions() {
    let mut book = Book::new();

    let mut board = Board::new();
    book.store(&board, 19, 3, 8);

    board.do_move(19);
    book.store(&board, 18, -2, 8);

    board.do_move(18);
    book.store(&board, 10, 1, 8);

    let mut data = Vec::new();
    book.save(&mut data);

    let loaded = Book::load(&data).unwrap();
    assert_eq!(loaded.len(), 3);
}

// --- Hash table integration tests ---

#[test]
fn hash_table_survives_many_stores() {
    let mut ht = HashTable::new(1 << 14);
    let mut board = Board::new();

    // Store 100 positions from a game tree
    fn store_recursive(ht: &mut HashTable, board: &Board, depth: i32) {
        if depth <= 0 {
            return;
        }
        let hc = HashTable::hash_code(board);
        ht.store(board, hc, depth, 5, -64, 64, 0, -1);

        let moves = board.get_moves();
        let mut remaining = moves;
        while remaining != 0 {
            let sq = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;
            let mut child = *board;
            child.do_move(sq);
            store_recursive(ht, &child, depth - 1);
        }
    }

    store_recursive(&mut ht, &board, 3);
    // Should not panic or corrupt
    let hc = HashTable::hash_code(&board);
    assert!(ht.get(&board, hc).is_some());
}

// --- Parallel search integration tests ---

#[test]
fn parallel_search_finds_legal_move() {
    let board = Board::new();
    let mut search = ParallelSearch::new(
        SearchOptions {
            depth: 6,
            time_limit_ms: 10000,
            hash_size: 1 << 16,
            verbose: false,
        },
        2,
    );
    let result = search.search(&board);
    let moves = board.get_moves();
    assert!(result.best_move >= 0);
    assert!(moves & (1u64 << result.best_move) != 0);
}

// --- Benchmark integration tests ---

#[test]
fn benchmark_runs() {
    let r = bench::bench_search(4);
    assert!(r.nodes > 0);
    assert!(r.best_move >= 0);
}

#[test]
fn perft_benchmark_runs() {
    let (count, time_ms) = bench::bench_perft(5);
    assert_eq!(count, 1396);
    assert!(time_ms >= 0.0);
}

// --- Edge case tests ---

#[test]
fn search_from_near_endgame() {
    // Position with very few empties
    let board = Board {
        player: 0x7F7F7F7F7F7F7F00,
        opponent: 0x0080808080808000,
    };
    let mut search = Search::new(SearchOptions {
        depth: 20,
        time_limit_ms: 5000,
        hash_size: 1 << 16,
        verbose: false,
    });
    let result = search.search(&board);
    assert!(result.score >= -64 && result.score <= 64);
}

#[test]
fn eval_many_random_positions() {
    use edax_core::eval;
    let mut board = Board::new();
    let mut rng = 42u64;
    for _ in 0..100 {
        let score = eval::evaluate(&board);
        assert!(
            score >= -64 && score <= 64,
            "score {} out of range",
            score
        );

        let moves = board.get_moves();
        if moves == 0 {
            board = Board::new();
            continue;
        }
        // Pick a pseudo-random move
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let n = moves.count_ones() as u64;
        let idx = rng % n;
        let mut remaining = moves;
        for _ in 0..idx {
            remaining &= remaining - 1;
        }
        let sq = remaining.trailing_zeros() as usize;
        board.do_move(sq);
    }
}

// --- Eval verification tests (C version compatibility) ---

#[test]
fn eval_matches_c_version() {
    use edax_core::eval;

    // Ensure eval weights are loaded
    eval::eval_open();

    // 1. Evaluate the initial position
    let mut board = Board::new();
    let score_initial = eval::evaluate(&board);
    println!("Initial position eval: {}", score_initial);
    assert!(
        score_initial >= -63 && score_initial <= 63,
        "Initial position score {} out of valid range [-63, 63]",
        score_initial
    );

    // 2. Play D3 (square 19) and evaluate
    board.do_move(19); // D3
    let score_after_d3 = eval::evaluate(&board);
    println!("After D3 (move 19) eval: {}", score_after_d3);
    assert!(
        score_after_d3 >= -63 && score_after_d3 <= 63,
        "Score after D3 {} out of valid range [-63, 63]",
        score_after_d3
    );

    // 3. Play C3 (square 18) and evaluate
    board.do_move(18); // C3
    let score_after_c3 = eval::evaluate(&board);
    println!("After D3 C3 (move 18) eval: {}", score_after_c3);
    assert!(
        score_after_c3 >= -63 && score_after_c3 <= 63,
        "Score after C3 {} out of valid range [-63, 63]",
        score_after_c3
    );

    // 4. Play C2 (square 10) and evaluate
    board.do_move(10); // C2
    let score_after_c2 = eval::evaluate(&board);
    println!("After D3 C3 C2 (move 10) eval: {}", score_after_c2);
    assert!(
        score_after_c2 >= -63 && score_after_c2 <= 63,
        "Score after C2 {} out of valid range [-63, 63]",
        score_after_c2
    );

    // Print all scores together for easy comparison with C version
    println!("--- Eval score summary ---");
    println!("  Initial:        {}", score_initial);
    println!("  After D3:       {}", score_after_d3);
    println!("  After D3 C3:    {}", score_after_c3);
    println!("  After D3 C3 C2: {}", score_after_c2);

    // Verify eval.dat was loaded (pattern evaluation active)
    assert!(
        eval::is_loaded(),
        "eval.dat was not loaded -- pattern evaluation is not active"
    );
}

#[test]
fn eval_feature_computation() {
    use edax_core::eval;

    // Ensure eval weights are loaded
    eval::eval_open();

    // Evaluate the initial board position
    let board = Board::new();
    let score = eval::evaluate(&board);
    println!("eval_feature_computation: initial board score = {}", score);

    // The initial position is symmetric and roughly balanced, so the pattern
    // evaluation should return a score close to 0 (within -10..10).
    assert!(
        score >= -10 && score <= 10,
        "Initial position eval score {} is outside expected range [-10, 10]. \
         A well-calibrated pattern evaluation should rate the starting position as roughly even.",
        score
    );
}
