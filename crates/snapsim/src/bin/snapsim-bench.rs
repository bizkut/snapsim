use serde::Deserialize;
use snapsim::{game::Game, policy::Policy, random::PythonRandom};
use std::{env, fs::File, hint::black_box, process::ExitCode, time::Instant};

#[derive(Deserialize)]
struct Config {
    cards: Vec<String>,
    locations: Vec<String>,
    policy: String,
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 4 {
        return Err("usage: snapsim-bench CONFIG GAMES SEED".into());
    }
    let config: Config = serde_json::from_reader(File::open(&args[1])?)?;
    let games: u64 = args[2].parse()?;
    let seed: u64 = args[3].parse()?;
    if games == 0 {
        return Err("games must be positive".into());
    }
    let policy = match config.policy.as_str() {
        "spread" => Policy::Spread,
        "margin" => Policy::Margin,
        _ => return Err("unknown policy".into()),
    };
    let start = Instant::now();
    for episode in seed..seed.checked_add(games).ok_or("seed range overflows")? {
        let mut deals = PythonRandom::seed(i128::from(episode));
        let decks: [Vec<String>; 2] = std::array::from_fn(|_| {
            deals
                .sample_indices(config.cards.len(), 12)
                .into_iter()
                .map(|i| config.cards[i].clone())
                .collect()
        });
        let mut rng = PythonRandom::seed(i128::from(episode));
        let mut game = Game::new([&decks[0], &decks[1]], &config.locations, &mut rng)?;
        while game.turn <= game.total_turns {
            black_box(game.play_turn(policy));
        }
        black_box(game.winner());
    }
    let seconds = start.elapsed().as_secs_f64();
    println!(
        "{{\"engine\":\"rust\",\"policy\":\"{}\",\"games\":{},\"seed\":{},\"seconds\":{},\"games_per_second\":{}}}",
        config.policy,
        games,
        seed,
        seconds,
        games as f64 / seconds
    );
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
