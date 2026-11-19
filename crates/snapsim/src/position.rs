//! Reconstruct a planning state from the Python advisor's public position format.
use crate::{
    cards::{self, CardDef},
    data,
    game::{Card, Game, Lane, Player},
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
}
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
}
#[derive(Clone, Debug, Deserialize, Serialize)]
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
        owner,
        cost: row.cost,
        created: false,
        permanent: row.power,
        ongoing: 0,
        location_power: 0,
        lane: None,
        revealed: false,
        abilities_removed: false,
        repeating: false,
        turn_played: 0,
        watches: Vec::new(),
        state: def.initial_state.map(|init| init()),
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
                .map(|spent| Box::new(spent) as Box<dyn std::any::Any>)
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
        reveal_priority: priority,
        total_turns: position.total_turns,
        played: Default::default(),
        pending_plays: Default::default(),
        played_on_reveal: [false; 2],
        delays_reveals: false,
        delayed_reveals: Default::default(),
        current_location: 0,
        location_reveal_order: [0, 1, 2],
        destroyed: None,
        rng: RefCell::new(rng),
    };
    // Python's Game._from_position: revealed lanes count as already scheduled, and the
    // remaining reveals follow the hidden lanes in lane order.
    let mut order: Vec<usize> = (0..3).filter(|&lane| game.lanes[lane].revealed).collect();
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
    for r in &position.hand {
        let mut c = make_card(&r.id, 0)?;
        if let Some(cost) = r.cost {
            c.cost = cost;
        }
        if let Some(power) = r.power {
            c.permanent = power;
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
    let mut powers = Vec::new();
    let mut revealed = Vec::new();
    for (lane, src) in position.locations.iter().enumerate() {
        for (owner, refs) in [&src.mine, &src.theirs].into_iter().enumerate() {
            for r in refs {
                let mut c = make_card(&r.id, owner)?;
                c.cost = r.cost.unwrap_or(c.cost);
                c.lane = Some(lane);
                c.turn_played = r.played_turn.unwrap_or(0);
                c.revealed = r.revealed;
                let id = game.cards.len();
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
            if sources.iter().any(|&s| game.cards[s].def.blocks_reveal) {
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
                if let Some(repetitions) = game.cards[source].def.reveal_repetitions {
                    let extra: usize = sources
                        .iter()
                        .filter(|&&other| {
                            other != source && game.cards[other].owner == game.cards[source].owner
                        })
                        .map(|&other| game.cards[other].def.extra_ongoing_runs as usize)
                        .sum();
                    runs += repetitions(&game, source, id) as usize * base * (1 + extra);
                }
            }
            game.cards[id].watches = vec![(position.turn, lane); runs];
        }
    }
    game.ongoing();
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
        json!({"id": c.def.id, "owner": c.owner, "cost": game.card_cost(id), "power": c.power(), "revealed": c.revealed, "turn_played": c.turn_played, "watches": c.watches})
    };
    json!({ "turn": game.turn, "total_turns": game.total_turns, "priority": game.reveal_priority,
        "players": game.players.iter().map(|p| json!({"energy": p.energy, "pending_power": p.next_play_power, "hand": p.hand.iter().map(|&id| card(id)).collect::<Vec<_>>(), "deck": p.deck.iter().map(|&id| game.cards[id].def.id).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "locations": game.lanes.iter().map(|l| json!({"id": l.observed_id.as_deref().unwrap_or(l.def.id), "revealed": l.revealed, "revealed_turn": l.revealed_turn, "spent": l.state.as_ref().and_then(|s| s.downcast_ref::<[bool;2]>()), "cards": l.cards.iter().map(|&id| card(id)).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "pending_reveals": game.delayed_reveals.iter().map(|p| p.iter().map(|&id| json!([game.cards[id].def.id, game.cards[id].lane])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "played": game.played.iter().map(|p| p.iter().map(|&(id, lane)| json!([game.cards[id].def.id, lane])).collect::<Vec<_>>()).collect::<Vec<_>>() })
}
