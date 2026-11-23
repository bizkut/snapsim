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
    #[serde(default)]
    pub factions: Vec<String>,
    #[serde(default)]
    pub faction_cards: Vec<String>,
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
    let mut cards: Vec<CardData> = serde_json::from_str(include_str!("../../../data/cards.json"))
        .expect("invalid bundled card data");
    let supplement: Vec<CardData> =
        serde_json::from_str(include_str!("../../../data/client_cards.json"))
            .expect("invalid bundled client card data");
    let existing: std::collections::HashMap<_, _> = cards
        .iter()
        .enumerate()
        .map(|(index, card)| (card.carddefid.clone(), index))
        .collect();
    for mut card in supplement {
        if let Some(&index) = existing.get(&card.carddefid) {
            cards[index].factions = std::mem::take(&mut card.factions);
            cards[index].faction_cards = std::mem::take(&mut card.faction_cards);
        } else {
            cards.push(card);
        }
    }
    cards
}

pub fn locations() -> Vec<LocationData> {
    serde_json::from_str(include_str!("../../../data/locations.json"))
        .expect("invalid bundled location data")
}
