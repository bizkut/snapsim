//! Compare newline-delimited Python advisor records against the Rust search.
use serde::Deserialize;
use serde_json::Value;
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    process,
};

#[derive(Deserialize)]
struct Record {
    source: String,
    position: Value,
    options: Value,
    expected: Value,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    snapsim::cards::use_reference_pool();
    let path = env::args().nth(1).ok_or("expected corpus path")?;
    let mut count = 0;
    let mut mismatches = 0;
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        let record: Record = serde_json::from_str(&line)?;
        let actual =
            snapsim::advise::advise_json(&record.position.to_string(), &record.options.to_string());
        count += 1;
        match actual {
            Ok(result) => {
                let value: Value = serde_json::from_str(&result)?;
                if value != record.expected {
                    mismatches += 1;
                    eprintln!(
                        "record {count} {} policy {} mismatch\nexpected {}\nactual {}",
                        record.source, record.options["playout"], record.expected, value
                    );
                }
            }
            Err(error) => {
                mismatches += 1;
                eprintln!(
                    "record {count} {} policy {}: {error}",
                    record.source, record.options["playout"]
                );
            }
        }
    }
    println!("{count} records, {mismatches} mismatches");
    if mismatches != 0 {
        process::exit(1);
    }
    Ok(())
}
