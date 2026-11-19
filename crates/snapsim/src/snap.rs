//! SNAP schema-v1 observations and stateful visibility watches.
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum SnapError {
    Invalid(String),
    InvalidType(String),
    NotPlanning(String),
}

fn array(v: &Value) -> &[Value] {
    v.as_array().map_or(&[], Vec::as_slice)
}
fn key(v: &Value) -> String {
    v.to_string()
}
fn cards(zones: &Value) -> Vec<&Value> {
    ["graveyard", "banished"]
        .iter()
        .flat_map(|name| array(&zones[*name]["cards"]))
        .filter(|c| c["card_id"].as_str().is_some_and(|s| !s.is_empty()))
        .collect()
}
fn listed(zone: &Value) -> Option<Vec<Value>> {
    let ids = zone["card_ids"].as_array()?;
    if ids.iter().any(|id| id.is_null() || id == "None") {
        None
    } else {
        Some(ids.clone())
    }
}
fn remaining(deck: &[Value], added: Option<&[Value]>, visible: &[&Value]) -> Vec<Value> {
    if added.is_none() {
        let ids: HashSet<String> = visible.iter().map(|c| key(&c["card_id"])).collect();
        return deck
            .iter()
            .filter(|id| !ids.contains(&key(id)))
            .cloned()
            .collect();
    }
    let mut counts = HashMap::<String, usize>::new();
    let mut seen = HashSet::new();
    for card in visible {
        let entity = &card["entity_id"];
        if !entity.is_null() && !seen.insert(key(entity)) {
            continue;
        }
        *counts.entry(key(&card["card_id"])).or_default() += 1;
    }
    deck.iter()
        .chain(added.unwrap_or_default())
        .filter_map(|id| {
            let count = counts.entry(key(id)).or_default();
            if *count > 0 {
                *count -= 1;
                None
            } else {
                Some(id.clone())
            }
        })
        .collect()
}
pub(crate) fn revealed_id(location: &Value) -> Value {
    let id = &location["location_id"];
    if id.as_str().is_some_and(|s| {
        s.strip_prefix("RevealOn")
            .is_some_and(|tail| !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()))
    }) {
        Value::Null
    } else {
        id.clone()
    }
}
fn card_ref(card: &Value, board: bool) -> Value {
    let mut result = json!({"id":card["card_id"], "cost":card["cost"], "power":card["power"], "revealed":card["revealed"]});
    if board {
        let turn = if card["revealed"] == true {
            &card["turn_revealed"]
        } else {
            &card["staged_turn"]
        };
        if !turn.is_null() {
            result["played_turn"] = turn.clone();
        }
    }
    result
}
#[derive(Default)]
pub struct LocationWatch {
    turn: Option<i64>,
    lanes: HashMap<usize, (Value, Value)>,
}
impl LocationWatch {
    pub fn update(&mut self, observation: &Value) -> Vec<Value> {
        let game = &observation["game"];
        let turn = game["turn"].as_i64().unwrap_or(0);
        if self.turn.is_none_or(|t| turn < t) {
            self.lanes.clear();
        }
        self.turn = Some(turn);
        array(&game["locations"])
            .iter()
            .enumerate()
            .map(|(lane, location)| {
                let id = revealed_id(location);
                if id.is_null() {
                    self.lanes.insert(lane, (Value::Null, Value::Null));
                } else if let Some((old, revealed)) = self.lanes.get_mut(&lane) {
                    if *old != id {
                        *old = id;
                        *revealed = json!(turn);
                    }
                } else {
                    self.lanes.insert(
                        lane,
                        (id, if turn <= 1 { json!(turn) } else { Value::Null }),
                    );
                }
                self.lanes[&lane].1.clone()
            })
            .collect()
    }
}
#[derive(Default)]
pub struct DeckWatch {
    game_key: Value,
    turn: Option<i64>,
    previous: Option<(i64, Vec<Value>, Option<Vec<Value>>)>,
    seen: HashSet<String>,
    added: Vec<Value>,
}
impl DeckWatch {
    pub fn update(&mut self, observation: &Value, deck: Option<&[Value]>) -> Vec<Value> {
        let game = &observation["game"];
        let turn = game["turn"].as_i64().unwrap_or(0);
        if self.game_key != game["game_key"] || self.turn.is_some_and(|t| turn < t) {
            self.previous = None;
            self.seen.clear();
            self.added.clear();
        }
        self.game_key = game["game_key"].clone();
        self.turn = Some(turn);
        let Some(local) = array(&game["players"])
            .iter()
            .find(|p| p["slot"] == "local")
        else {
            self.previous = None;
            return self.added.clone();
        };
        let zones = &local["zones"];
        let hand_cards = array(&zones["hand"]["cards"]);
        let mut hand: Vec<(String, Value)> = Vec::new();
        for card in hand_cards {
            if card["entity_id"].is_null() {
                continue;
            }
            let entity = key(&card["entity_id"]);
            if let Some((_, id)) = hand.iter_mut().find(|(e, _)| *e == entity) {
                *id = card["card_id"].clone();
            } else {
                hand.push((entity, card["card_id"].clone()));
            }
        }
        if hand.len() as i64 != zones["hand"]["count"].as_i64().unwrap_or(-1) {
            self.previous = None;
            return self.added.clone();
        }
        let mut visible: Vec<Value> = hand_cards.to_vec();
        for lane in array(&game["locations"]) {
            for card in array(&lane["cards"]) {
                if card["owner_entity_id"] == local["entity_id"] {
                    visible.push(card.clone());
                }
            }
        }
        visible.extend(cards(zones).into_iter().cloned());
        let seen = self.seen.clone();
        self.seen
            .extend(visible.iter().map(|card| key(&card["entity_id"])));
        let count = zones["deck"]["count"].as_i64().unwrap_or(0);
        let previous = self
            .previous
            .replace((count, visible, listed(&zones["deck"])));
        let Some((old_count, old_visible, mut old_remaining)) = previous else {
            return self.added.clone();
        };
        let drop = old_count - count;
        let mut new: Vec<Value> = hand
            .into_iter()
            .filter(|(entity, _)| !seen.contains(entity))
            .map(|(_, id)| id)
            .collect();
        if new.is_empty() || drop < 0 || drop == new.len() as i64 {
            return self.added.clone();
        }
        if drop > 0 {
            if old_remaining.is_none() {
                old_remaining = deck.map(|d| {
                    remaining(
                        d,
                        Some(&self.added),
                        &old_visible.iter().collect::<Vec<_>>(),
                    )
                });
            }
            let Some(old) = old_remaining else {
                return self.added.clone();
            };
            if old.len() as i64 != old_count {
                return self.added.clone();
            }
            let mut excess = Vec::<(Value, i64)>::new();
            for id in &new {
                if let Some((_, n)) = excess.iter_mut().find(|(v, _)| v == id) {
                    *n += 1;
                } else {
                    excess.push((id.clone(), 1));
                }
            }
            for id in old {
                if let Some((_, n)) = excess.iter_mut().find(|(v, _)| *v == id) {
                    *n -= 1;
                }
            }
            let forced: Vec<Value> = excess
                .into_iter()
                .flat_map(|(id, n)| std::iter::repeat_n(id, n.max(0) as usize))
                .collect();
            if forced.len() as i64 > new.len() as i64 - drop {
                return self.added.clone();
            }
            new = forced;
        }
        self.added
            .extend(new.into_iter().filter(|id| !id.is_null() && id != "None"));
        self.added.clone()
    }
}
pub fn position_from_snap(observation: &Value, options: &Value) -> Result<Value, SnapError> {
    let invalid = |s: &str| SnapError::Invalid(s.to_owned());
    if !observation.is_object() {
        return Err(invalid("SNAP observation must be a JSON object"));
    }
    if observation["status"] == "no-active-game" {
        return Err(invalid("SNAP observation has no active game"));
    }
    if observation["schema_version"].as_i64() != Some(1) {
        return Err(invalid("Unsupported SNAP schema_version (expected 1)"));
    }
    let game = &observation["game"];
    let players = array(&game["players"]);
    if players.is_empty() {
        return Err(invalid("SNAP observation has no active game (no players)"));
    }
    let Some(local) = players.iter().rev().find(|p| p["slot"] == "local") else {
        return Err(invalid(
            "SNAP observation needs both local and opponent players",
        ));
    };
    let Some(opponent) = players.iter().rev().find(|p| p["slot"] == "opponent") else {
        return Err(invalid(
            "SNAP observation needs both local and opponent players",
        ));
    };
    let source = &observation["source"];
    if source["terminal"] == true {
        return Err(SnapError::NotPlanning("the game is over".into()));
    }
    let turn = game["turn"]
        .as_i64()
        .ok_or_else(|| invalid("missing turn"))?;
    if local.get("ended_turn").is_some_and(Value::is_null) {
        return Err(SnapError::InvalidType("ended_turn is null".into()));
    }
    if local["ended_turn"].as_i64().unwrap_or(0) >= turn {
        return Err(SnapError::NotPlanning(format!(
            "turn {turn} is already ended and is being revealed"
        )));
    }
    let zones = &local["zones"];
    let opponent_zones = &opponent["zones"];
    if game["legal_actions"].as_array().is_some_and(Vec::is_empty)
        && zones["hand"]["count"].as_i64().unwrap_or(0) != 0
    {
        return Err(SnapError::NotPlanning(format!(
            "turn {turn} has no action table yet"
        )));
    }
    if game.get("locations").and_then(Value::as_array).is_none()
        || game.get("total_turns").is_none()
        || local.get("energy").is_none()
        || !zones["hand"].is_object()
        || !zones["deck"].is_object()
        || !opponent_zones["hand"].is_object()
        || !opponent_zones["deck"].is_object()
    {
        return Err(invalid("SNAP observation is missing required game fields"));
    }
    let mut remaining_cards = listed(&zones["deck"]);
    let hand_cards = array(&zones["hand"]["cards"]);
    let mut hand = hand_cards.iter().collect::<Vec<_>>();
    hand.sort_by_key(|card| card["zone_position"].as_i64().unwrap_or(0));
    let hand: Vec<Value> = hand.into_iter().map(|c| card_ref(c, false)).collect();
    let deck = options["deck"].as_array();
    let mut visible: Vec<&Value> = hand_cards.iter().collect();
    let mut seen = Vec::<Value>::new();
    let mut locations = Vec::new();
    for (lane, location) in array(&game["locations"]).iter().enumerate() {
        let id = revealed_id(location);
        if !id.is_null()
            && options["revealed_turns"]
                .as_array()
                .is_some_and(|turns| lane >= turns.len())
        {
            return Err(invalid("revealed_turns has too few entries"));
        }
        let mut mine = Vec::new();
        let mut theirs = Vec::new();
        let mut face_down = 0;
        let mut board_cards: Vec<_> = array(&location["cards"]).iter().collect();
        board_cards.sort_by_key(|card| card["zone_position"].as_i64().unwrap_or(i64::MAX));
        for card in board_cards {
            if card["owner_entity_id"] == local["entity_id"] {
                mine.push(card_ref(card, true));
                visible.push(card);
            } else if card["owner_entity_id"] == opponent["entity_id"] {
                if card["card_id"].as_str().is_some_and(|s| !s.is_empty())
                    && card["revealed"] == true
                {
                    theirs.push(card_ref(card, true));
                    seen.push(card["card_id"].clone());
                } else {
                    face_down += 1;
                }
            }
        }
        let revealed = if id.is_null() {
            Value::Null
        } else {
            options["revealed_turns"]
                .as_array()
                .and_then(|v| v.get(lane))
                .cloned()
                .unwrap_or(Value::Null)
        };
        let mut lane_ref = json!({"id":id,"mine":mine,"theirs":theirs,"theirs_face_down":face_down,"revealed_turn":revealed});
        if lane_ref["id"] == "OttosLab"
            && lane_ref["revealed_turn"] == turn
            && !mine
                .iter()
                .chain(&theirs)
                .any(|c| c["played_turn"] == turn && c["revealed"] == true)
        {
            lane_ref["ottos_lab_spent"] = json!([false, false]);
        }
        locations.push(lane_ref);
    }
    if remaining_cards.is_none()
        && let Some(deck) = deck
    {
        visible.extend(cards(zones));
        remaining_cards = Some(remaining(
            deck,
            options["deck_added"].as_array().map(Vec::as_slice),
            &visible,
        ));
    }
    for card in cards(opponent_zones) {
        seen.push(card["card_id"].clone());
    }
    let mut unique = HashSet::new();
    seen.retain(|v| unique.insert(key(v)));
    let mode = if options["mode"].is_null() && source["league"] == "RankedRecruit" {
        json!("recruit")
    } else {
        options["mode"].clone()
    };
    Ok(
        json!({"turn":game["turn"],"total_turns":game["total_turns"],"energy":local["energy"],"hand":hand,"deck":remaining_cards.unwrap_or_default(),"deck_count":zones["deck"]["count"],"locations":locations,"opponent":{"hand":opponent_zones["hand"]["count"],"deck":opponent_zones["deck"]["count"],"energy":null,"seen":seen,"deck_list":null,"pool":null},"priority":null,"mode":mode}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_track_replacements_and_match_reset() {
        let mut watch = LocationWatch::default();
        assert_eq!(watch.update(&json!({"game":{"turn":1,"locations":[{"location_id":"RevealOn1"},{"location_id":"Asgard"}]}})), json!([null, 1]).as_array().unwrap().clone());
        assert_eq!(watch.update(&json!({"game":{"turn":2,"locations":[{"location_id":"OttosLab"},{"location_id":"Asgard"}]}})), json!([2, 1]).as_array().unwrap().clone());
        assert_eq!(
            watch.update(&json!({"game":{"turn":1,"locations":[{"location_id":"Asgard"}]}})),
            json!([1]).as_array().unwrap().clone()
        );
    }

    #[test]
    fn remaining_counts_entities_and_generated_copies() {
        let first = json!({"entity_id": 1, "card_id":"AntMan"});
        let second = json!({"entity_id": 2, "card_id":"AntMan"});
        assert_eq!(
            remaining(
                &[json!("AntMan"), json!("Hulk")],
                Some(&[json!("AntMan")]),
                &[&first, &first, &second]
            ),
            vec![json!("Hulk")]
        );
        assert_eq!(
            remaining(
                &[json!("AntMan"), json!("AntMan"), json!("Hulk")],
                None,
                &[&first]
            ),
            vec![json!("Hulk")]
        );
    }

    #[test]
    fn deck_watch_tracks_generation_and_ignores_draws() {
        let mut watch = DeckWatch::default();
        let frame = |turn, deck_count, hand: Value| {
            json!({
                "game": {"turn": turn, "game_key": "one", "locations": [],
                    "players": [{"slot":"local", "entity_id":1,
                        "zones":{"hand":{"count":hand.as_array().unwrap().len(),"cards":hand},
                                 "deck":{"count":deck_count}}}]}
            })
        };
        assert_eq!(
            watch.update(&frame(1, 3, json!([])), None),
            Vec::<Value>::new()
        );
        assert_eq!(
            watch.update(
                &frame(1, 2, json!([{"entity_id":1,"card_id":"Hulk"}])),
                None
            ),
            Vec::<Value>::new()
        );
        assert_eq!(
            watch.update(
                &frame(
                    1,
                    2,
                    json!([{"entity_id":1,"card_id":"Hulk"},{"entity_id":2,"card_id":"Hulk"}])
                ),
                None
            ),
            vec![json!("Hulk")]
        );
        assert_eq!(
            watch.update(&frame(0, 3, json!([])), None),
            Vec::<Value>::new()
        );
    }
}
