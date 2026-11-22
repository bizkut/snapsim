//! Coverage and hidden-information sampling metadata for public positions.
use crate::{
    cards, data, locations,
    position::{self, Position},
};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashSet};
use std::sync::LazyLock;

static RECRUIT_LOCATIONS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    data::locations()
        .into_iter()
        .filter(|d| {
            !d.carddefid.is_empty()
                && d.status == "released"
                && matches!(d.difficulty.as_str(), "Basic" | "Standard")
        })
        .map(|d| d.carddefid)
        .collect()
});

pub fn supported_cards() -> Vec<&'static str> {
    let mut ids: Vec<_> = cards::registry().map(|d| d.id).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}
pub fn supported_locations() -> Vec<&'static str> {
    let mut ids: Vec<_> = locations::REGISTRY.iter().map(|d| d.id).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}
pub fn supported() -> Value {
    json!({"cards": supported_cards(), "locations": supported_locations()})
}

fn opponent_candidates(position: &Position) -> (&'static str, Vec<String>) {
    let opponent = &position.opponent;
    let (source, pool) = if let Some(pool) = &opponent.deck_list {
        ("deck_list", pool.clone())
    } else if let Some(pool) = &opponent.pool {
        ("pool", pool.clone())
    } else if position.mode.as_deref() == Some("recruit")
        && !position::outside(opponent.seen.iter().chain(&opponent.known))
    {
        ("mode:recruit", position::recruit_cards())
    } else {
        ("all-supported", position::all_cards())
    };
    let excluded: HashSet<_> = opponent.seen.iter().chain(&opponent.known).collect();
    let mut remaining: Vec<_> = pool
        .into_iter()
        .filter(|id| !excluded.contains(id))
        .collect();
    remaining.sort();
    (source, remaining)
}

pub fn sampling_priors(position: &Position) -> Value {
    let (source, cards) = opponent_candidates(position);
    let revealed: HashSet<_> = position
        .locations
        .iter()
        .filter_map(|l| l.id.as_ref())
        .collect();
    let recruit = position.mode.as_deref() == Some("recruit")
        && revealed.iter().all(|id| RECRUIT_LOCATIONS.contains(*id));
    let loc_source = if recruit {
        "mode:recruit"
    } else {
        "all-supported"
    };
    let loc_count = position::all_locations(recruit)
        .iter()
        .filter(|id| !revealed.contains(id))
        .count();
    let missing = position
        .deck_count
        .unwrap_or(position.deck.len())
        .saturating_sub(position.deck.len());
    let (deck_source, deck_size) = if missing == 0 {
        ("listed", position.deck.len())
    } else {
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
        let recruit = position.mode.as_deref() == Some("recruit")
            && !position::outside(known.iter().copied());
        let pool = if recruit {
            position::recruit_cards()
        } else {
            position::all_cards()
        };
        (
            if recruit {
                "mode:recruit"
            } else {
                "all-supported"
            },
            pool.iter().filter(|id| !known.contains(id)).count(),
        )
    };
    json!({"opponent_cards": {"source":source,"size":cards.len(),"weighting":if source == "mode:recruit" && position.opponent.prior.as_deref() != Some("uniform") {"weighted"} else {"uniform"}}, "hidden_locations":{"source":loc_source,"size":loc_count}, "own_deck":{"source":deck_source,"size":deck_size}})
}

pub fn coverage(position: &Position) -> Value {
    let mut card_ids: BTreeSet<&str> = position
        .hand
        .iter()
        .map(|c| c.id.as_str())
        .chain(position.deck.iter().map(String::as_str))
        .chain(position.opponent.seen.iter().map(String::as_str))
        .chain(position.opponent.known.iter().map(String::as_str))
        .chain(
            position
                .opponent
                .deck_list
                .iter()
                .flatten()
                .map(String::as_str),
        )
        .chain(position.opponent.pool.iter().flatten().map(String::as_str))
        .collect();
    let mut loc_ids = BTreeSet::new();
    let mut state = BTreeSet::new();
    let mut extra_state = BTreeSet::new();
    // A ledger entry proves augments are in use, but it does not enumerate
    // attached abilities that have not produced a stat update yet.
    if !cards::reference_rules_enabled()
        && position
            .hand
            .iter()
            .chain(
                position
                    .locations
                    .iter()
                    .flat_map(|lane| lane.mine.iter().chain(&lane.theirs)),
            )
            .any(|card| {
                card.stat_updates.as_ref().is_some_and(|updates| {
                    updates
                        .iter()
                        .any(|update| update.updater_augment_id.is_some())
                })
            })
        && (!position.hand.is_empty()
            || position.locations.iter().any(|lane| !lane.mine.is_empty()))
    {
        extra_state.insert("Card augment attachments are not exported".to_owned());
    }
    for id in &position.opponent.seen {
        if matches!(id.as_str(), "IronFist" | "Enchantress") {
            state.insert(id.as_str());
        }
    }
    let previous = position.turn - 1;
    let delayed = previous > 0
        && position
            .locations
            .iter()
            .any(|l| l.id.as_deref() == Some("Lemuria") && l.revealed_turn == Some(previous));
    let mut face_down = 0;
    for lane in &position.locations {
        if position.turn == 5
            && lane
                .mine
                .iter()
                .any(|card| card.revealed && card.id == "Daredevil")
        {
            extra_state.insert("Daredevil-visible opponent plays not exported".to_owned());
        }
        face_down += lane.theirs_face_down;
        let refs: Vec<_> = lane
            .mine
            .iter()
            .map(|c| (0, c))
            .chain(lane.theirs.iter().map(|c| (1, c)))
            .collect();
        for (_, card) in &refs {
            let id = card.id.as_str();
            card_ids.insert(id);
            if card.revealed {
                if id == "HumanTorchFirstSteps"
                    && card.played_turn.is_none_or(|turn| turn < position.turn)
                {
                    extra_state.insert("Human Torch First Steps doubling use unknown".to_owned());
                }
                if id == "PowerStone" {
                    extra_state
                        .insert("Power Stone distinct played-stone history unknown".to_owned());
                }
                if card.played_turn.is_none_or(|turn| turn < position.turn) {
                    if id == "JuggernautHorseman" {
                        extra_state
                            .insert("Juggernaut Horseman objective completion unknown".to_owned());
                    }
                    if id == "Nebula" {
                        extra_state.insert("Nebula pre-end trigger state unknown".to_owned());
                    } else if id == "TheThingFirstSteps" {
                        extra_state
                            .insert("The Thing First Steps fill-trigger use unknown".to_owned());
                    }
                }
                if matches!(id, "IronFist" | "Enchantress")
                    || (card.played_turn.is_none() && matches!(id, "Hawkeye" | "JessicaJones"))
                {
                    state.insert(id);
                }
            } else if card.played_turn != Some(position.turn)
                && !(delayed && card.played_turn == Some(previous))
            {
                state.insert(id);
            }
        }
        for (owner, target) in &refs {
            if !target.revealed
                || target.played_turn != Some(previous)
                || !matches!(target.id.as_str(), "Hawkeye" | "JessicaJones")
            {
                continue;
            }
            for (source_owner, source) in &refs {
                if std::ptr::eq(*target, *source) || !source.revealed {
                    continue;
                }
                let repeats =
                    source.id == "JoaquinTorres" && source_owner == owner && target.id == "Hawkeye";
                let amplified = repeats
                    && refs.iter().any(|(other_owner, other)| {
                        !std::ptr::eq(*other, *source)
                            && other.revealed
                            && other.id == "Onslaught"
                            && other_owner == source_owner
                            && (other.played_turn.is_none() || other.played_turn == Some(previous))
                    });
                if amplified
                    || ((source.played_turn.is_none() || source.played_turn == Some(previous))
                        && (source.id == "Cosmo" || repeats))
                {
                    state.insert(target.id.as_str());
                    break;
                }
            }
        }
        if let Some(id) = &lane.id {
            loc_ids.insert(id.as_str());
            if id == "TimeTheater" && position.turn < position.total_turns {
                if !position.deck.is_empty() && position.hand.len() < 7 {
                    extra_state.insert("Time Theater use unknown for seat 0".to_owned());
                }
                if position.opponent.deck > 0 && position.opponent.hand < 7 {
                    extra_state.insert("Time Theater use unknown for seat 1".to_owned());
                }
            }
            if id == "QuantumRealm" {
                for (_, card) in &refs {
                    if card.revealed
                        && card.played_turn.is_none_or(|turn| {
                            turn < position.turn
                                && lane.revealed_turn.is_none_or(|reveal| turn >= reveal)
                        })
                    {
                        extra_state.insert(format!("Quantum Realm base unknown for {}", card.id));
                    }
                }
            }
            if matches!(id.as_str(), "TheAbbey" | "Raft")
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
            {
                extra_state.insert(format!("{id} consumed trigger state unknown"));
            }
            if (lane.revealed_turn.is_none()
                && matches!(
                    id.as_str(),
                    "Lemuria" | "Sakaar" | "TinkerersWorkshop" | "ProjectPegasus"
                ))
                || (id == "OttosLab" && lane.ottos_lab_spent.is_none())
                // These per-player trigger/action histories are not in live observations.
                || matches!(id.as_str(), "Vormir" | "CollapsedMine")
            {
                state.insert(id.as_str());
            }
        }
    }
    let unknown_cards: Vec<_> = card_ids
        .into_iter()
        .filter(|id| cards::get(id).is_none())
        .collect();
    let unknown_locations: Vec<_> = loc_ids
        .into_iter()
        .filter(|id| locations::get(id).is_none())
        .collect();
    let deck = position
        .deck_count
        .filter(|&n| n != position.deck.len())
        .map(|n| json!({"listed":position.deck.len(),"counted":n}));
    let hidden = position.opponent.hand + position.opponent.deck + face_down;
    let unseen = opponent_candidates(position).1.len();
    let opponent = (hidden > unseen).then(|| json!({"hidden":hidden,"unseen_in_pool":unseen}));
    let state: BTreeSet<String> = state
        .into_iter()
        .map(str::to_owned)
        .chain(extra_state)
        .collect();
    json!({"cards":unknown_cards,"locations":unknown_locations,"state":state,"deck":deck,"opponent":opponent,"face_down":if delayed || face_down == 0 {None} else {Some(face_down)}})
}
