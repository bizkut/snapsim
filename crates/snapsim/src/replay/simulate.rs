use super::{Difference, Frame, History, Staging, entity, label};
use crate::{
    game::{Game, Lane, OngoingSource, Player, Stat, StatOperation},
    locations, position,
    random::PythonRandom,
    zones::CardZone,
};
use serde_json::Value;
use std::{cell::RefCell, collections::BTreeSet};

fn sample(pool: &[String], count: usize, rng: &mut PythonRandom) -> Vec<String> {
    if count <= pool.len() {
        rng.sample_indices(pool.len(), count)
            .into_iter()
            .map(|i| pool[i].clone())
            .collect()
    } else {
        (0..count)
            .map(|_| pool[(rng.random() * pool.len() as f64) as usize].clone())
            .collect()
    }
}
fn number(value: &Value) -> Option<i32> {
    value.as_i64().and_then(|n| i32::try_from(n).ok())
}

/// Recover only modifications proven to predate this turn; the first visible
/// ledger may also contain effects produced by the current resolution.
fn hidden_preplay_power(
    play: &super::Play,
    staged: &Staging<'_>,
    frames: &[Frame],
    initial: i32,
) -> Result<Option<i32>, String> {
    // The frozen reference samples hidden pre-play power; parity gates keep that.
    if crate::cards::reference_rules_enabled() {
        return Ok(None);
    }
    let Some(visible) = frames
        .iter()
        .find_map(|frame| entity(frame, &play.card.entity).filter(|card| card.revealed))
    else {
        return Ok(None);
    };
    let Some(updates) = visible.stat_updates.as_array() else {
        return Ok(None);
    };
    let prior = |update: &Value| {
        let source = update["updater_card_id"].as_str();
        // PyramidOfRamaTutBehavior::AddRock (0x7d75c10) creates this
        // particular Rock in hand and sets its power before it can be staged.
        // Its creation provenance, not merely the updater name, is required.
        if play.id == "Rock"
            && visible.created_by_location == "PyramidOfRamaTut"
            && update["updater_location_id"] == "PyramidOfRamaTut"
            && update["update"] == "Set"
            && update["value"] == 3
        {
            return true;
        }
        matches!(
            source,
            Some("Surge" | "AmericaChavez" | "MrFantasticFirstSteps")
        ) && staged.before.board.iter().any(|card| {
            card.owner == play.card.owner && card.revealed && card.id == update["updater_card_id"]
        }) && (source == Some("MrFantasticFirstSteps")
            || !staged.plays.iter().any(|other| {
                if other.card.owner != play.card.owner || other.flip_line >= play.flip_line {
                    return false;
                }
                if other.id == "Odin" {
                    return true;
                }
                source == Some("Surge")
                    && frames
                        .iter()
                        .find_map(|frame| {
                            entity(frame, &other.card.entity).filter(|card| card.revealed)
                        })
                        .and_then(|card| card.stat_updates.as_array())
                        .is_none_or(|ledger| {
                            ledger
                                .iter()
                                .any(|entry| entry["updater_card_id"] == "Surge")
                        })
            }))
    };
    let mut power = initial;
    for (index, update) in updates.iter().enumerate() {
        if update["stat"] != "Power" {
            continue;
        }
        let current = update["updater_card_id"].as_str().is_some_and(|source| {
            staged.plays.iter().any(|other| {
                other.card.owner == play.card.owner
                    && other.id == source
                    && other.flip_line <= play.flip_line
            })
        });
        if current {
            continue;
        }
        let anonymous_set = update["update"] == "Set"
            && update["updater_card_id"].is_null()
            && update["updater_location_id"].is_null();
        // A later proven pre-turn update orders an otherwise anonymous Set.
        let before_turn = prior(update) || anonymous_set && updates[index + 1..].iter().any(prior);
        if !before_turn {
            return Err(format!(
                "pre-play Power update for {} [{}] cannot be timed from the recorded ledger",
                label(&play.id),
                label(&play.card.entity)
            ));
        }
        let value = number(&update["value"]).ok_or_else(|| {
            format!(
                "invalid pre-play Power update for {} [{}]",
                label(&play.id),
                label(&play.card.entity)
            )
        })?;
        match update["update"].as_str() {
            Some("Add") => power += value,
            Some("Set") => power = value,
            _ => {
                return Err(format!(
                    "unsupported pre-play Power operation for {} [{}]",
                    label(&play.id),
                    label(&play.card.entity)
                ));
            }
        }
    }
    Ok(Some(power))
}
fn make(id: &str, owner: usize, cost: &Value, power: &Value) -> Result<crate::game::Card, String> {
    if crate::cards::get(id).is_none() {
        return Err(format!("no card rules for {id}"));
    }
    let mut card = position::make_card(id, owner).map_err(|_| format!("no card rules for {id}"))?;
    if let Some(cost) = number(cost) {
        card.cost = cost;
    }
    if let Some(power) = number(power) {
        card.permanent = power;
    }
    Ok(card)
}
fn push(
    game: &mut Game<'_>,
    id: &str,
    owner: usize,
    cost: &Value,
    power: &Value,
) -> Result<usize, String> {
    let index = game.cards.len();
    game.cards.push(make(id, owner, cost, power)?);
    Ok(index)
}

/// One recorded or simulated stat-update row: target card, stat, operation, value.
type UpdateRow = (usize, &'static str, &'static str, i64);

/// Ongoing rows a card's current text produces, from the simulated ledger.
fn ongoing_rows(game: &Game<'_>, source: usize) -> Vec<UpdateRow> {
    let mut rows = Vec::new();
    for (target, card) in game.cards.iter().enumerate() {
        for update in &card.stat_updates {
            if update.ongoing
                && matches!(
                    update.ongoing_source,
                    Some(OngoingSource::Card { card, .. }) if card == source
                )
            {
                rows.push((
                    target,
                    match update.stat {
                        Stat::Power => "Power",
                        Stat::Cost => "Cost",
                    },
                    match update.operation {
                        StatOperation::Add => "Add",
                        StatOperation::Multiply => "Multiply",
                        StatOperation::Set => "Set",
                    },
                    i64::from(update.value),
                ));
            }
        }
    }
    rows.sort_unstable();
    rows
}

/// MystiqueBehavior::CopyAbilitiesAndPlayRequirements (0x7f085a0) installs the
/// last played Ongoing card's text through Card::CopyText (0x7cbadb4). The
/// reader exports no copied-ability identity: the holder's abilities inventory
/// is empty and the copy is visible only through Ongoing rows whose updater is
/// the holder's own definition. Restore it only when exactly one allied Ongoing
/// definition reproduces every recorded Mystique-updated row; otherwise the
/// text is unknown and the turn cannot be checked.
fn restore_copied_ongoing(
    game: &mut Game<'_>,
    start: &Frame,
    target: &Frame,
    cards: &[(Value, usize)],
) -> Result<(), String> {
    let mut observed: Vec<UpdateRow> = Vec::new();
    for (entity_id, index) in cards {
        let Some(card) = entity(start, entity_id) else {
            continue;
        };
        for entry in card.stat_updates.as_array().into_iter().flatten() {
            if entry["updater_card_id"] != "Mystique" {
                continue;
            }
            let stat = match entry["stat"].as_str() {
                Some("Power") => "Power",
                Some("Cost") => "Cost",
                _ => return Err("Mystique stat update has an unknown stat".into()),
            };
            let operation = match entry["update"].as_str() {
                Some("Add") => "Add",
                Some("Multiply") => "Multiply",
                Some("Set") => "Set",
                _ => return Err("Mystique stat update has an unknown operation".into()),
            };
            let value = entry["value"]
                .as_i64()
                .ok_or("Mystique stat update has no value")?;
            observed.push((*index, stat, operation, value));
        }
    }
    let mut holders = cards
        .iter()
        .filter(|(_, index)| game.cards[*index].def.id == "Mystique");
    let Some((holder_entity, holder)) = holders.next() else {
        return if observed.is_empty() {
            Ok(())
        } else {
            Err("Mystique's recorded Ongoing updates have no visible holder".into())
        };
    };
    if holders.next().is_some() {
        return Err("Mystique's copied Ongoing text has multiple possible holders".into());
    }
    let holder = *holder;
    let actions = start
        .complete_past_actions(game.cards[holder].owner)
        .ok_or("Mystique copied Ongoing text requires complete owner play history")?;
    let holder_position = actions
        .iter()
        .position(|action| {
            action.kind == "play" && Some(action.card_entity_id) == holder_entity.as_i64()
        })
        .ok_or("Mystique copied Ongoing text requires its recorded play")?;
    let prior = actions[..holder_position]
        .iter()
        .rev()
        .find(|action| action.kind == "play")
        .ok_or("Mystique copied Ongoing text has no recorded previous play")?;
    let source_from_history = cards
        .iter()
        .find(|(id, _)| id.as_i64() == Some(prior.card_entity_id))
        .map(|(_, index)| *index)
        .ok_or("Mystique copied Ongoing source is no longer identifiable on the board")?;
    if observed.is_empty() {
        if start.locations != target.locations {
            return Err("Mystique copied Ongoing text is not identifiable at the target".into());
        }
        let mut rng = game.fork_rng();
        let mut snapshot = game.fork(rng.as_mut());
        for lane in &mut snapshot.lanes {
            lane.cards.clear();
        }
        for &(_, index) in cards {
            snapshot.cards[index].lane = None;
            snapshot.cards[index].zone = None;
        }
        let mut target_cards = Vec::new();
        for card in &target.board {
            if !card.revealed {
                return Err("Mystique target contains an unrevealed card".into());
            }
            let id = card
                .id
                .as_str()
                .ok_or("Mystique target card identity is missing")?;
            let index = if let Some((_, index)) = cards.iter().find(|(id, _)| *id == card.entity) {
                if snapshot.cards[*index].def.id != id || snapshot.cards[*index].owner != card.owner
                {
                    return Err("Mystique target card changed identity or owner".into());
                }
                *index
            } else {
                push(&mut snapshot, id, card.owner, &card.cost, &Value::Null)?
            };
            snapshot.cards[index].lane = Some(card.lane);
            snapshot.cards[index].zone = Some(CardZone::Location(card.lane));
            snapshot.cards[index].revealed = true;
            snapshot.push_location(card.lane, index);
            target_cards.push((&card.entity, index));
        }
        let mut target_rows = Vec::new();
        for (entity_id, index) in &target_cards {
            let card = entity(target, entity_id).unwrap();
            let updates = card
                .stat_updates
                .as_array()
                .ok_or("Mystique target stat-update ledger is missing")?;
            for entry in updates {
                if entry["updater_card_id"] != "Mystique" {
                    continue;
                }
                let stat = match entry["stat"].as_str() {
                    Some("Power") => "Power",
                    Some("Cost") => "Cost",
                    _ => return Err("Mystique target stat update has an unknown stat".into()),
                };
                let operation = match entry["update"].as_str() {
                    Some("Add") => "Add",
                    Some("Multiply") => "Multiply",
                    Some("Set") => "Set",
                    _ => return Err("Mystique target stat update has an unknown operation".into()),
                };
                target_rows.push((
                    *index,
                    stat,
                    operation,
                    entry["value"]
                        .as_i64()
                        .ok_or("Mystique target stat update has no value")?,
                ));
            }
        }
        if target_rows.is_empty() {
            return Err("Mystique copied Ongoing text is not identified by target updates".into());
        }
        target_rows.sort_unstable();
        let mut tried = Vec::new();
        let mut matched = Vec::new();
        for &(_, source) in cards {
            let card = &game.cards[source];
            if source != source_from_history
                || card.owner != game.cards[holder].owner
                || !card.revealed
                || !card.has_ongoing(false)
                || (card.turn_played != 0
                    && game.cards[holder].turn_played != 0
                    && card.turn_played > game.cards[holder].turn_played)
                || snapshot.cards[source].lane.is_none()
            {
                continue;
            }
            let def = card.default_ability().id;
            if tried.contains(&def) {
                continue;
            }
            tried.push(def);
            let mut rng = snapshot.fork_rng();
            let mut trial = snapshot.fork(rng.as_mut());
            trial.is_wait_for_staging = false;
            if trial.copy_text(holder, source, false) && ongoing_rows(&trial, holder) == target_rows
            {
                matched.push(source);
            }
        }
        return match matched[..] {
            [source] => {
                game.copy_text(holder, source, false);
                Ok(())
            }
            _ => Err(
                "Mystique copied Ongoing text is not uniquely identified by target updates".into(),
            ),
        };
    }
    observed.sort_unstable();
    let owner = game.cards[holder].owner;
    let played = game.cards[holder].turn_played;
    let mut tried: Vec<&str> = Vec::new();
    let mut matched: Vec<usize> = Vec::new();
    for &(_, source) in cards {
        let card = &game.cards[source];
        if source != source_from_history
            || card.owner != owner
            || !card.revealed
            || card.lane.is_none()
            || !card.has_ongoing(false)
            || (card.turn_played != 0 && played != 0 && card.turn_played > played)
        {
            continue;
        }
        let def = card.default_ability().id;
        if tried.contains(&def) {
            continue;
        }
        tried.push(def);
        let mut rng = game.fork_rng();
        let mut trial = game.fork(rng.as_mut());
        trial.is_wait_for_staging = false;
        if trial.copy_text(holder, source, false) && ongoing_rows(&trial, holder) == observed {
            matched.push(source);
        }
    }
    match matched[..] {
        [source] => {
            game.copy_text(holder, source, false);
            Ok(())
        }
        [] => Err(format!(
            "Mystique [{}] copied Ongoing text is not identified by its recorded updates",
            label(&cards.iter().find(|(_, i)| *i == holder).unwrap().0)
        )),
        _ => Err(format!(
            "Mystique [{}] copied Ongoing text is ambiguous",
            label(&cards.iter().find(|(_, i)| *i == holder).unwrap().0)
        )),
    }
}
fn diff(item: String, cards: Vec<String>, lanes: Vec<usize>) -> Difference {
    Difference { item, cards, lanes }
}

type StagingOrderItem = (usize, usize, usize, usize, i64, usize);

fn sort_staging_edges(
    moves: usize,
    count: usize,
    edges: &[(usize, usize)],
    order: &mut Vec<StagingOrderItem>,
) -> Result<(), String> {
    let mut emitted = vec![false; count];
    order.clear();
    for rank in 0..count {
        let node = (0..count)
            .find(|&node| {
                !emitted[node] && edges.iter().all(|&(from, to)| to != node || emitted[from])
            })
            .ok_or("recorded staging order conflicts with this order variant")?;
        emitted[node] = true;
        let (kind, index) = if node < moves {
            (0, node)
        } else {
            (1, node - moves)
        };
        order.push((rank, 0, kind, 0, 0, index));
    }
    Ok(())
}

pub(super) fn simulate(
    start: &Frame,
    target: &Frame,
    staged: &Staging<'_>,
    frames: &[Frame],
    history: &History,
    local_deck: &[String],
    trial: (usize, usize, bool),
) -> Result<(Vec<Difference>, Option<crate::game::ReplayHooks>), String> {
    let (seed, order_mask, collect_hooks) = trial;
    let mut hidden_rng = PythonRandom::seed(seed as i128);
    let mut game_rng = PythonRandom::seed(seed as i128);
    let mut known = history.seen.clone();
    for card in &start.board {
        if card.revealed
            && let Some(id) = card.id.as_str()
            && !known[card.owner].iter().any(|s| s == id)
        {
            known[card.owner].push(id.to_owned());
        }
    }
    for play in &staged.plays {
        if let Some(id) = play.id.as_str()
            && !known[play.card.owner].iter().any(|s| s == id)
        {
            known[play.card.owner].push(id.to_owned());
        }
    }
    let pool = if start.recruit && !position::outside(known[1].iter()) {
        position::recruit_cards()
    } else {
        position::all_cards()
    };
    let mut others: Vec<_> = pool
        .iter()
        .filter(|id| !known[1].contains(id))
        .cloned()
        .collect();
    if others.is_empty() {
        others = pool.clone();
    }
    let remaining: Vec<_> = local_deck
        .iter()
        .filter(|id| !known[0].contains(id) && !start.hand.iter().any(|c| c["card_id"] == **id))
        .cloned()
        .collect();
    let my_count = start.deck_counts[0].as_u64().unwrap_or(0) as usize;
    let my_deck = sample(
        if remaining.len() == my_count {
            &remaining
        } else {
            &pool
        },
        my_count,
        &mut hidden_rng,
    );
    let opp_hand = start.hand_counts[1].as_u64().unwrap_or(0) as usize;
    let hidden = sample(
        &others,
        opp_hand + start.deck_counts[1].as_u64().unwrap_or(0) as usize,
        &mut hidden_rng,
    );
    let lanes: [Lane; 3] = std::array::from_fn(|i| {
        let id = start.locations[i].as_str();
        let def = id
            .and_then(locations::get)
            .unwrap_or(&position::UNKNOWN_LOCATION);
        let revealed_turn = start.location_turns[i].as_u64().map(|v| v as u8);
        let state = if id == Some("OttosLab") {
            Some(Box::new(std::array::from_fn::<_, 2, _>(|seat| {
                history
                    .lab_spent
                    .contains(&(i, start.location_turns[i].clone(), seat))
            })) as Box<dyn crate::game::CardState>)
        } else if !crate::cards::reference_rules_enabled() {
            start
                .location_history
                .get(i)
                .and_then(Option::as_ref)
                .and_then(|history| match id {
                    Some("Vormir") => history
                        .vormir_consumed
                        .as_ref()?
                        .complete()
                        .map(|spent| Box::new(spent) as Box<dyn crate::game::CardState>),
                    Some("TimeTheater") => history
                        .time_theater_used
                        .as_ref()?
                        .complete()
                        .map(|spent| Box::new(spent) as Box<dyn crate::game::CardState>),
                    Some("TheAbbey" | "Raft") => history
                        .is_given
                        .map(|spent| Box::new(spent) as Box<dyn crate::game::CardState>),
                    _ => None,
                })
                .or_else(|| def.initial_state.map(|init| init()))
        } else {
            def.initial_state.map(|init| init())
        };
        Lane {
            def,
            observed_id: None,
            revealed: id.is_some(),
            destroyed: start.destroyed_lanes[i],
            ability_flags: crate::game::LocationAbilityFlags::default(),
            copied_reveal_turn: None,
            revealed_turn,
            cards: Vec::new(),
            bonus: [0; 2],
            multiplier: [1; 2],
            contributions: Vec::new(),
            state,
        }
    });
    for i in 0..3 {
        if let Some(id) = start.locations[i].as_str() {
            if start.location_turns[i].is_null() && matches!(id, "Lemuria" | "Sakaar") {
                return Err(format!("reveal turn of {id} is unknown"));
            }
            if id == "OttosLab"
                && (start.location_turns[i].is_null()
                    || history
                        .lab_unknown
                        .contains(&(i, start.location_turns[i].clone())))
            {
                return Err("Otto's Lab spent state is unknown".into());
            }
        }
    }
    let mut order: Vec<_> = (0..3).filter(|&i| lanes[i].revealed).collect();
    let current_location = if start.destroyed_lanes.iter().any(|&destroyed| destroyed) {
        usize::try_from(start.turn).unwrap_or(0).min(3)
    } else {
        order.len()
    };
    order.extend((0..3).filter(|&i| !lanes[i].revealed));
    let starting_location_ids: Vec<&'static str> = lanes.iter().map(|lane| lane.def.id).collect();
    let mut game = Game {
        cards: Vec::new(),
        ability_call_depth: 0,
        replay_hooks: None,
        players: [Player::default(), Player::default()],
        human_seat: Some(0),
        lanes,
        location_pool: position::all_locations(start.recruit)
            .iter()
            .filter_map(|id| locations::get(id))
            .collect(),
        starting_location_ids,
        turn: start.turn as u8,
        stakes: None,
        next_resolution_order: 1,
        is_wait_for_staging: true,
        is_start_turn: false,
        total_turns: start.total_turns.as_u64().unwrap_or(6) as u8,
        reveal_priority: None,
        played: Default::default(),
        pending_plays: Default::default(),
        played_on_reveal: [false; 2],
        delays_reveals: start
            .locations
            .iter()
            .zip(&start.location_turns)
            .any(|(id, turn)| id == "Lemuria" && turn.as_i64() == Some(start.turn)),
        ending_reveals: false,
        delayed_reveals: Default::default(),
        deferred_added_reveals: Vec::new(),
        post_resolve_plays: Vec::new(),
        end_turn_lane_order: None,
        current_location,
        location_reveal_order: [order[0], order[1], order[2]],
        destroyed: Some(Vec::new()),
        score_overrides: [None; 2],
        current_ongoing_source: None,
        merge_batches: Vec::new(),
        staging_order: None,
        move_provenance_known: false,
        last_move_source: None,
        rng: RefCell::new(crate::game::GameRandom::Borrowed(&mut game_rng)),
    };
    // CaveOfTheDragonBehavior::AddCard (0x7d45454) makes one random
    // selection per player. A visible new card with this exact provenance
    // proves the result; consume the ordinary RNG draw in the location hook.
    // The frozen reference must retain its original seed-only simulation.
    if !crate::cards::reference_rules_enabled() && start.turn == 4 {
        for lane in 0..3 {
            if start.locations[lane] != "CaveOfTheDragon" {
                continue;
            }
            let catalog = crate::data::cards();
            let mut choices = [None; 2];
            let mut first_revealed = [None; 2];
            for (seat, choice) in choices.iter_mut().enumerate() {
                let mut observed = target.board.iter().filter(|card| {
                    card.owner == seat
                        && card.revealed
                        && card.created_by_location == "CaveOfTheDragon"
                        && card.flipped.as_i64() == Some(start.turn)
                        && entity(start, &card.entity).is_none()
                });
                if let Some(card) = observed.next()
                    && observed.next().is_none()
                    && let Some(id) = card.id.as_str()
                    && let Some(def) = crate::cards::get(id)
                    && crate::cards::deck_eligible_ids().contains(&def.id)
                    && catalog.iter().any(|row| {
                        row.carddefid == id
                            && !row.random_pool_excluded
                            && locations::cave_candidate(row)
                    })
                {
                    *choice = Some(def.id);
                    first_revealed[seat] = frames
                        .iter()
                        .filter(|frame| frame.line > start.line && frame.line <= target.line)
                        .find(|frame| {
                            entity(frame, &card.entity).is_some_and(|entry| entry.revealed)
                        })
                        .map(|frame| frame.line);
                }
            }
            // A strictly earlier visible reveal proves the resolution order;
            // a shared or missing frame does not.
            if let [Some(a), Some(b)] = first_revealed
                && a != b
            {
                game.reveal_priority = Some(usize::from(b < a));
            }
            if choices.iter().any(Option::is_some) {
                game.lanes[lane].state = Some(Box::new(choices));
            }
        }
    }
    for card in &start.hand {
        let id = card["card_id"].as_str().ok_or("missing card_id")?;
        let index = push(&mut game, id, 0, &card["cost"], &card["power"])?;
        game.cards[index].zone = Some(CardZone::Hand(0));
        game.put_hand(0, game.players[0].hand.len(), index);
    }
    for id in &my_deck {
        let index = push(&mut game, id, 0, &Value::Null, &Value::Null)?;
        game.cards[index].zone = Some(CardZone::Deck(0));
        game.push_deck(0, index);
    }
    for (i, id) in hidden.iter().enumerate() {
        let index = push(&mut game, id, 1, &Value::Null, &Value::Null)?;
        if i < opp_hand {
            game.cards[index].zone = Some(CardZone::Hand(1));
            game.put_hand(1, game.players[1].hand.len(), index);
        } else {
            game.cards[index].zone = Some(CardZone::Deck(1));
            game.push_deck(1, index);
        }
    }
    let mut cards: Vec<(Value, usize)> = Vec::new();
    let mut board: Vec<_> = start.board.iter().filter(|c| c.revealed).collect();
    board.sort_by_key(|c| (c.lane, c.owner, c.slot.as_i64().unwrap_or(0)));
    for card in board {
        let id = card.id.as_str().ok_or("missing card_id")?;
        let index = push(&mut game, id, card.owner, &card.cost, &Value::Null)?;
        if let Some(reference) = &card.reference {
            position::apply_card_reference(&mut game.cards[index], reference, start.turn as u8);
        }
        let first = frames
            .iter()
            .find_map(|frame| {
                entity(frame, &card.entity)
                    .filter(|c| c.revealed && c.id.as_str().is_some())
                    .map(|c| (frame, c))
            })
            .ok_or("missing reveal")?;
        let flipped = card
            .flipped
            .as_i64()
            .filter(|n| *n != 0)
            .or_else(|| first.0.ended.then_some(first.0.turn));
        if matches!(id, "Hawkeye" | "JessicaJones") && flipped.is_none() {
            return Err(format!(
                "reveal turn of {id} [{}] is unknown",
                label(&card.entity)
            ));
        }
        if matches!(id, "Hawkeye" | "JessicaJones")
            && flipped == Some(start.turn - 1)
            && first.1.lane != card.lane
        {
            return Err(format!(
                "original reveal lane of {id} [{}] is uncertain",
                label(&card.entity)
            ));
        }
        game.cards[index].zone = Some(CardZone::Location(card.lane));
        game.cards[index].lane = Some(card.lane);
        game.cards[index].revealed = true;
        game.cards[index].turn_played = flipped.unwrap_or(0) as u8;
        game.push_location(card.lane, index);
        cards.push((card.entity.clone(), index));
    }
    if !crate::cards::reference_rules_enabled() {
        game.refresh_location_ability_flags();
    }
    if !crate::cards::reference_rules_enabled() {
        for seat in 0..2 {
            if let Some(actions) = start.complete_past_actions(seat) {
                for action in actions.into_iter().filter(|action| action.kind == "play") {
                    if let Some((_, index)) = cards
                        .iter()
                        .find(|(entity, _)| entity.as_i64() == Some(action.card_entity_id))
                    {
                        game.players[seat].play_history.push((action.turn, *index));
                    }
                }
            }
        }
    }
    if !crate::cards::reference_rules_enabled() {
        restore_copied_ongoing(&mut game, start, target, &cards)?;
    }
    for &(_, index) in &cards {
        if matches!(game.cards[index].def.id, "Hawkeye" | "JessicaJones")
            && game.cards[index].turn_played as i64 == start.turn - 1
        {
            let lane = game.cards[index].lane.unwrap();
            let before = game.cards[index].turn_played;
            // Reconstruction heuristic, not a client predicate: excluding hidden
            // abilities is provisional; restored cards have no hidden instances.
            let sources: Vec<_> = game.lanes[lane]
                .cards
                .iter()
                .copied()
                .filter(|&s| {
                    game.cards[s].revealed
                        && game.cards[s].has_ongoing(false)
                        && game.cards[s].turn_played < before
                })
                .collect();
            if game.lanes[lane].ability_flags.on_reveal
                && !sources.iter().any(|&s| {
                    game.cards[s].ongoing_guard_abilities().any(|a| {
                        a.supports(crate::engine_ext::abilities::AbilityKind::Ongoing)
                            && a.def.blocks_reveal
                    })
                })
            {
                let active = game.location_abilities_active(lane)
                    && game.lanes[lane].revealed_turn.unwrap_or(0) <= before;
                let base = if active {
                    game.lanes[lane].def.ongoing_repeats as usize
                } else {
                    1
                };
                let mut runs = if active {
                    game.lanes[lane].def.reveal_repeats as usize
                } else {
                    1
                };
                for &source in &sources {
                    for repetitions in game.cards[source]
                        .ongoing_guard_abilities()
                        .filter(|a| a.supports(crate::engine_ext::abilities::AbilityKind::Ongoing))
                        .filter_map(|a| a.def.reveal_repetitions)
                    {
                        let extra: usize = sources
                            .iter()
                            .filter(|&&other| {
                                other != source
                                    && game.cards[other].owner == game.cards[source].owner
                            })
                            .map(|&other| {
                                game.cards[other]
                                    .ongoing_guard_abilities()
                                    .filter(|a| {
                                        a.supports(
                                            crate::engine_ext::abilities::AbilityKind::Ongoing,
                                        )
                                    })
                                    .map(|a| a.def.extra_ongoing_runs as usize)
                                    .sum::<usize>()
                            })
                            .sum();
                        runs += repetitions(&game, source, index) as usize * base * (1 + extra);
                    }
                }
                let hawkeye = game.cards[index].def.id == "Hawkeye";
                for _ in 0..runs {
                    game.cards[index].attach_watch(
                        index,
                        start.turn as u8,
                        lane,
                        hawkeye,
                        if hawkeye { 4 } else { 5 },
                    );
                }
            }
        }
    }
    // Reconstruct the full ongoing ledger before entering the staging window.
    game.is_wait_for_staging = false;
    game.ongoing();
    for (entity_id, index) in &cards {
        let refcard = entity(start, entity_id).unwrap();
        let power = number(&refcard.power).ok_or_else(|| {
            format!(
                "no public power for {} [{}]",
                game.cards[*index].def.id,
                label(entity_id)
            )
        })?;
        game.cards[*index].permanent =
            power - game.cards[*index].ongoing - game.cards[*index].location_power;
    }
    let locally_played: Vec<_> = staged
        .plays
        .iter()
        .filter(|p| p.card.owner == 0 && p.turn == start.turn)
        .map(|p| &p.card.entity)
        .collect();
    if start.hand.len() as i64 != start.hand_counts[0].as_i64().unwrap_or(-1)
        || staged.before.hand.len() as i64 != staged.before.hand_counts[0].as_i64().unwrap_or(-1)
        || start
            .hand
            .iter()
            .filter(|c| {
                !staged
                    .before
                    .hand
                    .iter()
                    .any(|b| b["entity_id"] == c["entity_id"])
            })
            .any(|c| !locally_played.contains(&&c["entity_id"]))
        || locally_played.iter().any(|id| {
            !start.hand.iter().any(|c| c["entity_id"] == **id)
                || staged.before.hand.iter().any(|c| c["entity_id"] == **id)
        })
        || staged
            .before
            .hand
            .iter()
            .any(|c| !start.hand.iter().any(|s| s["entity_id"] == c["entity_id"]))
    {
        return Err("local hand/staging transition is incomplete".into());
    }
    let original = game.players[0].hand.clone();
    game.players[0].hand = staged
        .before
        .hand
        .iter()
        .map(|card| {
            original[start
                .hand
                .iter()
                .position(|held| held["entity_id"] == card["entity_id"])
                .unwrap()]
        })
        .collect();
    for (card, &index) in staged.before.hand.iter().zip(&game.players[0].hand) {
        if let Some(power) = number(&card["power"]) {
            game.cards[index].permanent =
                power - game.cards[index].ongoing - game.cards[index].location_power;
        }
    }
    game.ongoing();
    for (card, &index) in staged.before.hand.iter().zip(&game.players[0].hand) {
        if let Some(observed) = number(&card["cost"]) {
            let printed = position::make_card(game.cards[index].def.id, 0)?.cost;
            let mut costs: Vec<_> = (0..observed + 3).collect();
            costs.sort_by_key(|&cost| (cost != printed, (cost - observed).abs()));
            let mut found = false;
            for cost in costs {
                game.cards[index].cost = cost;
                if game.card_cost(index) == observed {
                    found = true;
                    break;
                }
            }
            if !found {
                game.cards[index].cost = observed;
            }
        }
    }
    game.is_wait_for_staging = true;
    game.truncate_sampled_hand(1, staged.opponent_count.max(0) as usize);
    // A resolved Morph's observed identity proves that card was in the
    // opposing hand when it resolved. Include it in the sampled hand rather
    // than hoping a small seed budget rediscovers a now-large card pool.
    // The frozen reference samples without this evidence; parity gates keep that.
    if !crate::cards::reference_rules_enabled()
        && let Some(observed) = staged.plays.iter().find_map(|play| {
            (play.card.owner == 0 && play.id == "Morph")
                .then(|| entity(target, &play.card.entity))
                .flatten()
                .and_then(|card| card.id.as_str())
                .filter(|id| *id != "Morph" && crate::cards::get(id).is_some())
        })
        && let Some(&slot) = game.players[1].hand.first()
    {
        game.cards[slot] = make(observed, 1, &Value::Null, &Value::Null)?;
    }
    // Staged cards can already be present when a later manual move fires.
    // Preserve the observed order rather than always applying moves first.
    let reference = crate::cards::reference_rules_enabled();
    // Complete local action history can constrain some same-snapshot pairs.
    // Keep the frozen and legacy paths byte-for-byte equivalent in their order.
    let has_recorded_order =
        !reference
            && staged.moves.iter().enumerate().any(|(m, (_, _, line))| {
                staged.plays.iter().enumerate().any(|(p, play)| {
                    play.stage_line == *line && staged.recorded_order(m, p).is_some()
                })
            });
    let mut next_bit = 0;
    let mut plays_before = vec![0usize; staged.moves.len()];
    if !reference && !has_recorded_order {
        for (index, (_, _, line)) in staged.moves.iter().enumerate() {
            for _ in staged.plays.iter().filter(|play| play.stage_line == *line) {
                plays_before[index] += order_mask >> next_bit & 1;
                next_bit += 1;
            }
        }
    }
    let mut order = Vec::with_capacity(staged.moves.len() + staged.plays.len());
    for (index, (_, _, line)) in staged.moves.iter().enumerate() {
        order.push((
            if reference { 0 } else { *line },
            2 * plays_before[index],
            0,
            0,
            0,
            index,
        ));
    }
    for (index, play) in staged.plays.iter().enumerate() {
        let rank = staged
            .plays
            .iter()
            .filter(|other| {
                other.stage_line == play.stage_line
                    && (other.card.lane, other.card.slot.as_i64().unwrap_or(0))
                        < (play.card.lane, play.card.slot.as_i64().unwrap_or(0))
            })
            .count();
        order.push((
            if reference { 1 } else { play.stage_line },
            if reference { 1 } else { 2 * rank + 1 },
            1,
            play.card.lane,
            play.card.slot.as_i64().unwrap_or(0),
            index,
        ));
    }
    if has_recorded_order {
        // Model each independently observed pair as an edge. Counting how many
        // plays precede a move is insufficient when an opponent play can be on
        // either side of a locally ordered play.
        let moves = staged.moves.len();
        let count = moves + staged.plays.len();
        let mut edges = Vec::new();
        for a in 0..count {
            for b in a + 1..count {
                let line_a = if a < moves {
                    staged.moves[a].2
                } else {
                    staged.plays[a - moves].stage_line
                };
                let line_b = if b < moves {
                    staged.moves[b].2
                } else {
                    staged.plays[b - moves].stage_line
                };
                if line_a != line_b {
                    edges.push(if line_a < line_b { (a, b) } else { (b, a) });
                } else if b < moves {
                    edges.push((a, b));
                } else if a >= moves
                    && let Some(cmp) = staged.recorded_play_order(a - moves, b - moves)
                {
                    match cmp {
                        std::cmp::Ordering::Less => edges.push((a, b)),
                        std::cmp::Ordering::Greater => edges.push((b, a)),
                        std::cmp::Ordering::Equal => {}
                    }
                }
            }
        }
        next_bit = 0;
        for (m, (_, _, line)) in staged.moves.iter().enumerate() {
            for (p, _) in staged
                .plays
                .iter()
                .enumerate()
                .filter(|(_, p)| p.stage_line == *line)
            {
                let before = if let Some(before) = staged.recorded_order(m, p) {
                    before
                } else {
                    let before = order_mask >> next_bit & 1 != 0;
                    next_bit += 1;
                    before
                };
                edges.push(if before {
                    (moves + p, m)
                } else {
                    (m, moves + p)
                });
            }
        }
        sort_staging_edges(moves, count, &edges, &mut order)?;
    }
    order.sort_unstable();
    for (_, _, kind, _, _, index) in order {
        if kind == 0 {
            let (entity_id, lane, _) = &staged.moves[index];
            let card = cards.iter().find(|(id, _)| id == entity_id).unwrap().1;
            if !game.move_card(card, *lane) {
                return Err(format!(
                    "observed pre-reveal move of {} [{}] cannot be staged",
                    game.cards[card].def.id,
                    label(entity_id)
                ));
            }
            game.ongoing();
            continue;
        }
        let play = &staged.plays[index];
        let id = play.id.as_str().ok_or("missing card_id")?;
        let index = push(&mut game, id, play.card.owner, &play.cost, &play.power)?;
        if play.card.owner == 1 {
            // PyramidOfRamaTutBehavior::AddRock (0x7d75c10) sets the
            // generated Rock's power to 3; use its visible provenance.
            if id == "Rock"
                && entity(target, &play.card.entity)
                    .is_some_and(|card| card.created_by_location == "PyramidOfRamaTut")
            {
                game.set_power(index, 3, 1);
            }
            if let Some(power) =
                hidden_preplay_power(play, staged, frames, game.cards[index].permanent)?
            {
                game.cards[index].permanent = power;
            } else {
                game.cards[index].permanent +=
                    2 * hidden_rng.randint(0, history.embassy_reveals as i64) as i32;
                if history.chavez_reveals > 0
                    && hidden_rng.randrange(0, start.hand_counts[1].as_i64().unwrap_or(0).max(1), 1)
                        == 0
                {
                    game.cards[index].permanent += 3;
                }
            }
        }
        game.cards[index].lane = Some(play.card.lane);
        game.cards[index].turn_played = play.turn as u8;
        game.push_location(play.card.lane, index);
        cards.push((play.card.entity.clone(), index));
        let seat = play.card.owner;
        let turn = play.turn as u8;
        let printed_cost = game.printed_cost(index);
        game.players[seat]
            .staged_history
            .push(crate::game::StagedPlay {
                turn,
                card: index,
                def_id: game.cards[index].def.id,
                printed_cost,
                created: game.cards[index].created,
                resolved: false,
            });
        game.players[seat]
            .staged_lane_history
            .push((turn, play.card.lane));
        if play.turn == start.turn {
            game.played[play.card.owner].push((index, play.card.lane));
        } else {
            game.pending_plays[play.card.owner].push((index, play.card.lane));
        }
    }
    game.replay_hooks = Some(RefCell::new(Default::default()));
    game.is_wait_for_staging = false;
    game.ongoing();
    for play in &staged.plays {
        let index = cards
            .iter()
            .find(|(id, _)| *id == play.card.entity)
            .unwrap()
            .1;
        if !crate::cards::reference_rules_enabled()
            && play.id == "Polymorph"
            && staged
                .plays
                .iter()
                .filter(|other| other.flip_line == play.flip_line)
                .count()
                == 1
        {
            // TransformCard 0x7d05c8c chooses a random one-cost-higher
            // character. Only a unique identity change observed after this
            // reveal and before the next play proves the random outcome.
            let next_flip = staged
                .plays
                .iter()
                .filter_map(|other| (other.flip_line > play.flip_line).then_some(other.flip_line))
                .min()
                .unwrap_or(target.line + 1);
            let observed: Vec<_> = start
                .board
                .iter()
                .filter_map(|before| {
                    if !before.revealed
                        || before.owner != play.card.owner
                        || before.lane != play.card.lane
                    {
                        return None;
                    }
                    let (_, target_index) =
                        cards.iter().find(|(entity, _)| *entity == before.entity)?;
                    let changed = frames
                        .iter()
                        .filter(|frame| frame.line > play.flip_line && frame.line < next_flip)
                        .filter_map(|frame| entity(frame, &before.entity))
                        .find(|after| after.revealed && after.id != before.id)?;
                    let id = changed.id.as_str()?;
                    let def = crate::cards::get(id)?;
                    if !crate::cards::deck_eligible_ids().contains(&def.id)
                        || !crate::data::cards().iter().any(|row| {
                            row.carddefid == id
                                && !row.random_pool_excluded
                                && row.card_type == "Character"
                                && row.cost == game.printed_cost(*target_index) + 1
                        })
                    {
                        return None;
                    }
                    Some((*target_index, def.id))
                })
                .collect();
            if let [choice] = observed.as_slice() {
                game.cards[index].state = Some(Box::new(*choice));
            }
        }
        game.reveal_with_staging(
            index,
            Some(crate::game::StagingContext {
                seat: play.card.owner,
                lane: play.card.lane,
            }),
        );
    }
    game.end_turn_effects();
    if target.terminal {
        if game.total_turns as i64 != start.turn {
            return Ok((
                vec![diff(
                    format!(
                        "game length: simulator {}, observed final turn {}",
                        game.total_turns, start.turn
                    ),
                    vec![],
                    vec![],
                )],
                game.replay_hooks
                    .filter(|_| collect_hooks)
                    .map(RefCell::into_inner),
            ));
        }
        game.finish_reveals();
    } else {
        if (game.total_turns as i64) < target.turn {
            return Ok((
                vec![diff(
                    format!(
                        "game length: simulator ended on turn {}, observed turn {}",
                        game.total_turns, target.turn
                    ),
                    vec![],
                    vec![],
                )],
                game.replay_hooks
                    .filter(|_| collect_hooks)
                    .map(RefCell::into_inner),
            ));
        }
        if target.turn != start.turn && target.turn <= 3 {
            let lane = (target.turn - 1) as usize;
            if !target.destroyed_lanes[lane]
                && start.locations[lane].is_null()
                && target.locations[lane].is_null()
            {
                return Err("next scheduled location was not observed revealed".into());
            }
            if !target.destroyed_lanes[lane]
                && start.locations[lane].is_null()
                && target.location_turns[lane].as_i64() == Some(target.turn)
                && !game.lanes[lane].revealed
            {
                let id = target.locations[lane].as_str().unwrap();
                game.lanes[lane].def = locations::get(id).unwrap();
                game.lanes[lane].state = game.lanes[lane].def.initial_state.map(|init| init());
                game.location_reveal_order[game.current_location] = lane;
            }
        }
        game.turn = target.turn as u8;
        // SakaarTutorialBehavior::PlayCard 0x7d7b5bc selects the highest
        // power card in the opponent's hand. A newly visible non-staged card
        // proves that choice was in the hand after the next draw.
        if !crate::cards::reference_rules_enabled() && target.turn <= 3 {
            let lane = (target.turn - 1) as usize;
            if start.locations[lane].is_null() && target.locations[lane] == "SakaarTutorial" {
                let candidates: Vec<_> = target
                    .board
                    .iter()
                    .filter(|card| {
                        card.owner == 1
                            && card.lane == lane
                            && card.revealed
                            && entity(start, &card.entity).is_none()
                            && !staged
                                .plays
                                .iter()
                                .any(|play| play.card.entity == card.entity)
                            && card
                                .id
                                .as_str()
                                .is_some_and(|id| crate::cards::get(id).is_some())
                    })
                    .collect();
                if let [observed] = candidates.as_slice() {
                    let id = observed.id.as_str().unwrap();
                    let selected = make(id, 1, &Value::Null, &Value::Null)?;
                    let ceiling = selected.power();
                    let mut available = game.players[1].hand.clone();
                    if let Some(slot) = game.next_draw_slot(1) {
                        available.push(game.players[1].deck[slot]);
                    }
                    if let Some(&slot) = available.first() {
                        game.cards[slot] = selected;
                        for &other in available.iter().skip(1) {
                            if game.cards[other].power() > ceiling {
                                let replacement = others.iter().find(|candidate| {
                                    !available.iter().any(|&index| {
                                        game.cards[index].def.id == candidate.as_str()
                                    }) && position::make_card(candidate, 1)
                                        .is_ok_and(|card| card.power() <= ceiling)
                                });
                                if let Some(replacement) = replacement {
                                    game.cards[other] =
                                        make(replacement, 1, &Value::Null, &Value::Null)?;
                                }
                            }
                        }
                    }
                }
            }
        }
        game.played = Default::default();
        game.start_turn();
    }
    let mut differences = compare(&game, target, &cards);
    if !crate::cards::reference_rules_enabled()
        && let Some((reason, lanes)) = undecoded_random_draw(&game, target, &cards)
    {
        // Only what happens in the lane of an undecoded draw is attributed to it;
        // a difference anywhere else is still the rules' to explain.
        differences.retain(|difference| !difference.lanes.iter().any(|lane| lanes.contains(lane)));
        if differences.is_empty() {
            return Err(reason);
        }
    }
    Ok((
        differences,
        game.replay_hooks
            .filter(|_| collect_hooks)
            .map(RefCell::into_inner),
    ))
}

/// `Game::GetRandomLocationDefId` (0x7db248c) rolls `WeightedTable.GetRandom`
/// (0x60708f0) over the location table; Black Vortex's random 6-cost card
/// (`TransformCardInto6Cost` 0x7d407d4) rolls a cost-filtered pool. Their
/// roll-to-outcome mapping is not decoded, so no seed can reproduce a
/// recorded outcome by construction. When the observed outcome is one the draw
/// could have produced but the trial drew another, the trial proves nothing
/// about the rules in that draw's lane; the effects of the observed outcome are not
/// simulated. Returns the reason and the affected (0-based) lanes.
fn undecoded_random_draw(
    game: &Game<'_>,
    target: &Frame,
    cards: &[(Value, usize)],
) -> Option<(String, Vec<usize>)> {
    let hooks = game.replay_hooks.as_ref()?.borrow();
    let mut reason = None;
    let mut lanes = Vec::new();
    for (lane, board) in game.lanes.iter().enumerate() {
        let Some(actual) = target.locations[lane].as_str() else {
            continue;
        };
        if board.revealed
            && actual != board.def.id
            && hooks.random_locations.iter().any(|(excluded, drawn)| {
                *drawn == board.def.id && game.location_in_random_pool(actual, excluded.as_deref())
            })
        {
            reason.get_or_insert_with(|| {
                format!(
                    "random location roll mapping undecoded (WeightedTable.GetRandom 0x60708f0): lane {} simulator drew {}, observed {actual}",
                    lane + 1,
                    board.def.id
                )
            });
            lanes.push(lane);
        }
    }
    for &(card, drawn) in &hooks.random_cards {
        let Some((entity_id, _)) = cards.iter().find(|(_, index)| *index == card) else {
            continue;
        };
        let Some(observed) = entity(target, entity_id).filter(|observed| observed.revealed) else {
            continue;
        };
        let Some(actual) = observed.id.as_str() else {
            continue;
        };
        if actual != drawn
            && crate::cards::get(actual)
                .is_some_and(|def| crate::cards::deck_eligible_ids().contains(&def.id))
            && crate::data::cards().iter().any(|row| {
                row.carddefid == actual
                    && !row.random_pool_excluded
                    && row.card_type == "Character"
                    && row.cost == 6
            })
        {
            reason.get_or_insert_with(|| {
                format!(
                    "random 6-cost card roll mapping undecoded (Black Vortex, TransformCardInto6Cost 0x7d407d4): simulator drew {drawn}, observed {actual}"
                )
            });
            lanes.extend(game.cards[card].lane);
            lanes.push(observed.lane);
        }
    }
    reason.map(|reason| (reason, lanes))
}

fn compare(game: &Game<'_>, target: &Frame, cards: &[(Value, usize)]) -> Vec<Difference> {
    let mut out = Vec::new();
    let seats = ["local", "opponent"];
    for (entity_id, index) in cards {
        let card = &game.cards[*index];
        let actual = entity(target, entity_id);
        let predicted = card.lane;
        let name = card.def.id.to_owned();
        let label = format!("{} [{}]", name, label(entity_id));
        let destroyed = game.destroyed.as_ref().and_then(|events| {
            events
                .iter()
                .rev()
                .find(|(card, _)| card == index)
                .map(|(_, lane)| *lane)
        });
        if actual.is_none() {
            let zone = if target.hand.iter().any(|c| c["entity_id"] == *entity_id) {
                Some("hand")
            } else {
                target
                    .out_of_play
                    .iter()
                    .find(|(id, _)| id == entity_id)
                    .map(|(_, zone)| *zone)
            };
            if let Some(lane) = predicted {
                out.push(diff(
                    format!(
                        "{label}: simulator kept it at lane {}; observed {}",
                        lane + 1,
                        zone.unwrap_or("off board")
                    ),
                    vec![name],
                    vec![lane],
                ));
            } else if zone == Some("graveyard") && destroyed.is_none() {
                out.push(diff(
                    format!("{label}: observed destroyed, simulator did not destroy it"),
                    vec![name],
                    vec![],
                ));
            } else if matches!(zone, Some("hand" | "banished"))
                && let Some(lane) = destroyed
            {
                out.push(diff(
                    format!(
                        "{label}: simulator destroyed it; observed in {}",
                        zone.unwrap()
                    ),
                    vec![name],
                    vec![lane],
                ));
            }
            continue;
        }
        let actual = actual.unwrap();
        let Some(lane) = predicted else {
            out.push(diff(
                format!(
                    "{label}: simulator {} it; observed at lane {}",
                    if destroyed.is_some() {
                        "destroyed"
                    } else {
                        "removed"
                    },
                    actual.lane + 1
                ),
                vec![name],
                vec![actual.lane],
            ));
            continue;
        };
        if lane != actual.lane {
            out.push(diff(
                format!(
                    "{label} location: simulator lane {}, observed lane {}",
                    lane + 1,
                    actual.lane + 1
                ),
                vec![name.clone()],
                vec![lane, actual.lane],
            ));
        }
        if card.owner != actual.owner {
            out.push(diff(
                format!(
                    "{label} owner: simulator {}, observed {}",
                    seats[card.owner], seats[actual.owner]
                ),
                vec![name.clone()],
                vec![actual.lane],
            ));
        }
        if card.revealed != actual.revealed {
            out.push(diff(
                format!(
                    "{label} reveal: simulator {}, observed {}",
                    if card.revealed { "True" } else { "False" },
                    if actual.revealed { "True" } else { "False" }
                ),
                vec![name.clone()],
                vec![actual.lane],
            ));
        } else if actual.revealed {
            if actual.id != name {
                out.push(diff(
                    format!("{label} identity: observed {}", label_value(&actual.id)),
                    vec![name.clone(), label_value(&actual.id)],
                    vec![actual.lane],
                ));
            }
            if number(&actual.power) != Some(card.power()) {
                out.push(diff(
                    format!(
                        "{label} power: simulator {}, observed {}",
                        card.power(),
                        label_value(&actual.power)
                    ),
                    vec![name.clone()],
                    vec![actual.lane],
                ));
            }
        }
    }
    type Addition = (usize, usize, Option<String>, Option<i32>);
    fn increment(counts: &mut Vec<(Addition, usize)>, key: Addition) {
        if let Some((_, count)) = counts.iter_mut().find(|(existing, _)| *existing == key) {
            *count += 1;
        } else {
            counts.push((key, 1));
        }
    }
    let mut predicted = Vec::new();
    let mut observed = Vec::new();
    for (lane, board) in game.lanes.iter().enumerate() {
        for &index in &board.cards {
            if cards.iter().any(|(_, id)| *id == index) {
                continue;
            }
            let c = &game.cards[index];
            increment(
                &mut predicted,
                (
                    lane,
                    c.owner,
                    c.revealed.then(|| c.def.id.to_owned()),
                    c.revealed.then(|| c.power()),
                ),
            );
        }
    }
    for card in &target.board {
        if cards.iter().any(|(id, _)| *id == card.entity) {
            continue;
        }
        increment(
            &mut observed,
            (
                card.lane,
                card.owner,
                card.id.as_str().map(str::to_owned),
                if card.revealed {
                    number(&card.power)
                } else {
                    None
                },
            ),
        );
    }
    for (excess, other, origin) in [
        (&observed, &predicted, "observed"),
        (&predicted, &observed, "simulator"),
    ] {
        for ((lane, seat, id, power), count) in excess {
            let count = count.saturating_sub(
                other
                    .iter()
                    .find(|((l, s, name, p), _)| l == lane && s == seat && name == id && p == power)
                    .map_or(0, |(_, count)| *count),
            );
            if count > 0 {
                out.push(diff(
                    format!(
                        "lane {} {}: {origin}-only addition {count} × {} (power {})",
                        lane + 1,
                        seats[*seat],
                        id.as_deref().unwrap_or("face-down"),
                        power.map_or("None".to_owned(), |p| p.to_string())
                    ),
                    id.iter().cloned().collect(),
                    vec![*lane],
                ));
            }
        }
    }
    for (lane, board) in game.lanes.iter().enumerate() {
        let id = (board.revealed && !board.def.id.is_empty()).then_some(board.def.id);
        if board.destroyed != target.destroyed_lanes[lane] {
            out.push(diff(
                format!(
                    "lane {} destroyed: simulator {}, observed {}",
                    lane + 1,
                    board.destroyed,
                    target.destroyed_lanes[lane]
                ),
                vec![],
                vec![lane],
            ));
        }
        let actual = target.locations[lane].as_str();
        if id != actual {
            out.push(diff(
                format!(
                    "lane {} location: simulator {}, observed {}",
                    lane + 1,
                    id.unwrap_or("face-down"),
                    actual.unwrap_or("face-down")
                ),
                vec![],
                vec![lane],
            ));
        } else if let (Some(id), Some(observed), Some(predicted)) = (
            actual,
            target.location_turns[lane].as_u64(),
            board.revealed_turn,
        ) && observed != predicted as u64
        {
            out.push(diff(
                format!(
                    "lane {} {id} reveal turn: simulator {predicted}, observed {observed}",
                    lane + 1
                ),
                vec![],
                vec![lane],
            ));
        }
        for (seat, name) in seats.iter().enumerate() {
            let total = number(&target.totals[lane][name]);
            let expected = total.unwrap_or_else(|| {
                target
                    .board
                    .iter()
                    .filter(|c| c.lane == lane && c.owner == seat && c.revealed)
                    .map(|c| number(&c.power).unwrap_or(0))
                    .sum()
            });
            let score = if total.is_some() {
                game.powers()[lane][seat]
            } else {
                board
                    .cards
                    .iter()
                    .filter(|&&id| game.cards[id].revealed && game.cards[id].owner == seat)
                    .map(|&id| game.cards[id].power())
                    .sum()
            };
            if score != expected {
                let involved: BTreeSet<_> = board
                    .cards
                    .iter()
                    .filter(|&&id| game.cards[id].revealed && game.cards[id].owner == seat)
                    .map(|&id| game.cards[id].def.id.to_owned())
                    .collect();
                out.push(diff(
                    format!(
                        "lane {} {name} {}: simulator {score}, observed {expected}",
                        lane + 1,
                        if total.is_some() {
                            "total"
                        } else {
                            "visible total"
                        }
                    ),
                    involved.into_iter().collect(),
                    vec![lane],
                ));
            }
        }
    }
    out
}
fn label_value(value: &Value) -> String {
    label(value)
}
#[cfg(test)]
mod staging_order_tests {
    use super::sort_staging_edges;

    #[test]
    fn mixed_owner_constraints_are_applied_independently() {
        // Node 0 is a move, 1 a local play, and 2 an opponent play.
        // Local play is proven after the move, opponent play before it.
        let mut order = Vec::new();
        sort_staging_edges(1, 3, &[(0, 1), (2, 0)], &mut order).unwrap();
        assert_eq!(
            order
                .iter()
                .map(|item| (item.2, item.5))
                .collect::<Vec<_>>(),
            [(1, 1), (0, 0), (1, 0)]
        );
        // The other opponent ordering remains possible.
        sort_staging_edges(1, 3, &[(0, 1), (0, 2)], &mut order).unwrap();
        assert_eq!(order[0].2, 0);
    }

    #[test]
    fn contradictory_staging_constraints_are_not_simulated() {
        let mut order = Vec::new();
        assert!(sort_staging_edges(1, 3, &[(0, 1), (1, 2), (2, 0)], &mut order).is_err());
    }
}
