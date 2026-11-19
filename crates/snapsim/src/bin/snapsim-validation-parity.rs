use serde_json::Value;
use snapsim::position::Position;
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    snapsim::cards::use_reference_pool();
    let path = env::args().nth(1).ok_or("expected JSONL corpus path")?;
    let mut accepted = 0;
    let mut rejected = 0;
    let mut out_of_range = 0;
    let mut differences = 0;
    for (index, line) in BufReader::new(File::open(path)?).lines().enumerate() {
        let record: Value = serde_json::from_str(&line?)?;
        let input = &record["position"];
        let expected = record["accepted"].as_bool().ok_or("missing accepted")?;
        let text = serde_json::to_string(input)?;
        let result = Position::from_json(&text);
        let outside = record["out_of_range"]
            .as_bool()
            .ok_or("missing out_of_range")?;
        if outside {
            out_of_range += 1;
        } else if expected {
            accepted += 1;
        } else {
            rejected += 1;
        }
        if result.is_ok() != (expected && !outside) {
            differences += 1;
            if differences <= 20 {
                eprintln!(
                    "case {}: Python accepted={expected}, Rust={:?}; input={text}",
                    index + 1,
                    result.as_ref().err()
                );
            }
        } else if let Ok(position) = result {
            let mut normalized = input.clone();
            for lane in normalized["locations"]
                .as_array_mut()
                .ok_or("missing locations")?
            {
                lane.as_object_mut()
                    .ok_or("invalid lane")?
                    .entry("id")
                    .or_insert(Value::Null);
            }
            let parsed: Position = serde_json::from_value(normalized)?;
            if serde_json::to_value(position)? != serde_json::to_value(parsed)? {
                differences += 1;
                eprintln!("case {}: serde result differs", index + 1);
            }
        }
    }
    println!(
        "In-range accepted {accepted}, rejected {rejected}; out-of-range {out_of_range}; differences {differences}"
    );
    if differences != 0 {
        return Err("validation parity failed".into());
    }
    Ok(())
}
