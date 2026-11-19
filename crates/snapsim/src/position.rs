//! Reconstruct a planning state from the Python advisor's public position format.
use crate::{
    cards::{self, CardDef},
    data,
    game::{Card, CardStatUpdate, Game, Lane, Player, Stat, StatOperation},
    locations::{self, LocationDef},
    random::PythonRandom,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

fn yes() -> bool {
    true
}
fn six() -> u8 {
    6
}
/// An observed client ledger entry. Source is a seat, not an entity ID.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ObservedStatUpdate {
    pub stat: String,
    pub update: String,
    pub value: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updater_card_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updater_location_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updater_faction_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updater_augment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updater_league_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CardRef {
    pub id: String,
    #[serde(default)]
    pub cost: Option<i32>,
    #[serde(default)]
    pub power: Option<i32>,
    #[serde(default = "yes")]
    pub revealed: bool,
    #[serde(default)]
    pub played_turn: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activated: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_moves_used: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_activate: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_targets: Option<Vec<usize>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_resolution_order: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns_drawn: Option<Vec<(u8, bool)>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moves_per_turn: Option<HashMap<String, u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_card_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_location_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_player: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merged_to_entity_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stat_updates: Option<Vec<ObservedStatUpdate>>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LocationRef {
    pub id: Option<String>,
    pub mine: Vec<CardRef>,
    pub theirs: Vec<CardRef>,
    #[serde(default)]
    pub theirs_face_down: usize,
    #[serde(default)]
    pub revealed_turn: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ottos_lab_spent: Option<[bool; 2]>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Opponent {
    pub hand: usize,
    pub deck: usize,
    #[serde(default)]
    pub energy: Option<i32>,
    #[serde(default)]
    pub seen: Vec<String>,
    #[serde(default)]
    pub deck_list: Option<Vec<String>>,
    #[serde(default)]
    pub pool: Option<Vec<String>>,
    #[serde(default)]
    pub pending_power: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graveyard: Option<Vec<CardRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banished: Option<Vec<CardRef>>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Position {
    pub turn: u8,
    #[serde(default = "six")]
    pub total_turns: u8,
    pub energy: i32,
    pub hand: Vec<CardRef>,
    pub deck: Vec<String>,
    pub locations: [LocationRef; 3],
    pub opponent: Opponent,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub pending_power: i32,
    #[serde(default)]
    pub deck_count: Option<usize>,
    /// Per-seat snap state for the current turn, when observed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapped_this_turn: Option<[bool; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graveyard: Option<Vec<CardRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banished: Option<Vec<CardRef>>,
}

static CARD_DATA: LazyLock<HashMap<String, data::CardData>> = LazyLock::new(|| {
    data::cards()
        .into_iter()
        .map(|x| (x.carddefid.clone(), x))
        .collect()
});
static LOCATION_DATA: LazyLock<HashMap<String, data::LocationData>> = LazyLock::new(|| {
    data::locations()
        .into_iter()
        .map(|x| (x.carddefid.clone(), x))
        .collect()
});
static CARD_FALLBACKS: LazyLock<HashMap<&'static str, CardDef>> = LazyLock::new(|| {
    CARD_DATA
        .keys()
        .filter(|id| cards::get(id).is_none())
        .map(|id| (id.as_str(), CardDef::vanilla(id.as_str())))
        .collect()
});
static LOCATION_FALLBACKS: LazyLock<HashMap<&'static str, LocationDef>> = LazyLock::new(|| {
    LOCATION_DATA
        .keys()
        .filter(|id| locations::get(id).is_none())
        .map(|id| {
            (
                id.as_str(),
                locations::location(id.as_str(), 0, 0, 1, false),
            )
        })
        .collect()
});
pub(crate) static UNKNOWN_LOCATION: LocationDef = locations::location("", 0, 0, 1, false);
pub(crate) fn location_def(id: &str) -> Option<&'static LocationDef> {
    locations::get(id).or_else(|| LOCATION_FALLBACKS.get(id))
}
pub(crate) fn card_def(id: &str) -> Result<&'static CardDef, String> {
    cards::get(id)
        .or_else(|| CARD_FALLBACKS.get(id))
        .ok_or_else(|| format!("unknown card ID {id:?}"))
}
pub(crate) fn recruit_source(s: &str) -> bool {
    matches!(
        s,
        "Starter Card" | "Recruit Season" | "Collection Level 1-14" | "Series 1"
    )
}
pub(crate) fn outside<'a>(ids: impl IntoIterator<Item = &'a String>) -> bool {
    ids.into_iter().any(|id| {
        CARD_DATA
            .get(id)
            .is_some_and(|d| d.status == "released" && !recruit_source(&d.source))
    })
}
pub fn recruit_cards() -> Vec<String> {
    cards::deck_eligible_ids()
        .iter()
        .filter(|id| {
            CARD_DATA
                .get(**id)
                .is_some_and(|d| d.status == "released" && recruit_source(&d.source))
        })
        .map(|id| (*id).to_owned())
        .collect()
}
pub(crate) fn all_cards() -> Vec<String> {
    cards::deck_eligible_ids()
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}
pub(crate) fn all_locations(recruit: bool) -> Vec<String> {
    let mut ids: Vec<_> = locations::REGISTRY
        .iter()
        .filter(|d| {
            !recruit
                || LOCATION_DATA.get(d.id).is_some_and(|x| {
                    x.status == "released" && matches!(x.difficulty.as_str(), "Basic" | "Standard")
                })
        })
        .map(|d| d.id.to_owned())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}
fn sample<T: Clone>(rng: &mut PythonRandom, pool: &[T], count: usize) -> Result<Vec<T>, String> {
    if count > pool.len() {
        return Err(format!(
            "cannot draw {count} distinct items from a pool of {}",
            pool.len()
        ));
    }
    Ok(rng
        .sample_indices(pool.len(), count)
        .into_iter()
        .map(|i| pool[i].clone())
        .collect())
}
pub(crate) fn make_card(id: &str, owner: usize) -> Result<Card, String> {
    let def = card_def(id)?;
    let row = CARD_DATA
        .get(id)
        .ok_or_else(|| format!("unknown card ID {id:?}"))?;
    Ok(Card {
        def,
        override_ability: None,
        morphed_from: None,
        morphed_from_entity: None,
        replaced_card_def_id: None,
        owner,
        cost: row.cost,
        ongoing_cost: 0,
        temporary_hand_cost: None,
        created: false,
        created_by_card_id: None,
        created_by_location_id: None,
        created_by_player: None,
        observed_entity_id: None,
        merged_to_entity_id: None,
        permanent: row.power,
        stat_updates: Vec::new(),
        ongoing: 0,
        location_power: 0,
        last_lane: None,
        lane: None,
        revealed: false,
        abilities_removed: false,
        latest_resolution_order: 0,
        repeating: false,
        move_turns: Vec::new(),
        player_moves_used: 0,
        activated: false,
        move_staged: false,
        activate_staged: false,
        observed_can_activate: None,
        observed_move_targets: None,
        draw_extends_quickdraw: false,
        merged_to: None,
        drawn_turn: None,
        turn_played: 0,
        attached_abilities: crate::game::initial_attachments(def),
        state: def.initial_state.map(|init| init()),
    })
}

fn apply_card_reference(card: &mut Card, reference: &CardRef, turn: u8) {
    card.activated = reference.activated.unwrap_or(false);
    card.created = reference.created.unwrap_or(false);
    card.latest_resolution_order = reference.latest_resolution_order.unwrap_or(0);
    card.created_by_card_id
        .clone_from(&reference.created_by_card_id);
    card.created_by_location_id
        .clone_from(&reference.created_by_location_id);
    card.created_by_player = reference.created_by_player.filter(|&seat| seat < 2);
    card.observed_entity_id = reference.entity_id;
    card.merged_to_entity_id = reference.merged_to_entity_id;
    if let Some(draws) = &reference.turns_drawn {
        // Card.IsQuickdrawActive (0x7cb1b88), predicate 0x7cbe0d0.
        if let Some(&(drawn, extended)) = draws
            .iter()
            .rev()
            .find(|&&(drawn, extended)| {
                drawn == turn || (extended && drawn.checked_add(1) == Some(turn))
            })
            .or_else(|| draws.last())
        {
            card.drawn_turn = Some(drawn);
            card.draw_extends_quickdraw = extended;
        }
    }
    if let Some(moves) = &reference.moves_per_turn {
        let mut turns: Vec<_> = moves
            .iter()
            .filter_map(|(key, count)| key.parse::<u8>().ok().map(|turn| (turn, *count)))
            .collect();
        turns.sort_unstable_by_key(|&(turn, _)| turn);
        for (turn, count) in turns {
            card.move_turns
                .extend(std::iter::repeat_n(turn, count as usize));
            card.player_moves_used = card.player_moves_used.saturating_add(count);
        }
    } else {
        card.player_moves_used = reference.player_moves_used.unwrap_or(0);
    }
}

fn needs_observed_reference(reference: &CardRef) -> bool {
    reference.entity_id.is_some()
        || reference.merged_to_entity_id.is_some()
        || reference.latest_resolution_order.is_some()
        || reference.turns_drawn.is_some()
        || reference.stat_updates.is_some()
}

fn observed_update(update: &ObservedStatUpdate) -> Option<CardStatUpdate> {
    let stat = match update.stat.as_str() {
        "Power" => Stat::Power,
        "Cost" => Stat::Cost,
        _ => return None,
    };
    let operation = match update.update.as_str() {
        "Add" => StatOperation::Add,
        "Multiply" => StatOperation::Multiply,
        "Set" => StatOperation::Set,
        _ => return None,
    };
    Some(CardStatUpdate {
        stat,
        operation,
        value: update.value,
        // Unknown sources must not be mistaken for either player's effects.
        source: update.source.filter(|&seat| seat < 2).unwrap_or(usize::MAX),
        ongoing: false,
    })
}
pub fn recruit_weights() -> HashMap<String, f64> {
    let pool = recruit_cards();
    let mut sizes: HashMap<&str, usize> = HashMap::new();
    for id in &pool {
        *sizes.entry(&CARD_DATA[id].source).or_default() += 1;
    }
    let reveals = |s: &str| match s {
        "Starter Card" => 15,
        "Collection Level 1-14" => 5,
        "Series 1" => 4,
        "Recruit Season" => 2,
        _ => 0,
    };
    let total = 12 + sizes.keys().map(|s| reveals(s)).sum::<usize>();
    pool.iter()
        .map(|id| {
            let source = CARD_DATA[id].source.as_str();
            (
                id.clone(),
                (reveals(source) as f64 / sizes[source] as f64 + 12.0 / pool.len() as f64)
                    / total as f64,
            )
        })
        .collect()
}

pub fn build_game<'a>(position: &Position, rng: &'a mut PythonRandom) -> Result<Game<'a>, String> {
    if position.turn == 0 || position.turn > position.total_turns || position.hand.len() > 7 {
        return Err("invalid turn or hand size".into());
    }
    let mut deck_ids = position.deck.clone();
    let missing = position
        .deck_count
        .unwrap_or(deck_ids.len())
        .saturating_sub(deck_ids.len());
    if missing > 0 {
        let known: HashSet<_> = position
            .hand
            .iter()
            .map(|c| &c.id)
            .chain(position.deck.iter())
            .chain(
                position
                    .locations
                    .iter()
                    .flat_map(|l| l.mine.iter().map(|c| &c.id)),
            )
            .chain(position.graveyard.iter().flatten().map(|c| &c.id))
            .chain(position.banished.iter().flatten().map(|c| &c.id))
            .collect();
        let pool = if position.mode.as_deref() == Some("recruit") && !outside(known.iter().copied())
        {
            recruit_cards()
        } else {
            all_cards()
        };
        let candidates: Vec<_> = pool.into_iter().filter(|id| !known.contains(id)).collect();
        deck_ids.extend(sample(rng, &candidates, missing)?);
    }
    deck_ids.sort();
    rng.shuffle(&mut deck_ids);
    if let Some(count) = position.deck_count {
        deck_ids.truncate(count);
    }
    let opp = &position.opponent;
    let recruit = opp.deck_list.is_none()
        && opp.pool.is_none()
        && position.mode.as_deref() == Some("recruit")
        && !outside(opp.seen.iter());
    let mut pool = if let Some(list) = &opp.deck_list {
        list.clone()
    } else if let Some(list) = &opp.pool {
        list.clone()
    } else if recruit {
        recruit_cards()
    } else {
        all_cards()
    };
    pool.sort();
    let mut candidates: Vec<_> = pool
        .iter()
        .filter(|id| !opp.seen.contains(id))
        .cloned()
        .collect();
    let count = opp.hand
        + opp.deck
        + position
            .locations
            .iter()
            .map(|l| l.theirs_face_down)
            .sum::<usize>();
    let hidden = if count == 0 {
        Vec::new()
    } else if candidates.len() >= count {
        if recruit && opp.prior.as_deref() != Some("uniform") {
            candidates.dedup();
            let weights = recruit_weights();
            let mut clocks: Vec<_> = candidates
                .iter()
                .map(|id| ((-rng.random()).ln_1p() / weights[id], id.clone()))
                .collect();
            clocks.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
            if count > clocks.len() {
                return Err("not enough distinct weighted cards".into());
            }
            clocks.into_iter().take(count).map(|(_, id)| id).collect()
        } else {
            sample(rng, &candidates, count)?
        }
    } else {
        if pool.is_empty() {
            return Err(format!(
                "opponent needs {count} hidden cards, but the pool is empty"
            ));
        }
        while candidates.len() < count {
            let index = (rng.random() * pool.len() as f64) as usize;
            candidates.push(pool[index].clone());
        }
        rng.shuffle(&mut candidates);
        candidates
    };
    let known_locations: Vec<_> = position
        .locations
        .iter()
        .filter_map(|l| l.id.as_ref())
        .collect();
    let recruit_locations = position.mode.as_deref() == Some("recruit")
        && known_locations.iter().all(|id| {
            LOCATION_DATA.get(id.as_str()).is_some_and(|d| {
                d.status == "released" && matches!(d.difficulty.as_str(), "Basic" | "Standard")
            })
        });
    let possible: Vec<_> = all_locations(recruit_locations)
        .into_iter()
        .filter(|id| !known_locations.contains(&id))
        .collect();
    let hidden_locations = sample(rng, &possible, 3 - known_locations.len())?;
    let mut selected = hidden_locations.into_iter();
    let lanes: [Lane; 3] = std::array::from_fn(|i| {
        let src = &position.locations[i];
        let id = src.id.clone().unwrap_or_else(|| selected.next().unwrap());
        let def = location_def(&id).unwrap_or(&UNKNOWN_LOCATION);
        Lane {
            def,
            observed_id: (def.id != id).then_some(id),
            revealed: src.id.is_some(),
            revealed_turn: src.id.as_ref().map(|_| {
                src.revealed_turn
                    .unwrap_or(((i + 1) as u8).min(position.turn))
            }),
            cards: Vec::new(),
            bonus: [0; 2],
            multiplier: [1; 2],
            state: src
                .ottos_lab_spent
                .map(|spent| Box::new(spent) as Box<dyn crate::game::CardState>)
                .or_else(|| def.initial_state.map(|init| init())),
        }
    });
    let priority = match position.priority.as_deref() {
        None => None,
        Some("me") => Some(0),
        Some("opponent") => Some(1),
        _ => return Err("invalid priority".into()),
    };
    let mut game = Game {
        cards: Vec::new(),
        players: [Player::default(), Player::default()],
        lanes,
        location_pool: all_locations(position.mode.as_deref() == Some("recruit"))
            .iter()
            .map(|id| location_def(id).expect("registered location"))
            .collect(),
        turn: position.turn,
        next_resolution_order: 1,
        is_wait_for_staging: true,
        reveal_priority: priority,
        total_turns: position.total_turns,
        played: Default::default(),
        pending_plays: Default::default(),
        played_on_reveal: [false; 2],
        delays_reveals: false,
        delayed_reveals: Default::default(),
        deferred_added_reveals: Vec::new(),
        current_location: 0,
        location_reveal_order: [0, 1, 2],
        destroyed: None,
        rng: RefCell::new(rng),
    };
    // Python's Game._from_position: revealed lanes count as already scheduled, and the
    // remaining reveals follow the hidden lanes in lane order.
    let mut order: Vec<usize> = (0..3).filter(|&lane| game.lanes[lane].revealed).collect();
    if let Some(snapped) = position.snapped_this_turn {
        for (seat, did_snap) in snapped.into_iter().enumerate() {
            if did_snap {
                game.players[seat].snapped_turns.push(position.turn);
            }
        }
    }
    game.current_location = order.len();
    order.extend((0..3).filter(|&lane| !game.lanes[lane].revealed));
    game.location_reveal_order = [order[0], order[1], order[2]];
    game.delays_reveals = game
        .lanes
        .iter()
        .any(|l| l.revealed && l.def.delays_reveals && l.revealed_turn == Some(position.turn));
    game.players[0].energy = position.energy;
    game.players[0].next_play_power = position.pending_power;
    game.players[1].next_play_power = opp.pending_power;
    let mut observed_refs: Vec<(usize, &CardRef)> = Vec::new();
    let mut powers = Vec::new();
    for r in &position.hand {
        let mut c = make_card(&r.id, 0)?;
        apply_card_reference(&mut c, r, position.turn);
        if let Some(cost) = r.cost {
            c.cost = cost;
        }
        if let Some(power) = r.power {
            c.permanent = power;
            if r.entity_id.is_some() {
                powers.push((game.cards.len(), power));
            }
        }
        if needs_observed_reference(r) {
            observed_refs.push((game.cards.len(), r));
        }
        game.players[0].hand.push(game.cards.len());
        game.cards.push(c);
    }
    for id in &deck_ids {
        game.players[0].deck.push(game.cards.len());
        game.cards.push(make_card(id, 0)?);
    }
    for (i, id) in hidden.iter().take(opp.hand + opp.deck).enumerate() {
        let index = game.cards.len();
        game.cards.push(make_card(id, 1)?);
        if i < opp.hand {
            game.players[1].hand.push(index);
        } else {
            game.players[1].deck.push(index);
        }
    }
    let delayed = position.turn.checked_sub(1).filter(|&t| {
        t > 0
            && position
                .locations
                .iter()
                .any(|l| l.id.as_deref() == Some("Lemuria") && l.revealed_turn == Some(t))
    });
    let mut hidden_board = hidden.iter().skip(opp.hand + opp.deck);
    let mut revealed = Vec::new();
    for (lane, src) in position.locations.iter().enumerate() {
        for (owner, refs) in [&src.mine, &src.theirs].into_iter().enumerate() {
            for r in refs {
                let mut c = make_card(&r.id, owner)?;
                c.cost = r.cost.unwrap_or(c.cost);
                c.lane = Some(lane);
                c.turn_played = r.played_turn.unwrap_or(0);
                c.revealed = r.revealed;
                apply_card_reference(&mut c, r, position.turn);
                c.observed_can_activate = r.can_activate;
                c.observed_move_targets = r.move_targets.as_ref().map(|lanes| {
                    lanes
                        .iter()
                        .filter(|&&lane| lane < 3)
                        .fold(0u8, |mask, &lane| mask | (1 << lane))
                });
                let id = game.cards.len();
                if needs_observed_reference(r) {
                    observed_refs.push((id, r));
                }
                game.cards.push(c);
                game.lanes[lane].cards.push(id);
                if r.revealed {
                    revealed.push(id);
                } else if r.played_turn == Some(position.turn) {
                    game.played[owner].push((id, lane));
                } else if delayed.is_some() && r.played_turn == delayed {
                    game.delayed_reveals[owner].push(id);
                }
                if let Some(power) = r.power {
                    powers.push((id, power));
                }
            }
        }
        for _ in 0..src.theirs_face_down {
            let mut c = make_card(hidden_board.next().unwrap(), 1)?;
            c.lane = Some(lane);
            let id = game.cards.len();
            if let Some(t) = delayed {
                c.turn_played = t;
                game.delayed_reveals[1].push(id);
            }
            game.cards.push(c);
            game.lanes[lane].cards.push(id);
        }
    }
    // The reader exposes these public zones without the lane of destruction.
    // Keep the lane unknown rather than manufacturing a location.
    for (owner, graveyard, banished) in [
        (
            0,
            position.graveyard.as_deref(),
            position.banished.as_deref(),
        ),
        (1, opp.graveyard.as_deref(), opp.banished.as_deref()),
    ] {
        for (is_graveyard, refs) in [(true, graveyard), (false, banished)] {
            for r in refs.unwrap_or_default() {
                let mut card = make_card(&r.id, owner)?;
                apply_card_reference(&mut card, r, position.turn);
                card.cost = r.cost.unwrap_or(card.cost);
                card.permanent = r.power.unwrap_or(card.permanent);
                card.revealed = r.revealed;
                card.turn_played = r.played_turn.unwrap_or(0);
                let id = game.cards.len();
                if needs_observed_reference(r) {
                    observed_refs.push((id, r));
                }
                game.cards.push(card);
                if is_graveyard {
                    game.players[owner].graveyard.push((id, None));
                } else {
                    game.players[owner].banished.push(id);
                }
            }
        }
    }
    let entities: HashMap<i64, usize> = observed_refs
        .iter()
        .filter_map(|&(id, r)| r.entity_id.map(|entity| (entity, id)))
        .collect();
    for &(id, r) in &observed_refs {
        game.cards[id].merged_to = r
            .merged_to_entity_id
            .filter(|&entity| entity != 0)
            .and_then(|entity| entities.get(&entity).copied());
    }
    // Use observed resolution order; use the historic deterministic approximation
    // only for cards whose order is unavailable.
    let mut resolution_order = revealed.clone();
    let first = priority.unwrap_or(0);
    resolution_order.sort_by_key(|&id| {
        let card = &game.cards[id];
        (
            card.turn_played,
            usize::from(card.owner != first),
            card.lane.unwrap_or(0),
            game.zone_position(id).unwrap_or(0),
        )
    });
    let explicit: HashMap<_, _> = observed_refs
        .iter()
        .filter_map(|(id, r)| r.latest_resolution_order.map(|order| (*id, order)))
        .collect();
    if explicit.is_empty() {
        for id in resolution_order {
            game.cards[id].latest_resolution_order = game.next_resolution_order;
            game.next_resolution_order += 1;
        }
    } else {
        let mut occupied: HashSet<u32> = explicit.values().copied().collect();
        for id in resolution_order {
            let order = if let Some(&order) = explicit.get(&id) {
                order
            } else {
                while occupied.contains(&game.next_resolution_order) {
                    game.next_resolution_order = game
                        .next_resolution_order
                        .checked_add(1)
                        .ok_or("resolution order overflow")?;
                }
                let order = game.next_resolution_order;
                occupied.insert(order);
                game.next_resolution_order =
                    order.checked_add(1).ok_or("resolution order overflow")?;
                order
            };
            game.cards[id].latest_resolution_order = order;
        }
        if let Some(&maximum) = occupied.iter().max() {
            game.next_resolution_order = game
                .next_resolution_order
                .max(maximum.checked_add(1).ok_or("resolution order overflow")?);
        }
    }
    for id in revealed {
        if matches!(game.cards[id].def.id, "Hawkeye" | "JessicaJones")
            && position.turn > 1
            && game.cards[id].turn_played == position.turn - 1
        {
            let lane = game.cards[id].lane.unwrap();
            let before = game.cards[id].turn_played;
            let active =
                game.lanes[lane].revealed && game.lanes[lane].revealed_turn.unwrap_or(0) <= before;
            let sources: Vec<_> = game.lanes[lane]
                .cards
                .iter()
                .copied()
                .filter(|&s| {
                    game.cards[s].revealed
                        && game.cards[s].has_ongoing()
                        && game.cards[s].turn_played < before
                })
                .collect();
            if sources
                .iter()
                .any(|&s| game.cards[s].ability_def().blocks_reveal)
            {
                continue;
            }
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
                if let Some(repetitions) = game.cards[source].ability_def().reveal_repetitions {
                    let extra: usize = sources
                        .iter()
                        .filter(|&&other| {
                            other != source && game.cards[other].owner == game.cards[source].owner
                        })
                        .map(|&other| game.cards[other].ability_def().extra_ongoing_runs as usize)
                        .sum();
                    runs += repetitions(&game, source, id) as usize * base * (1 + extra);
                }
            }
            let hawkeye = game.cards[id].def.id == "Hawkeye";
            for _ in 0..runs {
                game.cards[id].attach_watch(
                    id,
                    position.turn,
                    lane,
                    hawkeye,
                    if hawkeye { 4 } else { 5 },
                );
            }
        }
    }
    // The client CardStatUpdate exposes stat, operation, value, player and
    // updater IDs, but no ongoing flag. get_Power/get_Cost already report
    // totals: seed ledger rows as history, never reapply them as stat deltas.
    for &(id, reference) in &observed_refs {
        if let Some(draws) = &reference.turns_drawn {
            let owner = game.cards[id].owner;
            game.players[owner]
                .draws
                .extend(draws.iter().map(|&(turn, _)| (turn, id)));
        }
        if let Some(updates) = &reference.stat_updates {
            game.cards[id]
                .stat_updates
                .extend(updates.iter().filter_map(observed_update));
        }
    }
    for player in &mut game.players {
        player.draws.sort_by_key(|&(turn, _)| turn);
    }
    game.ongoing();
    // The reader does not label ongoing rows. An observed row is removed only
    // when its named active updater and an identical recomputed contribution
    // match; a multiset match preserves repeated independent contributions.
    // Faction, augment and league updaters are historical, never card/location ongoing.
    if observed_refs.iter().any(|(_, r)| {
        r.stat_updates
            .as_ref()
            .is_some_and(|updates| !updates.is_empty())
    }) {
        let mut active_cards: HashSet<_> = game
            .lanes
            .iter()
            .flat_map(|lane| &lane.cards)
            .filter_map(|&id| {
                let card = &game.cards[id];
                (card.revealed && card.has_ongoing()).then_some(card.def.id)
            })
            .collect();
        active_cards.extend(
            game.players
                .iter()
                .flat_map(|player| &player.hand)
                .filter_map(|&id| {
                    let card = &game.cards[id];
                    (!card.abilities_removed && card.ability_def().in_hand_ongoing.is_some())
                        .then_some(card.def.id)
                }),
        );
        let active_locations: HashSet<_> = game
            .lanes
            .iter()
            .filter(|lane| lane.revealed)
            .map(|lane| lane.def.id)
            .collect();
        let mut removed = false;
        for &(id, reference) in &observed_refs {
            let Some(observed) = &reference.stat_updates else {
                continue;
            };
            let mut used = HashSet::new();
            let mut remove = Vec::new();
            let mut historical = 0;
            for entry in observed {
                let Some(update) = observed_update(entry) else {
                    continue;
                };
                let is_external = entry.updater_faction_id.is_some()
                    || entry.updater_augment_id.is_some()
                    || entry.updater_league_id.is_some();
                let is_active_updater = !is_external
                    && (entry
                        .updater_card_id
                        .as_deref()
                        .is_some_and(|name| active_cards.contains(name))
                        || entry
                            .updater_location_id
                            .as_deref()
                            .is_some_and(|name| active_locations.contains(name)));
                if is_active_updater
                    && let Some((index, _)) =
                        game.cards[id]
                            .stat_updates
                            .iter()
                            .enumerate()
                            .find(|(index, current)| {
                                current.ongoing
                                    && !used.contains(index)
                                    && current.stat == update.stat
                                    && current.operation == update.operation
                                    && current.value == update.value
                                    && (update.source == usize::MAX
                                        || current.source == update.source)
                            })
                {
                    used.insert(index);
                    remove.push(historical);
                }
                historical += 1;
            }
            for index in remove.into_iter().rev() {
                game.cards[id].stat_updates.remove(index);
                removed = true;
            }
        }
        if removed {
            game.ongoing();
        }
    }
    for (id, power) in powers {
        game.cards[id].permanent = power - game.cards[id].ongoing - game.cards[id].location_power;
    }
    for (index, r) in position.hand.iter().enumerate() {
        if let Some(observed) = r.cost {
            let id = game.players[0].hand[index];
            let printed = CARD_DATA[&r.id].cost;
            let mut costs: Vec<_> = (0..observed + 3).collect();
            costs.sort_by_key(|&cost| (cost != printed, (cost - observed).abs()));
            let mut found = false;
            for cost in costs {
                game.cards[id].cost = cost;
                if game.card_cost(id) == observed {
                    found = true;
                    break;
                }
            }
            if !found {
                game.cards[id].cost = observed;
            }
        }
    }
    game.players[1].energy = opp.energy.unwrap_or_else(|| {
        position.turn as i32
            + game
                .lanes
                .iter()
                .enumerate()
                .filter(|(_, l)| l.revealed)
                .map(|(i, l)| {
                    (if l.revealed_turn == Some(position.turn) {
                        l.def.reveal_energy
                    } else {
                        0
                    }) + l.def.energy_bonus.map_or(0, |f| f(&game, i, 1))
                })
                .sum::<i32>()
    });
    Ok(game)
}

/// Canonical, ownership-preserving materialized state for cross-language parity.
pub fn state_json(game: &Game<'_>) -> Value {
    let card = |id: usize| {
        let c = &game.cards[id];
        json!({"id": c.def.id, "owner": c.owner, "cost": game.card_cost(id), "power": c.power(), "revealed": c.revealed, "turn_played": c.turn_played, "watches": c.watch_contexts()})
    };
    json!({ "turn": game.turn, "total_turns": game.total_turns, "priority": game.reveal_priority,
        "players": game.players.iter().map(|p| json!({"energy": p.energy, "pending_power": p.next_play_power, "hand": p.hand.iter().map(|&id| card(id)).collect::<Vec<_>>(), "deck": p.deck.iter().map(|&id| game.cards[id].def.id).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "locations": game.lanes.iter().map(|l| json!({"id": l.observed_id.as_deref().unwrap_or(l.def.id), "revealed": l.revealed, "revealed_turn": l.revealed_turn, "spent": l.state.as_deref().and_then(|s| s.as_any().downcast_ref::<[bool;2]>()), "cards": l.cards.iter().map(|&id| card(id)).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "pending_reveals": game.delayed_reveals.iter().map(|p| p.iter().map(|&id| json!([game.cards[id].def.id, game.cards[id].lane])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "played": game.played.iter().map(|p| p.iter().map(|&(id, lane)| json!([game.cards[id].def.id, lane])).collect::<Vec<_>>()).collect::<Vec<_>>() })
}
