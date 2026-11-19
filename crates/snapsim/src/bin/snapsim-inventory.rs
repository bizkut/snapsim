use serde_json::json;
use snapsim::{cards, locations};

fn main() {
    let mut card_ids: Vec<_> = cards::REGISTRY.iter().map(|entry| entry.id).collect();
    let mut location_ids: Vec<_> = locations::REGISTRY.iter().map(|entry| entry.id).collect();
    card_ids.sort_unstable();
    location_ids.sort_unstable();
    println!("{}", json!({"cards": card_ids, "locations": location_ids}));
}
