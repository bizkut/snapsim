//! Locations, batch D1: rules from the 59.15.1 client.

use super::{LocationDef, location};
use crate::game::{Event, Game};

pub const CELESTIAL_CIRCUIT: LocationDef = LocationDef {
    on_turn_end: Some(celestial_circuit),
    ..location("CelestialCircuit", 0, 0, 1, false)
};
pub const DEATHS_DOMAIN: LocationDef = LocationDef {
    on_event: Some(deaths_domain),
    ..location("DeathsDomain", 0, 0, 1, false)
};
pub const ISLAND_PRISON: LocationDef = LocationDef {
    allows_move_from: Some(prevent_move),
    ..location("IslandPrison", 0, 0, 1, false)
};
pub const ISLE_OF_SILENCE: LocationDef = LocationDef {
    disables_ongoing: true,
    ..location("IsleOfSilence", 0, 0, 1, false)
};
pub const QUANTUM_REALM: LocationDef = LocationDef {
    on_event: Some(quantum_realm),
    ..location("QuantumRealm", 0, 0, 1, false)
};
pub const SUBTERRANEA_TUTORIAL: LocationDef = LocationDef {
    on_reveal: Some(subterranea_tutorial),
    ..location("SubterraneaTutorial", 0, 0, 1, false)
};
pub const TIME_THEATER: LocationDef = LocationDef {
    before_draw: Some(time_theater),
    initial_state: Some(|| Box::new([false; 2])),
    ..location("TimeTheater", 0, 0, 1, false)
};

// CelestialCircuitBehavior::ReduceCost @ 0x7d467dc.
fn celestial_circuit(game: &mut Game<'_>, lane: usize) {
    if game.turn != 4 {
        return;
    }
    let powers = game.powers()[lane];
    let priority = game.reveal_priority.unwrap_or(0);
    for seat in [priority, 1 - priority] {
        if powers[seat] >= 12 {
            for card in game.players[seat].hand.clone() {
                game.add_cost(card, -1, seat);
            }
        }
    }
}

// DeathsDomainBehavior::DestroyCard @ 0x7d4def8 uses the player's
// played-lane history, even if the card moved during its resolution.
fn deaths_domain(game: &mut Game<'_>, lane: usize, event: Event) {
    if let Event::CardResolved { card } = event
        && game
            .played_cards_at(game.cards[card].owner, lane)
            .any(|played| played == card)
    {
        game.destroy_card(card);
    }
}

// QuantumRealmBehavior::SetCardPower @ 0x7d765ac applies to cards
// still in a location after their staged play here resolves.
fn quantum_realm(game: &mut Game<'_>, lane: usize, event: Event) {
    if let Event::CardResolved { card } = event
        && game
            .played_cards_at(game.cards[card].owner, lane)
            .any(|played| played == card)
        && game.cards[card].lane.is_some()
    {
        game.set_base_power(card, 2);
    }
}

// IslandPrisonBehavior::PreventMove @ 0x7d59fd0.
fn prevent_move(_: &Game<'_>, _: usize, _: usize, _: usize, _: usize) -> bool {
    false
}

// SubterraneaTutorialBehavior::ShuffleRocks @ 0x7d85498 uses
// Game::get_HumanPlayer to preserve the tutorial's two designated draws.
// LocationDefList::CreateSubterranea @ 0x7a739d8 configures four rocks.
fn subterranea_tutorial(game: &mut Game<'_>, _: usize) {
    let Some(human) = game.human_seat else { return };
    let original_count = game.players[human].deck.len();
    let priority = game.reveal_priority.unwrap_or(0);
    for seat in [priority, 1 - priority] {
        for _ in 0..4 {
            let rock = game.create_card("Rock", seat);
            game.cards[rock].created_by_location_id = Some("SubterraneaTutorial".into());
            let index = game
                .rng
                .borrow_mut()
                .randbelow(game.players[seat].deck.len() + 1);
            game.put_deck(seat, index, rock);
        }
    }
    let Some(&bottom) = game.players[human].deck.last() else {
        return;
    };
    let cards: Vec<_> = game.players[human]
        .deck
        .iter()
        .take(original_count)
        .copied()
        .collect();
    for (index, card) in cards.into_iter().enumerate() {
        if index != 2 && index != 4 && card != bottom {
            game.transform_into_copy(card, bottom);
        }
    }
}

// TimeTheaterBehavior::ReplaceDraw @ 0x7d8d750.
fn time_theater(game: &mut Game<'_>, lane: usize, seat: usize) -> Option<usize> {
    let last = game.players[seat].draws.last()?.1;
    let spent = game.lanes[lane]
        .state
        .as_deref_mut()?
        .as_any_mut()
        .downcast_mut::<[bool; 2]>()?;
    if spent[seat] {
        return None;
    }
    spent[seat] = true;
    let copy = game.copy_card(last, seat);
    game.cards[copy].created_by_location_id = Some("TimeTheater".into());
    Some(copy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    fn game<'a>(rng: &'a mut PythonRandom) -> Game<'a> {
        let deck: Vec<String> = [
            "Hawkeye",
            "IronMan",
            "Nightcrawler",
            "Hulk",
            "Bishop",
            "Angela",
            "Wolfsbane",
            "KaZar",
            "JessicaJones",
            "BlueMarvel",
            "Ultron",
            "SquirrelGirl",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        Game::new(
            [&deck, &deck],
            &["Ruins".into(), "Xandar".into(), "Wakanda".into()],
            rng,
        )
        .unwrap()
    }
    fn place(game: &mut Game<'_>, name: &str, lane: usize) -> usize {
        let id = game.create_card(name, 0);
        game.add_card(id, lane, 0, false);
        game.cards[id].revealed = true;
        id
    }
    #[test]
    fn prison_blocks_forced_and_voluntary_movement_only_while_revealed() {
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        game.lanes[0].def = &ISLAND_PRISON;
        let id = place(&mut game, "Nightcrawler", 0);
        game.lanes[0].revealed = true;
        assert!(!game.can_player_move(id, 1, 0));
        assert!(!game.move_card(id, 1));
        game.lanes[0].revealed = false;
        assert!(game.can_player_move(id, 1, 0));
        assert!(game.move_card(id, 1));
    }
    #[test]
    fn silence_suppresses_ongoing_but_not_card_text() {
        let mut rng = PythonRandom::seed(8);
        let mut game = game(&mut rng);
        game.lanes[0].def = &ISLE_OF_SILENCE;
        let id = place(&mut game, "IronMan", 0);
        game.lanes[0].revealed = true;
        game.ongoing();
        assert_eq!(game.lanes[0].multiplier[0], 1);
        assert!(!game.ongoing_active(id));
        assert!(game.cards[id].has_ongoing(false));
        game.lanes[0].revealed = false;
        game.ongoing();
        assert_eq!(game.lanes[0].multiplier[0], 2);
    }
    #[test]
    fn base_power_override_preserves_ordered_modifiers() {
        let mut rng = PythonRandom::seed(9);
        let mut game = game(&mut rng);
        let id = place(&mut game, "Hawkeye", 0);
        game.add_power(id, 3, 0);
        game.multiply_power(id, 2, 0);
        game.set_base_power(id, 2);
        assert_eq!(game.cards[id].permanent, 10);
        game.set_base_power(id, 2);
        assert_eq!(game.cards[id].permanent, 10);
        let copy = game.copy_card(id, 0);
        assert_eq!(game.cards[copy].base_power_override, Some(2));
    }
    #[test]
    fn theater_replaces_first_draw_per_seat_without_consuming_deck() {
        let mut rng = PythonRandom::seed(10);
        let mut game = game(&mut rng);
        game.lanes[0].def = &TIME_THEATER;
        game.lanes[0].state = Some(Box::new([false; 2]));
        game.lanes[0].revealed = true;
        let last = game.players[0].draws.last().unwrap().1;
        let size = game.players[0].deck.len();
        game.draw(0);
        let copy = *game.players[0].hand.last().unwrap();
        assert_eq!(game.cards[copy].def.id, game.cards[last].def.id);
        assert_eq!(game.players[0].deck.len(), size);
        game.draw(0);
        assert_eq!(game.players[0].deck.len(), size - 1);
    }
    #[test]
    fn celestial_threshold_and_turn_are_exact() {
        let mut rng = PythonRandom::seed(11);
        let mut game = game(&mut rng);
        let id = place(&mut game, "Hulk", 0);
        game.cards[id].permanent = 12;
        let hand = game.players[0].hand[0];
        let cost = game.cards[hand].cost;
        game.turn = 3;
        celestial_circuit(&mut game, 0);
        assert_eq!(game.cards[hand].cost, cost);
        game.turn = 4;
        celestial_circuit(&mut game, 0);
        assert_eq!(game.cards[hand].cost, cost - 1);
    }
    #[test]
    fn tutorial_requires_known_human_seat() {
        let mut rng = PythonRandom::seed(12);
        let mut game = game(&mut rng);
        let lengths = [game.players[0].deck.len(), game.players[1].deck.len()];
        subterranea_tutorial(&mut game, 0);
        assert_eq!(game.players[0].deck.len(), lengths[0]);
        assert_eq!(game.players[1].deck.len(), lengths[1]);
        game.human_seat = Some(1);
        subterranea_tutorial(&mut game, 0);
        assert_eq!(game.players[0].deck.len(), lengths[0] + 4);
        assert_eq!(game.players[1].deck.len(), lengths[1] + 4);
    }
    #[test]
    fn coverage_reports_only_relevant_hidden_location_state() {
        let mut position = crate::position::Position {
            turn: 3,
            total_turns: 6,
            deck: vec!["Hawkeye".into()],
            ..Default::default()
        };
        position.locations[0].id = Some("TimeTheater".into());
        let state = crate::coverage::coverage(&position)["state"].to_string();
        assert!(state.contains("Time Theater use unknown for seat 0"));
        assert!(!state.contains("seat 1"));
        position.turn = 6;
        let state = crate::coverage::coverage(&position)["state"].to_string();
        assert!(!state.contains("Time Theater"));
        position.locations[0].id = Some("QuantumRealm".into());
        position.locations[0].revealed_turn = Some(2);
        position.locations[0].mine.push(crate::position::CardRef {
            id: "Hawkeye".into(),
            revealed: true,
            played_turn: Some(3),
            ..Default::default()
        });
        let state = crate::coverage::coverage(&position)["state"].to_string();
        assert!(state.contains("Quantum Realm base unknown for Hawkeye"));
        position.locations[0].mine[0].played_turn = Some(1);
        let state = crate::coverage::coverage(&position)["state"].to_string();
        assert!(!state.contains("Quantum Realm"));
    }
}
