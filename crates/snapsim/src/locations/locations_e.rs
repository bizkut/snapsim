//! Locations, batch E: rules from the 59.15.1 client.

use super::{LocationDef, location};
use crate::game::{Action, Event, Game};

// KnowhereBehavior::PreventOnReveals @ 0x7d5d934.
pub const KNOWHERE: LocationDef = LocationDef {
    disables_on_reveal: true,
    ..location("Knowhere", 0, 0, 1, false)
};
pub const MORAG: LocationDef = LocationDef {
    allows_play_with_count: Some(morag),
    ..location("Morag", 0, 0, 1, false)
};
pub const SAKAAR_TUTORIAL: LocationDef = LocationDef {
    on_reveal: Some(sakaar_tutorial),
    ..location("SakaarTutorial", 0, 0, 1, false)
};
pub const SANCTUM_SANCTORUM: LocationDef = LocationDef {
    allows_play: Some(no_play),
    ..location("SanctumSanctorum", 0, 0, 1, false)
};
pub const PIT_OF_EXILE: LocationDef = LocationDef {
    allows_play: Some(pit_of_exile),
    ..location("PitOfExile", 0, 0, 1, false)
};
pub const PET_AVENGERS_MANSION: LocationDef = LocationDef {
    allows_play_elsewhere: Some(pet_avengers_mansion),
    ..location("PetAvengersMansion", 0, 0, 1, false)
};
pub const SPIDER_ISLAND: LocationDef = LocationDef {
    on_event: Some(spider_island),
    ..location("SpiderIsland", 0, 0, 1, false)
};
pub const YGGDRASIL: LocationDef = LocationDef {
    on_turn_end: Some(yggdrasil),
    ..location("Yggdrasil", 0, 0, 1, false)
};
pub const THUNDERBOLTS_TOWER: LocationDef = LocationDef {
    on_turn_end: Some(thunderbolts_tower),
    ..location("ThunderboltsTower", 0, 0, 1, false)
};
pub const THE_ABBEY: LocationDef = LocationDef {
    initial_state: Some(|| Box::new(false)),
    on_turn_start: Some(first_to_fill),
    on_event: Some(first_to_fill_event),
    ..location("TheAbbey", 0, 0, 1, false)
};
pub const RAFT: LocationDef = LocationDef {
    initial_state: Some(|| Box::new(false)),
    on_turn_start: Some(first_to_fill),
    on_event: Some(first_to_fill_event),
    ..location("Raft", 0, 0, 1, false)
};

// TheAbbeyBehavior::TriggerGiveCard @ 0x7d88474 and
// RaftBehavior::TriggerGiveCard @ 0x7d76ec8 share one IsGiven context,
// checked once and consumed after visiting both players.
fn first_to_fill(game: &mut Game<'_>, lane: usize) {
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
    let raft = game.lanes[lane].def.id == "Raft";
    let mut eligible = [false; 2];
    for (seat, value) in eligible.iter_mut().enumerate() {
        let count = game.lanes[lane]
            .cards
            .iter()
            .filter(|&&id| game.cards[id].owner == seat)
            .count();
        *value = if raft {
            count == 4 && game.players[seat].hand.len() < 7
        } else {
            count == 2
        };
    }
    if !eligible.iter().any(|&value| value) {
        return;
    }
    *game.lanes[lane]
        .state
        .as_deref_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<bool>()
        .unwrap() = true;
    for (seat, qualifies) in eligible.into_iter().enumerate() {
        if qualifies {
            if raft {
                if let Some(id) = game.random_card(|row| row.cost == 6) {
                    let card = game.create_card(id, seat);
                    game.set_cost(card, 0, seat);
                    game.add_hand(seat, card, false);
                }
            } else {
                game.draw(seat);
            }
        }
    }
}

fn first_to_fill_event(game: &mut Game<'_>, lane: usize, event: Event) {
    if matches!(
        event,
        Event::BeforeCardRevealed { .. }
            | Event::CardResolved { .. }
            | Event::BeforeCardMoved { .. }
    ) {
        first_to_fill(game, lane);
    }
}

// SakaarTutorialBehavior::PlayCard @ 0x7d7b5bc: AI chooses maximum
// power; the human chooses minimum cost, then minimum power, keeping ties.
fn sakaar_tutorial(game: &mut Game<'_>, lane: usize) {
    let Some(human) = game.human_seat else { return };
    for seat in 0..2 {
        if !game.has_space(lane, seat, 0) {
            continue;
        }
        let card = if seat == human {
            game.players[seat]
                .hand
                .iter()
                .copied()
                .min_by_key(|&id| (game.card_cost(id), game.cards[id].power()))
        } else {
            game.players[seat].hand.iter().copied().reduce(|best, id| {
                if game.cards[id].power() > game.cards[best].power() {
                    id
                } else {
                    best
                }
            })
        };
        if let Some(card) = card {
            game.apply_staged_action(seat, Action::Play { card, lane }, 0);
            game.reveal(card);
        }
    }
}

// MoragBehavior::SetAsFirstCardCantBePlayedHere @ 0x7d67748.
fn morag(game: &Game<'_>, seat: usize, queued: usize) -> bool {
    queued > 0
        || game.players[seat]
            .staged_history
            .iter()
            .any(|play| play.turn == game.turn)
}

// SanctumSanctorumBehavior::DisableCardPlay @ 0x7d7d7d0.
fn no_play(_: &Game<'_>, _: usize, _: usize, _: usize, _: usize) -> bool {
    false
}

// PitOfExileBehavior::CanPlay @ 0x7d72284 uses current power.
fn pit_of_exile(game: &Game<'_>, _: usize, card: usize, _: usize, _: usize) -> bool {
    game.cards[card].power() < 10
}

// PetAvengersMansionBehavior::CardShouldBePlayedHere @ 0x7d71d1c.
fn pet_avengers_mansion(game: &Game<'_>, lane: usize, _: usize, _: usize, _: usize) -> bool {
    game.lanes[lane].revealed_turn != Some(game.turn)
}

// SpiderIslandBehavior::MoveCard @ 0x7d824bc uses the card's current
// location, while testing whether it was played at Spider Island.
fn spider_island(game: &mut Game<'_>, lane: usize, event: Event) {
    let Event::CardResolved { card } = event else {
        return;
    };
    let seat = game.cards[card].owner;
    if !game.played_cards_at(seat, lane).any(|id| id == card) {
        return;
    }
    let Some(from) = game.cards[card].lane else {
        return;
    };
    let mut targets = [0; 2];
    let mut count = 0;
    for target in 0..3 {
        if target != from
            && game.has_space(target, seat, 0)
            && (!game.lanes[target].revealed
                || game.lanes[target]
                    .def
                    .allows_addition
                    .is_none_or(|test| test(game, target, card, target, seat)))
        {
            targets[count] = target;
            count += 1;
        }
    }
    if count != 0 {
        let target = targets[game.rng.borrow_mut().randbelow(count)];
        game.move_card(card, target);
    }
}

// YggdrasilBehavior::BuffPlayerCards @ 0x7d9b6f8 chooses one of
// both other locations, even if empty or unrevealed.
fn yggdrasil(game: &mut Game<'_>, lane: usize) {
    let choice = game.rng.borrow_mut().randbelow(2);
    let target = if choice < lane { choice } else { choice + 1 };
    for card in game.lanes[target].cards.clone() {
        if game.cards[card].revealed {
            game.add_power(card, 1, game.cards[card].owner);
        }
    }
}

// ThunderboltsTowerBehavior::ReduceCardPower @ 0x7d8d1c0.
fn thunderbolts_tower(game: &mut Game<'_>, lane: usize) {
    if game.turn == 5 {
        for card in game.lanes[lane].cards.clone() {
            if game.cards[card].revealed {
                game.add_power(card, -2, game.cards[card].owner);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    fn game(rng: &mut PythonRandom) -> Game<'_> {
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
        Game::new(
            [&deck, &deck],
            &["Ruins", "Xandar", "Wakanda"].map(str::to_owned),
            rng,
        )
        .unwrap()
    }

    #[test]
    fn play_restrictions_check_turn_and_current_power() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let card = game.create_card("Hawkeye", 0);
        game.lanes[0].def = &MORAG;
        game.lanes[0].revealed = true;
        assert!(!game.can_play_with_count(card, 0, 0, 10, 0, 0));
        assert!(game.can_play_with_count(card, 0, 0, 10, 0, 1));
        game.lanes[0].def = &PIT_OF_EXILE;
        assert!(game.can_play(card, 0, 0, 10, 0));
        game.add_power(card, 10, 0);
        assert!(!game.can_play(card, 0, 0, 10, 0));
        game.lanes[0].def = &SANCTUM_SANCTORUM;
        assert!(!game.can_play(card, 0, 0, 10, 0));
    }

    #[test]
    fn knowhere_blocks_reveal_without_removing_text() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        game.lanes[1].def = &KNOWHERE;
        game.lanes[1].revealed = true;
        let card = game.create_card("Medusa", 0);
        assert!(game.add_card(card, 1, 0, false));
        game.cards[card].revealed = false;
        let power = game.cards[card].power();
        game.reveal(card);
        assert_eq!(game.cards[card].power(), power);
        assert!(!game.cards[card].abilities_removed);
    }

    #[test]
    fn tower_affects_only_revealed_cards_on_turn_five() {
        let mut rng = PythonRandom::seed(2);
        let mut game = game(&mut rng);
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 0, 0, false));
        game.cards[card].revealed = true;
        let power = game.cards[card].power();
        game.turn = 4;
        thunderbolts_tower(&mut game, 0);
        assert_eq!(game.cards[card].power(), power);
        game.turn = 5;
        thunderbolts_tower(&mut game, 0);
        assert_eq!(game.cards[card].power(), power - 2);
    }

    #[test]
    fn abbey_rewards_both_qualifying_sides_only_once() {
        let mut rng = PythonRandom::seed(4);
        let mut game = game(&mut rng);
        game.lanes[0].def = &THE_ABBEY;
        game.lanes[0].state = Some(Box::new(false));
        for seat in 0..2 {
            for _ in 0..2 {
                let card = game.create_card("Hawkeye", seat);
                assert!(game.add_card(card, 0, seat, false));
            }
        }
        let before = [game.players[0].hand.len(), game.players[1].hand.len()];
        first_to_fill(&mut game, 0);
        assert_eq!(game.players[0].hand.len(), before[0] + 1);
        assert_eq!(game.players[1].hand.len(), before[1] + 1);
        first_to_fill(&mut game, 0);
        assert_eq!(game.players[0].hand.len(), before[0] + 1);
        assert_eq!(game.players[1].hand.len(), before[1] + 1);
    }

    #[test]
    fn mansion_restricts_only_its_reveal_turn() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        game.lanes[0].def = &PET_AVENGERS_MANSION;
        game.lanes[0].revealed = true;
        game.turn = 3;
        game.lanes[0].revealed_turn = Some(3);
        let card = game.create_card("Hawkeye", 0);
        assert!(!game.can_play(card, 1, 0, 10, 0));
        assert!(game.can_play(card, 0, 0, 10, 0));
        game.turn = 4;
        assert!(game.can_play(card, 1, 0, 10, 0));
    }

    #[test]
    fn spider_island_moves_to_only_available_destination() {
        let mut rng = PythonRandom::seed(2);
        let mut game = game(&mut rng);
        game.turn = 3;
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_card(card, 0, 0, false));
        game.cards[card].turn_played = 3;
        game.played[0].push((card, 0));
        for _ in 0..4 {
            let blocker = game.create_card("Rock", 0);
            assert!(game.add_card(blocker, 2, 0, false));
        }
        spider_island(&mut game, 0, Event::CardResolved { card });
        assert_eq!(game.cards[card].lane, Some(1));
    }

    #[test]
    fn random_location_pools_exclude_tutorials() {
        for recruit in [false, true] {
            let pool = crate::position::all_locations(recruit);
            assert!(
                !pool
                    .iter()
                    .any(|id| id == "SakaarTutorial" || id == "SubterraneaTutorial")
            );
        }
    }
}
