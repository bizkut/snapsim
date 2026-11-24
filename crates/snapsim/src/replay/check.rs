use super::{History, ResultRow, entity, label, matches, report, staging, turns};
use crate::{cards, locations};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

fn theater_unknown(_reveal: Option<i64>, turn: i64, target_turn: i64, can_draw: bool) -> bool {
    target_turn > turn && can_draw
}

fn quantum_unknown(reveal: Option<i64>, played: Option<i64>, turn: i64) -> bool {
    played.is_none_or(|played| played < turn && reveal.is_none_or(|r| played >= r))
}

/// Largest number of tied move/play pairs whose relative order is enumerated
/// (2^3 = 8 orders).
const MAX_ORDER_TIES: usize = 3;

/// Number of orders to try: one bit per unresolved tied pair.
fn order_variants(ties: usize) -> Result<usize, String> {
    if ties > MAX_ORDER_TIES {
        return Err(format!(
            "{ties} simultaneous play/move order ties exceed the enumeration limit of {MAX_ORDER_TIES}"
        ));
    }
    Ok(1 << ties)
}

impl super::Staging<'_> {
    /// Only accept an exact, unambiguous match to the reconstructed local
    /// actions. A cleared, partial or activation-containing list is unknown.
    fn recorded_actions(&self) -> Option<Vec<(&str, &Value)>> {
        let record = &self.before.local_staged_order;
        if record["complete"] != true || record["turn"].as_i64()? != self.before.turn {
            return None;
        }
        let actions = record["actions"].as_array()?;
        let mut expected = Vec::new();
        for (entity, _, _) in &self.moves {
            if self
                .before
                .board
                .iter()
                .any(|card| card.entity == *entity && card.owner == 0)
            {
                expected.push(("move", entity));
            }
        }
        for item in &self.plays {
            if item.card.owner == 0 && item.turn == self.before.turn {
                expected.push(("play", &item.card.entity));
            }
        }
        if actions.len() != expected.len() {
            return None;
        }
        let mut result = Vec::with_capacity(actions.len());
        for action in actions {
            let kind = action["kind"].as_str()?;
            let entity = &action["card_entity_id"];
            let position = expected
                .iter()
                .position(|(k, id)| *k == kind && *id == entity)?;
            if result.contains(&(kind, entity)) {
                return None;
            }
            expected.remove(position);
            result.push((kind, entity));
        }
        Some(result)
    }
    /// Compare against current-turn resolution numbers. Missing, unchanged or
    /// repeated-entity evidence cannot corroborate a staged sequence.
    fn resolution_agrees(&self, start: &super::Frame, target: &super::Frame) -> Option<bool> {
        let actions = self.recorded_actions()?;
        if actions.len() < 2 {
            return None;
        }
        let mut seen = Vec::new();
        let mut orders = Vec::with_capacity(actions.len());
        for (_, id) in actions {
            if seen.contains(&id) {
                return None;
            }
            seen.push(id);
            let card = entity(target, id).filter(|card| card.owner == 0 && card.revealed)?;
            let order = card.latest_resolution_order.as_i64()?;
            let previous = entity(start, id)
                .and_then(|card| card.latest_resolution_order.as_i64())
                .or_else(|| {
                    start
                        .hand
                        .iter()
                        .find(|card| card["entity_id"] == *id)
                        .and_then(|card| card["latest_resolution_order"].as_i64())
                })?;
            if order <= previous {
                return None;
            }
            orders.push(order);
        }
        if orders.windows(2).any(|pair| pair[0] == pair[1]) {
            return None;
        }
        Some(orders.windows(2).all(|pair| pair[0] < pair[1]))
    }

    pub(super) fn recorded_order(&self, move_index: usize, play_index: usize) -> Option<bool> {
        if !self.recorded_order_verified {
            return None;
        }
        let (moved, _, line) = self.moves.get(move_index)?;
        let play = self.plays.get(play_index)?;
        if *line != play.stage_line || play.card.owner != 0 {
            return None;
        }
        let actions = self.before.local_staged_order["actions"].as_array()?;
        let move_rank = actions
            .iter()
            .position(|action| action["kind"] == "move" && action["card_entity_id"] == *moved)?;
        let play_rank = actions.iter().position(|action| {
            action["kind"] == "play" && action["card_entity_id"] == play.card.entity
        })?;
        Some(play_rank < move_rank)
    }

    pub(super) fn recorded_play_order(&self, a: usize, b: usize) -> Option<std::cmp::Ordering> {
        if !self.recorded_order_verified {
            return None;
        }
        let a = self.plays.get(a)?;
        let b = self.plays.get(b)?;
        if a.card.owner != 0 || b.card.owner != 0 || a.stage_line != b.stage_line {
            return None;
        }
        let actions = self.before.local_staged_order["actions"].as_array()?;
        let a = actions.iter().position(|action| {
            action["kind"] == "play" && action["card_entity_id"] == a.card.entity
        })?;
        let b = actions.iter().position(|action| {
            action["kind"] == "play" && action["card_entity_id"] == b.card.entity
        })?;
        Some(a.cmp(&b))
    }
}

fn io_error(path: &Path, error: std::io::Error) -> String {
    if let Some(code) = error.raw_os_error() {
        let message = error.to_string();
        let message = message.split(" (os error ").next().unwrap_or(&message);
        let name = path
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('\'', "\\'");
        format!("[Errno {code}] {message}: '{name}'")
    } else {
        error.to_string()
    }
}
fn observed_evidence(
    samples: &[&super::Frame],
    target: Option<&super::Frame>,
    plays: Option<&[super::Play]>,
) -> super::ObservedEvidence {
    let mut evidence = super::ObservedEvidence::default();
    for frame in samples.iter().copied().chain(target) {
        evidence.present_cards.extend(
            frame
                .board
                .iter()
                .filter(|card| card.revealed)
                .filter_map(|card| card.id.as_str().map(str::to_owned)),
        );
        evidence.present_locations.extend(
            frame
                .locations
                .iter()
                .filter_map(|id| id.as_str().map(str::to_owned)),
        );
    }
    if let Some(plays) = plays {
        for play in plays {
            if let Some(id) = play.id.as_str() {
                *evidence.played_cards.entry(id.to_owned()).or_default() += 1;
            }
        }
    }
    // Only a newly appended ledger entry on the same already-observed entity
    // can be assigned to this turn. Missing ledgers are unknown, not empty.
    if let (Some(before), Some(after)) =
        (samples.first(), target.or_else(|| samples.last().copied()))
    {
        for card in &after.board {
            let Some(previous) = entity(before, &card.entity) else {
                continue;
            };
            let (Some(old), Some(new)) = (
                previous.stat_updates.as_array(),
                card.stat_updates.as_array(),
            ) else {
                continue;
            };
            let mut remaining = old.clone();
            for update in new {
                if let Some(index) = remaining.iter().position(|item| item == update) {
                    remaining.remove(index);
                    continue;
                }
                if let Some(id) = update["updater_card_id"].as_str() {
                    evidence.updater_cards.insert(id.to_owned());
                }
                if let Some(id) = update["updater_location_id"].as_str() {
                    evidence.updater_locations.insert(id.to_owned());
                }
            }
        }
    }
    evidence
}

pub fn check_file(path: &Path, seeds: usize) -> Result<Vec<ResultRow>, String> {
    check_file_impl(path, seeds, false)
}

/// Check a recording while collecting hook executions for coverage reporting.
pub fn check_file_with_coverage(path: &Path, seeds: usize) -> Result<Vec<ResultRow>, String> {
    check_file_impl(path, seeds, true)
}

fn check_file_impl(
    path: &Path,
    seeds: usize,
    collect_hooks: bool,
) -> Result<Vec<ResultRow>, String> {
    if seeds == 0 {
        return Err("--seeds must be positive".into());
    }
    let deck_path = path.with_extension("deck.json");
    let deck: Vec<String> = if deck_path.exists() {
        let text = fs::read_to_string(&deck_path).map_err(|e| io_error(&deck_path, e))?;
        let data: Value =
            serde_json::from_str(&text).map_err(|e| super::json_error::decode_error(&text, &e))?;
        let decks = data["decks"].as_array().map_or(&[][..], Vec::as_slice);
        if decks.len() == 1 {
            decks[0]["card_ids"].as_array().map_or_else(Vec::new, |v| {
                v.iter()
                    .filter_map(|s| s.as_str().map(str::to_owned))
                    .collect()
            })
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let text = fs::read_to_string(path).map_err(|e| io_error(path, e))?;
    let mut observations = Vec::new();
    for (i, line) in text.split_inclusive('\n').enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let row: Value =
            serde_json::from_str(line).map_err(|e| super::json_error::decode_error(line, &e))?;
        if row.get("heartbeat").is_some() {
            continue;
        }
        let observation = if row.get("envelope").is_some() {
            row["envelope"]["observation"].clone()
        } else {
            row
        };
        observations.push((i + 1, observation));
    }
    check_observations_impl(observations, &deck, seeds, collect_hooks)
}

/// Check line-numbered SNAP observations without reading a recording file.
pub fn check_observations(
    observations: impl IntoIterator<Item = (usize, Value)>,
    deck: &[String],
    seeds: usize,
) -> Result<Vec<ResultRow>, String> {
    check_observations_impl(observations, deck, seeds, false)
}

fn check_observations_impl(
    observations: impl IntoIterator<Item = (usize, Value)>,
    deck: &[String],
    seeds: usize,
    collect_hooks: bool,
) -> Result<Vec<ResultRow>, String> {
    if seeds == 0 {
        return Err("--seeds must be positive".into());
    }
    let mut output = Vec::new();
    for (index, frames) in matches(observations)?.iter().enumerate() {
        let mut history = History::default();
        let groups = turns(frames);
        let mut seen_locations = vec![Value::Null; 3];
        let mut noted = Vec::new();
        for (group_index, samples) in groups.iter().enumerate() {
            let turn = samples[0].turn;
            let start = samples.iter().find(|f| f.ready);
            if let Some(start) = start {
                for (lane, location) in start.locations.iter().enumerate() {
                    if *location != seen_locations[lane] && *location == "WakandanEmbassy" {
                        history.embassy_reveals += 1;
                    }
                }
                seen_locations.clone_from(&start.locations);
            }
            let next = groups.get(group_index + 1).copied().unwrap_or(&[]);
            let mut target = if !next.is_empty() && next[0].turn == turn + 1 {
                next.iter().find(|f| f.ready)
            } else {
                None
            };
            if next.is_empty()
                && samples.last().is_some_and(|f| {
                    f.terminal && !f.conceded && f.total_turns.as_i64() == Some(turn)
                })
            {
                target = samples.last();
            }
            let mut checked = None;
            let mut order_disagrees = false;
            let mut used_samples: Vec<_> = samples.iter().collect();
            let result = (|| -> Result<ResultRow, String> {
                let start = start.ok_or("no settled planning frame before either side played")?;
                let target = target.ok_or_else(|| {
                    if next
                        .last()
                        .or_else(|| samples.last())
                        .is_some_and(|f| f.conceded)
                    {
                        "match conceded before resolution"
                    } else {
                        "no settled next-turn or completed final frame"
                    }
                })?;
                if !crate::cards::reference_rules_enabled() && turn > 1 {
                    match &start.previous_turn_locations {
                        None => {
                            return Err("previous-turn location identities are not recorded".into());
                        }
                        Some(previous)
                            if previous
                                .ids
                                .iter()
                                .flatten()
                                .any(|id| matches!(id.as_str(), "Feast" | "AltarOfDeath")) =>
                        {
                            return Err("Feast/Altar of Death prior-turn pending energy grants are not recorded".into());
                        }
                        Some(previous)
                            if !previous.complete
                                && start
                                    .locations
                                    .iter()
                                    .any(|id| id == "Feast" || id == "AltarOfDeath") =>
                        {
                            return Err("previous-turn location identities are incomplete".into());
                        }
                        _ => {}
                    }
                }
                used_samples.retain(|f| f.line >= start.line);
                if used_samples.iter().any(|frame| !frame.lane_layout_complete)
                    || !target.lane_layout_complete
                {
                    return Err(
                        "destroyed-lane layout cannot be established from location entity IDs"
                            .into(),
                    );
                }
                let mut missing_cards = BTreeSet::new();
                let mut missing_locations = BTreeSet::new();
                for f in used_samples.iter().copied().chain(std::iter::once(target)) {
                    for card in &f.board {
                        if card.revealed
                            && card.id.as_str().is_none_or(|id| cards::get(id).is_none())
                        {
                            missing_cards.insert(label(&card.id));
                        }
                    }
                    for location in &f.locations {
                        if let Some(id) = location.as_str()
                            && locations::get(id).is_none()
                        {
                            missing_locations.insert(id.to_owned());
                        }
                    }
                }
                let mut missing = Vec::new();
                if !missing_cards.is_empty() {
                    missing.push(format!(
                        "no card rules for {}",
                        missing_cards.into_iter().collect::<Vec<_>>().join(", ")
                    ));
                }
                if !missing_locations.is_empty() {
                    missing.push(format!(
                        "no location rules for {}",
                        missing_locations.into_iter().collect::<Vec<_>>().join(", ")
                    ));
                }
                if !missing.is_empty() {
                    return Err(missing.join("; "));
                }
                if target
                    .board
                    .iter()
                    .any(|card| card.revealed && card.power.is_null())
                {
                    return Err("target has revealed cards with no recorded power".into());
                }
                // FullLocationPowerAugmentBehavior::BuffPerFullLocation (0x7e59a60)
                // is an ongoing ability whose attachment is not exported. A changed
                // augment ledger cannot safely be reconstructed from its final value.
                if !crate::cards::reference_rules_enabled()
                    && target.board.iter().any(|card| {
                        let augment_updates = |card: &super::BoardCard| {
                            card.stat_updates
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter(|entry| !entry["updater_augment_id"].is_null())
                                .cloned()
                                .collect::<Vec<_>>()
                        };
                        let after = augment_updates(card);
                        let before = entity(start, &card.entity)
                            .map(augment_updates)
                            .unwrap_or_default();
                        (!after.is_empty() || !before.is_empty()) && before != after
                    })
                {
                    return Err("augment ability state is not recorded".into());
                }
                if !crate::cards::reference_rules_enabled()
                    && start.board.iter().any(|card| {
                        card.revealed
                            && card.id == "HumanTorchFirstSteps"
                            && card.flipped.as_i64().is_none_or(|t| t < turn)
                    })
                {
                    return Err("Human Torch First Steps doubling use is not recorded".into());
                }
                if !crate::cards::reference_rules_enabled()
                    && target.board.iter().any(|stone| {
                        stone.revealed
                            && stone.id == "PowerStone"
                            && start.complete_past_actions(stone.owner).is_none()
                            && target.board.iter().any(|thanos| {
                                thanos.revealed
                                    && thanos.owner == stone.owner
                                    && (thanos.id == "Thanos" || thanos.id == "ThanosChampion")
                            })
                    })
                {
                    return Err("Power Stone distinct played-stone history is not recorded".into());
                }
                for (lane, id) in start.locations.iter().enumerate() {
                    if !crate::cards::reference_rules_enabled()
                        && (id == "TheAbbey" || id == "Raft")
                        && start.location_turns[lane]
                            .as_i64()
                            .is_none_or(|revealed| revealed < turn)
                        && start
                            .location_history
                            .get(lane)
                            .and_then(Option::as_ref)
                            .and_then(|history| history.is_given)
                            .is_none()
                    {
                        return Err(format!("{id} consumed trigger state is not recorded"));
                    }
                    if id == "TimeTheater"
                        && theater_unknown(
                            start.location_turns[lane].as_i64(),
                            start.turn,
                            target.turn,
                            (0..2)
                                .any(|seat| start.deck_counts[seat].as_u64().is_none_or(|n| n > 0)),
                        )
                        && (crate::cards::reference_rules_enabled()
                            || start
                                .location_history
                                .get(lane)
                                .and_then(Option::as_ref)
                                .and_then(|history| history.time_theater_used.as_ref())
                                .and_then(|flags| flags.complete())
                                != Some([true, true]))
                    {
                        return Err("Time Theater per-player draw use is not recorded".into());
                    }
                    if id == "QuantumRealm"
                        && start.board.iter().any(|card| {
                            card.lane == lane
                                && card.revealed
                                && quantum_unknown(
                                    start.location_turns[lane].as_i64(),
                                    card.flipped.as_i64(),
                                    start.turn,
                                )
                        })
                    {
                        return Err("Quantum Realm card base power is not recorded".into());
                    }
                }
                // Player::HasSkippedTurn also checks staged moves and activations;
                // recordings do not expose a complete action history.
                if start.locations.iter().any(|id| id == "CollapsedMine")
                    && start
                        .board
                        .iter()
                        .any(|card| card.revealed && card.id == "Rock")
                {
                    return Err("Collapsed Mine skip actions are not fully recorded".into());
                }
                // The location's trigger consumption is not part of snapshots.
                if start
                    .locations
                    .iter()
                    .zip(&start.location_turns)
                    .enumerate()
                    .any(|(lane, (id, revealed))| {
                        id == "Vormir"
                            && revealed.as_i64().is_none_or(|t| t < turn)
                            && (crate::cards::reference_rules_enabled()
                                || start
                                    .location_history
                                    .get(lane)
                                    .and_then(Option::as_ref)
                                    .and_then(|history| history.vormir_consumed.as_ref())
                                    .and_then(|flags| flags.complete())
                                    .is_none())
                    })
                {
                    return Err("Vormir's per-player trigger state is not recorded".into());
                }
                for card in &start.board {
                    if card.revealed && card.flipped.as_i64().is_none_or(|t| t < turn) {
                        if card.id == "Nebula"
                            && (crate::cards::reference_rules_enabled()
                                || card
                                    .reference
                                    .as_ref()
                                    .and_then(|r| r.nebula_active())
                                    .is_none())
                        {
                            return Err("Nebula pre-end trigger state is not recorded".into());
                        }
                        if card.id == "TheThingFirstSteps"
                            && (crate::cards::reference_rules_enabled()
                                || card
                                    .reference
                                    .as_ref()
                                    .and_then(|r| r.thing_fill_used())
                                    .is_none())
                        {
                            return Err(
                                "The Thing First Steps fill-trigger use is not recorded".into()
                            );
                        }
                        if !crate::cards::reference_rules_enabled()
                            && card.id == "JuggernautHorseman"
                            && card
                                .reference
                                .as_ref()
                                .and_then(|r| r.objective())
                                .is_none()
                        {
                            return Err(
                                "Juggernaut Horseman objective state is not recorded".into()
                            );
                        }
                    }
                }
                // Target is a next-turn planning frame, after World Forge's
                // start-of-turn morph and any replacement's reveal effects.
                if target.turn != turn && start.locations.iter().any(|id| id == "WorldForge") {
                    return Err("World Forge changes the board at the next turn start".into());
                }
                for card in &start.board {
                    if !card.revealed {
                        let first = frames
                            .iter()
                            .find(|f| entity(f, &card.entity).is_some_and(|c| !c.revealed));
                        if first.is_none_or(|f| f.turn != turn - 1)
                            || !start.locations.iter().zip(&start.location_turns).any(
                                |(id, reveal)| id == "Lemuria" && reveal.as_i64() == Some(turn - 1),
                            )
                        {
                            return Err(
                                "first planning frame has face-down cards of uncertain timing"
                                    .into(),
                            );
                        }
                    } else if card.id == "Forge" && cards::reference_rules_enabled() {
                        return Err(
                            "pending Forge boost not observable at the start of the turn".into(),
                        );
                    }
                }
                if target.turn != turn
                    && target.board.iter().any(|card| {
                        !card.revealed
                            && (card.staged.as_i64() == Some(target.turn)
                                || !used_samples
                                    .iter()
                                    .any(|f| entity(f, &card.entity).is_some()))
                    })
                {
                    return Err("first settled next-turn frame already contains new plays".into());
                }
                let delays = start
                    .locations
                    .iter()
                    .zip(&start.location_turns)
                    .any(|(id, reveal)| id == "Lemuria" && reveal.as_i64() == Some(turn));
                let mut staged = staging(start, &used_samples, target, frames, delays)?;
                match staged.resolution_agrees(start, target) {
                    Some(true) => staged.recorded_order_verified = true,
                    Some(false) => order_disagrees = true,
                    None => {}
                }
                checked = Some((staged.plays.clone(), delays));
                // A pending Forge trigger is irrelevant if its owner cannot
                // play a card this turn. Local staging is recorded; opponent
                // staging is not, so prove the latter only in an inert board
                // with an empty hand and no cards or locations that can create,
                // remove, hide, or play another card between observations.
                let opponent_cannot_play = staged.plays.is_empty()
                    && staged.moves.is_empty()
                    && used_samples
                        .iter()
                        .copied()
                        .chain(std::iter::once(target))
                        .all(|frame| {
                            frame.hand_counts[1].as_i64() == Some(0)
                                && frame.locations.iter().all(|id| id == "Ruins")
                                && frame
                                    .board
                                    .iter()
                                    .all(|card| card.revealed && card.id == "Forge")
                                && frame.board.len() == start.board.len()
                                && frame
                                    .board
                                    .iter()
                                    .all(|card| entity(start, &card.entity).is_some())
                        });
                if start.board.iter().any(|card| {
                    card.revealed
                        && card.id == "Forge"
                        && if card.owner == 0 {
                            staged.plays.iter().any(|play| play.card.owner == 0)
                        } else {
                            !opponent_cannot_play
                        }
                }) {
                    return Err(
                        "pending Forge boost not observable at the start of the turn".into(),
                    );
                }
                // Spell01NicoMinoruBehaviorDef::AddNextCardTriggerToPlayer
                // (0x7f46cbc) installs a PostCardResolved trigger. The recorded
                // Nico identity does not expose the chosen spell or whether its
                // next-card trigger is still pending at this planning frame.
                if start.board.iter().any(|card| {
                    card.revealed
                        && card.id == "NicoMinoru"
                        && staged
                            .plays
                            .iter()
                            .any(|play| play.card.owner == card.owner)
                }) {
                    return Err("Nico Minoru's spell and pending next-card effect are not observable at the start of the turn".into());
                }
                // Spell01NicoMinoruBehaviorDef::AddNextCardTriggerToPlayer
                // (0x7f46cbc) also affects a later play in the same turn.
                // A newly revealed Nico does not expose her prepared spell.
                if !cards::reference_rules_enabled()
                    && staged.plays.iter().enumerate().any(|(index, play)| {
                        play.id == "NicoMinoru"
                            && staged.plays[index + 1..]
                                .iter()
                                .any(|later| later.card.owner == play.card.owner)
                    })
                {
                    return Err(
                        "Nico Minoru's spell for a later play this turn is not observable".into(),
                    );
                }
                // MaverickBehavior::AddBuffCardTriggerToPlayer (0x7ef54b8)
                // creates a player-owned trigger; observations do not expose
                // activation timing or whether that trigger remains pending.
                if start.board.iter().any(|card| {
                    card.revealed
                        && card.id == "Maverick"
                        && staged
                            .plays
                            .iter()
                            .any(|play| play.card.owner == card.owner)
                }) {
                    return Err("Maverick's pending next-card effect is not observable at the start of the turn".into());
                }
                // TaLoBehavior::PlayCards (0x7d87688) can alter a hidden
                // opponent hand card on a previous turn; its identity and
                // accumulated power are not recorded until it is played.
                if staged.plays.iter().any(|play| play.card.owner == 1)
                    && frames.iter().any(|frame| {
                        frame.turn < turn
                            && frame.locations.iter().zip(&frame.location_turns).any(
                                |(id, revealed)| {
                                    id == "TaLo"
                                        && revealed.as_i64().is_some_and(|t| t <= frame.turn)
                                },
                            )
                            && frame.hand_counts[1].as_i64().is_none_or(|count| count > 0)
                    })
                {
                    return Err("Ta Lo may have buffed an unobserved opponent hand card".into());
                }
                let variants = if cards::reference_rules_enabled() {
                    1
                } else {
                    let ties = staged
                        .moves
                        .iter()
                        .enumerate()
                        .map(|(move_index, (_, _, line))| {
                            staged
                                .plays
                                .iter()
                                .enumerate()
                                .filter(|(play_index, play)| {
                                    play.stage_line == *line
                                        && staged.recorded_order(move_index, *play_index).is_none()
                                })
                                .count()
                        })
                        .sum();
                    order_variants(ties)?
                };
                let mut best = None;
                let mut last_error = None;
                for seed in 0..seeds {
                    for order in 0..variants {
                        let (diffs, hooks) = match super::simulate::simulate(
                            start,
                            target,
                            &staged,
                            frames,
                            &history,
                            deck,
                            (seed, order, collect_hooks),
                        ) {
                            Ok(result) => result,
                            Err(error) => {
                                last_error = Some(error);
                                continue;
                            }
                        };
                        if diffs.is_empty() {
                            let mut row = report::consistent(
                                index + 1,
                                turn,
                                staged.before.line,
                                target.line,
                                seed,
                                seeds,
                            );
                            row.hooks = hooks;
                            if variants > 1 {
                                row.order_enumerated = true;
                                row.detail.push_str(&format!(
                                    "; play/move order unknown, matched 1 of {variants} orders (mask {order:#b})"
                                ));
                            }
                            return Ok(row);
                        }
                        if best.as_ref().is_none_or(
                            |(_, previous, _): &(
                                usize,
                                Vec<super::Difference>,
                                Option<crate::game::ReplayHooks>,
                            )| { diffs.len() < previous.len() },
                        ) {
                            best = Some((seed, diffs, hooks));
                        }
                    }
                }
                if let Some(error) = last_error {
                    return Err(error);
                }
                let (seed, differences, hooks) = best.expect("positive seeds");
                // With schema-v1 observations an opponent's first revealed
                // power may include hidden hand buffs or effects already
                // resolved in the same sample. If trials disagree, a changed
                // value without a ledger cannot establish pre-play power.
                if !cards::reference_rules_enabled() {
                    let catalog = crate::data::cards();
                    for play in staged.plays.iter().filter(|play| play.card.owner == 1) {
                        let Some(visible) = frames.iter().find_map(|frame| {
                            entity(frame, &play.card.entity).filter(|card| card.revealed)
                        }) else {
                            continue;
                        };
                        if visible.stat_updates.as_array().is_some() {
                            continue;
                        }
                        let Some(id) = play.id.as_str() else { continue };
                        let Some(row) = catalog.iter().find(|row| row.carddefid == id) else {
                            continue;
                        };
                        if visible
                            .power
                            .as_i64()
                            .is_some_and(|power| power != i64::from(row.power))
                        {
                            return Err(format!(
                                "opponent pre-play Power for {id} [{}] cannot be established without a stat ledger",
                                label(&play.card.entity)
                            ));
                        }
                    }
                }
                let mut cards: BTreeSet<String> = differences
                    .iter()
                    .flat_map(|d| d.cards.iter().cloned())
                    .collect();
                cards.extend(
                    staged
                        .plays
                        .iter()
                        .filter_map(|p| p.id.as_str().map(str::to_owned)),
                );
                let mut lanes: BTreeSet<usize> = differences
                    .iter()
                    .flat_map(|d| d.lanes.iter().copied())
                    .collect();
                lanes.extend(staged.plays.iter().map(|p| p.card.lane));
                let locations: BTreeSet<String> = [start, target]
                    .into_iter()
                    .flat_map(|f| {
                        lanes
                            .iter()
                            .filter_map(|&lane| f.locations[lane].as_str().map(str::to_owned))
                    })
                    .collect();
                let mut row = report::inconsistent(
                    index + 1,
                    turn,
                    (staged.before.line, target.line),
                    (seeds, seed),
                    differences,
                    cards.into_iter().collect(),
                    locations.into_iter().collect(),
                );
                row.hooks = hooks;
                Ok(row)
            })();
            let mut row = result.unwrap_or_else(|error| {
                report::uncheckable(
                    index + 1,
                    turn,
                    used_samples[0].line,
                    used_samples[used_samples.len() - 1].line,
                    &error,
                )
            });
            if order_disagrees {
                row.detail.push_str("; local staged order disagrees with observed resolution order; enumerated instead");
            }
            if collect_hooks {
                let recovered = if checked.is_none() {
                    start.zip(target).and_then(|(start, target)| {
                        let delays =
                            start.locations.iter().zip(&start.location_turns).any(
                                |(id, reveal)| id == "Lemuria" && reveal.as_i64() == Some(turn),
                            );
                        staging(start, &used_samples, target, frames, delays)
                            .ok()
                            .map(|s| s.plays)
                    })
                } else {
                    None
                };
                let plays = checked
                    .as_ref()
                    .map(|(plays, _)| plays.as_slice())
                    .or(recovered.as_deref());
                row.observed = Some(observed_evidence(&used_samples, target, plays));
            }
            output.push(row);
            if let Some(start) = start {
                if let Some((plays, delays)) = &checked {
                    if !delays {
                        for play in plays {
                            let lane = play.card.lane;
                            if start.locations[lane] == "OttosLab" {
                                history.lab_spent.push((
                                    lane,
                                    start.location_turns[lane].clone(),
                                    play.card.owner,
                                ));
                            }
                        }
                    }
                } else {
                    for (lane, id) in start.locations.iter().enumerate() {
                        if *id == "OttosLab" {
                            history
                                .lab_unknown
                                .push((lane, start.location_turns[lane].clone()));
                        }
                    }
                }
            }
            for frame in used_samples {
                for card in &frame.board {
                    if !card.revealed || card.id.as_str().is_none() {
                        continue;
                    }
                    let id = card.id.as_str().unwrap().to_owned();
                    if !history.seen[card.owner].contains(&id) {
                        history.seen[card.owner].push(id.clone());
                    }
                    if !noted.contains(&card.entity) && card.owner == 1 && id == "AmericaChavez" {
                        history.chavez_reveals += 1;
                    }
                    if !noted.contains(&card.entity) {
                        noted.push(card.entity.clone());
                    }
                }
            }
        }
    }
    Ok(output)
}
#[cfg(test)]
mod location_state_tests {
    use super::{order_variants, quantum_unknown, theater_unknown};

    #[test]
    fn stat_update_evidence_requires_a_new_entry_on_the_same_visible_entity() {
        use serde_json::json;
        let observation = |turn, updates| {
            json!({
                "schema_version": 2,
                "game": {
                    "turn": turn,
                    "game_key": "one",
                    "players": [
                        {"slot": "local", "entity_id": 1},
                        {"slot": "opponent", "entity_id": 2}
                    ],
                    "locations": [
                        {"location_id": "Ruins", "cards": [{
                            "entity_id": 9, "owner_entity_id": 1, "card_id": "Hawkeye",
                            "revealed": true, "stat_updates": updates
                        }]},
                        {"location_id": "Ruins", "cards": []},
                        {"location_id": "Ruins", "cards": []}
                    ]
                }
            })
        };
        let update = json!({"updater_card_id": "Okoye", "value": 1});
        let frames = super::matches(vec![
            (1, observation(1, json!([]))),
            (2, observation(2, json!([update]))),
        ])
        .unwrap();
        let evidence = super::observed_evidence(&[&frames[0][0]], Some(&frames[0][1]), None);
        assert!(evidence.updater_cards.contains("Okoye"));
        let frames = super::matches(vec![
            (1, observation(1, serde_json::Value::Null)),
            (
                2,
                observation(2, json!([{"updater_card_id": "Okoye", "value": 1}])),
            ),
        ])
        .unwrap();
        let evidence = super::observed_evidence(&[&frames[0][0]], Some(&frames[0][1]), None);
        assert!(evidence.updater_cards.is_empty());
    }

    #[test]
    fn replay_enumerates_one_ambiguous_move_and_play() {
        assert_eq!(order_variants(0), Ok(1));
        assert_eq!(order_variants(1), Ok(2));
        assert_eq!(order_variants(2), Ok(4));
        assert_eq!(order_variants(3), Ok(8));
        let error = order_variants(4).unwrap_err();
        assert!(
            error.contains("4 simultaneous") && error.contains("limit of 3"),
            "{error}"
        );
    }

    #[test]
    fn local_order_requires_complete_unambiguous_staging() {
        use serde_json::{Value, json};
        let observation = json!({
            "schema_version": 2,
            "game": {
                "turn": 2, "game_key": "one",
                "players": [
                    {"slot": "local", "entity_id": 1},
                    {"slot": "opponent", "entity_id": 2}
                ],
                "locations": [
                    {"location_id": "Ruins", "cards": [
                        {"entity_id": 9, "owner_entity_id": 1, "card_id": "Nightcrawler", "revealed": true},
                        {"entity_id": 10, "owner_entity_id": 1, "card_id": "Wasp", "revealed": false},
                        {"entity_id": 11, "owner_entity_id": 2, "card_id": "Hulk", "revealed": true}
                    ]},
                    {"location_id": "Ruins", "cards": []},
                    {"location_id": "Ruins", "cards": []}
                ]
            }
        });
        let frames = super::matches(vec![(7, observation)]).unwrap();
        let mut frame = frames[0][0].clone();
        let play = |card: &super::super::BoardCard| super::super::Play {
            card: card.clone(),
            id: card.id.clone(),
            cost: Value::Null,
            power: Value::Null,
            turn: 2,
            stage_line: 7,
            flip_line: 8,
        };
        let plays = vec![play(&frame.board[1]), play(&frame.board[2])];
        let valid = json!({"turn":2,"complete":true,"actions":[
            {"kind":"play","card_entity_id":10},{"kind":"move","card_entity_id":9}
        ]});
        for (record, expected) in [
            (valid.clone(), Some(true)),
            (
                json!({"turn":2,"complete":true,"actions":[
                    {"kind":"move","card_entity_id":9},{"kind":"play","card_entity_id":10}
                ]}),
                Some(false),
            ),
            (Value::Null, None),
            (
                json!({"turn":2,"complete":false,"actions":valid["actions"]}),
                None,
            ),
            (
                json!({"turn":1,"complete":true,"actions":valid["actions"]}),
                None,
            ),
            (
                json!({"turn":2,"complete":true,"actions":[{"kind":"play","card_entity_id":10}]}),
                None,
            ),
            (
                json!({"turn":2,"complete":true,"actions":[
                    {"kind":"play","card_entity_id":10},{"kind":"play","card_entity_id":10}
                ]}),
                None,
            ),
            (
                json!({"turn":2,"complete":true,"actions":[
                    {"kind":"play","card_entity_id":10},{"kind":"move","card_entity_id":9},
                    {"kind":"activate","card_entity_id":9}
                ]}),
                None,
            ),
        ] {
            frame.local_staged_order = record;
            let mut staged = super::super::Staging {
                plays: plays.clone(),
                moves: vec![(json!(9), 0, 7)],
                before: &frame,
                opponent_count: 0,
                recorded_order_verified: false,
            };
            staged.recorded_order_verified = staged.recorded_actions().is_some();
            assert_eq!(staged.recorded_order(0, 0), expected);
            assert_eq!(staged.recorded_order(0, 1), None);
        }
        frame.local_staged_order = json!({"turn":2,"complete":true,"actions":[
            {"kind":"play","card_entity_id":10},{"kind":"move","card_entity_id":9}
        ]});
        let mut start = frame.clone();
        start.board[0].latest_resolution_order = json!(1);
        start.board[1].latest_resolution_order = json!(0);
        let mut target = frame.clone();
        target.board[0].latest_resolution_order = json!(5);
        target.board[1].latest_resolution_order = json!(4);
        target.board[1].revealed = true;
        let staged = super::super::Staging {
            plays: plays.clone(),
            moves: vec![(json!(9), 0, 7)],
            before: &frame,
            opponent_count: 0,
            recorded_order_verified: false,
        };
        assert_eq!(staged.resolution_agrees(&start, &target), Some(true));
        assert_eq!(staged.recorded_order(0, 0), None);
        target.board[0].latest_resolution_order = json!(2);
        assert_eq!(staged.resolution_agrees(&start, &target), Some(false));
        target.board[0].latest_resolution_order = json!(1);
        assert_eq!(staged.resolution_agrees(&start, &target), None);
        frame.local_staged_order = json!({"turn":2,"complete":true,"actions":[
            {"kind":"play","card_entity_id":12},{"kind":"move","card_entity_id":9},
            {"kind":"play","card_entity_id":10}
        ]});
        let mut second = plays[0].clone();
        second.card.entity = json!(12);
        let staged = super::super::Staging {
            plays: vec![plays[0].clone(), second, plays[1].clone()],
            moves: vec![(json!(9), 0, 7)],
            before: &frame,
            opponent_count: 0,
            recorded_order_verified: true,
        };
        assert_eq!(
            staged.recorded_play_order(0, 1),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(staged.recorded_play_order(0, 2), None);
        assert_eq!(staged.recorded_order(0, 0), Some(false));
        assert_eq!(staged.recorded_order(0, 1), Some(true));
    }

    #[test]
    fn theater_old_reveal_needs_unknown_draw_context() {
        assert!(theater_unknown(Some(2), 3, 4, true));
        assert!(theater_unknown(Some(3), 3, 4, true));
        assert!(!theater_unknown(Some(2), 3, 3, true));
        assert!(!theater_unknown(Some(2), 3, 4, false));
    }

    #[test]
    fn quantum_old_play_needs_unknown_base() {
        assert!(quantum_unknown(Some(2), Some(2), 3));
        assert!(quantum_unknown(Some(2), None, 3));
        assert!(!quantum_unknown(Some(2), Some(1), 3));
        assert!(!quantum_unknown(Some(2), Some(3), 3));
    }
}
