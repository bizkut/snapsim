use serde::Deserialize;
use serde_json::{Value, json};
use snapsim::{game::Game, policy::Policy, random::PythonRandom, trace::first_divergence};
use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    process::ExitCode,
};

#[derive(Deserialize)]
struct Config {
    cards: Vec<String>,
    locations: Vec<String>,
    policy: String,
}

/// A game that is known to leave the frozen reference because an existing rule was
/// deliberately changed to match the client. Its first divergence must be exactly `path`,
/// with exactly these reference (`expected`) and Rust (`actual`) values.
#[derive(Deserialize)]
struct KnownDivergence {
    policy: String,
    seed: i64,
    path: String,
    expected: Value,
    actual: Value,
    #[allow(dead_code)]
    reason: String,
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
    let known_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/divergences.json");
    let known: Vec<KnownDivergence> = serde_json::from_reader(File::open(&known_path)?)?;
    let known: Vec<_> = known
        .into_iter()
        .filter(|entry| entry.policy == config.policy)
        .collect();
    let mut seen = HashSet::new();
    let (mut matched, mut unexpected) = (0, 0);
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
        match first_divergence(&expected, &actual) {
            None => matched += 1,
            Some(diff)
                if known.iter().any(|entry| {
                    entry.seed == seed
                        && entry.path == diff.path
                        && diff.expected.as_ref() == Some(&entry.expected)
                        && diff.actual.as_ref() == Some(&entry.actual)
                }) =>
            {
                seen.insert((seed, diff.path));
            }
            Some(diff) => {
                unexpected += 1;
                eprintln!(
                    "seed {seed}, field {}: expected {:?}, actual {:?}",
                    diff.path, diff.expected, diff.actual
                );
            }
        }
    }
    let stale: Vec<_> = known
        .iter()
        .filter(|entry| !seen.contains(&(entry.seed, entry.path.clone())))
        .map(|entry| format!("{} {}", entry.seed, entry.path))
        .collect();
    if unexpected > 0 || !stale.is_empty() {
        return Err(format!(
            "{unexpected} unexpected divergences; known divergences that no longer occur: {stale:?}"
        )
        .into());
    }
    println!(
        "Matched {matched} {} games; {} known divergences (parity/divergences.json)",
        config.policy,
        seen.len()
    );
    Ok(())
}
fn main() -> ExitCode {
    snapsim::cards::use_reference_pool();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
