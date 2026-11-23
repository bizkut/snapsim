//! Locations, batch C: rules from the 59.15.1 client.

use super::{LocationDef, location};
use crate::game::Game;

pub const STAR_BRAND_CRATER: LocationDef = LocationDef {
    on_turn_start: Some(star_brand_crater),
    ..location("StarBrandCrater", 0, 0, 1, false)
};

pub const CROWN_CITY: LocationDef = LocationDef {
    conditional_ongoing: Some(crown_city),
    ..location("CrownCity", 0, 0, 1, false)
};

pub const NEW_YORK: LocationDef = LocationDef {
    grants_move_to: Some(new_york),
    ..location("NewYork", 0, 0, 1, false)
};

pub const PYRAMID_OF_RAMA_TUT: LocationDef = LocationDef {
    on_turn_end: Some(pyramid_of_rama_tut),
    ..location("PyramidOfRamaTut", 0, 0, 1, false)
};

pub const ASTEROID_M: LocationDef = LocationDef {
    on_event: Some(asteroid_m),
    ..location("AsteroidM", 0, 0, 1, false)
};

// AsteroidMBehavior::MoveCard @ 0x7d3c628 checks a staged resolved card's
// current cost and moves it here only while it remains at another location.
fn asteroid_m(game: &mut Game<'_>, lane: usize, event: crate::game::Event) {
    if let crate::game::Event::CardResolved { card } = event
        && game.cards[card].turn_played > 0
        && game.cards[card].lane.is_some_and(|source| source != lane)
        && matches!(game.card_cost(card), 3 | 4)
    {
        game.move_card(card, lane);
    }
}
// NewYorkBehavior::MakeCardMovableToThisLocation @ 0x7d6adbc grants
// revealed cards movement to this location only on the configured turn.
fn new_york(game: &Game<'_>, _: usize, _: usize, _: usize, _: usize) -> bool {
    game.turn == 6
}

// PyramidOfRamaTutBehavior::AddRock @ 0x7d75c10 and its predicate
// <AddRock>b__1_0 @ 0x7d76420 compare the card's identity to the token.
fn pyramid_of_rama_tut(game: &mut Game<'_>, _: usize) {
    let priority = game.reveal_priority.unwrap_or(0);
    for seat in [priority, 1 - priority] {
        if game.players[seat].hand.len() >= 7
            || game.players[seat]
                .hand
                .iter()
                .any(|&id| game.cards[id].def.id == "Rock")
        {
            continue;
        }
        let rock = game.create_card("Rock", seat);
        game.cards[rock].created_by_location_id = Some("PyramidOfRamaTut".into());
        game.add_hand(seat, rock, false);
        game.set_power(rock, 3, seat);
    }
}

// StarBrandCraterBehavior::AddEnergy @ 0x7d828a8: test each player's
// location power at the start of the turn and add temporary energy.
fn star_brand_crater(game: &mut Game<'_>, lane: usize) {
    let power = game.powers()[lane];
    for (seat, total) in power.into_iter().enumerate() {
        if total >= 10 {
            game.set_energy(seat, game.players[seat].energy + 1);
        }
    }
}

// CrownCityBehavior::BuffAdjacentLocations @ 0x7d4c204: ties give neither
// player a bonus; the winning player receives power at each adjacent lane.
// ActionReturns2: no extant adjacent location means no action was performed.
fn crown_city(game: &mut Game<'_>, lane: usize) -> bool {
    let adjacent = (lane > 0 && !game.lanes[lane - 1].destroyed)
        || (lane + 1 < game.lanes.len() && !game.lanes[lane + 1].destroyed);
    let Some(winner) = game.lane_winner(lane) else {
        return adjacent;
    };
    if lane > 0 {
        game.add_lane_bonus(lane - 1, winner, 4);
    }
    if lane + 1 < game.lanes.len() {
        game.add_lane_bonus(lane + 1, winner, 4);
    }
    adjacent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    fn game<'a>(rng: &'a mut PythonRandom) -> Game<'a> {
        let deck: Vec<String> = [
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
        .into_iter()
        .map(str::to_owned)
        .collect();
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        Game::new([&deck, &deck], &locations, rng).unwrap()
    }

    fn place(game: &mut Game<'_>, id: &str, seat: usize, lane: usize) -> usize {
        let card = game.cards.len();
        game.cards
            .push(crate::position::make_card(id, seat).unwrap());
        game.add_card(card, lane, seat, false);
        card
    }

    fn record_resolved(game: &mut Game<'_>, _: usize, event: crate::game::Event) {
        if let crate::game::Event::CardResolved { card } = event {
            game.cards[card].permanent = if game.players[game.cards[card].owner]
                .banished
                .contains(&card)
            {
                100
            } else {
                10
            };
        }
    }

    fn record_after_play(game: &mut Game<'_>, _: usize, card: usize) {
        game.cards[card].permanent *= 2;
    }

    const RECORD_ORDER: LocationDef = LocationDef {
        on_event: Some(record_resolved),
        after_card_play: Some(record_after_play),
        ..location("RecordOrder", 0, 0, 1, false)
    };

    #[test]
    fn post_resolved_dispatch_follows_skill_banishment_and_precedes_after_play() {
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        game.lanes[0].def = &RECORD_ORDER;
        game.lanes[0].revealed = true;
        let card = place(&mut game, "Hawkeye", 0, 0);
        game.cards[card].turn_played = 1;
        game.cards[card].abilities_removed = true;
        game.cards[card].revealed = false;
        game.reveal(card);
        assert_eq!(game.cards[card].permanent, 20);

        let skill = place(&mut game, "RemoteMines", 0, 0);
        game.cards[skill].turn_played = 1;
        game.cards[skill].abilities_removed = true;
        game.cards[skill].revealed = false;
        game.reveal(skill);
        assert!(game.players[0].banished.contains(&skill));
        assert_eq!(game.cards[skill].permanent, 100);
    }

    #[test]
    fn asteroid_moves_only_staged_resolutions_with_matching_current_cost() {
        let mut rng = PythonRandom::seed(8);
        let mut game = game(&mut rng);
        game.lanes[1].def = &ASTEROID_M;
        game.lanes[1].revealed = true;
        let card = place(&mut game, "Wolfsbane", 0, 0);
        asteroid_m(&mut game, 1, crate::game::Event::CardResolved { card });
        assert_eq!(game.cards[card].lane, Some(0));
        game.cards[card].turn_played = 1;
        game.cards[card].cost = 2;
        asteroid_m(&mut game, 1, crate::game::Event::CardResolved { card });
        assert_eq!(game.cards[card].lane, Some(0));
        game.cards[card].cost = 4;
        asteroid_m(&mut game, 1, crate::game::Event::CardResolved { card });
        assert_eq!(game.cards[card].lane, Some(1));
    }

    #[test]
    fn new_york_grants_only_movement_to_revealed_location_on_six() {
        let mut rng = PythonRandom::seed(3);
        let mut game = game(&mut rng);
        game.lanes[1].def = &NEW_YORK;
        let card = place(&mut game, "Hawkeye", 0, 0);
        game.turn = 6;
        assert!(!game.can_player_move(card, 1, 0));
        game.lanes[1].revealed = true;
        assert!(game.can_player_move(card, 1, 0));
        assert!(!game.can_player_move(card, 2, 0));
        game.turn = 5;
        assert!(!game.can_player_move(card, 1, 0));
    }

    #[test]
    fn pyramid_requires_no_rock_and_space_in_hand() {
        let mut rng = PythonRandom::seed(4);
        let mut game = game(&mut rng);
        game.players[0].hand.clear();
        game.players[1].hand.clear();
        pyramid_of_rama_tut(&mut game, 0);
        for seat in 0..2 {
            let rock = game.players[seat].hand[0];
            assert_eq!(game.cards[rock].def.id, "Rock");
            assert_eq!(game.cards[rock].power(), 3);
        }
        pyramid_of_rama_tut(&mut game, 0);
        assert_eq!(game.players[0].hand.len(), 1);
        game.players[1].hand.clear();
        for _ in 0..7 {
            let card = game.create_card("Hawkeye", 1);
            game.add_hand(1, card, false);
        }
        pyramid_of_rama_tut(&mut game, 0);
        assert_eq!(game.players[1].hand.len(), 7);
    }

    #[test]
    fn crater_checks_threshold_independently_for_each_player() {
        let mut rng = PythonRandom::seed(5);
        let mut game = game(&mut rng);
        let a = place(&mut game, "Hawkeye", 0, 1);
        let b = place(&mut game, "Hawkeye", 1, 1);
        game.cards[a].permanent = 10;
        game.cards[b].permanent = 9;
        game.players[0].energy = 3;
        game.players[1].energy = 3;
        star_brand_crater(&mut game, 1);
        assert_eq!([game.players[0].energy, game.players[1].energy], [4, 3]);
    }

    #[test]
    fn crown_city_gives_winner_both_adjacent_bonuses_but_none_on_ties() {
        let mut rng = PythonRandom::seed(6);
        let mut game = game(&mut rng);
        let a = place(&mut game, "Hawkeye", 0, 1);
        let b = place(&mut game, "Hawkeye", 1, 1);
        game.cards[a].permanent = 4;
        game.cards[b].permanent = 4;
        crown_city(&mut game, 1);
        assert_eq!(game.lanes[0].bonus, [0, 0]);
        game.cards[b].permanent = 5;
        crown_city(&mut game, 1);
        assert_eq!(game.lanes[0].bonus, [0, 4]);
        assert_eq!(game.lanes[2].bonus, [0, 4]);
    }
}
