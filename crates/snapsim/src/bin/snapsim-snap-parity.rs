use serde_json::{Value, json};
use snapsim::snap::{DeckWatch, LocationWatch, SnapError, position_from_snap};
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    process::ExitCode,
};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: snapsim-snap-parity CASES.jsonl")?;
    let mut location_watch = LocationWatch::default();
    let mut deck_watch = DeckWatch::default();
    let mut previous_file = String::new();
    let mut count = 0;
    let mut planning = 0;
    let mut errors = 0;
    for line in BufReader::new(File::open(path)?).lines() {
        let case: Value = serde_json::from_str(&line?)?;
        let file = case["file"].as_str().ok_or("missing file")?;
        if file != previous_file {
            location_watch = LocationWatch::default();
            deck_watch = DeckWatch::default();
            previous_file = file.to_owned();
        }
        let observation = &case["observation"];
        let turns = location_watch.update(observation);
        let added = deck_watch.update(observation, case["deck"].as_array().map(Vec::as_slice));
        let options = json!({"deck":case["deck"], "deck_added":added, "revealed_turns":turns, "plays_only":true});
        let actual = match position_from_snap(observation, &options) {
            Ok(value) => {
                planning += 1;
                json!({"position": value})
            }
            Err(error) => {
                errors += 1;
                json!({"error": match error { SnapError::NotPlanning(_) => "NotPlanningError", SnapError::Invalid(_) => "ValueError", SnapError::InvalidType(_) => "TypeError" }})
            }
        };
        for (field, value) in [
            ("turns", &options["revealed_turns"]),
            ("added", &options["deck_added"]),
            ("result", &actual),
        ] {
            if let Some(diff) = snapsim::trace::first_divergence(&case[field], value) {
                return Err(format!(
                    "{}:{} {field} {}: expected {:?}, got {:?}",
                    file, case["line"], diff.path, diff.expected, diff.actual
                )
                .into());
            }
        }
        count += 1;
    }
    println!("Matched {count} frames ({planning} positions, {errors} errors) and both watches");
    Ok(())
}
fn main() -> ExitCode {
    snapsim::cards::use_reference_pool();
    snapsim::cards::use_reference_rules();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
