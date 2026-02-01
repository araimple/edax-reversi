use edax_core::ai::{GreedyPlayer, MobilityPlayer, Player, RandomPlayer, SearchPlayer};
use edax_core::game::play_game;
use edax_core::protocol::EdaxEngine;
use edax_core::search::SearchOptions;
use std::env;
use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "interactive" | "i" => interactive_mode(),
            "match" | "m" => match_mode(),
            "tournament" | "t" => tournament_mode(),
            "bench" | "b" => bench_mode(),
            _ => {
                eprintln!("Usage: edax-cli [interactive|match|tournament|bench]");
                eprintln!("  interactive - Play against the engine");
                eprintln!("  match       - Watch AI vs AI matches");
                eprintln!("  tournament  - Full tournament between all AIs");
                eprintln!("  bench       - Benchmark search speed");
            }
        }
    } else {
        // Default: match mode
        match_mode();
    }
}

fn interactive_mode() {
    println!("========================================");
    println!("  Edax-RS: Interactive Othello Engine");
    println!("========================================");
    println!("Type 'help' for commands.\n");

    let mut engine = EdaxEngine::new(SearchOptions {
        depth: 10,
        time_limit_ms: 5000,
        hash_size: 1 << 18,
        verbose: true,
    });

    let output = engine.process_command("show");
    println!("{}", output);

    let stdin = io::stdin();
    loop {
        print!("edax> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            break;
        }

        let output = engine.process_command(line.trim());
        if output == "quit" {
            break;
        }
        if !output.is_empty() {
            println!("{}", output);
        }

        // Show board after each move
        if line.trim().starts_with("play") || line.trim().starts_with("go") {
            println!("{}", engine.process_command("show"));
        }
    }
}

fn match_mode() {
    println!("========================================");
    println!("  Edax-RS: AI vs AI Othello Match");
    println!("========================================\n");

    // Match 1: Search(depth=4) vs Mobility (verbose)
    println!("--- Match 1: Search(depth=4) vs Mobility ---\n");
    let mut black = SearchPlayer::new(4);
    let mut white = MobilityPlayer;
    let _result = play_game(&mut black, &mut white, true);
    println!();

    // Match 2: Search(depth=4) vs Greedy
    println!("--- Match 2: Search(depth=4) vs Greedy ---\n");
    let mut black = SearchPlayer::new(4);
    let mut white = GreedyPlayer;
    let _result = play_game(&mut black, &mut white, true);
    println!();

    // Match 3: Mobility vs Greedy (verbose)
    println!("--- Match 3: Mobility(Black) vs Greedy(White) ---\n");
    let mut black = MobilityPlayer;
    let mut white = GreedyPlayer;
    let _result = play_game(&mut black, &mut white, true);
    println!();

    // Match 4: Random vs Random (100 games)
    println!("--- Match 4: Random vs Random (100 games) ---\n");
    run_match("Random", "Random", 100, || Box::new(RandomPlayer::new(rand_seed())), || Box::new(RandomPlayer::new(rand_seed())));

    // Match 5: Search(depth=4) vs Random (50 games)
    println!("--- Match 5: Search(depth=4) vs Random (50 games) ---\n");
    run_match("Search", "Random", 50, || Box::new(SearchPlayer::new(4)), || Box::new(RandomPlayer::new(rand_seed())));

    // Match 6: Mobility vs Random (100 games)
    println!("--- Match 6: Mobility vs Random (100 games) ---\n");
    run_match("Mobility", "Random", 100, || Box::new(MobilityPlayer), || Box::new(RandomPlayer::new(rand_seed())));

    // Match 7: Greedy vs Random (100 games)
    println!("--- Match 7: Greedy vs Random (100 games) ---\n");
    run_match("Greedy", "Random", 100, || Box::new(GreedyPlayer), || Box::new(RandomPlayer::new(rand_seed())));
}

fn tournament_mode() {
    println!("========================================");
    println!("  Edax-RS: AI Tournament");
    println!("========================================\n");

    let ai_names = ["Random", "Greedy", "Mobility", "Search(d=4)"];
    let n = ai_names.len();
    let games_per_pair = 20;

    // wins[i][j] = wins for ai_names[i] as black vs ai_names[j] as white
    let mut wins = vec![vec![0u32; n]; n];
    let mut draws = vec![vec![0u32; n]; n];

    for i in 0..n {
        for j in 0..n {
            if i == j { continue; }
            for g in 0..games_per_pair {
                let mut black = make_ai(i, g as u64);
                let mut white = make_ai(j, g as u64 + 1000);
                let result = play_game(black.as_mut(), white.as_mut(), false);
                match result.winner() {
                    "Black" => wins[i][j] += 1,
                    "Draw" => draws[i][j] += 1,
                    _ => {}
                }
            }
        }
    }

    // Print tournament table
    print!("{:>15}", "");
    for name in &ai_names {
        print!("{:>12}", name);
    }
    println!();

    for i in 0..n {
        print!("{:>15}", ai_names[i]);
        let mut total_score = 0.0f64;
        for j in 0..n {
            if i == j {
                print!("{:>12}", "-");
            } else {
                let total_w = wins[i][j];
                let total_l = games_per_pair - wins[i][j] - draws[i][j];
                let d = draws[i][j];
                total_score += total_w as f64 + d as f64 * 0.5;
                print!("{:>4}-{}-{:<4}", total_w, d, total_l);
            }
        }
        println!("  total: {:.1}", total_score);
    }
}

fn bench_mode() {
    println!("========================================");
    println!("  Edax-RS: Search Benchmark");
    println!("========================================\n");

    use std::time::Instant;

    let board = edax_core::board::Board::new();

    for depth in [4, 6, 8, 10, 12] {
        let mut search = edax_core::search::Search::new(SearchOptions {
            depth,
            time_limit_ms: 60000,
            hash_size: 1 << 18,
            verbose: false,
        });

        let start = Instant::now();
        let result = search.search(&board);
        let elapsed = start.elapsed();

        let nps = if elapsed.as_millis() > 0 {
            result.nodes * 1000 / elapsed.as_millis() as u64
        } else {
            0
        };

        println!(
            "depth {:2}: score {:+3} move {} nodes {:>10} time {:>7.1}ms nps {:>10}",
            depth,
            result.score,
            if result.best_move >= 0 {
                edax_core::board::Board::square_to_string(result.best_move as usize)
            } else {
                "--".to_string()
            },
            result.nodes,
            elapsed.as_secs_f64() * 1000.0,
            nps,
        );
    }
}

fn run_match<F1, F2>(name1: &str, name2: &str, games: u32, make_black: F1, make_white: F2)
where
    F1: Fn() -> Box<dyn Player>,
    F2: Fn() -> Box<dyn Player>,
{
    let mut wins1 = 0;
    let mut wins2 = 0;
    let mut draws = 0;
    for _ in 0..games {
        let mut b = make_black();
        let mut w = make_white();
        let r = play_game(b.as_mut(), w.as_mut(), false);
        match r.winner() {
            "Black" => wins1 += 1,
            "White" => wins2 += 1,
            _ => draws += 1,
        }
    }
    println!(
        "{}(Black) vs {}(White) ({} games):",
        name1, name2, games
    );
    println!("  {} wins: {}", name1, wins1);
    println!("  {} wins: {}", name2, wins2);
    println!("  Draws:       {}", draws);
    println!();
}

fn make_ai(index: usize, seed: u64) -> Box<dyn Player> {
    match index {
        0 => Box::new(RandomPlayer::new(seed * 37 + 7)),
        1 => Box::new(GreedyPlayer),
        2 => Box::new(MobilityPlayer),
        3 => Box::new(SearchPlayer::new(4)),
        _ => Box::new(RandomPlayer::new(seed)),
    }
}

static mut RAND_STATE: u64 = 12345;
fn rand_seed() -> u64 {
    unsafe {
        RAND_STATE = RAND_STATE.wrapping_mul(6364136223846793005).wrapping_add(1);
        RAND_STATE
    }
}
