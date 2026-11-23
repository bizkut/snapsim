//! Public observation framing for replay checking.
use crate::snap::{LocationWatch, revealed_id};
use serde_json::Value;

/// A board entity as visible at one observation, with hidden information removed.
#[derive(Clone, Debug)]
pub struct BoardCard {
    pub entity: Value,
    pub owner: usize,
    pub lane: usize,
    pub slot: Value,
    pub id: Value,
    pub revealed: bool,
    pub power: Value,
    pub cost: Value,
    pub flipped: Value,
    pub staged: Value,
    pub created_by_location: Value,
    /// Reader ledger is only public after the opponent's card is revealed.
    pub stat_updates: Value,
    pub reference: Option<crate::position::CardRef>,
}

/// One valid observation with the visibility history known at that point.
#[derive(Clone, Debug)]
pub struct Frame {
    pub line: usize,
    pub turn: i64,
    pub total_turns: Value,
    pub key: Value,
    pub players: [Value; 2],
    pub terminal: bool,
    pub conceded: bool,
    pub ended: bool,
    pub ready: bool,
    pub recruit: bool,
    pub locations: Vec<Value>,
    /// Whether the three physical lane slots are identifiable in this frame.
    pub lane_layout_complete: bool,
    pub destroyed_lanes: [bool; 3],
    pub location_turns: Vec<Value>,
    pub previous_turn_locations: Option<crate::position::PreviousTurnLocations>,
    pub observed_history: Option<crate::position::ObservedHistory>,
    pub location_history: Vec<Option<crate::position::LocationHistoryState>>,
    pub totals: Vec<Value>,
    pub board: Vec<BoardCard>,
    pub hand: Vec<Value>,
    pub hand_counts: [Value; 2],
    pub deck_counts: [Value; 2],
    pub out_of_play: Vec<(Value, &'static str)>,
}

impl Frame {
    pub(super) fn complete_past_actions(
        &self,
        seat: usize,
    ) -> Option<Vec<&crate::position::HistoryAction>> {
        let history = self.observed_history.as_ref()?;
        let completeness = history.action_completeness.as_ref()?;
        let turns = match seat {
            0 => &completeness.local,
            1 => &completeness.opponent,
            _ => return None,
        };
        let turn = u8::try_from(self.turn).ok()?;
        if (1..turn).any(|n| turns.get(&n.to_string()) != Some(&true)) {
            return None;
        }
        let owner = if seat == 0 { "local" } else { "opponent" };
        let mut actions = Vec::new();
        let mut orders = std::collections::HashSet::new();
        for action in history
            .actions
            .as_ref()?
            .iter()
            .filter(|a| a.owner == owner && a.turn < turn)
        {
            if action.turn == 0
                || !action.resolved
                || action.turn_resolved != Some(action.turn)
                || !orders.insert((action.turn, action.resolution_order?))
                || self
                    .board
                    .iter()
                    .filter(|card| {
                        card.owner == seat
                            && card.revealed
                            && card.entity.as_i64() == Some(action.card_entity_id)
                            && card.id == action.card_id
                    })
                    .count()
                    != 1
            {
                return None;
            }
            actions.push(action);
        }
        actions.sort_by_key(|action| (action.turn, action.resolution_order));
        Some(actions)
    }
}

fn array(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|n| n != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(value) => !value.is_empty(),
        Value::Object(value) => !value.is_empty(),
    }
}

/// Split valid schema-one or schema-two observations at SNAP match boundaries.
pub fn matches(
    observations: impl IntoIterator<Item = (usize, Value)>,
) -> Result<Vec<Vec<Frame>>, String> {
    let mut matches = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut watch = LocationWatch::default();
    let mut lane_entities: Option<[Value; 3]> = None;
    let mut destroyed_lanes = [false; 3];
    for (line, observation) in observations {
        if !truthy(&observation) || observation["status"] == "no-active-game" {
            continue;
        }
        if observation["schema_version"] != 1 && observation["schema_version"] != 2 {
            return Err(format!("line {line}: expected SNAP schema_version 1 or 2"));
        }
        let game = &observation["game"];
        let Some(turn) = game["turn"].as_i64().filter(|turn| *turn != 0) else {
            continue;
        };
        let players = array(&game["players"]);
        let Some(local) = players.iter().rev().find(|p| p["slot"] == "local") else {
            continue;
        };
        let Some(opponent) = players.iter().rev().find(|p| p["slot"] == "opponent") else {
            continue;
        };
        if players
            .iter()
            .any(|p| p["slot"] != "local" && p["slot"] != "opponent")
        {
            continue;
        }
        let players = [local, opponent];
        let ids = [local["entity_id"].clone(), opponent["entity_id"].clone()];
        let terminal = truthy(&observation["source"]["terminal"]);
        if frames.last().is_some_and(|last| {
            turn < last.turn
                || game["game_key"] != last.key
                || ids != last.players
                || (last.terminal && !terminal)
        }) {
            matches.push(std::mem::take(&mut frames));
            watch = LocationWatch::default();
            lane_entities = None;
            destroyed_lanes = [false; 3];
        }
        let observed_locations = array(&game["locations"]);
        let valid_ids = observed_locations
            .iter()
            .all(|location| location["entity_id"].as_i64().is_some())
            && observed_locations
                .iter()
                .enumerate()
                .all(|(index, location)| {
                    observed_locations[..index]
                        .iter()
                        .all(|other| other["entity_id"] != location["entity_id"])
                });
        let mut layout_complete = false;
        let mut lanes: [Option<&Value>; 3] = [None; 3];
        if observed_locations.len() == 3
            && observed_locations.iter().all(Value::is_object)
            && (valid_ids
                || observed_locations
                    .iter()
                    .all(|location| location["entity_id"].is_null()))
            && !destroyed_lanes.iter().any(|&destroyed| destroyed)
        {
            for (lane, location) in observed_locations.iter().enumerate() {
                lanes[lane] = Some(location);
            }
            layout_complete = true;
            lane_entities = valid_ids
                .then(|| std::array::from_fn(|i| observed_locations[i]["entity_id"].clone()));
        } else if game["locations"].is_array()
            && observed_locations.len() <= 3
            && valid_ids
            && let Some(entities) = &lane_entities
        {
            let mut last = None;
            let mut valid = true;
            for location in observed_locations {
                let Some(lane) = entities.iter().position(|id| *id == location["entity_id"]) else {
                    valid = false;
                    break;
                };
                if destroyed_lanes[lane] || last.is_some_and(|previous| lane <= previous) {
                    valid = false;
                    break;
                }
                lanes[lane] = Some(location);
                last = Some(lane);
            }
            if valid {
                layout_complete = true;
                for lane in 0..3 {
                    destroyed_lanes[lane] |= lanes[lane].is_none();
                }
            }
        }
        if !layout_complete {
            lane_entities = None;
            lanes = [None; 3];
        }
        let ended = local["ended_turn"]
            .as_i64()
            .is_some_and(|ended| ended >= turn);
        let zones = [&local["zones"], &opponent["zones"]];
        let mut board = Vec::new();
        for (lane, location) in lanes
            .iter()
            .enumerate()
            .filter_map(|(lane, location)| location.map(|location| (lane, location)))
        {
            for card in array(&location["cards"]) {
                let owner = ids
                    .iter()
                    .position(|id| *id == card["owner_entity_id"])
                    .ok_or_else(|| format!("line {line}: unknown board owner"))?;
                let revealed = truthy(&card["revealed"]);
                board.push(BoardCard {
                    entity: card["entity_id"].clone(),
                    owner,
                    lane,
                    slot: card["zone_position"].clone(),
                    id: if revealed {
                        card["card_id"].clone()
                    } else {
                        Value::Null
                    },
                    revealed,
                    power: if revealed || owner == 0 {
                        card["power"].clone()
                    } else {
                        Value::Null
                    },
                    cost: if revealed || owner == 0 {
                        card["cost"].clone()
                    } else {
                        Value::Null
                    },
                    flipped: if revealed {
                        card["turn_revealed"].clone()
                    } else {
                        Value::Null
                    },
                    staged: if owner == 0 {
                        card["staged_turn"].clone()
                    } else {
                        Value::Null
                    },
                    created_by_location: if revealed || owner == 0 {
                        card["created_by_location_id"].clone()
                    } else {
                        Value::Null
                    },
                    stat_updates: if revealed || owner == 0 {
                        card["stat_updates"].clone()
                    } else {
                        Value::Null
                    },
                    reference: if observation["schema_version"] == 2 && revealed {
                        serde_json::from_value(crate::snap::card_ref(
                            card,
                            true,
                            [&local["entity_id"], &opponent["entity_id"]],
                            true,
                        ))
                        .ok()
                    } else {
                        None
                    },
                });
            }
        }
        let mut out_of_play = Vec::new();
        for zone in zones {
            for name in ["graveyard", "banished"] {
                out_of_play.extend(
                    array(&zone[name]["cards"])
                        .iter()
                        .map(|card| (card["entity_id"].clone(), name)),
                );
            }
        }
        let previous_turn_locations = if frames.last().is_some_and(|last| last.turn == turn) {
            frames
                .last()
                .and_then(|last| last.previous_turn_locations.clone())
        } else if frames.last().is_some_and(|last| last.turn == turn - 1) {
            let mut ids: [Vec<String>; 3] = Default::default();
            let mut complete = true;
            for prior in frames
                .iter()
                .rev()
                .take_while(|frame| frame.turn == turn - 1)
            {
                complete &= prior.lane_layout_complete;
                for (lane, id) in prior.locations.iter().enumerate() {
                    if let Some(id) = id.as_str()
                        && !ids[lane].iter().any(|seen| seen == id)
                    {
                        ids[lane].push(id.to_owned());
                    }
                }
            }
            Some(crate::position::PreviousTurnLocations { complete, ids })
        } else {
            serde_json::from_value(game["previous_turn_locations"].clone()).ok()
        };
        frames.push(Frame {
            line,
            turn,
            total_turns: game["total_turns"].clone(),
            key: game["game_key"].clone(),
            players: ids,
            terminal,
            conceded: players.iter().any(|p| truthy(&p["conceded"])),
            ended,
            ready: !terminal
                && !ended
                && !truthy(&game["visuals_pending"])
                && !truthy(&game["pending_stage_request"])
                && truthy(&game["legal_actions"]),
            recruit: observation["source"]["league"] == "RankedRecruit",
            locations: lanes
                .iter()
                .map(|location| location.map_or(Value::Null, revealed_id))
                .collect(),
            lane_layout_complete: layout_complete,
            destroyed_lanes,
            previous_turn_locations,
            // History action lane indices are validated against the compact reader list;
            // they cannot safely be interpreted as physical slots after destruction.
            observed_history: if observation["schema_version"] == 2
                && observed_locations.len() == 3
                && layout_complete
            {
                serde_json::from_value::<crate::position::ObservedHistory>(game["history"].clone())
                    .ok()
                    .filter(|history| {
                        u8::try_from(turn).is_ok_and(|turn| {
                            crate::snap::valid_history(history, game, [local, opponent], turn)
                        })
                    })
            } else {
                None
            },
            location_turns: if layout_complete {
                watch.update_lanes(turn, lanes.iter().copied())
            } else {
                vec![Value::Null; 3]
            },
            location_history: lanes
                .iter()
                .map(|location| {
                    location.and_then(|location| {
                        if observation["schema_version"] == 2 && !revealed_id(location).is_null() {
                            serde_json::from_value(location["history_state"].clone()).ok()
                        } else {
                            None
                        }
                    })
                })
                .collect(),
            totals: lanes
                .iter()
                .map(|location| {
                    location.map_or(Value::Null, |location| location["total_power"].clone())
                })
                .collect(),
            board,
            hand: array(&zones[0]["hand"]["cards"]).to_vec(),
            hand_counts: [
                zones[0]["hand"]["count"].clone(),
                zones[1]["hand"]["count"].clone(),
            ],
            deck_counts: [
                zones[0]["deck"]["count"].clone(),
                zones[1]["deck"]["count"].clone(),
            ],
            out_of_play,
        });
    }
    if !frames.is_empty() {
        matches.push(frames);
    }
    Ok(matches)
}

/// Group consecutive frames by turn without merging later repeats.
pub fn turns(frames: &[Frame]) -> Vec<&[Frame]> {
    frames
        .chunk_by(|left, right| left.turn == right.turn)
        .collect()
}

/// A staged card whose public identity was established by a later flip.
#[derive(Clone, Debug)]
pub struct Play {
    pub card: BoardCard,
    pub id: Value,
    pub cost: Value,
    pub power: Value,
    pub turn: i64,
    pub stage_line: usize,
    pub flip_line: usize,
}

/// The ordered inputs that were visible before the first reveal.
pub struct Staging<'a> {
    pub plays: Vec<Play>,
    pub moves: Vec<(Value, usize, usize)>,
    pub before: &'a Frame,
    pub opponent_count: i64,
}

fn entity<'a>(frame: &'a Frame, id: &Value) -> Option<&'a BoardCard> {
    frame.board.iter().find(|card| card.entity == *id)
}

fn label(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "None".into(),
        _ => value.to_string(),
    }
}

/// Recover submitted plays and manual moves from the pre-flip observations.
pub fn staging<'a>(
    start: &'a Frame,
    samples: &[&'a Frame],
    target: &Frame,
    all_frames: &[Frame],
    delays: bool,
) -> Result<Staging<'a>, String> {
    let locked: Vec<_> = samples
        .iter()
        .copied()
        .filter(|f| f.ended && !f.terminal)
        .collect();
    if locked.is_empty() {
        return Err("no submitted, pre-reveal frame".into());
    }
    let first = locked.into_iter().find(|frame| {
        frame.board.iter().any(|card| {
            card.revealed && entity(start, &card.entity).is_none_or(|old| !old.revealed)
        })
    });
    let before: Vec<_> = samples
        .iter()
        .copied()
        .filter(|f| !f.terminal && first.is_none_or(|flip| f.line < flip.line))
        .collect();
    let Some(&last) = before.last().filter(|frame| frame.ended) else {
        return Err("no observation after submission and before the first flip".into());
    };
    let mut staged: Vec<_> = last
        .board
        .iter()
        .filter(|card| !card.revealed)
        .cloned()
        .collect();
    if let Some(flip) = first {
        let added: Vec<_> = flip
            .board
            .iter()
            .filter(|card| card.owner == 1 && entity(last, &card.entity).is_none())
            .cloned()
            .collect();
        if !added.is_empty() {
            let pulls = flip.board.iter().any(|card| {
                card.revealed
                    && card.owner == 0
                    && staged.iter().any(|s| s.entity == card.entity)
                    && start.locations[card.lane] == "OttosLab"
            });
            let drop = last.hand_counts[1].as_i64().unwrap_or(0)
                - flip.hand_counts[1].as_i64().unwrap_or(0);
            if pulls || drop != added.len() as i64 {
                return Err(
                    "first flip and opponent staging/additions could not be separated".into(),
                );
            }
            staged.extend(added);
        }
    }
    let opponent_count = last.hand_counts[1].as_i64().unwrap_or(0)
        - staged
            .iter()
            .filter(|card| entity(last, &card.entity).is_none())
            .count() as i64;
    let expected = start.hand_counts[1].as_i64().unwrap_or(0)
        - staged
            .iter()
            .filter(|card| card.owner == 1 && entity(start, &card.entity).is_none())
            .count() as i64;
    if opponent_count != expected {
        return Err("opponent hand count does not account for all pre-reveal plays".into());
    }
    let mut plays = Vec::new();
    for card in staged {
        let Some((flip, visible)) = all_frames.iter().find_map(|frame| {
            entity(frame, &card.entity)
                .filter(|c| c.revealed && truthy(&c.id))
                .map(|c| (frame, c))
        }) else {
            return Err(format!(
                "staged entity [{}] was never shown revealed",
                label(&card.entity)
            ));
        };
        if flip.line <= start.line
            || flip.turn > start.turn + i64::from(delays)
            || (!delays && flip.line > target.line)
        {
            return Err(format!(
                "reveal timing of {} [{}] is unknown",
                label(&visible.id),
                label(&card.entity)
            ));
        }
        let held = (card.owner == 0)
            .then(|| {
                start
                    .hand
                    .iter()
                    .find(|held| held["entity_id"] == card.entity)
            })
            .flatten();
        let (cost, power) = if let Some(held) = held {
            (held["cost"].clone(), held["power"].clone())
        } else if card.owner == 0 && !card.revealed {
            (card.cost.clone(), card.power.clone())
        } else {
            (Value::Null, Value::Null)
        };
        let mut entered = first.map_or(last.line, |frame| frame.line);
        for frame in before.iter().rev() {
            if entity(frame, &card.entity).is_none() {
                break;
            }
            entered = frame.line;
        }
        let existed = entity(start, &card.entity).is_some();
        if existed {
            entered = all_frames
                .iter()
                .find(|frame| entity(frame, &card.entity).is_some_and(|c| !c.revealed))
                .map_or(entered, |frame| frame.line);
        }
        plays.push(Play {
            card,
            id: visible.id.clone(),
            cost,
            power,
            turn: start.turn - i64::from(existed),
            stage_line: entered,
            flip_line: flip.line,
        });
    }
    plays.sort_by_key(|play| play.flip_line);
    for tied in plays.chunk_by_mut(|a, b| a.flip_line == b.flip_line) {
        if tied.len() > 1 && !delays {
            for (index, a) in tied.iter().enumerate() {
                for b in &tied[index + 1..] {
                    if a.card.owner != b.card.owner
                        || (a.stage_line == b.stage_line
                            && (a.card.lane != b.card.lane || a.card.slot == b.card.slot))
                    {
                        return Err(
                            "cards flipped between the same observations; reveal order is unknown"
                                .into(),
                        );
                    }
                }
            }
        }
        tied.sort_by_key(|play| (play.stage_line, play.card.slot.as_i64().unwrap_or(0)));
    }
    let mut lanes: Vec<_> = start
        .board
        .iter()
        .filter(|card| card.revealed)
        .map(|card| (card, card.lane))
        .collect();
    let mut moves = Vec::new();
    for frame in before.iter().skip(1) {
        if frame.locations != start.locations {
            return Err("locations changed during staging before any observed reveal".into());
        }
        for (prior, lane) in &mut lanes {
            let Some(card) = entity(frame, &prior.entity).filter(|c| c.revealed) else {
                return Err(format!(
                    "{} [{}] left the board before reveals",
                    label(&prior.id),
                    label(&prior.entity)
                ));
            };
            if card.id != prior.id || card.owner != prior.owner {
                return Err(format!(
                    "{} [{}] changed identity or owner before reveals",
                    label(&prior.id),
                    label(&prior.entity)
                ));
            }
            if card.lane != *lane {
                moves.push((prior.entity.clone(), card.lane, frame.line));
                *lane = card.lane;
            }
        }
    }
    Ok(Staging {
        plays,
        moves,
        before: last,
        opponent_count,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Consistent,
    Inconsistent,
    Uncheckable,
}

#[derive(Clone, Debug)]
pub struct Difference {
    pub item: String,
    pub cards: Vec<String>,
    pub lanes: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct ResultRow {
    pub match_number: usize,
    pub turn: i64,
    pub status: Status,
    pub detail: String,
    pub order_enumerated: bool,
    pub differences: Vec<Difference>,
    pub cards: Vec<String>,
    pub locations: Vec<String>,
}

#[derive(Default)]
struct History {
    seen: [Vec<String>; 2],
    embassy_reveals: usize,
    chavez_reveals: usize,
    lab_spent: Vec<(usize, Value, usize)>,
    lab_unknown: Vec<(usize, Value)>,
}

mod check;
mod json_error;
pub mod report;
mod simulate;
pub use check::{check_file, check_observations};

#[cfg(test)]
mod tests {
    use super::{check_observations, matches, turns};
    use serde_json::{Value, json};

    fn observation(turn: u8, key: &str, terminal: bool) -> Value {
        json!({
            "schema_version": 1,
            "source": {"terminal": terminal},
            "game": {
                "turn": turn, "total_turns": 6, "game_key": key,
                "previous_turn_locations":{"complete":true,"ids":[["Ruins"],[],[]]},
                "legal_actions": [1], "visuals_pending": false, "pending_stage_request": false,
                "players": [
                    {"slot": "local", "entity_id": 1, "ended_turn": 0, "zones": {"hand": {"count": 0, "cards": []}, "deck": {"count": 0}}},
                    {"slot": "opponent", "entity_id": 2, "ended_turn": 0, "zones": {"hand": {"count": 0}, "deck": {"count": 0}}}
                ],
                "locations": [
                    {"location_id": "Ruins", "cards": []},
                    {"location_id": "RevealOn2", "cards": []},
                    {"location_id": "RevealOn3", "cards": []}
                ]
            }
        })
    }

    fn identified_observation(turn: u8) -> Value {
        let mut value = observation(turn, "identified", false);
        for lane in 0..3 {
            value["game"]["locations"][lane]["entity_id"] = json!(10 + lane);
            value["game"]["locations"][lane]["location_id"] = json!("Ruins");
        }
        value
    }

    #[test]
    fn missing_location_preserves_physical_lane_and_survivor_card() {
        let start = identified_observation(3);
        let mut destroyed = identified_observation(4);
        destroyed["game"]["locations"]
            .as_array_mut()
            .unwrap()
            .remove(1);
        destroyed["game"]["locations"][1]["cards"] = json!([{
            "entity_id": 50, "owner_entity_id": 1, "card_id": "Wasp",
            "revealed": true, "power": 1
        }]);
        let frames = matches(vec![(1, start), (2, destroyed)]).unwrap();
        let frame = &frames[0][1];
        assert!(frame.lane_layout_complete);
        assert_eq!(frame.destroyed_lanes, [false, true, false]);
        assert_eq!(
            frame.locations,
            [json!("Ruins"), Value::Null, json!("Ruins")]
        );
        assert_eq!(frame.board[0].lane, 2);
    }

    #[test]
    fn two_destroyed_lanes_remain_distinct_from_hidden_lanes() {
        let full = identified_observation(3);
        let mut partial = identified_observation(4);
        let locations = partial["game"]["locations"].as_array_mut().unwrap();
        locations.remove(2);
        locations.remove(0);
        locations[0]["location_id"] = json!("RevealOn2");
        let frames = matches(vec![(1, full), (2, partial)]).unwrap();
        let frame = &frames[0][1];
        assert!(frame.lane_layout_complete);
        assert_eq!(frame.destroyed_lanes, [true, false, true]);
        assert_eq!(frame.locations, [Value::Null, Value::Null, Value::Null]);
    }

    #[test]
    fn replay_after_location_destruction_checks_surviving_lanes() {
        let full = identified_observation(3);
        let mut start = identified_observation(4);
        start["game"]["locations"].as_array_mut().unwrap().remove(1);
        let mut submitted = start.clone();
        submitted["game"]["players"][0]["ended_turn"] = json!(4);
        let mut next = start.clone();
        next["game"]["turn"] = json!(5);
        let result = check_observations(
            vec![(1, full), (2, start), (3, submitted), (4, next)],
            &[],
            1,
        )
        .unwrap();
        assert_eq!(result[1].status, super::Status::Consistent);
    }

    #[test]
    fn replay_detects_unexplained_location_destruction() {
        let start = identified_observation(4);
        let mut submitted = start.clone();
        submitted["game"]["players"][0]["ended_turn"] = json!(4);
        let mut target = identified_observation(5);
        target["game"]["locations"]
            .as_array_mut()
            .unwrap()
            .remove(1);
        let result =
            check_observations(vec![(1, start), (2, submitted), (3, target)], &[], 1).unwrap();
        assert_eq!(result[0].status, super::Status::Inconsistent);
        assert!(
            result[0]
                .differences
                .iter()
                .any(|difference| difference.item.contains("lane 2 destroyed"))
        );
    }

    #[test]
    fn missing_or_ambiguous_location_ids_fail_closed() {
        let mut first = identified_observation(4);
        first["game"]["locations"].as_array_mut().unwrap().remove(1);
        let mut next = first.clone();
        next["game"]["turn"] = json!(5);
        let result = check_observations(vec![(1, first), (2, next)], &[], 1).unwrap();
        assert_eq!(result[0].status, super::Status::Uncheckable);
        assert!(result[0].detail.contains("destroyed-lane layout"));

        let full = identified_observation(3);
        let mut duplicate = identified_observation(4);
        duplicate["game"]["locations"][1]["entity_id"] = json!(10);
        let frames = matches(vec![(1, full), (2, duplicate)]).unwrap();
        assert!(!frames[0][1].lane_layout_complete);
        let full = identified_observation(3);
        let mut unknown = identified_observation(4);
        unknown["game"]["locations"]
            .as_array_mut()
            .unwrap()
            .remove(1);
        unknown["game"]["locations"][1]["entity_id"] = json!(999);
        let frames = matches(vec![(1, full), (2, unknown)]).unwrap();
        assert!(!frames[0][1].lane_layout_complete);

        let mut extra = identified_observation(4);
        extra["game"]["locations"]
            .as_array_mut()
            .unwrap()
            .push(json!({"entity_id": 13}));
        let frames = matches(vec![(1, extra)]).unwrap();
        assert!(!frames[0][0].lane_layout_complete);
        let full = identified_observation(3);
        let mut absent = identified_observation(4);
        absent["game"]["locations"] = Value::Null;
        let frames = matches(vec![(1, full), (2, absent)]).unwrap();
        assert!(!frames[0][1].lane_layout_complete);
    }

    #[test]
    fn location_ids_reset_between_matches() {
        let full = identified_observation(3);
        let mut missing = identified_observation(4);
        missing["game"]["locations"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        missing["game"]["game_key"] = json!("new-match");
        let frames = matches(vec![(1, full), (2, missing)]).unwrap();
        assert_eq!(frames.len(), 2);
        assert!(!frames[1][0].lane_layout_complete);
    }

    #[test]
    fn groups_matches_and_consecutive_turns() {
        let frames = matches(vec![
            (1, observation(1, "a", false)),
            (2, observation(2, "a", false)),
            (3, observation(2, "a", true)),
            (4, observation(2, "a", false)),
            (5, observation(1, "b", false)),
        ])
        .unwrap();
        assert_eq!(frames.iter().map(Vec::len).collect::<Vec<_>>(), [3, 1, 1]);
        assert_eq!(
            turns(&frames[0])
                .iter()
                .map(|t| t.len())
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert!(frames[0][0].ready);
        assert!(!frames[0][2].ready);
        assert!(frames[0][0].locations[1].is_null());
    }

    #[test]
    fn resolves_an_empty_submitted_turn() {
        let mut start = observation(4, "a", false);
        start["game"]["locations"][1]["location_id"] = json!("Ruins");
        start["game"]["locations"][2]["location_id"] = json!("Ruins");
        let mut submitted = start.clone();
        submitted["game"]["players"][0]["ended_turn"] = json!(4);
        let mut next = observation(5, "a", false);
        next["game"]["locations"][1]["location_id"] = json!("Ruins");
        next["game"]["locations"][2]["location_id"] = json!("Ruins");
        let results =
            check_observations(vec![(1, start), (2, submitted), (3, next)], &[], 1).unwrap();
        assert_eq!(results[0].status, super::Status::Consistent);
        assert_eq!(
            results[0].detail,
            "lines 2→3; seed 0 (matched after 1/1 attempts)"
        );
        assert_eq!(results[1].status, super::Status::Uncheckable);
    }

    #[test]
    fn revealed_generic_location_has_no_identity() {
        let start = observation(4, "a", false);
        let mut submitted = start.clone();
        submitted["game"]["players"][0]["ended_turn"] = json!(4);
        let next = observation(5, "a", false);
        let results =
            check_observations(vec![(1, start), (2, submitted), (3, next)], &[], 1).unwrap();
        assert_eq!(results[0].status, super::Status::Consistent);
    }

    #[test]
    fn nico_hidden_next_card_spell_is_not_reported_as_rule_failure() {
        let mut start = observation(2, "nico", false);
        start["game"]["locations"][0]["cards"] = json!([{
            "entity_id": 35, "owner_entity_id": 2, "card_id": "NicoMinoru",
            "revealed": true, "power": 2, "turn_revealed": 1
        }]);
        start["game"]["locations"][1]["location_id"] = json!("Proscenium");
        start["game"]["players"][1]["zones"]["hand"]["count"] = json!(1);
        let mut submitted = start.clone();
        submitted["game"]["players"][0]["ended_turn"] = json!(2);
        submitted["game"]["players"][1]["zones"]["hand"]["count"] = json!(0);
        submitted["game"]["locations"][1]["cards"] = json!([{
            "entity_id": 34, "owner_entity_id": 2, "revealed": false
        }]);
        let mut revealed = submitted.clone();
        revealed["game"]["locations"][1]["cards"] = json!([{
            "entity_id": 34, "owner_entity_id": 2, "card_id": "Forge",
            "revealed": true, "power": 4, "turn_revealed": 2
        }]);
        let mut next = revealed.clone();
        next["game"]["turn"] = json!(3);
        next["game"]["players"][0]["ended_turn"] = json!(0);
        next["game"]["locations"][2]["location_id"] = json!("Ruins");
        next["game"]["locations"][1]["cards"][0]["card_id"] = json!("Demon");
        next["game"]["locations"][1]["cards"][0]["power"] = json!(6);
        let rows = check_observations(
            vec![(1, start), (2, submitted), (3, revealed), (4, next)],
            &[],
            1,
        )
        .unwrap();
        assert_eq!(rows[0].status, super::Status::Uncheckable);
        assert!(rows[0].detail.contains("Nico Minoru's spell"));
    }

    #[test]
    fn same_turn_nico_spell_before_ally_is_not_checkable() {
        let mut start = observation(2, "nico-same-turn", false);
        start["game"]["locations"][1]["location_id"] = json!("Ruins");
        start["game"]["players"][1]["zones"]["hand"]["count"] = json!(2);
        let mut submitted = start.clone();
        submitted["game"]["players"][0]["ended_turn"] = json!(2);
        submitted["game"]["players"][1]["zones"]["hand"]["count"] = json!(0);
        submitted["game"]["locations"][1]["cards"] = json!([
            {"entity_id": 34, "owner_entity_id": 2, "zone_position": 0, "revealed": false},
            {"entity_id": 35, "owner_entity_id": 2, "zone_position": 1, "revealed": false}
        ]);
        let mut nico = submitted.clone();
        nico["game"]["locations"][1]["cards"][0] = json!({
            "entity_id": 34, "owner_entity_id": 2, "zone_position": 0,
            "card_id": "NicoMinoru", "revealed": true, "power": 2, "turn_revealed": 2
        });
        let mut ally = nico.clone();
        ally["game"]["locations"][1]["cards"][1] = json!({
            "entity_id": 35, "owner_entity_id": 2, "zone_position": 1,
            "card_id": "Agony", "revealed": true, "power": 2, "turn_revealed": 2
        });
        let mut next = ally.clone();
        next["game"]["turn"] = json!(3);
        next["game"]["players"][0]["ended_turn"] = json!(0);
        next["game"]["locations"][2]["location_id"] = json!("Ruins");
        next["game"]["locations"][1]["cards"][1]["card_id"] = json!("Demon");
        next["game"]["locations"][1]["cards"][1]["power"] = json!(6);
        let rows = check_observations(
            vec![(1, start), (2, submitted), (3, nico), (4, ally), (5, next)],
            &[],
            1,
        )
        .unwrap();
        assert_eq!(rows[0].status, super::Status::Uncheckable);
        assert!(rows[0].detail.contains("spell for a later play this turn"));
    }

    #[test]
    fn maverick_pending_activation_is_not_reported_as_rule_failure() {
        let mut start = observation(2, "maverick", false);
        start["game"]["locations"][0]["cards"] = json!([{
            "entity_id": 35, "owner_entity_id": 2, "card_id": "Maverick",
            "revealed": true, "power": 2, "turn_revealed": 1
        }]);
        start["game"]["locations"][1]["location_id"] = json!("Proscenium");
        start["game"]["players"][1]["zones"]["hand"]["count"] = json!(1);
        let mut submitted = start.clone();
        submitted["game"]["players"][0]["ended_turn"] = json!(2);
        submitted["game"]["players"][1]["zones"]["hand"]["count"] = json!(0);
        submitted["game"]["locations"][1]["cards"] = json!([{
            "entity_id": 34, "owner_entity_id": 2, "revealed": false
        }]);
        let mut revealed = submitted.clone();
        revealed["game"]["locations"][1]["cards"] = json!([{
            "entity_id": 34, "owner_entity_id": 2, "card_id": "Hulk",
            "revealed": true, "power": 14, "turn_revealed": 2
        }]);
        let mut next = revealed.clone();
        next["game"]["turn"] = json!(3);
        next["game"]["players"][0]["ended_turn"] = json!(0);
        next["game"]["locations"][2]["location_id"] = json!("Ruins");
        let rows = check_observations(
            vec![(1, start), (2, submitted), (3, revealed), (4, next)],
            &[],
            1,
        )
        .unwrap();
        assert_eq!(rows[0].status, super::Status::Uncheckable);
        assert!(rows[0].detail.contains("Maverick's pending"));
    }

    #[test]
    fn ta_lo_hidden_hand_power_is_not_reported_as_rule_failure() {
        let mut prior = observation(1, "talo", false);
        prior["game"]["locations"][0]["location_id"] = json!("TaLo");
        prior["game"]["players"][1]["zones"]["hand"]["count"] = json!(1);
        let mut start = prior.clone();
        start["game"]["turn"] = json!(2);
        start["game"]["locations"][1]["location_id"] = json!("Ruins");
        let mut submitted = start.clone();
        submitted["game"]["players"][0]["ended_turn"] = json!(2);
        submitted["game"]["players"][1]["zones"]["hand"]["count"] = json!(0);
        submitted["game"]["locations"][1]["cards"] = json!([{
            "entity_id": 34, "owner_entity_id": 2, "revealed": false
        }]);
        let mut revealed = submitted.clone();
        revealed["game"]["locations"][1]["cards"] = json!([{
            "entity_id": 34, "owner_entity_id": 2, "card_id": "Hulk",
            "revealed": true, "power": 15, "turn_revealed": 2
        }]);
        let mut next = revealed.clone();
        next["game"]["turn"] = json!(3);
        next["game"]["players"][0]["ended_turn"] = json!(0);
        next["game"]["locations"][2]["location_id"] = json!("Ruins");
        let rows = check_observations(
            vec![
                (1, prior),
                (2, start),
                (3, submitted),
                (4, revealed),
                (5, next),
            ],
            &[],
            1,
        )
        .unwrap();
        assert_eq!(rows[1].status, super::Status::Uncheckable);
        assert!(rows[1].detail.contains("Ta Lo"));
    }

    fn hidden_power_case(updates: Value, power: i32) -> Vec<super::ResultRow> {
        let mut start = observation(2, "hidden-power", false);
        start["game"]["locations"][0]["cards"] = json!([{
            "entity_id": 35, "owner_entity_id": 2, "card_id": "Surge",
            "revealed": true, "power": 1, "turn_revealed": 1
        }]);
        start["game"]["locations"][1]["location_id"] = json!("Ruins");
        start["game"]["players"][1]["zones"]["hand"]["count"] = json!(1);
        let mut submitted = start.clone();
        submitted["game"]["players"][0]["ended_turn"] = json!(2);
        submitted["game"]["players"][1]["zones"]["hand"]["count"] = json!(0);
        submitted["game"]["locations"][1]["cards"] = json!([{
            "entity_id": 34, "owner_entity_id": 2, "revealed": false
        }]);
        let mut revealed = submitted.clone();
        revealed["game"]["locations"][1]["cards"] = json!([{
            "entity_id": 34, "owner_entity_id": 2, "card_id": "Agony",
            "revealed": true, "power": power, "turn_revealed": 2,
            "stat_updates": updates
        }]);
        let mut next = revealed.clone();
        next["game"]["turn"] = json!(3);
        next["game"]["players"][0]["ended_turn"] = json!(0);
        next["game"]["locations"][2]["location_id"] = json!("Ruins");
        check_observations(
            vec![(1, start), (2, submitted), (3, revealed), (4, next)],
            &[],
            1,
        )
        .unwrap()
    }

    #[test]
    fn hidden_deck_buff_is_recovered_from_revealed_ledger() {
        let rows = hidden_power_case(
            json!([{
                "stat": "Power", "update": "Add", "value": 1,
                "updater_card_id": "Surge"
            }]),
            3,
        );
        assert_eq!(rows[0].status, super::Status::Consistent);
    }

    #[test]
    fn hidden_ambiguous_set_is_not_guessed() {
        let rows = hidden_power_case(
            json!([{
                "stat": "Power", "update": "Set", "value": 6
            }]),
            6,
        );
        assert_eq!(rows[0].status, super::Status::Uncheckable);
        assert!(rows[0].detail.contains("cannot be timed"));
    }

    #[test]
    fn rejects_invalid_schema_and_zero_seeds() {
        assert_eq!(
            matches(vec![(42, json!({"schema_version": 3}))]).unwrap_err(),
            "line 42: expected SNAP schema_version 1 or 2"
        );
        assert_eq!(
            check_observations(Vec::<(usize, Value)>::new(), &[], 0).unwrap_err(),
            "--seeds must be positive"
        );
    }
}
