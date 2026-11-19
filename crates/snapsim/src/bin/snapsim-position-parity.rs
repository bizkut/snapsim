use serde::Deserialize;
use serde_json::Value;
use snapsim::{
    position::{Position, build_game, state_json},
    random::PythonRandom,
    trace::first_divergence,
};
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    process::ExitCode,
};

#[derive(Deserialize)]
struct Case {
    position: Position,
    seed: i128,
    state: Value,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: snapsim-position-parity CASES.jsonl")?;
    let mut count = 0;
    for line in BufReader::new(File::open(path)?).lines() {
        let case: Case = serde_json::from_str(&line?)?;
        let mut rng = PythonRandom::seed(case.seed);
        let actual = state_json(&build_game(&case.position, &mut rng)?);
        if let Some(diff) = first_divergence(&case.state, &actual) {
            return Err(format!(
                "case {count}, seed {}, field {}: expected {:?}, actual {:?}",
                case.seed, diff.path, diff.expected, diff.actual
            )
            .into());
        }
        count += 1;
    }
    println!("Matched {count} position/seed pairs");
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
