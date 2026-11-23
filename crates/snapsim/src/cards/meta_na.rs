//! Rules that use immutable discard and destruction event histories.
use super::CardDef;
use crate::game::{CardState, Event, Game};

pub(crate) static TOKENS: &[&str] = &["DoomBot2099"];

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        ongoing: Some(knull),
        ongoing_ability: true,
        ..CardDef::vanilla("Knull")
    },
    CardDef {
        ongoing: Some(muse),
        ongoing_ability: true,
        ..CardDef::vanilla("Muse")
    },
    CardDef {
        in_hand_ongoing: Some(stature),
        ..CardDef::vanilla("Stature")
    },
    CardDef {
        reveal: Some(wiccan),
        ..CardDef::vanilla("Wiccan")
    },
    CardDef {
        end_turn: Some(lockheed),
        end_turn_trigger: true,
        ..CardDef::vanilla("Lockheed")
    },
    CardDef {
        post_start_turn: Some(jim_hammond),
        ..CardDef::vanilla("JimHammond")
    },
    CardDef {
        end_turn: Some(doom_2099),
        end_turn_trigger: true,
        ..CardDef::vanilla("Doom2099")
    },
    CardDef {
        ongoing: Some(doom_bot_2099),
        ongoing_ability: true,
        ..CardDef::vanilla("DoomBot2099")
    },
    CardDef {
        reveal: Some(hela),
        ..CardDef::vanilla("Hela")
    },
    CardDef {
        reveal: Some(elixir),
        ..CardDef::vanilla("Elixir")
    },
    CardDef {
        reveal: Some(death_fractured),
        ..CardDef::vanilla("DeathFracturedFrontier")
    },
    CardDef {
        ongoing: Some(speed),
        ongoing_ability: true,
        ..CardDef::vanilla("Speed")
    },
    CardDef {
        reveal: Some(starlord_sun),
        ..CardDef::vanilla("StarlordMasterOfTheSun")
    },
    CardDef {
        reveal: Some(triton),
        ..CardDef::vanilla("Triton")
    },
    CardDef {
        in_hand_ongoing: Some(she_hulk_cost),
        on_event: Some(she_hulk_event),
        initial_state: Some(she_hulk_state),
        end_turn_trigger: true,
        end_turn_trigger_hidden: true,
        ..CardDef::vanilla("SheHulk")
    },
    CardDef {
        on_event: Some(galacta),
        initial_state: Some(galacta_state),
        ..CardDef::vanilla("Galacta")
    },
    CardDef {
        on_event: Some(storm_horseman),
        initial_state: Some(storm_horseman_state),
        ..CardDef::vanilla("StormHorseman")
    },
    CardDef {
        reveal: Some(chameleon),
        ..CardDef::vanilla("Chameleon")
    },
    CardDef {
        reveal: Some(leader),
        ..CardDef::vanilla("Leader")
    },
];

// KnullBehavior::GetPowerChange 0x7ee1774 sums recorded death powers,
// not the current powers or the contents of the graveyard.
fn knull(game: &mut Game<'_>, card: usize) {
    let amount = game
        .players
        .iter()
        .flat_map(|p| &p.destruction_history)
        .map(|event| event.power)
        .sum();
    game.add_ongoing_power(card, amount, game.cards[card].owner);
}

// MuseBehavior::BuffSelf 0x7f0774c counts distinct allied death locations.
fn muse(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut lanes = [false; 3];
    for event in &game.players[owner].destruction_history {
        if let Some(lane) = event.lane.filter(|&lane| lane < lanes.len()) {
            lanes[lane] = true;
        }
    }
    game.add_ongoing_power(
        card,
        3 * lanes.into_iter().filter(|&seen| seen).count() as i32,
        owner,
    );
}

// StatureBehavior::ReduceCost 0x7cc56b4 sets cost to one after an enemy discard.
fn stature(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if !game.players[1 - owner].discard_history.is_empty() {
        game.set_ongoing_cost(card, 1, owner);
    }
}

// WiccanBehavior::AddPermanentEnergy 0x7cf26f8 invokes
// Player::SpentAllEnergyPreviousTurns and adds two permanent energy.
fn wiccan(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if (1..game.turn).all(|turn| {
        game.players[owner]
            .energy_history
            .iter()
            .any(|&(recorded, _, all)| recorded == turn && all)
    }) {
        game.add_permanent_energy(owner, 2);
    }
}

// LockheedBehavior::AfflictCard 0x7ee95d0; predicate 0x7ee9b50
// uses the definition recorded in CardPlayed, even after transformation.
fn lockheed(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if !game.players[owner]
        .resolved_play_history
        .iter()
        .any(|event| event.turn == game.turn && event.printed_cost == 1)
    {
        return;
    }
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&target| game.cards[target].owner != owner && game.cards[target].revealed)
        .collect();
    if !targets.is_empty() {
        let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
        game.add_power(target, -2, owner);
    }
}

// JimHammondBehavior::BoostPower 0x7ed4390 requires a location and
// positive Player.BonusEnergy, adding that much power before staging.
fn jim_hammond(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let bonus = game.bonus_energy(owner);
    if game.cards[card].lane.is_some() && bonus > 0 {
        game.add_power(card, bonus, owner);
    }
}

// Doom2099Behavior::AddDoomBot 0x7e8f5b0 counts staged plays, not
// surviving board cards, and chooses among spawnable locations.
fn doom_2099(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner]
        .staged_history
        .iter()
        .filter(|play| play.turn == game.turn)
        .count()
        != 1
    {
        return;
    }
    let bot = game.create_card("DoomBot2099", owner);
    let lanes: Vec<_> = (0..3)
        .filter(|&lane| game.can_add(bot, lane, owner))
        .collect();
    if lanes.is_empty() {
        game.cards.pop();
    } else {
        let lane = lanes[game.rng.borrow_mut().randbelow(lanes.len())];
        game.add_card(bot, lane, owner, true);
    }
}

// DoomBot2099Behavior::BuffDooms 0x7e9005c and selector 0x7e9057c
// use the definition's Card_Modified list, excluding the source itself.
fn doom_bot_2099(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&target| {
            target != card
                && game.cards[target].owner == owner
                && game.cards[target].revealed
                && matches!(
                    game.cards[target].def.id,
                    "Doom2099" | "DrDoom" | "DoomBot" | "DoomBot2099"
                )
        })
        .collect();
    for target in targets {
        game.add_ongoing_power(target, 1, owner);
    }
}

// HelaBehavior::ResurrectDiscardedCardsByCost 0x7ebd5d4 snapshots the
// discard zone, visits distinct printed costs in insertion order, selects
// a still-discarded card for each, then chooses a spawnable location.
fn hela(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let discarded = game.players[owner].discard.clone();
    let mut costs = Vec::new();
    for &id in &discarded {
        let cost = game.printed_cost(id);
        if !costs.contains(&cost) {
            costs.push(cost);
        }
    }
    for cost in costs {
        let candidates: Vec<_> = discarded
            .iter()
            .copied()
            .filter(|&id| {
                game.printed_cost(id) == cost && game.players[owner].discard.contains(&id)
            })
            .collect();
        if candidates.is_empty() {
            continue;
        }
        let target = candidates[game.rng.borrow_mut().randbelow(candidates.len())];
        let lanes: Vec<_> = (0..3)
            .filter(|&lane| game.can_add(target, lane, owner))
            .collect();
        if !lanes.is_empty() {
            let lane = lanes[game.rng.borrow_mut().randbelow(lanes.len())];
            game.resurrect_from_zone(target, lane, owner);
        }
    }
}

// ElixirBehavior::ReviveDestroyedCard 0x7e9fc44 and ReviveCandidates
// 0x7e9fb08 sample last-turn destruction events still in a graveyard.
fn elixir(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let candidates: Vec<_> = game.players[owner]
        .destruction_history
        .iter()
        .filter(|event| event.turn.checked_add(1) == Some(game.turn))
        .map(|event| event.card)
        .filter(|&id| {
            game.players.iter().any(|player| {
                player.discard.contains(&id)
                    || player.graveyard.iter().any(|&(other, _)| other == id)
            })
        })
        .collect();
    if !candidates.is_empty() {
        let target = candidates[game.rng.borrow_mut().randbelow(candidates.len())];
        game.resurrect_from_zone(target, lane, owner);
    }
}

// DeathFracturedFrontierBehavior::ReviveAllDestroyedCards 0x7e89b84
// snapshots the owner's destroyed graveyard and chooses a location per card.
fn death_fractured(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let destroyed: Vec<_> = game.players[owner]
        .graveyard
        .iter()
        .map(|&(id, _)| id)
        .collect();
    for target in destroyed {
        if !game.is_character(target)
            || !game.players[owner]
                .graveyard
                .iter()
                .any(|&(id, _)| id == target)
        {
            continue;
        }
        let lanes: Vec<_> = (0..3)
            .filter(|&lane| game.can_add(target, lane, owner))
            .collect();
        if !lanes.is_empty() {
            let lane = lanes[game.rng.borrow_mut().randbelow(lanes.len())];
            game.resurrect_from_zone(target, lane, owner);
        }
    }
}

// SpeedBehavior::GetPowerChange 0x7f46858 counts ended turns without
// excess, and adjusts the current turn using the current Energy value.
fn speed(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let prior = game.players[owner]
        .energy_history
        .iter()
        .filter(|&&(turn, _, all)| turn < game.turn && all)
        .count() as i32;
    let current = i32::from(game.players[owner].energy == 0);
    game.add_ongoing_power(card, prior + current, owner);
}

// StarlordMasterOfTheSunBehavior::AddEnergyTriggerToPlayer 0x7cc51d0
// schedules one energy per previous turn ended with excess.
fn starlord_sun(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let amount = game.players[owner]
        .turn_energy_history
        .iter()
        .filter(|event| event.turn < game.turn && event.remaining > 0)
        .count() as i32;
    if amount > 0 {
        game.schedule_energy(owner, game.turn + 1, amount);
    }
}

// TritonBehavior::GetLastPlayedByPlayersLastTurn 0x7ce2660 selects
// the last revealed character in each player's prior-turn staging order.
fn triton(game: &mut Game<'_>, card: usize) {
    let Some(previous) = game.turn.checked_sub(1) else {
        return;
    };
    let first = game.reveal_priority.unwrap_or(0);
    let targets: Vec<_> = [first, 1 - first]
        .into_iter()
        .filter_map(|seat| {
            game.players[seat]
                .staged_history
                .iter()
                .rev()
                .find(|play| {
                    play.turn == previous
                        && game.cards[play.card].revealed
                        && game.is_character(play.card)
                })
                .map(|play| play.card)
        })
        .collect();
    let owner = game.cards[card].owner;
    for target in targets {
        game.add_power(target, 3, owner);
    }
}

#[derive(Clone, Default)]
struct SheHulkState {
    remaining: i32,
}

fn she_hulk_state() -> Box<dyn CardState> {
    Box::new(SheHulkState::default())
}

// SheHulkBehavior::GetRemainingEnergy 0x7f3a208 stores the owner's
// end-of-turn energy on this instance for ReduceCost 0x7f3a40c.
fn she_hulk_event(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::TurnEnd) {
        return;
    }
    let remaining = game.players[game.cards[card].owner].energy;
    if let Some(state) = game.cards[card]
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<SheHulkState>())
    {
        state.remaining = remaining.max(0);
    }
}

fn she_hulk_cost(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let remaining = game.cards[card]
        .ability_state()
        .and_then(|state| state.as_any().downcast_ref::<SheHulkState>())
        .map_or(0, |state| state.remaining);
    game.add_ongoing_cost(card, -remaining, owner);
}

#[derive(Clone, Default)]
struct GalactaState {
    triggered_turn: Option<u8>,
}

fn galacta_state() -> Box<dyn CardState> {
    Box::new(GalactaState::default())
}

// GalactaBehavior::BuffCard 0x7ead88c uses per-instance TriggerActivatedTurn,
// first accounting for earlier resolved plays at other lanes still in play.
fn galacta(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::StagedCardResolving {
        card: target,
        seat,
        lane,
    } = event
    else {
        return;
    };
    let Some(source_lane) = game.cards[card].lane else {
        return;
    };
    if seat != game.cards[card].owner || lane == source_lane || target == card {
        return;
    }
    let used = game.cards[card]
        .ability_state()
        .and_then(|state| state.as_any().downcast_ref::<GalactaState>())
        .is_some_and(|state| state.triggered_turn == Some(game.turn));
    let prior = game.players[seat].resolved_play_history.iter().any(|play| {
        play.turn == game.turn
            && play.card != card
            && play.staged_lane.is_some_and(|l| l != source_lane)
            && game.cards[play.card].lane.is_some()
    });
    if used {
        return;
    }
    if let Some(state) = game.cards[card]
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<GalactaState>())
    {
        state.triggered_turn = Some(game.turn);
        if !prior {
            game.add_power(target, 2, seat);
        }
    }
}

#[derive(Clone, Default)]
struct StormHorsemanState {
    completed: bool,
}

fn storm_horseman_state() -> Box<dyn CardState> {
    Box::new(StormHorsemanState::default())
}

// StormHorsemanBehavior::CalculateObjectiveProgress 0x7cc801c counts
// the longest consecutive run of turns ended with energy. AddPower
// 0x7cc8358 completes once at PostEndOfTurn.
fn storm_horseman(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::AfterTurnEnd) {
        return;
    }
    let owner = game.cards[card].owner;
    if game.cards[card].lane.is_none() && !game.players[owner].hand.contains(&card) {
        return;
    }
    let history = &game.players[owner].turn_energy_history;
    let reached = history.windows(2).any(|pair| {
        pair[0].remaining > 0
            && pair[1].remaining > 0
            && pair[0].turn.checked_add(1) == Some(pair[1].turn)
    });
    if !reached {
        return;
    }
    let Some(state) = game.cards[card]
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<StormHorsemanState>())
    else {
        return;
    };
    if state.completed {
        return;
    }
    state.completed = true;
    let mut targets = game.players[owner].hand.clone();
    if let Some(lane) = game.cards[card].lane {
        targets.extend(
            game.lanes[lane]
                .cards
                .iter()
                .copied()
                .filter(|&target| game.cards[target].revealed && game.cards[target].owner == owner),
        );
    }
    for target in targets {
        if target != card {
            game.add_power(target, 2, owner);
        }
    }
}

// ChameleonBehavior::CopyText 0x7e80194 samples other allied cards here
// revealed this turn, staged plays first, then other revealed additions.
fn chameleon(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let mut candidates: Vec<_> = game.players[owner]
        .staged_history
        .iter()
        .filter(|play| play.turn == game.turn)
        .map(|play| play.card)
        .filter(|&target| {
            target != card
                && game.cards[target].lane == Some(lane)
                && game.cards[target].turn_revealed == Some(game.turn)
                && game.cards[target].merged_to.is_none()
        })
        .collect();
    for &target in &game.lanes[lane].cards {
        if target != card
            && game.cards[target].owner == owner
            && game.cards[target].revealed
            && game.cards[target].turn_revealed == Some(game.turn)
            && !candidates.contains(&target)
        {
            candidates.push(target);
        }
    }
    if !candidates.is_empty() {
        let target = candidates[game.rng.borrow_mut().randbelow(candidates.len())];
        game.copy_text(card, target, true);
    }
}

// LeaderBehavior::CopyPowerfulOpposingCards 0x7ee6ffc snapshots the
// opposing staged highest-power cards, including ties, then copies each to
// its current location or its immediately previous location.
fn leader(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let enemy = 1 - owner;
    let staged = &game.players[enemy].staged_history;
    let Some(maximum) = staged
        .iter()
        .filter(|play| play.turn == game.turn && game.is_character(play.card))
        .map(|play| game.cards[play.card].power())
        .max()
    else {
        return;
    };
    let targets: Vec<_> = staged
        .iter()
        .filter(|play| {
            play.turn == game.turn
                && game.is_character(play.card)
                && game.cards[play.card].power() == maximum
        })
        .map(|play| play.card)
        .collect();
    for target in targets {
        let lane = game.cards[target]
            .lane
            .or_else(|| match game.cards[target].previous_zone {
                Some(crate::zones::CardZone::Location(lane)) => Some(lane),
                _ => None,
            });
        if let Some(lane) = lane {
            let copy = game.copy_card(target, owner);
            if !game.add_card(copy, lane, owner, true) {
                game.cards.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{game::CardZoneEvent, random::PythonRandom};

    fn setup(rng: &mut PythonRandom) -> Game<'_> {
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned)
        .to_vec();
        let lanes = ["Ruins", "Xandar", "KamarTaj"].map(str::to_owned).to_vec();
        Game::new([&deck, &deck], &lanes, rng).unwrap()
    }
    fn event(card: usize, lane: Option<usize>, power: i32) -> CardZoneEvent {
        CardZoneEvent {
            turn: 1,
            card,
            power,
            lane,
        }
    }
    #[test]
    fn knull_sums_snapshots_across_both_players() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("Knull", 0);
        game.players[0]
            .destruction_history
            .push(event(card, Some(0), 7));
        game.players[1]
            .destruction_history
            .push(event(card, Some(1), -2));
        knull(&mut game, card);
        assert_eq!(game.cards[card].ongoing, 5);
    }
    #[test]
    fn muse_counts_each_allied_lane_only_once() {
        let mut rng = PythonRandom::seed(2);
        let mut game = setup(&mut rng);
        let card = game.create_card("Muse", 0);
        game.players[0].destruction_history.extend([
            event(card, Some(0), 1),
            event(card, Some(0), 2),
            event(card, Some(2), 1),
        ]);
        game.players[1]
            .destruction_history
            .push(event(card, Some(1), 1));
        muse(&mut game, card);
        assert_eq!(game.cards[card].ongoing, 6);
    }
    #[test]
    fn stature_uses_opponent_discard_events() {
        let mut rng = PythonRandom::seed(3);
        let mut game = setup(&mut rng);
        let card = game.create_card("Stature", 0);
        game.players[0].hand.push(card);
        game.players[1].discard_history.push(event(card, None, 1));
        stature(&mut game, card);
        assert_eq!(game.card_cost(card), 1);
    }
    #[test]
    fn wiccan_requires_every_previous_turn_to_be_fully_spent() {
        let mut rng = PythonRandom::seed(4);
        let mut game = setup(&mut rng);
        let card = game.create_card("Wiccan", 0);
        game.turn = 3;
        game.players[0]
            .energy_history
            .extend([(1, 1, true), (2, 1, false)]);
        wiccan(&mut game, card);
        assert_eq!(game.players[0].permanent_energy, 0);
        game.players[0].energy_history[1].2 = true;
        wiccan(&mut game, card);
        assert_eq!(game.players[0].permanent_energy, 2);
    }
    #[test]
    fn lockheed_uses_immutable_printed_cost() {
        let mut rng = PythonRandom::seed(5);
        let mut game = setup(&mut rng);
        let card = game.create_card("Lockheed", 0);
        let enemy = game.create_card("Cyclops", 1);
        let played = game.create_card("Hulk", 0);
        game.add_card(card, 0, 0, false);
        game.add_card(enemy, 0, 1, false);
        game.cards[enemy].revealed = true;
        game.players[0]
            .resolved_play_history
            .push(crate::game::ResolvedPlayEvent {
                turn: game.turn,
                card: played,
                def_id: "Wasp",
                printed_cost: 1,
                staged_lane: Some(1),
                lane: Some(1),
                power: Some(1),
            });
        let before = game.cards[enemy].power();
        lockheed(&mut game, card);
        assert_eq!(game.cards[enemy].power(), before - 2);
    }
    #[test]
    fn jim_hammond_uses_bonus_after_default_energy() {
        let mut rng = PythonRandom::seed(6);
        let mut game = setup(&mut rng);
        let card = game.create_card("JimHammond", 0);
        game.add_card(card, 0, 0, false);
        game.turn = 3;
        game.players[0].energy = 5;
        let before = game.cards[card].power();
        jim_hammond(&mut game, card);
        assert_eq!(game.cards[card].power(), before + 2);
    }
    #[test]
    fn doom_counts_staged_plays_even_after_they_leave() {
        let mut rng = PythonRandom::seed(7);
        let mut game = setup(&mut rng);
        let card = game.create_card("Doom2099", 0);
        let played = game.create_card("Wasp", 0);
        game.add_card(card, 0, 0, false);
        game.players[0]
            .staged_history
            .push(crate::game::StagedPlay {
                turn: game.turn,
                card: played,
                def_id: "Wasp",
                printed_cost: 1,
                created: false,
                resolved: true,
            });
        doom_2099(&mut game, card);
        assert!(
            game.lanes
                .iter()
                .flat_map(|lane| &lane.cards)
                .any(|&id| game.cards[id].def.id == "DoomBot2099")
        );
    }
    #[test]
    fn doom_bot_buffs_other_allied_dooms() {
        let mut rng = PythonRandom::seed(8);
        let mut game = setup(&mut rng);
        let bot = game.create_card("DoomBot2099", 0);
        let doom = game.create_card("DrDoom", 0);
        game.add_card(bot, 0, 0, false);
        game.add_card(doom, 1, 0, false);
        game.cards[doom].revealed = true;
        game.ongoing();
        assert_eq!(game.cards[doom].ongoing, 1);
        assert_eq!(game.cards[bot].ongoing, 0);
    }
    #[test]
    fn hela_revives_one_card_per_printed_cost() {
        let mut rng = PythonRandom::seed(9);
        let mut game = setup(&mut rng);
        let card = game.create_card("Hela", 0);
        let first = game.create_card("MistyKnight", 0);
        let second = game.create_card("Hawkeye", 0);
        let third = game.create_card("Cyclops", 0);
        game.players[0].discard.extend([first, second, third]);
        game.players[0]
            .graveyard_order
            .extend([first, second, third]);
        hela(&mut game, card);
        assert_eq!(
            [first, second]
                .into_iter()
                .filter(|&id| game.cards[id].lane.is_some())
                .count(),
            1
        );
        assert!(game.cards[third].lane.is_some());
    }
    #[test]
    fn elixir_revives_last_turns_destroyed_card() {
        let mut rng = PythonRandom::seed(10);
        let mut game = setup(&mut rng);
        game.turn = 2;
        let card = game.create_card("Elixir", 0);
        let target = game.create_card("Cyclops", 0);
        game.add_card(card, 0, 0, false);
        game.players[0].graveyard.push((target, Some(1)));
        game.players[0]
            .destruction_history
            .push(event(target, Some(1), 4));
        elixir(&mut game, card);
        assert_eq!(game.cards[target].lane, Some(0));
    }
    #[test]
    fn fractured_death_resurrects_destroyed_characters() {
        let mut rng = PythonRandom::seed(11);
        let mut game = setup(&mut rng);
        let card = game.create_card("DeathFracturedFrontier", 0);
        let first = game.create_card("Wasp", 0);
        let second = game.create_card("Cyclops", 0);
        game.players[0]
            .graveyard
            .extend([(first, Some(0)), (second, Some(1))]);
        death_fractured(&mut game, card);
        assert!(game.cards[first].lane.is_some());
        assert!(game.cards[second].lane.is_some());
    }
    #[test]
    fn speed_counts_prior_and_current_fully_spent_turns() {
        let mut rng = PythonRandom::seed(12);
        let mut game = setup(&mut rng);
        let card = game.create_card("Speed", 0);
        game.turn = 3;
        game.players[0]
            .energy_history
            .extend([(1, 1, true), (2, 1, false)]);
        game.players[0].energy = 0;
        speed(&mut game, card);
        assert_eq!(game.cards[card].ongoing, 2);
    }
    #[test]
    fn starlord_schedules_energy_for_unspent_turns() {
        let mut rng = PythonRandom::seed(13);
        let mut game = setup(&mut rng);
        let card = game.create_card("StarlordMasterOfTheSun", 0);
        game.turn = 3;
        game.players[0].turn_energy_history.extend([
            crate::game::TurnEnergyEvent {
                turn: 1,
                spent: 0,
                remaining: 1,
                bonus: 0,
            },
            crate::game::TurnEnergyEvent {
                turn: 2,
                spent: 2,
                remaining: 0,
                bonus: 0,
            },
        ]);
        starlord_sun(&mut game, card);
        assert_eq!(game.players[0].scheduled_energy, [(4, 1)]);
    }
    #[test]
    fn triton_uses_last_revealed_character_in_staging_order() {
        let mut rng = PythonRandom::seed(14);
        let mut game = setup(&mut rng);
        game.turn = 2;
        let card = game.create_card("Triton", 0);
        let first = game.create_card("Wasp", 0);
        let last = game.create_card("Cyclops", 0);
        game.add_card(first, 0, 0, false);
        game.add_card(last, 1, 0, false);
        game.cards[first].revealed = true;
        game.cards[last].revealed = true;
        for id in [first, last] {
            game.players[0]
                .staged_history
                .push(crate::game::StagedPlay {
                    turn: 1,
                    card: id,
                    def_id: game.cards[id].def.id,
                    printed_cost: game.printed_cost(id),
                    created: false,
                    resolved: true,
                });
        }
        let before = game.cards[last].power();
        triton(&mut game, card);
        assert_eq!(game.cards[last].power(), before + 3);
    }
    #[test]
    fn she_hulk_remembers_energy_only_after_its_end_turn_event() {
        let mut rng = PythonRandom::seed(16);
        let mut game = setup(&mut rng);
        let card = game.create_card("SheHulk", 0);
        game.players[0].hand.push(card);
        game.players[0].energy = 2;
        she_hulk_event(&mut game, card, Event::TurnEnd);
        she_hulk_cost(&mut game, card);
        assert_eq!(game.card_cost(card), 4);
    }
    #[test]
    fn galacta_buffs_only_first_other_lane_staged_play() {
        let mut rng = PythonRandom::seed(17);
        let mut game = setup(&mut rng);
        let card = game.create_card("Galacta", 0);
        let first = game.create_card("Wasp", 0);
        let second = game.create_card("Cyclops", 0);
        game.add_card(card, 0, 0, false);
        game.add_card(first, 1, 0, false);
        game.add_card(second, 2, 0, false);
        let before = game.cards[first].power();
        let second_before = game.cards[second].power();
        galacta(
            &mut game,
            card,
            Event::StagedCardResolving {
                card: first,
                seat: 0,
                lane: 1,
            },
        );
        galacta(
            &mut game,
            card,
            Event::StagedCardResolving {
                card: second,
                seat: 0,
                lane: 2,
            },
        );
        assert_eq!(game.cards[first].power(), before + 2);
        assert_eq!(game.cards[second].power(), second_before);
    }
    #[test]
    fn storm_completes_once_after_two_consecutive_excess_turns() {
        let mut rng = PythonRandom::seed(18);
        let mut game = setup(&mut rng);
        let card = game.create_card("StormHorseman", 0);
        let target = game.create_card("Cyclops", 0);
        game.players[0].hand.extend([card, target]);
        game.players[0].turn_energy_history.extend([
            crate::game::TurnEnergyEvent {
                turn: 1,
                spent: 0,
                remaining: 1,
                bonus: 0,
            },
            crate::game::TurnEnergyEvent {
                turn: 2,
                spent: 1,
                remaining: 1,
                bonus: 0,
            },
        ]);
        let before = game.cards[target].power();
        storm_horseman(&mut game, card, Event::AfterTurnEnd);
        storm_horseman(&mut game, card, Event::AfterTurnEnd);
        assert_eq!(game.cards[target].power(), before + 2);
    }
    #[test]
    fn chameleon_copies_allied_card_revealed_this_turn() {
        let mut rng = PythonRandom::seed(19);
        let mut game = setup(&mut rng);
        let card = game.create_card("Chameleon", 0);
        let source = game.create_card("IronMan", 0);
        game.add_card(card, 0, 0, false);
        game.add_card(source, 0, 0, false);
        game.cards[source].revealed = true;
        game.cards[source].turn_revealed = Some(game.turn);
        chameleon(&mut game, card);
        assert_eq!(game.cards[card].default_ability().id, "IronMan");
    }
    #[test]
    fn leader_copies_highest_opposing_staged_cards_to_their_lanes() {
        let mut rng = PythonRandom::seed(20);
        let mut game = setup(&mut rng);
        let card = game.create_card("Leader", 0);
        let low = game.create_card("Wasp", 1);
        let high = game.create_card("Cyclops", 1);
        game.add_card(low, 0, 1, false);
        game.add_card(high, 1, 1, false);
        for id in [low, high] {
            game.players[1]
                .staged_history
                .push(crate::game::StagedPlay {
                    turn: game.turn,
                    card: id,
                    def_id: game.cards[id].def.id,
                    printed_cost: game.printed_cost(id),
                    created: false,
                    resolved: true,
                });
        }
        leader(&mut game, card);
        assert!(game.lanes[1].cards.iter().any(|&id| id != high
            && game.cards[id].owner == 0
            && game.cards[id].def.id == "Cyclops"));
        assert!(
            !game.lanes[0]
                .cards
                .iter()
                .any(|&id| id != low && game.cards[id].owner == 0)
        );
    }
}
