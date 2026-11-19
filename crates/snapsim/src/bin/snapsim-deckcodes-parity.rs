use serde_json::{Value, json};
use snapsim::deckcodes::{encode_deck, parse_deck};
use std::io::{self, BufRead};

fn main() {
    snapsim::cards::use_reference_pool();
    snapsim::cards::use_reference_rules();
    for line in io::stdin().lock().lines() {
        let line = line.expect("reading case");
        let case: Value = serde_json::from_str(&line).expect("invalid case");
        let result = match case["operation"].as_str().expect("missing operation") {
            "parse" => parse_deck(case["text"].as_str().expect("missing text")).map(|v| json!(v)),
            "encode" => {
                let ids: Vec<&str> = case["ids"]
                    .as_array()
                    .expect("missing ids")
                    .iter()
                    .map(|v| v.as_str().expect("non-string ID"))
                    .collect();
                encode_deck(&ids).map(|v| json!(v))
            }
            "file" => {
                let (decks, errors) = snapsim::deckcodes::parse_deck_file(
                    case["text"].as_str().expect("missing text"),
                    case["label"].as_str().expect("missing label"),
                );
                Ok(json!({"decks": decks, "errors": errors}))
            }
            _ => panic!("unknown operation"),
        };
        println!(
            "{}",
            match result {
                Ok(value) => json!({"value": value}),
                Err(error) => json!({"error": error}),
            }
        );
    }
}
