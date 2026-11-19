use serde_json::{Value, json};
use std::io::{self, BufRead};

fn main() {
    snapsim::cards::use_reference_pool();
    snapsim::cards::use_reference_rules();
    for line in io::stdin().lock().lines() {
        let line = line.expect("reading case");
        let mut case: Value = serde_json::from_str(&line).expect("invalid case");
        if matches!(case["command"].as_str(), Some("compare" | "benchmark")) {
            case["args"]["plays_only"] = Value::Bool(true);
        }
        let result = match case["command"].as_str().expect("missing command") {
            "compare" => snapsim::compare::compare(&case["args"]),
            "benchmark" => snapsim::compare::benchmark(&case["args"]),
            "deal" => {
                let args = &case["args"];
                let mut rng = snapsim::random::PythonRandom::seed(i128::from(
                    args["seed"].as_i64().expect("missing seed"),
                ));
                let weighted = args["weighted"].as_bool().expect("missing weighted");
                Ok(json!([
                    snapsim::compare::recruit_deck(&mut rng, weighted),
                    snapsim::compare::recruit_deck(&mut rng, weighted)
                ]))
            }
            _ => panic!("unknown command"),
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
