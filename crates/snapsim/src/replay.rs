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
    pub location_turns: Vec<Value>,
    pub totals: Vec<Value>,
    pub board: Vec<BoardCard>,
    pub hand: Vec<Value>,
    pub hand_counts: [Value; 2],
    pub deck_counts: [Value; 2],
    pub out_of_play: Vec<(Value, &'static str)>,
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

/// Split valid schema-one observations at the same match boundaries as SNAP.
pub fn matches(
    observations: impl IntoIterator<Item = (usize, Value)>,
) -> Result<Vec<Vec<Frame>>, String> {
    let mut matches = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut watch = LocationWatch::default();
    for (line, observation) in observations {
        if !truthy(&observation) || observation["status"] == "no-active-game" {
            continue;
        }
        if observation["schema_version"] != 1 {
            return Err(format!("line {line}: expected SNAP schema_version 1"));
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
            || array(&game["locations"]).len() != 3
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
        }
        let ended = local["ended_turn"]
            .as_i64()
            .is_some_and(|ended| ended >= turn);
        let zones = [&local["zones"], &opponent["zones"]];
        let mut board = Vec::new();
        for (lane, location) in array(&game["locations"]).iter().enumerate() {
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
            locations: array(&game["locations"]).iter().map(revealed_id).collect(),
            location_turns: watch.update(&observation),
            totals: array(&game["locations"])
                .iter()
                .map(|location| location["total_power"].clone())
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
    pub moves: Vec<(Value, usize)>,
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
                moves.push((prior.entity.clone(), card.lane));
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
    fn rejects_invalid_schema_and_zero_seeds() {
        assert_eq!(
            matches(vec![(42, json!({"schema_version": 2}))]).unwrap_err(),
            "line 42: expected SNAP schema_version 1"
        );
        assert_eq!(
            check_observations(Vec::<(usize, Value)>::new(), &[], 0).unwrap_err(),
            "--seeds must be positive"
        );
    }
}
