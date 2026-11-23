//! SNAP schema-v1/v2 observations and stateful visibility watches.
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
fn has_extended_card_fields(card: &Value) -> bool {
    [
        "latest_resolution_order",
        "turns_drawn",
        "activated",
        "moves_per_turn",
        "created",
        "created_by_card_id",
        "created_by_location_id",
        "created_by_player_entity_id",
        "merged_to_entity_id",
        "stat_updates",
    ]
    .iter()
    .any(|name| card.get(*name).is_some())
}

pub(crate) fn card_ref(card: &Value, board: bool, seats: [&Value; 2], v2: bool) -> Value {
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
    if v2
        && card["turn_revealed"]
            .as_u64()
            .is_some_and(|turn| (1..=u8::MAX as u64).contains(&turn))
    {
        result["turn_revealed"] = card["turn_revealed"].clone();
    }
    if has_extended_card_fields(card) {
        for name in [
            "entity_id",
            "latest_resolution_order",
            "turns_drawn",
            "activated",
            "moves_per_turn",
            "created",
            "created_by_card_id",
            "created_by_location_id",
            "merged_to_entity_id",
        ] {
            if !card[name].is_null() {
                result[name] = card[name].clone();
            }
        }
        let seat = |entity: &Value| {
            (!entity.is_null())
                .then(|| seats.iter().position(|id| *id == entity))
                .flatten()
        };
        if let Some(owner) = seat(&card["created_by_player_entity_id"]) {
            result["created_by_player"] = json!(owner);
        }
        if let Some(updates) = card["stat_updates"].as_array() {
            result["stat_updates"] = Value::Array(updates.iter().map(|update| {
                let mut entry = json!({"stat":update["stat"],"update":update["update"],"value":update["value"]});
                if let Some(source) = seat(&update["player_source_entity_id"]) {
                    entry["source"] = json!(source);
                }
                for name in ["updater_card_id", "updater_location_id", "updater_faction_id", "updater_augment_id", "updater_league_id"] {
                    if !update[name].is_null() { entry[name] = update[name].clone(); }
                }
                entry
            }).collect());
        }
    }
    if v2 {
        if card["entity_id"].as_i64().is_some_and(|id| id >= 0) {
            result["entity_id"] = card["entity_id"].clone();
        }
        use crate::position::{AbilityState, Augments, PowerModifiers};
        if let Ok(modifiers) =
            serde_json::from_value::<PowerModifiers>(card["power_modifiers"].clone())
            && modifiers.items.len() <= 512
            && modifiers
                .items
                .iter()
                .all(|item| matches!(item.operation.as_str(), "Add" | "Multiply" | "Set"))
        {
            result["power_modifiers"] = json!(modifiers);
        }
        if let Some(base) = card["effective_base_power"]
            .as_i64()
            .and_then(|n| i32::try_from(n).ok())
        {
            result["effective_base_power"] = json!(base);
        }
        if let Ok(ability) = serde_json::from_value::<AbilityState>(card["ability_state"].clone())
            && ability.abilities.len() <= 512
            && ability.abilities.iter().all(|entry| {
                (((card["card_id"] == "TheThingFirstSteps"
                    && entry.ability_def_id == "TheThingFirstStepsEndOfTurnTrigger")
                    || (card["card_id"] == "Nebula"
                        && entry.ability_def_id == "NebulaPreEndOfTurnTrigger"))
                    && entry.kind.as_deref() == Some("trigger")
                    && entry.lifetime.is_none()
                    && entry.has_triggered_once.is_none())
                    || (((card["card_id"] == "NegasonicTeenageWarhead"
                        && entry.ability_def_id
                            == "NegasonicTeenageWarheadPostCardResolvedTrigger")
                        || (card["card_id"] == "BlackKnight"
                            && entry.ability_def_id == "BlackKnightPostCardDiscardedTrigger"))
                        && entry.kind.as_deref() == Some("trigger")
                        && entry.lifetime.as_deref() == Some("OneTime")
                        && entry.state.is_none())
            })
            && ability.objective.as_ref().is_none_or(|objective| {
                (card["card_id"] == "JuggernautHorseman" || card["card_id"] == "ArchangelHorseman")
                    && matches!(
                        objective.state.as_str(),
                        "None"
                            | "InProgress"
                            | "GoalReached"
                            | "ProcessingCompletion"
                            | "Completed"
                    )
                    && objective.progress <= i32::MAX as u32
                    && objective.goal <= i32::MAX as u32
            })
        {
            result["ability_state"] = json!(ability);
        }
        if let Ok(augments) = serde_json::from_value::<Augments>(card["augments"].clone())
            && augments.items.len() <= 512
            && augments.items.iter().all(|item| {
                !item.id.is_empty()
                    && item.id.len() <= 256
                    && item.kind.is_none()
                    && item.ability_indexes.is_empty()
            })
            && augments
                .items
                .iter()
                .map(|item| item.index)
                .collect::<HashSet<_>>()
                .len()
                == augments.items.len()
        {
            result["augments"] = json!(augments);
        }
    }
    result
}

pub(crate) fn valid_history(
    history: &crate::position::ObservedHistory,
    game: &Value,
    players: [&Value; 2],
    turn: u8,
) -> bool {
    if players[0]["entity_id"].as_i64().is_none_or(|id| id < 0)
        || players[1]["entity_id"].as_i64().is_none_or(|id| id < 0)
        || players[0]["entity_id"] == players[1]["entity_id"]
    {
        return false;
    }
    let locations = array(&game["locations"]);
    let mut visible = HashMap::new();
    let mut duplicate = false;
    let mut add = |card: &Value, owner: usize| {
        if let (Some(entity), Some(id)) = (card["entity_id"].as_i64(), card["card_id"].as_str())
            && entity >= 0
            && !id.is_empty()
            && id.len() <= 256
        {
            duplicate |= visible.insert(entity, (owner, id.to_owned())).is_some();
        }
    };
    for (seat, player) in players.into_iter().enumerate() {
        for zone in if seat == 0 {
            &["hand", "graveyard", "banished"][..]
        } else {
            &["graveyard", "banished"][..]
        } {
            for card in array(&player["zones"][*zone]["cards"]) {
                add(card, seat);
            }
        }
    }
    for location in locations {
        for card in array(&location["cards"]) {
            let owner = if card["owner_entity_id"] == players[0]["entity_id"] {
                Some(0)
            } else if card["owner_entity_id"] == players[1]["entity_id"] && card["revealed"] == true
            {
                Some(1)
            } else {
                None
            };
            if let Some(owner) = owner {
                add(card, owner);
            }
        }
    }
    if duplicate {
        return false;
    }
    if let Some(actions) = &history.actions {
        if actions.len() > 512 {
            return false;
        }
        for action in actions {
            let owner = match action.owner.as_str() {
                "local" => 0,
                "opponent" => 1,
                _ => return false,
            };
            if action.turn > turn
                || action.card_entity_id < 0
                || !matches!(action.kind.as_str(), "play" | "move" | "activate")
                || (owner == 1 && !action.resolved)
                || (!action.resolved
                    && (action.turn_resolved.is_some() || action.resolution_order.is_some()))
                || action.turn_resolved.is_some_and(|t| t > turn)
                || action.resolution_order.is_some_and(|n| n > i32::MAX as u32)
                || locations
                    .get(action.location_slot)
                    .is_none_or(|l| revealed_id(l).is_null())
                || visible
                    .get(&action.card_entity_id)
                    .is_none_or(|(seat, id)| *seat != owner || id != &action.card_id)
            {
                return false;
            }
        }
    }
    if let Some(completeness) = &history.action_completeness {
        for turns in [&completeness.local, &completeness.opponent] {
            if turns.len() > 100
                || turns.keys().any(|key| {
                    key.parse::<u8>()
                        .ok()
                        .is_none_or(|n| n > turn || n.to_string() != *key)
                })
            {
                return false;
            }
        }
    }
    if let Some(draws) = &history.own_draws {
        if draws.complete && draws.card_ids.is_none() {
            return false;
        }
        if draws.card_ids.as_ref().is_some_and(|ids| {
            ids.len() > 512 || ids.iter().any(|id| id.is_empty() || id.len() > 256)
        }) {
            return false;
        }
        if draws
            .last_drawn_entity_id
            .is_some_and(|id| !visible.get(&id).is_some_and(|entry| entry.0 == 0))
        {
            return false;
        }
    }
    true
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
        self.update_lanes(turn, array(&game["locations"]).iter().map(Some))
    }

    pub(crate) fn update_lanes<'a>(
        &mut self,
        turn: i64,
        locations: impl IntoIterator<Item = Option<&'a Value>>,
    ) -> Vec<Value> {
        if self.turn.is_none_or(|t| turn < t) {
            self.lanes.clear();
        }
        self.turn = Some(turn);
        locations
            .into_iter()
            .enumerate()
            .map(|(lane, location)| {
                let id = location.map_or(Value::Null, revealed_id);
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
    let version = observation["schema_version"].as_i64();
    if !matches!(version, Some(1 | 2)) {
        return Err(invalid("Unsupported SNAP schema_version (expected 1 or 2)"));
    }
    let mut prior_ids = HashSet::new();
    if let Some(prior) = options.get("opponent_seen_prior") {
        let prior = prior
            .as_array()
            .ok_or_else(|| invalid("opponent_seen_prior must be an array"))?;
        for value in prior {
            let id = value.as_str().filter(|id| !id.is_empty()).ok_or_else(|| {
                invalid("opponent_seen_prior entries must be nonempty card ID strings")
            })?;
            if crate::cards::get(id).is_none() || crate::cards::is_token(id) {
                return Err(invalid(
                    "opponent_seen_prior contains a non-deck-eligible card ID",
                ));
            }
            prior_ids.insert(id);
            if prior_ids.len() > 12 {
                return Err(invalid(
                    "opponent_seen_prior contains more than 12 unique card IDs",
                ));
            }
        }
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
    let raw_locations = array(&game["locations"]);
    let unavailable = || {
        invalid(
            "SNAP observation does not contain three locations; destroyed lane layout is unavailable",
        )
    };
    let ordered: Vec<Option<&Value>> = if let Some(layout) = options.get("lane_layout") {
        let ids = layout["lanes"]
            .as_array()
            .filter(|ids| ids.len() == 3)
            .filter(|_| layout["complete"] == true)
            .ok_or_else(unavailable)?;
        let mut seen = HashSet::new();
        let mut ordered = Vec::with_capacity(3);
        for id in ids {
            if id.is_null() {
                ordered.push(None);
            } else {
                let entity = id.as_i64().ok_or_else(unavailable)?;
                if !seen.insert(entity) {
                    return Err(unavailable());
                }
                let mut matching = raw_locations
                    .iter()
                    .filter(|location| location["entity_id"] == *id);
                let location = matching.next().ok_or_else(unavailable)?;
                if matching.next().is_some() {
                    return Err(unavailable());
                }
                ordered.push(Some(location));
            }
        }
        if seen.len() != raw_locations.len() {
            return Err(unavailable());
        }
        ordered
    } else if raw_locations.len() == 3 {
        raw_locations.iter().map(Some).collect()
    } else {
        return Err(unavailable());
    };
    let mut remaining_cards = listed(&zones["deck"]);
    let hand_cards = array(&zones["hand"]["cards"]);
    let mut hand = hand_cards.iter().collect::<Vec<_>>();
    hand.sort_by_key(|card| card["zone_position"].as_i64().unwrap_or(0));
    let seats = [&local["entity_id"], &opponent["entity_id"]];
    let hand: Vec<Value> = hand
        .into_iter()
        .map(|c| card_ref(c, false, seats, version == Some(2)))
        .collect();
    let deck = options["deck"].as_array();
    let mut visible: Vec<&Value> = hand_cards.iter().collect();
    let mut seen = Vec::<Value>::new();
    let mut locations = Vec::new();
    for (lane, location) in ordered.iter().enumerate() {
        let Some(location) = location else {
            locations.push(
                json!({"id":null,"destroyed":true,"mine":[],"theirs":[],"theirs_face_down":0}),
            );
            continue;
        };
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
                let mut reference = card_ref(card, true, seats, version == Some(2));
                if options["plays_only"] != true
                    && let Some(table) = game["legal_actions"]
                        .as_array()
                        .filter(|table| !table.is_empty())
                {
                    if let Some(legal) = table
                        .iter()
                        .find(|entry| entry["entity_id"] == card["entity_id"])
                    {
                        if let Some(activate) = legal["can_activate"].as_bool() {
                            reference["can_activate"] = json!(activate);
                        }
                        if legal["valid"] == false {
                            // An option the game marks invalid offers no move.
                            reference["move_targets"] = json!([]);
                        } else if let Some(targets) = legal["target_entity_ids"].as_array() {
                            let lanes: Vec<_> = ordered
                                .iter()
                                .enumerate()
                                .filter_map(|(index, loc)| {
                                    loc.filter(|loc| targets.contains(&loc["entity_id"]))
                                        .map(|_| index)
                                })
                                .collect();
                            reference["move_targets"] = json!(lanes);
                        }
                    } else if card["revealed"] == true {
                        // The game's option table lists every card that can act this turn, so a
                        // revealed card missing from it can neither move nor activate now. The
                        // reader's moves_per_turn stays empty after a move, so without this a
                        // Nightcrawler that already moved looked free to move again.
                        reference["move_targets"] = json!([]);
                        reference["can_activate"] = json!(false);
                    }
                }
                mine.push(reference);
                visible.push(card);
            } else if card["owner_entity_id"] == opponent["entity_id"] {
                if card["card_id"].as_str().is_some_and(|s| !s.is_empty())
                    && card["revealed"] == true
                {
                    theirs.push(card_ref(card, true, seats, version == Some(2)));
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
        if version == Some(2)
            && !lane_ref["id"].is_null()
            && let Some(raw) = location.get("history_state").filter(|v| !v.is_null())
            && let Ok(history) =
                serde_json::from_value::<crate::position::LocationHistoryState>(raw.clone())
        {
            lane_ref["history_state"] =
                serde_json::to_value(history).expect("location history serialization");
        }
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
    let mut known = Vec::new();
    if let Some(prior) = options.get("opponent_seen_prior").and_then(Value::as_array) {
        let mut unique = HashSet::new();
        known.extend(prior.iter().filter(|id| unique.insert(key(id))).cloned());
    }
    let mut unique = HashSet::new();
    seen.retain(|v| unique.insert(key(v)));
    let mode = if options["mode"].is_null() && source["league"] == "RankedRecruit" {
        json!("recruit")
    } else {
        options["mode"].clone()
    };
    let deck_created = crate::cards::meta_nd::deck_origins(
        remaining_cards.as_deref().unwrap_or_default(),
        deck.map(Vec::as_slice),
        options["deck_added"].as_array().map(Vec::as_slice),
    );
    let mut position = json!({"turn":game["turn"],"total_turns":game["total_turns"],"energy":local["energy"],"hand":hand,"deck":remaining_cards.unwrap_or_default(),"deck_count":zones["deck"]["count"],"locations":locations,"opponent":{"hand":opponent_zones["hand"]["count"],"deck":opponent_zones["deck"]["count"],"energy":null,"seen":seen,"deck_list":null,"pool":null},"priority":null,"mode":mode});
    if version == Some(2)
        && let Some(provenance) = game.get("provenance")
    {
        position["provenance"] = provenance.clone();
        if version == Some(2)
            && game["provenance"]["schema_version"] == 2
            && let (Some(local_id), Some(opponent_id)) =
                (local["entity_id"].as_i64(), opponent["entity_id"].as_i64())
            && local_id >= 0
            && opponent_id >= 0
            && local_id != opponent_id
        {
            position["player_entity_ids"] = json!([local_id, opponent_id]);
        }
    }
    // Cube state is opt-in so existing adapter consumers retain their JSON contract.
    if options["include_stakes"] == true
        && let (Some(current), Some(next)) = (
            game["cube_value"].as_u64(),
            game["cube_value_next_turn"].as_u64(),
        )
        && u32::try_from(current).is_ok()
        && u32::try_from(next).is_ok()
    {
        position["stakes"] = json!({"current": current, "next": next});
        for (source, target) in [
            ("max", "maximum"),
            ("next_player_multiplier", "player_multiplier"),
            ("next_game_multiplier", "game_multiplier"),
            ("end_game_multiplier", "end_game_multiplier"),
        ] {
            if let Some(value) = game["stakes"][source].as_u64()
                && u32::try_from(value).is_ok()
            {
                position["stakes"][target] = json!(value);
            }
        }
        let eligibility = [
            local["snap"]["can_raise"].as_bool(),
            opponent["snap"]["can_raise"].as_bool(),
        ];
        if eligibility.iter().any(Option::is_some) {
            position["stakes"]["can_raise"] = json!(eligibility);
        }
        match game["stakes"]["last_raiser_seat"].as_str() {
            Some("local") => position["stakes"]["last_raiser"] = json!(0),
            Some("opponent") => position["stakes"]["last_raiser"] = json!(1),
            _ => {}
        }
    }
    if !crate::cards::reference_rules_enabled() && deck_created.iter().any(Option::is_some) {
        position["deck_created"] = json!(deck_created);
    }
    if let Some(maximum) = local.get("max_energy").and_then(Value::as_i64) {
        position["max_energy"] = json!(maximum);
    }
    for (name, player) in [("max_energy", opponent), ("default_energy", opponent)] {
        if let Some(value) = player.get(name).and_then(Value::as_i64) {
            position["opponent"][name] = json!(value);
        }
    }
    if let Some(value) = local.get("default_energy").and_then(Value::as_i64) {
        position["default_energy"] = json!(value);
    }
    if !known.is_empty() {
        position["opponent"]["known"] = json!(known);
    }
    for (owner, source) in [(0, zones), (1, opponent_zones)] {
        for name in ["graveyard", "banished"] {
            let mut public: Vec<_> = array(&source[name]["cards"])
                .iter()
                .filter(|card| card["card_id"].as_str().is_some_and(|id| !id.is_empty()))
                .collect();
            // Graveyard::GetDestroyedAndDiscardedCards 0x7dd80e8 traverses
            // zone insertion order, not a destroyed-then-discarded grouping.
            if public
                .iter()
                .all(|card| card["zone_position"].as_i64().is_some())
            {
                public.sort_by_key(|card| card["zone_position"].as_i64().unwrap());
            }
            // Legacy recordings already carried local graveyard cards but lacked the
            // extended reader schema; keep their position JSON unchanged.
            if (version == Some(2) && !public.is_empty())
                || public.iter().any(|card| has_extended_card_fields(card))
            {
                let refs: Vec<_> = public
                    .into_iter()
                    .map(|card| card_ref(card, true, seats, version == Some(2)))
                    .collect();
                if owner == 0 {
                    position[name] = json!(refs);
                } else {
                    position["opponent"][name] = json!(refs);
                }
            }
        }
    }
    if let (Some(mine), Some(theirs)) = (
        (options["include_stakes"] == true)
            .then_some(local["snap"]["requested_turns"].as_array())
            .flatten()
            .or_else(|| local["snapped_turns"].as_array()),
        (options["include_stakes"] == true)
            .then_some(opponent["snap"]["requested_turns"].as_array())
            .flatten()
            .or_else(|| opponent["snapped_turns"].as_array()),
    ) {
        position["snapped_this_turn"] = json!([
            mine.iter().any(|v| v.as_i64() == Some(turn)),
            theirs.iter().any(|v| v.as_i64() == Some(turn))
        ]);
    } else if let Some(snapped) = options.get("snapped_this_turn") {
        position["snapped_this_turn"] = snapped.clone();
    }
    if version == Some(2)
        // History slot validation uses the compact reader order, so discard it
        // when that order no longer represents the original three lanes.
        && ordered.iter().zip(raw_locations).all(|(ordered, raw)| {
            ordered.is_some_and(|location| std::ptr::eq(location, raw))
        })
        && raw_locations.len() == 3
        && let Ok(history) =
            serde_json::from_value::<crate::position::ObservedHistory>(game["history"].clone())
        && let Ok(turn) = u8::try_from(turn)
        && valid_history(&history, game, [local, opponent], turn)
    {
        position["history"] = json!(history);
    }
    // An explicit option overrides the observation; malformed evidence remains
    // absent, so coverage cannot mistake it for a complete history.
    if let Some(raw) = options
        .get("previous_turn_locations")
        .or_else(|| game.get("previous_turn_locations"))
        && let Ok(previous) =
            serde_json::from_value::<crate::position::PreviousTurnLocations>(raw.clone())
    {
        position["previous_turn_locations"] = json!(previous);
    }
    Ok(position)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prior_observation() -> Value {
        json!({"schema_version":1,"source":{},"game":{"turn":2,"total_turns":6,
        "players":[
            {"slot":"local","entity_id":10,"energy":2,"ended_turn":1,
             "zones":{"hand":{"count":0,"cards":[]},"deck":{"count":0}}},
            {"slot":"opponent","entity_id":20,
             "zones":{"hand":{"count":0},"deck":{"count":0},
                      "graveyard":{"cards":[{"card_id":"IronMan"}]},
                      "banished":{"cards":[{"card_id":"Cyclops"}]}}}],
        "locations":[{"location_id":"Ruins","cards":[
            {"card_id":"Hulk","owner_entity_id":20,"revealed":true},
            {"card_id":"AntMan","owner_entity_id":20,"revealed":false}
        ]},{"location_id":"Ruins","cards":[]},{"location_id":"Ruins","cards":[]}]}})
    }

    #[test]
    fn missing_destroyed_lane_layout_is_rejected() {
        let mut observation = prior_observation();
        observation["schema_version"] = json!(2);
        assert!(position_from_snap(&observation, &json!({})).is_ok());
        for count in [2, 1] {
            observation["game"]["locations"]
                .as_array_mut()
                .unwrap()
                .truncate(count);
            let error = position_from_snap(&observation, &json!({})).unwrap_err();
            assert!(
                matches!(error, SnapError::Invalid(message) if message.contains("destroyed lane layout is unavailable"))
            );
        }
    }

    #[test]
    fn destroyed_lane_layout_restores_original_indices() {
        let mut observation = prior_observation();
        observation["schema_version"] = json!(2);
        let lanes = observation["game"]["locations"].as_array_mut().unwrap();
        lanes[0]["entity_id"] = json!(101);
        lanes[1]["entity_id"] = json!(102);
        lanes[2]["entity_id"] = json!(103);
        lanes[2]["location_id"] = json!("Wakanda");
        lanes.remove(1);
        lanes.reverse();
        let position = position_from_snap(
            &observation,
            &json!({
                "lane_layout":{"complete":true,"lanes":[101,null,103]}
            }),
        )
        .unwrap();
        assert_eq!(position["locations"][0]["id"], "Ruins");
        assert_eq!(position["locations"][1]["destroyed"], true);
        assert_eq!(position["locations"][2]["id"], "Wakanda");
        for layout in [
            json!({"complete":false,"lanes":[101,null,103]}),
            json!({"complete":true,"lanes":[101,101,null]}),
            json!({"complete":true,"lanes":[101,null,999]}),
        ] {
            assert!(position_from_snap(&observation, &json!({"lane_layout":layout})).is_err());
        }
    }

    #[test]
    fn cube_state_is_opt_in_and_does_not_infer_ownership() {
        let mut observation = prior_observation();
        observation["game"]["cube_value"] = json!(2);
        observation["game"]["cube_value_next_turn"] = json!(4);
        assert!(
            position_from_snap(&observation, &json!({}))
                .unwrap()
                .get("stakes")
                .is_none()
        );
        let position = position_from_snap(&observation, &json!({"include_stakes": true})).unwrap();
        assert_eq!(position["stakes"], json!({"current": 2, "next": 4}));
        assert!(position.get("snapped_this_turn").is_none());
        observation["game"]["players"][0]["snap"] =
            json!({"requested_turns": [2], "can_raise": false});
        observation["game"]["players"][1]["snap"] =
            json!({"requested_turns": [], "can_raise": true});
        assert!(
            position_from_snap(&observation, &json!({}))
                .unwrap()
                .get("snapped_this_turn")
                .is_none()
        );
        let observed = position_from_snap(&observation, &json!({"include_stakes": true})).unwrap();
        assert_eq!(observed["snapped_this_turn"], json!([true, false]));
        observation["game"]["cube_value"] = json!(-1);
        assert!(
            position_from_snap(&observation, &json!({"include_stakes": true}))
                .unwrap()
                .get("stakes")
                .is_none()
        );
    }

    #[test]
    fn adapter_preserves_deck_tracker_origins() {
        let mut observation = prior_observation();
        observation["game"]["players"][0]["zones"]["deck"]["count"] = json!(3);
        let options = json!({"deck":["Wasp","Hulk"],"deck_added":["Rock"]});
        let position = position_from_snap(&observation, &options).unwrap();
        assert_eq!(position["deck"], json!(["Wasp", "Hulk", "Rock"]));
        assert_eq!(position["deck_created"], json!([false, false, true]));
    }

    #[test]
    fn public_graveyard_keeps_client_zone_order() {
        let mut observation = prior_observation();
        observation["schema_version"] = json!(2);
        observation["game"]["players"][1]["zones"]["graveyard"]["cards"] = json!([
            {"card_id":"Hulk","zone_position":2},
            {"card_id":"Wasp","zone_position":0},
            {"card_id":"Cyclops","zone_position":1}
        ]);
        let position = position_from_snap(&observation, &json!({})).unwrap();
        let ids: Vec<_> = position["opponent"]["graveyard"]
            .as_array()
            .unwrap()
            .iter()
            .map(|card| card["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["Wasp", "Cyclops", "Hulk"]);
    }

    #[test]
    fn observed_energy_baselines_are_forwarded_for_both_players() {
        let mut observation = prior_observation();
        observation["game"]["players"][0]["max_energy"] = json!(5);
        observation["game"]["players"][0]["default_energy"] = json!(3);
        observation["game"]["players"][1]["max_energy"] = json!(4);
        observation["game"]["players"][1]["default_energy"] = json!(2);
        let position = position_from_snap(&observation, &json!({})).unwrap();
        assert_eq!(position["max_energy"], 5);
        assert_eq!(position["default_energy"], 3);
        assert_eq!(position["opponent"]["max_energy"], 4);
        assert_eq!(position["opponent"]["default_energy"], 2);
    }

    #[test]
    fn previous_locations_accept_observation_and_override_option() {
        let mut observation = prior_observation();
        let previous = json!({"complete":true,"ids":[["Feast","Westview"],[],[]]});
        observation["game"]["previous_turn_locations"] = previous.clone();
        assert_eq!(
            position_from_snap(&observation, &json!({})).unwrap()["previous_turn_locations"],
            previous
        );
        let option = json!({"complete":false,"ids":[["AltarOfDeath"],[],[]]});
        assert_eq!(
            position_from_snap(&observation, &json!({"previous_turn_locations":option})).unwrap()["previous_turn_locations"],
            option
        );
        let malformed = json!({"previous_turn_locations":{"complete":true,"ids":["Ruins"]}});
        assert!(
            position_from_snap(&observation, &malformed)
                .unwrap()
                .get("previous_turn_locations")
                .is_none()
        );
    }

    #[test]
    fn v2_location_history_is_typed_and_v1_ignores_it() {
        let mut observation = prior_observation();
        observation["game"]["locations"][0]["location_id"] = json!("Vormir");
        observation["game"]["locations"][0]["history_state"] =
            json!({"vormir_consumed":{"local":true,"opponent":null}});
        let v1 = position_from_snap(&observation, &json!({})).unwrap();
        assert!(v1["locations"][0].get("history_state").is_none());
        observation["schema_version"] = json!(2);
        let v2 = position_from_snap(&observation, &json!({})).unwrap();
        assert_eq!(
            v2["locations"][0]["history_state"]["vormir_consumed"]["local"],
            true
        );
        assert!(v2["locations"][0]["history_state"]["vormir_consumed"]["opponent"].is_null());
        observation["game"]["locations"][0]["history_state"]["secret"] = json!(1);
        let invalid = position_from_snap(&observation, &json!({})).unwrap();
        assert!(invalid["locations"][0].get("history_state").is_none());
        observation["game"]["locations"][0]["location_id"] = json!("RevealOn1");
        observation["game"]["locations"][0]["history_state"] = json!({"is_given":true});
        let hidden = position_from_snap(&observation, &json!({})).unwrap();
        assert!(hidden["locations"][0].get("history_state").is_none());
        observation["schema_version"] = json!(3);
        assert!(position_from_snap(&observation, &json!({})).is_err());
    }

    #[test]
    fn v2_provenance_round_trips_without_changing_older_positions() {
        let mut observation = prior_observation();
        let provenance = json!({"schema_version":1,"entities":[{
            "kind":"card","entity_id":42,"card_id":"Nightcrawler","lane_index":null,
            "abilities":{"complete":true,"reasons":[],"items":[{
                "ability_def_id":"NightcrawlerMove","owner_def_type":"Card",
                "owner_def_id":"Nightcrawler","index":0,"turn_created":null}]},
            "movable_components":{"complete":true,"reasons":[],"items":[{
                "source_entity_id":42,"owner_def_type":"Card","owner_def_id":"Nightcrawler",
                "ability_index":0,"created_ticks":"638000000000000000",
                "destination_entity_id":11}]},
            "modifiers":{"complete":true,"reasons":[],"items":[{
                "type":"CardCostModifier","operation":"Add","amount":-1,
                "creator_entity_id":null,"owner_def_type":"Card","owner_def_id":"Beast",
                "player_entity_id":10,"turn_created":null,"expiry_turn":null}]},
            "play_requirements":{"complete":false,"reasons":["redacted"],"items":[]}
        }]});
        observation["game"]["provenance"] = provenance.clone();
        assert!(
            position_from_snap(&observation, &json!({}))
                .unwrap()
                .get("provenance")
                .is_none()
        );
        observation["schema_version"] = json!(2);
        let value = position_from_snap(&observation, &json!({})).unwrap();
        assert_eq!(value["provenance"], provenance);
        let parsed: crate::position::Provenance =
            serde_json::from_value(value["provenance"].clone()).unwrap();
        assert_eq!(
            parsed.entities[0].movable_components.items[0].created_ticks,
            "638000000000000000"
        );
    }

    #[test]
    fn provenance_schema_two_parses_ability_and_ledger() {
        let mut observation = prior_observation();
        observation["schema_version"] = json!(2);
        observation["game"]["provenance"] = json!({
            "schema_version": 2,
            "entities": [{
                "kind": "player", "entity_id": 10, "card_id": null, "lane_index": null,
                "abilities": {"complete": true, "reasons": [], "items": [{
                    "ability_def_id": "ExampleGrantedAbility", "owner_def_type": "Card",
                    "owner_def_id": "ExampleCard", "index": 4,
                    "origin_entity_id": 42, "created_turn": 3, "lifetime": 2, "active": true
                }]},
                "movable_components": {"complete": true, "reasons": [], "items": []}
            }],
            "ledger": {"complete": false, "match_start_seen": false,
                "reasons": ["missing_match_start"], "items": [{
                    "effect_key": "effect-1", "kind": "ability", "target_entity_id": 10,
                    "creator_entity_id": 42, "creator_def_id": "ExampleCard",
                    "created_turn": 3, "expiry_turn": 5, "attribution": "exact",
                    "reasons": [], "active": true, "ability_index": 4
                }]}
        });
        let value = position_from_snap(&observation, &json!({})).unwrap();
        let provenance: crate::position::Provenance =
            serde_json::from_value(value["provenance"].clone()).unwrap();
        assert_eq!(provenance.schema_version, 2);
        assert_eq!(
            provenance.entities[0].abilities.items[0].created_turn,
            Some(3)
        );
        let ledger = provenance.ledger.unwrap();
        assert!(!ledger.complete);
        assert_eq!(ledger.items[0].expiry_turn, Some(5));
        assert_eq!(ledger.items[0].active, Some(true));
    }

    #[test]
    fn v2_history_requires_visible_matching_entities_and_preserves_unknowns() {
        let mut observation = prior_observation();
        observation["schema_version"] = json!(2);
        observation["game"]["locations"][0]["cards"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "entity_id":42,"owner_entity_id":10,"card_id":"Hawkeye","revealed":true
            }));
        observation["game"]["history"] = json!({
            "actions":[{"owner":"local","turn":1,"kind":"play","location_slot":0,
                "card_entity_id":42,"card_id":"Hawkeye","resolved":true,
                "turn_resolved":1,"resolution_order":3}],
            "action_completeness":{"local":{"1":true,"2":false},"opponent":{"1":false}},
            "own_draws":{"card_ids":["Hawkeye","Hawkeye"],"complete":false,
                "last_drawn_entity_id":42,"last_drawn_copy_state":null}
        });
        let position = position_from_snap(&observation, &json!({})).unwrap();
        assert_eq!(
            position["history"]["own_draws"]["card_ids"],
            json!(["Hawkeye", "Hawkeye"])
        );
        assert_eq!(
            position["history"]["action_completeness"]["local"]["2"],
            false
        );
        assert_eq!(position["locations"][0]["mine"][0]["entity_id"], 42);
        observation["game"]["history"]["actions"][0]["card_id"] = json!("Hulk");
        let invalid = position_from_snap(&observation, &json!({})).unwrap();
        assert!(invalid.get("history").is_none());
        observation["game"]["history"]["actions"][0]["card_id"] = json!("Hawkeye");
        observation["game"]["history"]["actions"][0]["owner"] = json!("opponent");
        let secret = position_from_snap(&observation, &json!({})).unwrap();
        assert!(secret.get("history").is_none());
    }

    #[test]
    fn opponent_seen_prior_is_separate_from_current_evidence() {
        let value = position_from_snap(
            &prior_observation(),
            &json!({
                "opponent_seen_prior":["Cyclops","AntMan","Hulk","Medusa","AntMan"]
            }),
        )
        .unwrap();
        assert_eq!(
            value["opponent"]["seen"],
            json!(["Hulk", "IronMan", "Cyclops"])
        );
        assert_eq!(
            value["opponent"]["known"],
            json!(["Cyclops", "AntMan", "Hulk", "Medusa"])
        );
    }

    #[test]
    fn opponent_seen_prior_rejects_invalid_evidence() {
        for prior in [
            json!(null),
            json!("Hulk"),
            json!([null]),
            json!([1]),
            json!([""]),
            json!(["NotACard"]),
            json!(["Rock"]),
            json!(&crate::cards::deck_eligible_ids()[..13]),
        ] {
            assert!(matches!(
                position_from_snap(&prior_observation(), &json!({"opponent_seen_prior":prior})),
                Err(SnapError::Invalid(_))
            ));
        }
        let mut ids = crate::cards::deck_eligible_ids()[..12].to_vec();
        ids.push(ids[0]);
        assert!(
            position_from_snap(&prior_observation(), &json!({"opponent_seen_prior":ids})).is_ok()
        );
    }

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

    #[test]
    fn extended_cards_preserve_nulls_as_absence_and_map_source_seats() {
        let local = json!(10);
        let opponent = json!(20);
        let legacy = json!({"card_id":"Hulk","entity_id":42,"revealed":true});
        assert_eq!(
            card_ref(&legacy, false, [&local, &opponent], false),
            json!({"id":"Hulk","cost":null,"power":null,"revealed":true})
        );
        let card = json!({"card_id":"Hulk","entity_id":42,"revealed":true,
        "latest_resolution_order":null,"created":false,
        "created_by_player_entity_id":20,
        "stat_updates":[
            {"stat":"Power","update":"Multiply","value":-3,"player_source_entity_id":10,"updater_card_id":null,"updater_location_id":"Xandar"},
            {"stat":"Cost","update":"Set","value":2,"player_source_entity_id":99}
        ]});
        let reference = card_ref(&card, false, [&local, &opponent], false);
        assert_eq!(reference["entity_id"], 42);
        assert_eq!(reference["created"], false);
        assert_eq!(reference["created_by_player"], 1);
        assert!(reference.get("latest_resolution_order").is_none());
        assert_eq!(reference["stat_updates"][0]["source"], 0);
        assert!(reference["stat_updates"][1].get("source").is_none());
    }

    #[test]
    fn v2_reveal_turn_survives_card_reference_hydration() {
        let local = json!(10);
        let opponent = json!(20);
        let card = json!({"card_id":"Hulk","revealed":true,"turn_revealed":3});
        let projected = card_ref(&card, true, [&local, &opponent], true);
        let reference: crate::position::CardRef = serde_json::from_value(projected).unwrap();
        let mut restored = crate::position::make_card("Hulk", 0).unwrap();
        crate::position::apply_card_reference(&mut restored, &reference, 4);
        assert_eq!(restored.turn_revealed, Some(3));
        assert!(
            card_ref(&card, true, [&local, &opponent], false)
                .get("turn_revealed")
                .is_none()
        );
    }
}
