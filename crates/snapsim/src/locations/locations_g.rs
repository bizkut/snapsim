//! Location rules from the 59.15.1 client.
use super::{LocationDef, location};
use crate::game::{Event, Game};

pub const CAMELOT: LocationDef = LocationDef {
    on_turn_end: Some(camelot),
    ..location("Camelot", 0, 0, 1, false)
};
pub const WARRIOR_FALLS: LocationDef = LocationDef {
    on_turn_end: Some(warrior_falls),
    ..location("WarriorFalls", 0, 0, 1, false)
};
pub const RICKETY_BRIDGE: LocationDef = LocationDef {
    on_turn_end: Some(rickety_bridge),
    ..location("RicketyBridge", 0, 0, 1, false)
};
// EndGame 0x7d7e0fc and 0x7d8f724.
pub const SANDCASTLE: LocationDef = LocationDef {
    maximum_turns: 5,
    ..location("Sandcastle", 0, 0, 1, false)
};
pub const TVA: LocationDef = LocationDef {
    maximum_turns: 4,
    ..location("Tva", 0, 0, 1, false)
};
// AddPowerToOtherLocations 0x7d8b0b4 propagates each player's location power.
pub const THE_NEXUS: LocationDef = LocationDef {
    contributes_to_other_lanes: true,
    ..location("TheNexus", 0, 0, 1, false)
};
pub const VALHALLA: LocationDef = LocationDef {
    on_turn_end: Some(valhalla),
    ..location("Valhalla", 0, 0, 1, false)
};
pub const SUPERFLOW: LocationDef = LocationDef {
    energy_bonus: Some(superflow),
    ..location("Superflow", 0, 0, 1, false)
};
pub const EMPIRE_STATE_UNIVERSITY: LocationDef = LocationDef {
    energy_bonus: Some(empire_state_university),
    ..location("EmpireStateUniversity", 0, 0, 1, false)
};
pub const CLUBHOUSE: LocationDef = LocationDef {
    initial_state: Some(|| Box::new([false; 2])),
    on_turn_start: Some(clubhouse),
    on_event: Some(clubhouse_event),
    ..location("Clubhouse", 0, 0, 1, false)
};
pub const ESSEXS_LAB: LocationDef = LocationDef {
    on_turn_end: Some(essexs_lab),
    ..location("EssexsLab", 0, 0, 1, false)
};
pub const FEAST: LocationDef = LocationDef {
    on_event: Some(feast),
    ..location("Feast", 0, 0, 1, false)
};
pub const ALTAR_OF_DEATH: LocationDef = LocationDef {
    on_event: Some(altar_of_death),
    ..location("AltarOfDeath", 0, 0, 1, false)
};
pub const SMITHSONIAN_MUSEUM: LocationDef = LocationDef {
    ongoing: Some(smithsonian_museum),
    ..location("SmithsonianMuseum", 0, 0, 1, false)
};
pub const PARKER_INDUSTRIES: LocationDef = LocationDef {
    allows_play_globally_with_count: Some(parker_industries),
    ..location("ParkerIndustries", 0, 2, 1, false)
};
pub const FRACTURED_FRONTIER: LocationDef = LocationDef {
    modify_cost: Some(fractured_frontier),
    ..location("FracturedFrontier", 0, 0, 1, false)
};
pub const VALLEY_OF_THE_HAND: LocationDef = LocationDef {
    on_turn_end: Some(valley_of_the_hand),
    ..location("ValleyOfTheHand", 0, 0, 1, false)
};
pub const SAKAAR_GRAND_PRIX: LocationDef = LocationDef {
    on_turn_end: Some(sakaar_grand_prix),
    ..location("SakaarGrandPrix", 0, 0, 1, false)
};
pub const WHITE_HOT_ROOM: LocationDef = LocationDef {
    initial_state: Some(|| Box::new(false)),
    on_turn_start: Some(white_hot_room),
    on_event: Some(white_hot_room_event),
    ..location("WhiteHotRoom", 0, 0, 1, false)
};
pub const WEAPON_X_FACILITY: LocationDef = LocationDef {
    on_reveal: Some(weapon_x_facility),
    ..location("WeaponXFacility", 0, 0, 1, false)
};

// DiscardAndDraw 0x7d963ec randomly discards one hand card per player,
// then draws only if that discard succeeded.
fn weapon_x_facility(game: &mut Game<'_>, _: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        let count = game.players[seat].hand.len();
        if count == 0 {
            continue;
        }
        let choice = game.rng.borrow_mut().randbelow(count);
        let card = game.players[seat].hand[choice];
        if game.discard_card(seat, card) {
            game.draw(seat);
        }
    }
}

// AddPermanentEnergyToWinner 0x7d7b074 grants the winner +1 after turn 4.
fn sakaar_grand_prix(game: &mut Game<'_>, lane: usize) {
    if game.turn == 4 {
        let powers = game.powers()[lane];
        if powers[0] != powers[1] {
            game.add_permanent_energy(usize::from(powers[1] > powers[0]), 1);
        }
    }
}

// CheckLocationForPermanentEnergy 0x7d981fc uses one shared IsGiven
// context, processing both players who qualify before consuming it.
fn white_hot_room(game: &mut Game<'_>, lane: usize) {
    let spent = *game.lanes[lane]
        .state
        .as_deref()
        .unwrap()
        .as_any()
        .downcast_ref::<bool>()
        .unwrap();
    if spent {
        return;
    }
    let mut qualified = [false; 2];
    for (seat, eligible) in qualified.iter_mut().enumerate() {
        *eligible = game.lanes[lane]
            .cards
            .iter()
            .filter(|&&card| game.cards[card].owner == seat)
            .count()
            >= 4;
    }
    if qualified.iter().any(|&value| value) {
        *game.lanes[lane]
            .state
            .as_deref_mut()
            .unwrap()
            .as_any_mut()
            .downcast_mut::<bool>()
            .unwrap() = true;
        for (seat, eligible) in qualified.into_iter().enumerate() {
            if eligible {
                game.add_permanent_energy(seat, 3);
            }
        }
    }
}
fn white_hot_room_event(game: &mut Game<'_>, lane: usize, event: Event) {
    if matches!(
        event,
        Event::BeforeCardRevealed { .. }
            | Event::CardResolved { .. }
            | Event::BeforeCardMoved { .. }
    ) {
        white_hot_room(game, lane);
    }
}

// ResurrectDestroyed 0x7d929d0 shuffles each graveyard, then resurrects
// up to the number of open slots after turn five.
fn valley_of_the_hand(game: &mut Game<'_>, lane: usize) {
    if game.turn != 5 {
        return;
    }
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        let mut cards: Vec<_> = game.players[seat]
            .graveyard
            .iter()
            .map(|&(card, _)| card)
            .collect();
        if cards.is_empty() {
            continue;
        }
        for index in (1..cards.len()).rev() {
            let selected = game.rng.borrow_mut().randbelow(index + 1);
            cards.swap(index, selected);
        }
        let count = (4 - game.lanes[lane]
            .cards
            .iter()
            .filter(|&&card| game.cards[card].owner == seat)
            .count())
        .min(cards.len());
        for card in cards.into_iter().take(count) {
            game.revive_card(card, lane, 1, seat);
        }
    }
}

// ReduceCardCost 0x7d541dc checks reveal/draw-completed context then
// discounts each player's Hand::GetBottomCard by 2 during the reveal turn.
fn fractured_frontier(game: &Game<'_>, lane: usize, card: usize, cost: i32) -> i32 {
    let owner = game.cards[card].owner;
    if game.lanes[lane].revealed_turn == Some(game.turn)
        && game.players[owner].hand.last() == Some(&card)
    {
        cost - 2
    } else {
        cost
    }
}

// AddEnergy 0x7d712c0 grants energy on reveal; LimitPlay 0x7d7177c
// caps each player's staged plays globally only in that turn.
fn parker_industries(game: &Game<'_>, lane: usize, seat: usize, queued: usize) -> bool {
    game.lanes[lane].revealed_turn != Some(game.turn)
        || (queued == 0 && !game.has_played_on_turn(seat, game.turn))
}

// BuffCards 0x7d80278 counts each owner's revealed Ongoing cards here;
// predicate 0x7d80868 checks attached Ongoing abilities.
fn smithsonian_museum(game: &mut Game<'_>, lane: usize) {
    let mut count = [0; 2];
    for &card in &game.lanes[lane].cards {
        if game.cards[card].revealed && game.cards[card].has_ongoing() {
            count[game.cards[card].owner] += 1;
        }
    }
    for index in 0..game.lanes[lane].cards.len() {
        let card = game.lanes[lane].cards[index];
        if game.cards[card].revealed {
            game.add_location_power(card, count[game.cards[card].owner]);
        }
    }
}

// Constructor 0x7d3b6f4: contingent Destroy(CardPlayedHere), then
// AddEnergyNextTurn to PlayerWhoPlayedHere; GetEnergyToAdd 0x7d3b6ac.
fn altar_of_death(game: &mut Game<'_>, lane: usize, event: Event) {
    let Event::CardResolved { card } = event else {
        return;
    };
    let seat = game.cards[card].owner;
    if game.played_cards_at(seat, lane).any(|id| id == card) && game.destroy_card(card) {
        game.schedule_energy(seat, game.turn + 1, 2);
    }
}

// AddEnergyTriggerToAllPlayers 0x7d527bc schedules +1 for both players
// for each card resolved after being played here.
fn feast(game: &mut Game<'_>, lane: usize, event: Event) {
    let Event::CardResolved { card } = event else {
        return;
    };
    let seat = game.cards[card].owner;
    if game.played_cards_at(seat, lane).any(|id| id == card) {
        for player in 0..2 {
            game.schedule_energy(player, game.turn + 1, 1);
        }
    }
}

// DestroyToDistribute 0x7d51780 rolls a revealed ally per player, then
// rolls once per absolute point for each surviving revealed ally.
fn essexs_lab(game: &mut Game<'_>, lane: usize) {
    if game.turn != 4 {
        return;
    }
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        let candidates: Vec<_> = game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&card| game.cards[card].owner == seat && game.cards[card].revealed)
            .collect();
        if candidates.is_empty() {
            continue;
        }
        let choice = game.rng.borrow_mut().randbelow(candidates.len());
        let card = candidates[choice];
        let power = game.cards[card].power();
        if !game.destroy_card(card) || power == 0 {
            continue;
        }
        let remaining: Vec<_> = game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&other| {
                other != card && game.cards[other].owner == seat && game.cards[other].revealed
            })
            .collect();
        if remaining.is_empty() {
            continue;
        }
        let mut amounts = [0_i32; 4];
        for _ in 0..power.unsigned_abs() {
            let choice = game.rng.borrow_mut().randbelow(remaining.len());
            amounts[choice] += power.signum();
        }
        for (card, amount) in remaining.into_iter().zip(amounts) {
            if amount != 0 {
                game.add_power(card, amount, seat);
            }
        }
    }
}

// TriggerBuff 0x7d48b98 uses separate per-player given flags and buffs
// revealed allies elsewhere once; location predicate 0x7d49ccc.
fn clubhouse(game: &mut Game<'_>, lane: usize) {
    for seat in 0..2 {
        let spent = game.lanes[lane]
            .state
            .as_deref()
            .unwrap()
            .as_any()
            .downcast_ref::<[bool; 2]>()
            .unwrap()[seat];
        if spent
            || game.lanes[lane]
                .cards
                .iter()
                .filter(|&&card| game.cards[card].owner == seat)
                .count()
                < 4
        {
            continue;
        }
        game.lanes[lane]
            .state
            .as_deref_mut()
            .unwrap()
            .as_any_mut()
            .downcast_mut::<[bool; 2]>()
            .unwrap()[seat] = true;
        let targets: Vec<_> = game
            .lanes
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != lane)
            .flat_map(|(_, lane)| lane.cards.iter().copied())
            .filter(|&card| game.cards[card].owner == seat && game.cards[card].revealed)
            .collect();
        for card in targets {
            game.add_power(card, 1, seat);
        }
    }
}

fn clubhouse_event(game: &mut Game<'_>, lane: usize, event: Event) {
    if matches!(
        event,
        Event::BeforeCardRevealed { .. }
            | Event::CardResolved { .. }
            | Event::BeforeCardMoved { .. }
    ) {
        clubhouse(game, lane);
    }
}

// SuperflowBehavior::AddEnergy 0x7d85cc8 uses all occupants, face-down included.
fn superflow(game: &Game<'_>, lane: usize, seat: usize) -> i32 {
    i32::from(
        !game.lanes[lane]
            .cards
            .iter()
            .any(|&card| game.cards[card].owner == seat),
    )
}

// EmpireStateUniversityBehavior::AddEnergy 0x7d5115c grants both players
// temporary energy at the start of its configured turn.
fn empire_state_university(game: &Game<'_>, _: usize, _: usize) -> i32 {
    if game.turn == 6 { 2 } else { 0 }
}

// AddCardPower 0x7d90db0 repeats ThisCardResolvedTrigger abilities after
// turn four; predicate 0x7d917ac selects revealed cards with that trigger.
fn valhalla(game: &mut Game<'_>, lane: usize) {
    if game.turn != 4 {
        return;
    }
    let first = game.reveal_priority.unwrap_or(0);
    let mut cards = Vec::new();
    for seat in [first, 1 - first] {
        cards.extend(game.lanes[lane].cards.iter().copied().filter(|&card| {
            game.cards[card].owner == seat
                && game.cards[card].revealed
                && game.cards[card].has_resolve_trigger()
        }));
    }
    for card in cards {
        game.trigger_on_reveal(card);
    }
}

// DestroyAllCards 0x7d78f60 snapshots all occupants, including face-down cards.
fn rickety_bridge(game: &mut Game<'_>, lane: usize) {
    if game.lanes[lane].cards.len() > 1 {
        for card in game.lanes[lane].cards.clone() {
            game.destroy_card(card);
        }
    }
}

// DestroyWeakest 0x7d955e0 snapshots revealed cards and destroys every
// minimum-power tie unless all revealed cards tie; predicate 0x7d95c88.
fn warrior_falls(game: &mut Game<'_>, lane: usize) {
    let cards: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&card| game.cards[card].revealed)
        .collect();
    if cards.len() < 2 {
        return;
    }
    let minimum = cards
        .iter()
        .map(|&card| game.cards[card].power())
        .min()
        .unwrap();
    let targets: Vec<_> = cards
        .iter()
        .copied()
        .filter(|&card| game.cards[card].power() == minimum)
        .collect();
    if targets.len() != cards.len() {
        for card in targets {
            game.destroy_card(card);
        }
    }
}

// CamelotBehavior::SetCardPower @ 0x7d424e4 sets all revealed cards permanently.
fn camelot(game: &mut Game<'_>, lane: usize) {
    if game.turn == 5 {
        for card in game.lanes[lane].cards.clone() {
            if game.cards[card].revealed {
                game.set_power(card, 5, game.cards[card].owner);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn camelot_sets_only_revealed_cards_after_turn_five() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let a = game.create_card("Hawkeye", 0);
        let b = game.create_card("Hawkeye", 1);
        assert!(game.add_card(a, 0, 0, false));
        assert!(game.add_card(b, 0, 1, false));
        game.cards[a].revealed = true;
        game.cards[b].revealed = false;
        game.turn = 4;
        camelot(&mut game, 0);
        assert_ne!(game.cards[a].power(), 5);
        game.turn = 5;
        camelot(&mut game, 0);
        assert_eq!(game.cards[a].power(), 5);
        assert_ne!(game.cards[b].power(), 5);
    }
    #[test]
    fn warrior_falls_destroys_all_weakest_ties_unless_everyone_ties() {
        let mut rng = PythonRandom::seed(2);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let a = game.create_card("Hawkeye", 0);
        let b = game.create_card("Hawkeye", 1);
        let strong = game.create_card("Hulk", 0);
        assert!(game.add_card(a, 0, 0, false));
        assert!(game.add_card(b, 0, 1, false));
        game.cards[a].revealed = true;
        game.cards[b].revealed = true;
        warrior_falls(&mut game, 0);
        assert_eq!(game.lanes[0].cards.len(), 2);
        assert!(game.add_card(strong, 0, 0, false));
        game.cards[strong].revealed = true;
        warrior_falls(&mut game, 0);
        assert_eq!(game.lanes[0].cards, vec![strong]);
    }
    #[test]
    fn rickety_bridge_destroys_all_occupants_only_when_multiple() {
        let mut rng = PythonRandom::seed(3);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let a = game.create_card("Hawkeye", 0);
        let b = game.create_card("Hawkeye", 1);
        assert!(game.add_card(a, 0, 0, false));
        game.cards[a].revealed = true;
        rickety_bridge(&mut game, 0);
        assert_eq!(game.lanes[0].cards, vec![a]);
        assert!(game.add_card(b, 0, 1, false));
        game.cards[b].revealed = false;
        rickety_bridge(&mut game, 0);
        assert!(game.lanes[0].cards.is_empty());
    }
    #[test]
    fn sandcastle_limits_turns_to_five() {
        assert_eq!(SANDCASTLE.maximum_turns, 5);
    }
    #[test]
    fn tva_limits_turns_to_four() {
        assert_eq!(TVA.maximum_turns, 4);
    }
    #[test]
    fn nexus_contributes_power_to_other_lanes() {
        assert!(
            crate::locations::get("TheNexus")
                .unwrap()
                .contributes_to_other_lanes
        );
    }
    #[test]
    fn valhalla_repeats_revealed_resolve_triggers_after_turn_four() {
        let mut rng = PythonRandom::seed(4);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Medusa", 0);
        assert!(game.add_card(card, 1, 0, false));
        game.turn = 3;
        let before = game.cards[card].power();
        valhalla(&mut game, 1);
        assert_eq!(game.cards[card].power(), before);
        game.turn = 4;
        valhalla(&mut game, 1);
        assert!(game.cards[card].power() > before);
    }
    #[test]
    fn superflow_counts_face_down_occupants_per_seat() {
        let mut rng = PythonRandom::seed(5);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 0, 0, false));
        game.cards[card].revealed = false;
        assert_eq!(superflow(&game, 0, 0), 0);
        assert_eq!(superflow(&game, 0, 1), 1);
    }
    #[test]
    fn empire_state_university_grants_energy_only_on_turn_six() {
        let mut rng = PythonRandom::seed(6);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.turn = 5;
        assert_eq!(empire_state_university(&game, 0, 0), 0);
        game.turn = 6;
        assert_eq!(empire_state_university(&game, 0, 0), 2);
        assert_eq!(empire_state_university(&game, 0, 1), 2);
    }
    #[test]
    fn clubhouse_buffs_revealed_other_lane_cards_once_per_player() {
        let mut rng = PythonRandom::seed(7);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Clubhouse", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let target = game.create_card("Hawkeye", 0);
        assert!(game.add_card(target, 1, 0, false));
        let original = game.cards[target].power();
        for _ in 0..4 {
            let card = game.create_card("Hawkeye", 0);
            assert!(game.add_card(card, 0, 0, false));
        }
        clubhouse(&mut game, 0);
        clubhouse(&mut game, 0);
        assert_eq!(game.cards[target].power(), original + 1);
    }
    #[test]
    fn essexs_lab_distributes_destroyed_power_to_surviving_allies() {
        let mut rng = PythonRandom::seed(8);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        for _ in 0..3 {
            let card = game.create_card("Hawkeye", 0);
            assert!(game.add_card(card, 0, 0, false));
        }
        let original: i32 = game.lanes[0]
            .cards
            .iter()
            .map(|&card| game.cards[card].power())
            .sum();
        game.turn = 4;
        essexs_lab(&mut game, 0);
        assert_eq!(game.lanes[0].cards.len(), 2);
        assert_eq!(
            game.lanes[0]
                .cards
                .iter()
                .map(|&card| game.cards[card].power())
                .sum::<i32>(),
            original
        );
    }
    #[test]
    fn feast_schedules_one_energy_per_player_for_each_play() {
        let mut rng = PythonRandom::seed(9);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        game.cards[card].turn_played = 3;
        game.played[0].push((card, 0));
        game.turn = 3;
        feast(&mut game, 0, Event::CardResolved { card });
        assert_eq!(game.players[0].scheduled_energy, vec![(4, 1)]);
        assert_eq!(game.players[1].scheduled_energy, vec![(4, 1)]);
    }
    #[test]
    fn altar_grants_energy_only_when_destruction_succeeds() {
        let mut rng = PythonRandom::seed(10);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 0, 0, false));
        game.cards[card].turn_played = 3;
        game.played[0].push((card, 0));
        game.turn = 3;
        altar_of_death(&mut game, 0, Event::CardResolved { card });
        assert!(game.cards[card].lane.is_none());
        assert_eq!(game.players[0].scheduled_energy, vec![(4, 2)]);
        assert!(game.players[1].scheduled_energy.is_empty());
    }
    #[test]
    fn smithsonian_counts_each_sides_revealed_ongoing_cards() {
        let mut rng = PythonRandom::seed(11);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let ongoing = game.create_card("IronMan", 0);
        let ally = game.create_card("Hawkeye", 0);
        let enemy = game.create_card("Hawkeye", 1);
        for (card, seat) in [(ongoing, 0), (ally, 0), (enemy, 1)] {
            assert!(game.add_card(card, 0, seat, false));
        }
        let before = game.cards[ally].power();
        smithsonian_museum(&mut game, 0);
        assert_eq!(game.cards[ally].power(), before + 1);
        assert_eq!(game.cards[enemy].location_power, 0);
    }
    #[test]
    fn parker_limits_global_staged_plays_only_on_reveal_turn() {
        let mut rng = PythonRandom::seed(12);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.turn = 3;
        game.lanes[0].revealed_turn = Some(3);
        assert!(parker_industries(&game, 0, 0, 0));
        assert!(!parker_industries(&game, 0, 0, 1));
        game.turn = 4;
        assert!(parker_industries(&game, 0, 0, 1));
    }
    #[test]
    fn fractured_frontier_discounts_only_rightmost_on_reveal_turn() {
        let mut rng = PythonRandom::seed(13);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let first = game.create_card("Hawkeye", 0);
        let last = game.create_card("Bishop", 0);
        game.players[0].hand = vec![first, last];
        game.turn = 3;
        game.lanes[0].revealed_turn = Some(3);
        assert_eq!(fractured_frontier(&game, 0, first, 3), 3);
        assert_eq!(fractured_frontier(&game, 0, last, 3), 1);
        game.turn = 4;
        assert_eq!(fractured_frontier(&game, 0, last, 3), 3);
    }
    #[test]
    fn valley_revives_graveyard_cards_after_turn_five() {
        let mut rng = PythonRandom::seed(14);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 1, 0, false));
        assert!(game.destroy_card(card));
        game.turn = 4;
        valley_of_the_hand(&mut game, 0);
        assert_eq!(game.cards[card].lane, None);
        game.turn = 5;
        valley_of_the_hand(&mut game, 0);
        assert_eq!(game.cards[card].lane, Some(0));
    }
    #[test]
    fn sakaar_grants_winner_not_tie() {
        let mut rng = PythonRandom::seed(15);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.turn = 4;
        sakaar_grand_prix(&mut game, 0);
        assert_eq!(game.players[0].permanent_energy, 0);
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 0, 0, false));
        sakaar_grand_prix(&mut game, 0);
        assert_eq!(game.players[0].permanent_energy, 1);
    }
    #[test]
    fn white_hot_room_grants_only_once() {
        let mut rng = PythonRandom::seed(16);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["WhiteHotRoom", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].state = Some(Box::new(false));
        for _ in 0..4 {
            let card = game.create_card("Hawkeye", 0);
            assert!(game.add_card(card, 0, 0, false));
        }
        white_hot_room(&mut game, 0);
        white_hot_room(&mut game, 0);
        assert_eq!(game.players[0].permanent_energy, 3);
    }
    #[test]
    fn weapon_x_discards_then_draws_for_both_players() {
        let mut rng = PythonRandom::seed(17);
        let deck = [
            "Hawkeye",
            "Wolfsbane",
            "Bishop",
            "Nightcrawler",
            "AntMan",
            "Angela",
            "JessicaJones",
            "KaZar",
            "IronMan",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let hand = [game.players[0].hand.len(), game.players[1].hand.len()];
        weapon_x_facility(&mut game, 0);
        for (seat, expected) in hand.into_iter().enumerate() {
            assert_eq!(game.players[seat].hand.len(), expected);
            assert_eq!(game.players[seat].discard.len(), 1);
        }
    }
}
