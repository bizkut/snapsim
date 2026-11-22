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

// Player-owned triggers survive their source. A card's staging turn is not a
// reveal turn (e.g. Dark Dimension), so only resolved action history can
// establish expiration or consumption.
fn installed_effect_possible(position: &Position, id: &str) -> bool {
    let actions = position
        .history
        .as_ref()
        .and_then(|history| history.actions.as_ref());
    let active = |entity: Option<i64>| {
        if entity.is_none() {
            return true;
        }
        let Some(action) = actions.and_then(|actions| {
            actions.iter().find(|action| {
                action.card_id == id
                    && action.kind == "play"
                    && action.resolved
                    && entity.is_none_or(|entity| action.card_entity_id == entity)
            })
        }) else {
            return true;
        };
        // A later re-reveal is not an action history entry. Only use the
        // original play's time if the observed latest resolution still agrees.
        let latest = position
            .locations
            .iter()
            .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
            .chain(position.graveyard.iter().flatten())
            .chain(position.banished.iter().flatten())
            .chain(position.opponent.graveyard.iter().flatten())
            .chain(position.opponent.banished.iter().flatten())
            .find(|card| card.entity_id == entity)
            .and_then(|card| card.latest_resolution_order);
        if latest.is_none() || latest != action.resolution_order {
            return true;
        }
        let Some(turn) = action.turn_resolved else {
            return true;
        };
        if matches!(id, "Wave" | "Psylocke") {
            return turn.saturating_add(1) >= position.turn;
        }
        let seat = if action.owner == "local" { 0 } else { 1 };
        let Some(history) = position.complete_past_actions(seat) else {
            return true;
        };
        let Some(order) = action.resolution_order else {
            return true;
        };
        !history.iter().any(|later| {
            later.kind == "play"
                && later.resolved
                && later
                    .turn_resolved
                    .zip(later.resolution_order)
                    .is_some_and(|(t, o)| (t, o) > (turn, order))
        })
    };
    let visible = position
        .locations
        .iter()
        .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
        .filter(|card| card.revealed && card.id == id)
        .chain(
            position
                .graveyard
                .iter()
                .flatten()
                .filter(|card| card.id == id),
        )
        .chain(
            position
                .banished
                .iter()
                .flatten()
                .filter(|card| card.id == id),
        )
        .chain(
            position
                .opponent
                .graveyard
                .iter()
                .flatten()
                .filter(|card| card.id == id),
        )
        .chain(
            position
                .opponent
                .banished
                .iter()
                .flatten()
                .filter(|card| card.id == id),
        );
    if visible.into_iter().any(|card| active(card.entity_id)) {
        return true;
    }
    if let Some(actions) = actions
        && actions
            .iter()
            .filter(|action| action.card_id == id && action.kind == "play" && action.resolved)
            .any(|action| active(Some(action.card_entity_id)))
    {
        return true;
    }
    position.opponent.seen.iter().any(|seen| seen == id)
        && !actions.is_some_and(|actions| {
            actions
                .iter()
                .any(|action| action.card_id == id && action.kind == "play" && action.resolved)
        })
        && !position
            .locations
            .iter()
            .flat_map(|lane| &lane.theirs)
            .any(|card| card.id == id)
}

// These registered effects can morph locations. The observation currently has
// no location-change event history or previous location identity.
fn location_change_evidence(position: &Position) -> bool {
    let changes = |id: &str| {
        matches!(
            id,
            "Magik" | "ScarletWitch" | "RealityStone" | "NicoMinoru" | "OmniversalPresence"
        )
    };
    position.locations.iter().any(|lane| {
        lane.mine
            .iter()
            .chain(&lane.theirs)
            .any(|card| card.revealed && changes(&card.id))
            || lane
                .id
                .as_deref()
                .is_some_and(|id| matches!(id, "WorldForge" | "LosDiablos"))
    }) || position
        .graveyard
        .iter()
        .flatten()
        .chain(position.banished.iter().flatten())
        .chain(position.opponent.graveyard.iter().flatten())
        .chain(position.opponent.banished.iter().flatten())
        .any(|card| changes(&card.id))
        || position.opponent.seen.iter().any(|id| changes(id))
        || position
            .history
            .as_ref()
            .and_then(|history| history.actions.as_ref())
            .is_some_and(|actions| {
                actions
                    .iter()
                    .any(|action| action.resolved && changes(&action.card_id))
            })
}

fn text_change_evidence(position: &Position) -> bool {
    let changes = |id: &str| {
        matches!(
            id,
            "Enchantress"
                | "Echo"
                | "DeafeningChord"
                | "IronLad"
                | "NicoMinoru"
                | "SymbioteSpiderMan"
                | "TaoMandala"
                | "AncientOne"
                | "AstralProjection"
                | "TechnoOrganicVirus"
                | "Magus"
        )
    };
    position.locations.iter().any(|lane| {
        lane.mine
            .iter()
            .chain(&lane.theirs)
            .any(|card| card.revealed && changes(&card.id))
    }) || position
        .graveyard
        .iter()
        .flatten()
        .chain(position.banished.iter().flatten())
        .chain(position.opponent.graveyard.iter().flatten())
        .chain(position.opponent.banished.iter().flatten())
        .any(|card| changes(&card.id))
        || position.opponent.seen.iter().any(|id| changes(id))
        || position
            .history
            .as_ref()
            .and_then(|history| history.actions.as_ref())
            .is_some_and(|actions| {
                actions
                    .iter()
                    .any(|action| action.resolved && changes(&action.card_id))
            })
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
    if !cards::reference_rules_enabled() && position.turn > 1 && location_change_evidence(position)
    {
        extra_state.insert(
            "a location may have been replaced (pending location grants unknown)".to_owned(),
        );
    }
    // Stones may originate from Thanos even when Thanos himself is not visible.
    // The opponent's post-shuffle deck composition is not exported.
    let stone = |id: &str| {
        matches!(
            id,
            "SpaceStone" | "TimeStone" | "RealityStone" | "PowerStone" | "MindStone" | "SoulStone"
        )
    };
    let own_stone = position.hand.iter().any(|card| stone(&card.id))
        || position.deck.iter().any(|id| stone(id))
        || position
            .graveyard
            .iter()
            .flatten()
            .any(|card| stone(&card.id))
        || position
            .banished
            .iter()
            .flatten()
            .any(|card| stone(&card.id))
        || position
            .locations
            .iter()
            .any(|lane| lane.mine.iter().any(|card| stone(&card.id)));
    let opposing_stone = position
        .opponent
        .seen
        .iter()
        .chain(&position.opponent.known)
        .chain(position.opponent.deck_list.iter().flatten())
        .chain(position.opponent.pool.iter().flatten())
        .any(|id| stone(id))
        || position
            .opponent
            .graveyard
            .iter()
            .flatten()
            .any(|card| stone(&card.id))
        || position
            .opponent
            .banished
            .iter()
            .flatten()
            .any(|card| stone(&card.id))
        || position
            .locations
            .iter()
            .any(|lane| lane.theirs.iter().any(|card| stone(&card.id)));
    if !cards::reference_rules_enabled()
        && ((own_stone && position.deck_count != Some(position.deck.len())) || opposing_stone)
    {
        extra_state
            .insert("Infinity Stone owner's post-shuffle deck composition unknown".to_owned());
    }
    // ThorBehavior::AddMjolnir (0x7cdcc10) changes deck
    // composition, which is not exported for the opponent.
    if !cards::reference_rules_enabled() {
        let own_thor = position.locations.iter().any(|lane| {
            lane.mine
                .iter()
                .any(|card| card.id == "Thor" && card.revealed)
        }) || position
            .graveyard
            .iter()
            .flatten()
            .any(|card| card.id == "Thor")
            || position
                .banished
                .iter()
                .flatten()
                .any(|card| card.id == "Thor")
            || position
                .history
                .as_ref()
                .and_then(|history| history.actions.as_ref())
                .is_some_and(|actions| {
                    actions.iter().any(|action| {
                        action.owner == "local"
                            && action.card_id == "Thor"
                            && action.kind == "play"
                            && action.resolved
                    })
                });
        let opposing_thor = position.locations.iter().any(|lane| {
            lane.theirs
                .iter()
                .any(|card| card.id == "Thor" && card.revealed)
        }) || position.opponent.seen.iter().any(|id| id == "Thor");
        if (own_thor && position.deck_count != Some(position.deck.len())) || opposing_thor {
            extra_state.insert("Thor Mjolnir post-shuffle deck composition unknown".to_owned());
        }
        let widow_seen = position.locations.iter().any(|lane| {
            lane.mine
                .iter()
                .any(|card| card.id == "BlackWidow" && card.revealed)
        }) || position
            .graveyard
            .iter()
            .flatten()
            .any(|card| card.id == "BlackWidow")
            || position
                .banished
                .iter()
                .flatten()
                .any(|card| card.id == "BlackWidow")
            || position
                .history
                .as_ref()
                .and_then(|history| history.actions.as_ref())
                .is_some_and(|actions| {
                    actions
                        .iter()
                        .any(|action| action.owner == "local" && action.card_id == "BlackWidow")
                });
        if position.opponent.hand > 0 && widow_seen {
            extra_state.insert("Black Widow opponent WidowsBite hand fate unknown".to_owned());
        }
    }
    // Darkhawk counts cards, not identities: a reported own deck count is
    // sufficient even when the deck composition is sampled.
    if !cards::reference_rules_enabled()
        && position.deck_count.is_none()
        && (position
            .opponent
            .seen
            .iter()
            .chain(&position.opponent.known)
            .chain(position.opponent.deck_list.iter().flatten())
            .chain(position.opponent.pool.iter().flatten())
            .any(|id| id == "Darkhawk")
            || position
                .locations
                .iter()
                .any(|lane| lane.theirs.iter().any(|card| card.id == "Darkhawk")))
    {
        extra_state.insert("Darkhawk opponent deck count unknown".to_owned());
    }
    if !cards::reference_rules_enabled() {
        let own_cobra_in_hand = position.hand.iter().any(|card| card.id == "Cobra");
        let unknown_target = |cards: &[position::CardRef]| {
            cards
                .iter()
                .any(|card| card.revealed && card.created.is_none())
        };
        if position.locations.iter().any(|lane| {
            ((own_cobra_in_hand
                || lane
                    .mine
                    .iter()
                    .any(|card| card.id == "Cobra" && card.revealed))
                && unknown_target(&lane.theirs))
                || (lane
                    .theirs
                    .iter()
                    .any(|card| card.id == "Cobra" && card.revealed)
                    && unknown_target(&lane.mine))
        }) {
            extra_state.insert("Cobra target created provenance unknown".to_owned());
        }
    }
    if !cards::reference_rules_enabled() {
        let own = position.hand.iter().any(|card| card.id == "Diamondback")
            || position.deck.iter().any(|id| id == "Diamondback");
        let opponent = position
            .opponent
            .known
            .iter()
            .chain(&position.opponent.seen)
            .chain(position.opponent.deck_list.iter().flatten())
            .chain(position.opponent.pool.iter().flatten())
            .any(|id| id == "Diamondback");
        let unknown =
            |card: &position::CardRef| card.revealed && !card.has_complete_power_modifier_history();
        if position.locations.iter().any(|lane| {
            (own && lane.theirs.iter().any(unknown)) || (opponent && lane.mine.iter().any(unknown))
        }) {
            extra_state.insert("Diamondback target power modifier history incomplete".to_owned());
        }
    }
    if !cards::reference_rules_enabled() {
        let own = position.hand.iter().any(|card| card.id == "Patriot")
            || position.deck.iter().any(|id| id == "Patriot")
            || position
                .locations
                .iter()
                .any(|lane| lane.mine.iter().any(|card| card.id == "Patriot"));
        if own && text_change_evidence(position) {
            extra_state.insert("card text overrides not exported".to_owned());
        }
        let opponent = position
            .opponent
            .known
            .iter()
            .chain(&position.opponent.seen)
            .chain(position.opponent.deck_list.iter().flatten())
            .chain(position.opponent.pool.iter().flatten())
            .any(|id| id == "Patriot")
            || position
                .locations
                .iter()
                .any(|lane| lane.theirs.iter().any(|card| card.id == "Patriot"));
        let unknown = |card: &position::CardRef| {
            card.revealed && cards::no_text::has_no_text(&card.id).is_none()
        };
        if position.locations.iter().any(|lane| {
            (own && lane.mine.iter().any(unknown)) || (opponent && lane.theirs.iter().any(unknown))
        }) {
            extra_state.insert("Patriot target client behavior definition unknown".to_owned());
        }
    }
    if !cards::reference_rules_enabled() {
        for id in ["Wave", "Stick", "Psylocke"] {
            if installed_effect_possible(position, id) {
                extra_state.insert(format!("{id} effect may be active"));
            }
        }
    }
    if !cards::reference_rules_enabled() && position.turn > 1 {
        let own_infinaut = position.hand.iter().any(|card| card.id == "Infinaut")
            || position.deck.iter().any(|id| id == "Infinaut");
        let opposing_infinaut = position
            .opponent
            .known
            .iter()
            .chain(&position.opponent.seen)
            .chain(position.opponent.deck_list.iter().flatten())
            .chain(position.opponent.pool.iter().flatten())
            .any(|id| id == "Infinaut");
        for (seat, relevant) in [(0, own_infinaut), (1, opposing_infinaut)] {
            let complete = position.history.as_ref().is_some_and(|history| {
                history.actions.is_some()
                    && history
                        .action_completeness
                        .as_ref()
                        .is_some_and(|completeness| {
                            let turns = if seat == 0 {
                                &completeness.local
                            } else {
                                &completeness.opponent
                            };
                            turns.get(&(position.turn - 1).to_string()) == Some(&true)
                        })
            });
            if relevant && !complete {
                extra_state.insert(format!(
                    "Infinaut seat {seat} previous-turn staged play history unknown"
                ));
            }
        }
        let miles = position.hand.iter().any(|card| card.id == "MilesMorales")
            || position.deck.iter().any(|id| id == "MilesMorales")
            || position
                .opponent
                .known
                .iter()
                .chain(&position.opponent.seen)
                .chain(position.opponent.deck_list.iter().flatten())
                .chain(position.opponent.pool.iter().flatten())
                .any(|id| id == "MilesMorales");
        if miles {
            extra_state.insert(
                "Miles Morales previous-turn forced movement history not exported".to_owned(),
            );
        }
    }
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
            .any(|card| match &card.augments {
                Some(augments) if augments.complete && augments.items.is_empty() => false,
                Some(augments) if !augments.items.is_empty() => true,
                _ => card.stat_updates.as_ref().is_some_and(|updates| {
                    updates
                        .iter()
                        .any(|update| update.updater_augment_id.is_some())
                }),
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
        for (owner, card) in &refs {
            let id = card.id.as_str();
            card_ids.insert(id);
            if card.revealed {
                if id == "HumanTorchFirstSteps"
                    && card.played_turn.is_none_or(|turn| turn < position.turn)
                {
                    extra_state.insert("Human Torch First Steps doubling use unknown".to_owned());
                }
                if id == "PowerStone"
                    && (cards::reference_rules_enabled()
                        || position.complete_past_actions(*owner).is_none())
                {
                    extra_state
                        .insert("Power Stone distinct played-stone history unknown".to_owned());
                }
                if card.played_turn.is_none_or(|turn| turn < position.turn) {
                    if id == "JuggernautHorseman"
                        && (cards::reference_rules_enabled() || card.objective().is_none())
                    {
                        extra_state
                            .insert("Juggernaut Horseman objective completion unknown".to_owned());
                    }
                    if id == "Nebula"
                        && (cards::reference_rules_enabled() || card.nebula_active().is_none())
                    {
                        extra_state.insert("Nebula pre-end trigger state unknown".to_owned());
                    } else if id == "TheThingFirstSteps"
                        && (cards::reference_rules_enabled() || card.thing_fill_used().is_none())
                    {
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
            // A previous location morph may leave queued grants; public evidence of
            // morphing effects is gated separately above.
            if matches!(id.as_str(), "Feast" | "AltarOfDeath")
                && position.turn > 1
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
                && !cards::reference_rules_enabled()
            {
                extra_state.insert(format!(
                    "{id} prior-turn pending player energy grants not exported"
                ));
            }
            if id == "FracturedFrontier" && !cards::reference_rules_enabled() {
                if lane.revealed_turn.is_none() {
                    extra_state.insert("Fractured Frontier reveal turn unknown".to_owned());
                }
                if lane.revealed_turn.is_none_or(|turn| turn == position.turn) {
                    extra_state.insert(
                        "Fractured Frontier draw-completed turn and bottom hand order not exported"
                            .to_owned(),
                    );
                }
            }
            if !cards::reference_rules_enabled()
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
                && (id == "WhiteHotRoom" || id == "SakaarGrandPrix" && position.turn > 4)
            {
                extra_state.insert(format!(
                    "{id} prior permanent max energy grants not exported"
                ));
            }
            if id == "WhiteHotRoom"
                && !cards::reference_rules_enabled()
                && lane
                    .history_state
                    .as_ref()
                    .and_then(|state| state.is_given)
                    .is_none()
            {
                extra_state.insert("White Hot Room IsGiven trigger state unknown".to_owned());
            }
            if id == "ValleyOfTheHand" && position.turn <= 5 && !cards::reference_rules_enabled() {
                extra_state.insert(
                    "Valley of the Hand complete destroyed-card graveyard state not exported"
                        .to_owned(),
                );
            }
            if id == "Clubhouse"
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
                && !cards::reference_rules_enabled()
            {
                extra_state
                    .insert("Clubhouse per-player fill trigger state not exported".to_owned());
            }
            if id == "TimeTheater" && position.turn < position.total_turns {
                let spent = (!cards::reference_rules_enabled())
                    .then(|| {
                        lane.history_state
                            .as_ref()?
                            .time_theater_used
                            .as_ref()?
                            .complete()
                    })
                    .flatten();
                if !position.deck.is_empty()
                    && position.hand.len() < 7
                    && spent.is_none_or(|flags| !flags[0])
                {
                    extra_state.insert("Time Theater use unknown for seat 0".to_owned());
                }
                if position.opponent.deck > 0
                    && position.opponent.hand < 7
                    && spent.is_none_or(|flags| !flags[1])
                {
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
                        && (cards::reference_rules_enabled()
                            || card.verified_base_power().is_none())
                    {
                        extra_state.insert(format!("Quantum Realm base unknown for {}", card.id));
                    }
                }
            }
            if matches!(id.as_str(), "TheAbbey" | "Raft")
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
                && (cards::reference_rules_enabled()
                    || lane
                        .history_state
                        .as_ref()
                        .and_then(|h| h.is_given)
                        .is_none())
            {
                extra_state.insert(format!("{id} consumed trigger state unknown"));
            }
            if (lane.revealed_turn.is_none()
                && matches!(
                    id.as_str(),
                    "Lemuria" | "Sakaar" | "TinkerersWorkshop" | "ProjectPegasus"
                ))
                || (id == "OttosLab" && lane.ottos_lab_spent.is_none())
                || (id == "Vormir"
                    && (cards::reference_rules_enabled()
                        || !lane
                            .history_state
                            .as_ref()
                            .and_then(|h| h.vormir_consumed.as_ref())
                            .is_some_and(|v| v.local.is_some() && v.opponent.is_some())))
                || (id == "CollapsedMine"
                    && (cards::reference_rules_enabled()
                        || (0..2).any(|seat| position.current_action_present(seat).is_none())))
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

#[cfg(test)]
mod location_change_tests {
    use super::*;

    #[test]
    fn public_morphing_card_on_either_side_marks_possible_grants() {
        for seat in 0..2 {
            let mut position = Position {
                turn: 3,
                ..Position::default()
            };
            let card = position::CardRef {
                id: "ScarletWitch".to_owned(),
                revealed: true,
                ..position::CardRef::default()
            };
            if seat == 0 {
                position.locations[0].mine.push(card);
            } else {
                position.locations[0].theirs.push(card);
            }
            assert!(location_change_evidence(&position));
            assert!(
                coverage(&position)["state"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| {
                        value
                            == "a location may have been replaced (pending location grants unknown)"
                    })
            );
        }
    }
    #[test]
    fn patriot_with_public_text_changer_declines_unexported_overrides() {
        let mut position = Position {
            turn: 3,
            ..Position::default()
        };
        position.hand.push(position::CardRef {
            id: "Patriot".into(),
            ..Default::default()
        });
        assert!(
            !coverage(&position)["state"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "card text overrides not exported")
        );
        position.locations[0].theirs.push(position::CardRef {
            id: "Enchantress".into(),
            revealed: true,
            ..Default::default()
        });
        assert!(
            coverage(&position)["state"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "card text overrides not exported")
        );
    }
}
