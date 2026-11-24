//! Compute coverage, priors and strict unsupported results for JSONL positions.
use serde_json::json;
use snapsim::{
    advise::{AdviseOptions, advise},
    coverage::{coverage, sampling_priors},
    position::Position,
};
use std::io::{self, BufRead};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    snapsim::cards::use_reference_pool();
    snapsim::cards::use_reference_rules();
    let options = AdviseOptions {
        rollouts: 1,
        seed: 0,
        position_rollouts: 0,
        stake_rollouts: 0,
        stake_policy_rollouts: 32,
        max_plans: 1,
        time_limit: None,
        approximate: false,
        playout: "spread".into(),
        workers: 1,
        plays_only: true,
        optional_moves: false,
        best_reply: None,
        best_reply_estimate: true,
    };
    for (index, line) in io::stdin().lock().lines().enumerate() {
        let line = line?;
        if snapsim::trace::mentions_beyond_reference(&serde_json::from_str(&line)?) {
            println!("{}", json!({"beyond_reference": true}));
            continue;
        }
        let position = Position::from_json(&line)?;
        let coverage = coverage(&position);
        let incomplete = coverage
            .as_object()
            .unwrap()
            .values()
            .any(|v| !v.is_null() && v.as_array().is_none_or(|a| !a.is_empty()));
        let strict = if incomplete {
            Some(advise(&position, &options, &|| false)?)
        } else {
            None
        };
        let approximate = if index < 9 {
            Some(advise(
                &position,
                &AdviseOptions {
                    approximate: true,
                    ..options.clone()
                },
                &|| false,
            )?)
        } else {
            None
        };
        println!(
            "{}",
            json!({"coverage":coverage,"priors":sampling_priors(&position),"strict":strict,"approximate":approximate})
        );
    }
    Ok(())
}
