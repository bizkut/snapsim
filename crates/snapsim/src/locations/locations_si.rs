//! Sanctum Infinitum's per-player stored discard targets.
use super::{LocationDef, location};
use crate::game::{Event, Game};

pub const SANCTUM_INFINITUM: LocationDef = LocationDef {
    initial_state: Some(|| Box::new([None::<usize>; 2])),
    on_reveal: Some(discard),
    on_turn_start: Some(return_card),
    on_event: Some(check_return),
    ..location("SanctumInfinitum", 0, 0, 1, false)
};

// DiscardCard 0x7d7c3b8 selects randomly from each hand and stores the
// entity before attempting the discard, even if its discard is prevented.
fn discard(game: &mut Game<'_>, lane: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        let count = game.players[seat].hand.len();
        if count == 0 {
            continue;
        }
        let index = game.rng.borrow_mut().randbelow(count);
        let card = game.players[seat].hand[index];
        game.lanes[lane]
            .state
            .as_deref_mut()
            .unwrap()
            .as_any_mut()
            .downcast_mut::<[Option<usize>; 2]>()
            .unwrap()[seat] = Some(card);
        game.discard_card(seat, card);
    }
}

// ReturnCard 0x7d7c7bc checks both players on start of turn, before a
// resolve or move, and after resolution. ExecuteReturn 0x7d7cd18 moves the
// stored entity to hand, then gives it +5, clearing only successful targets.
fn return_card(game: &mut Game<'_>, lane: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        if game.lanes[lane]
            .cards
            .iter()
            .filter(|&&id| game.cards[id].owner == seat)
            .count()
            < 4
            || game.players[seat].hand.len() >= 7
        {
            continue;
        }
        let target = game.lanes[lane]
            .state
            .as_deref()
            .unwrap()
            .as_any()
            .downcast_ref::<[Option<usize>; 2]>()
            .unwrap()[seat];
        let Some(card) = target else {
            continue;
        };
        if game.resurrect_entity_to_hand(card, seat) {
            game.lanes[lane]
                .state
                .as_deref_mut()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<[Option<usize>; 2]>()
                .unwrap()[seat] = None;
            game.add_power(card, 5, seat);
        }
    }
}
fn check_return(game: &mut Game<'_>, lane: usize, event: Event) {
    if matches!(
        event,
        Event::BeforeCardRevealed { .. }
            | Event::CardResolved { .. }
            | Event::BeforeCardMoved { .. }
    ) {
        return_card(game, lane);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn selects_and_returns_each_players_own_target() {
        let mut rng = PythonRandom::seed(719);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let locations = ["SanctumInfinitum", "Ruins", "Xandar"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].def = &SANCTUM_INFINITUM;
        game.lanes[0].state = SANCTUM_INFINITUM.initial_state.map(|f| f());
        game.lanes[0].revealed = true;
        for seat in 0..2 {
            while !game.players[seat].hand.is_empty() {
                game.take_hand(seat, 0);
            }
        }
        let targets = [game.create_card("Hulk", 0), game.create_card("Cyclops", 1)];
        for (seat, &card) in targets.iter().enumerate() {
            game.add_hand(seat, card, false);
        }
        discard(&mut game, 0);
        for (seat, &card) in targets.iter().enumerate() {
            assert!(game.players[seat].discard.contains(&card));
            for _ in 0..4 {
                let filler = game.create_card("Wasp", seat);
                assert!(game.add_card(filler, 0, seat, false));
            }
        }
        return_card(&mut game, 0);
        for (seat, &card) in targets.iter().enumerate() {
            assert!(game.players[seat].hand.contains(&card));
            assert!(!game.players[seat].discard.contains(&card));
            assert_eq!(game.cards[card].power(), game.printed_power(card) + 5);
        }
        return_card(&mut game, 0);
        for &card in &targets {
            assert_eq!(game.cards[card].power(), game.printed_power(card) + 5);
        }
    }

    #[test]
    fn full_hand_defers_return_until_space_is_available() {
        let mut rng = PythonRandom::seed(720);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let locations = ["SanctumInfinitum", "Ruins", "Xandar"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].def = &SANCTUM_INFINITUM;
        game.lanes[0].state = SANCTUM_INFINITUM.initial_state.map(|f| f());
        game.lanes[0].revealed = true;
        while !game.players[0].hand.is_empty() {
            game.take_hand(0, 0);
        }
        while !game.players[1].hand.is_empty() {
            game.take_hand(1, 0);
        }
        let target = game.create_card("Hulk", 0);
        game.add_hand(0, target, false);
        discard(&mut game, 0);
        for _ in 0..4 {
            let filler = game.create_card("Wasp", 0);
            assert!(game.add_card(filler, 0, 0, false));
        }
        for _ in 0..7 {
            let filler = game.create_card("Wasp", 0);
            assert!(game.add_hand(0, filler, false));
        }
        return_card(&mut game, 0);
        assert!(game.players[0].discard.contains(&target));
        game.take_hand(0, 0);
        return_card(&mut game, 0);
        assert!(game.players[0].hand.contains(&target));
        assert_eq!(game.cards[target].power(), game.printed_power(target) + 5);
    }
}
