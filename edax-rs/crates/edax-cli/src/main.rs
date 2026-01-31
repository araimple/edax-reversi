use edax_core::ai::{GreedyPlayer, MobilityPlayer, RandomPlayer};
use edax_core::game::play_game;

fn main() {
    println!("========================================");
    println!("  Edax-RS: AI vs AI Othello Match");
    println!("========================================\n");

    // Match 1: Mobility vs Greedy (verbose, show the board)
    println!("--- Match 1: Mobility(Black) vs Greedy(White) ---\n");
    let mut black = MobilityPlayer;
    let mut white = GreedyPlayer;
    let result = play_game(&mut black, &mut white, true);
    println!();

    // Match 2: Greedy vs Mobility (roles swapped)
    println!("--- Match 2: Greedy(Black) vs Mobility(White) ---\n");
    let mut black = GreedyPlayer;
    let mut white = MobilityPlayer;
    let result2 = play_game(&mut black, &mut white, true);
    println!();

    // Match 3: Random vs Random (100 games, statistics)
    println!("--- Match 3: Random vs Random (100 games) ---\n");
    let mut black_wins = 0;
    let mut white_wins = 0;
    let mut draws = 0;
    for i in 0..100 {
        let mut b = RandomPlayer::new(i * 31 + 7);
        let mut w = RandomPlayer::new(i * 17 + 3);
        let r = play_game(&mut b, &mut w, false);
        match r.winner() {
            "Black" => black_wins += 1,
            "White" => white_wins += 1,
            _ => draws += 1,
        }
    }
    println!("Random vs Random (100 games):");
    println!("  Black wins: {}", black_wins);
    println!("  White wins: {}", white_wins);
    println!("  Draws:      {}", draws);
    println!();

    // Match 4: Mobility vs Random (100 games)
    println!("--- Match 4: Mobility vs Random (100 games) ---\n");
    let mut mob_wins = 0;
    let mut rand_wins = 0;
    let mut draws = 0;
    for i in 0..100 {
        let mut b = MobilityPlayer;
        let mut w = RandomPlayer::new(i * 13 + 1);
        let r = play_game(&mut b, &mut w, false);
        match r.winner() {
            "Black" => mob_wins += 1,
            "White" => rand_wins += 1,
            _ => draws += 1,
        }
    }
    println!("Mobility(Black) vs Random(White) (100 games):");
    println!("  Mobility wins: {}", mob_wins);
    println!("  Random wins:   {}", rand_wins);
    println!("  Draws:         {}", draws);
    println!();

    // Match 5: Greedy vs Random (100 games)
    println!("--- Match 5: Greedy vs Random (100 games) ---\n");
    let mut greedy_wins = 0;
    let mut rand_wins = 0;
    let mut draws = 0;
    for i in 0..100 {
        let mut b = GreedyPlayer;
        let mut w = RandomPlayer::new(i * 37 + 5);
        let r = play_game(&mut b, &mut w, false);
        match r.winner() {
            "Black" => greedy_wins += 1,
            "White" => rand_wins += 1,
            _ => draws += 1,
        }
    }
    println!("Greedy(Black) vs Random(White) (100 games):");
    println!("  Greedy wins: {}", greedy_wins);
    println!("  Random wins: {}", rand_wins);
    println!("  Draws:       {}", draws);
}
