use serde::Deserialize;
use serde_json::{Value, json};
use snapsim::{game::Game, policy::Policy, random::PythonRandom, trace::first_divergence};
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    process::ExitCode,
};

#[derive(Deserialize)]
struct Config {
    cards: Vec<String>,
    locations: Vec<String>,
    policy: String,
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: snapsim-parity CONFIG TRACE.jsonl".into());
    }
    let config: Config = serde_json::from_reader(File::open(&args[1])?)?;
    let policy = match config.policy.as_str() {
        "spread" => Policy::Spread,
        "margin" => Policy::Margin,
        "heuristic" => Policy::Heuristic,
        _ => return Err("unknown policy".into()),
    };
    let mut count = 0;
    for line in BufReader::new(File::open(&args[2])?).lines() {
        let expected: Value = serde_json::from_str(&line?)?;
        let seed = expected["seed"].as_i64().ok_or("missing seed")?;
        let mut deals = PythonRandom::seed(i128::from(seed));
        let decks: [Vec<String>; 2] = std::array::from_fn(|_| {
            deals
                .sample_indices(config.cards.len(), 12)
                .into_iter()
                .map(|index| config.cards[index].clone())
                .collect()
        });
        let mut rng = PythonRandom::seed(i128::from(seed));
        let mut game = Game::new([&decks[0], &decks[1]], &config.locations, &mut rng)?;
        let locations: Vec<_> = game.lanes.iter().map(|lane| lane.def.id).collect();
        let mut turns = Vec::new();
        while game.turn <= game.total_turns {
            turns.push(game.play_turn(policy));
        }
        let actual = json!({"seed": seed, "decks": decks, "locations": locations, "turns": turns, "winner": game.winner()});
        if let Some(diff) = first_divergence(&expected, &actual) {
            return Err(format!(
                "game {count}, seed {seed}, field {}: expected {:?}, actual {:?}",
                diff.path, diff.expected, diff.actual
            )
            .into());
        }
        count += 1;
    }
    println!("Matched {count} {} games", config.policy);
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
