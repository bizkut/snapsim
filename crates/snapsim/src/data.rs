//! The copied reference catalog is data, not the set of implemented rules.
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct CardData {
    pub carddefid: String,
    pub name: String,
    pub cost: i32,
    pub power: i32,
    pub ability: String,
    pub status: String,
    pub source: String,
    #[serde(rename = "type")]
    pub card_type: String,
    pub random_pool_excluded: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LocationData {
    pub carddefid: String,
    pub name: String,
    pub ability: String,
    pub status: String,
    pub rarity: String,
    pub difficulty: String,
}

pub fn cards() -> Vec<CardData> {
    serde_json::from_str(include_str!("../../../data/cards.json"))
        .expect("invalid bundled card data")
}

pub fn locations() -> Vec<LocationData> {
    serde_json::from_str(include_str!("../../../data/locations.json"))
        .expect("invalid bundled location data")
}
