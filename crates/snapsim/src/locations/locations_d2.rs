//! Locations, batch D2: rules from the 59.15.1 client.

use super::{LocationDef, location};
use crate::game::{Event, Game};

pub const VORMIR: LocationDef = LocationDef {
    initial_state: Some(|| Box::new([false; 2])),
    on_event: Some(vormir),
    ..location("Vormir", 0, 0, 1, false)
};
pub const WORLD_FORGE: LocationDef = LocationDef {
    on_turn_start: Some(world_forge),
    ..location("WorldForge", 0, 0, 1, false)
};
pub const CAVE_OF_THE_DRAGON: LocationDef = LocationDef {
    on_turn_end: Some(cave_of_the_dragon),
    ..location("CaveOfTheDragon", 0, 0, 1, false)
};
pub const COLLAPSED_MINE: LocationDef = LocationDef {
    on_reveal: Some(collapsed_mine_fill),
    on_turn_end: Some(collapsed_mine_skip),
    ..location("CollapsedMine", 0, 0, 1, false)
};
pub const DARK_DIMENSION: LocationDef = LocationDef {
    delays_reveals_until_end: true,
    ..location("DarkDimension", 0, 0, 1, false)
};
pub const BIFROST: LocationDef = LocationDef {
    on_turn_end: Some(bifrost),
    ..location("Bifrost", 0, 0, 1, false)
};
pub const BOARDWALK: LocationDef = LocationDef {
    on_turn_end: Some(boardwalk),
    ..location("Boardwalk", 0, 0, 1, false)
};

// VormirBehavior::DestroyCard @ 0x7d93fc8 stores consumption per player,
// even when the destruction itself is prevented.
fn vormir(game: &mut Game<'_>, lane: usize, event: Event) {
    let Event::CardResolved { card } = event else {
        return;
    };
    let owner = game.cards[card].owner;
    if game.cards[card].turn_played == 0 || !game.played_cards_at(owner, lane).any(|id| id == card)
    {
        return;
    }
    let state = game.lanes[lane]
        .state
        .as_deref_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<[bool; 2]>()
        .unwrap();
    if !state[owner] {
        state[owner] = true;
        game.destroy_card(card);
    }
}

// WorldForgeBehavior::MorphOtherLocation @ 0x7d995f4 and predicate
// 0x7d99ab4 select either other location, including an unrevealed one.
fn world_forge(game: &mut Game<'_>, lane: usize) {
    let others: Vec<_> = (0..3).filter(|&other| other != lane).collect();
    let target = others[game.rng.borrow_mut().randbelow(others.len())];
    let candidates: Vec<_> = game
        .location_pool
        .iter()
        .filter(|def| !game.lanes.iter().any(|other| other.def.id == def.id))
        .map(|def| def.id)
        .collect();
    if !candidates.is_empty() {
        let id = candidates[game.rng.borrow_mut().randbelow(candidates.len())];
        game.replace_location(target, id);
    }
}

// CaveOfTheDragonBehavior::AddCard @ 0x7d45454, predicate 0x7d45f70;
// the five exclusions are assembled by its constructor @ 0x7d45924.
pub(crate) fn cave_candidate(row: &crate::data::CardData) -> bool {
    row.cost >= 5
        && !matches!(
            row.carddefid.as_str(),
            "Galactus" | "Alioth" | "Destroyer" | "Ultron" | "Modok"
        )
}

fn cave_of_the_dragon(game: &mut Game<'_>, lane: usize) {
    if game.turn != 4 {
        return;
    }
    for seat in [
        game.reveal_priority.unwrap_or(0),
        1 - game.reveal_priority.unwrap_or(0),
    ] {
        if let Some(random) = game.random_card(cave_candidate)
            && game.has_space(lane, seat, 0)
        {
            // Replay may supply an observed, validated result. Still consume the
            // client's one random draw so subsequent effects retain their stream.
            let id = game.lanes[lane]
                .state
                .as_deref()
                .and_then(|state| state.as_any().downcast_ref::<[Option<&'static str>; 2]>())
                .and_then(|choices| choices[seat])
                .unwrap_or(random);
            let card = game.create_card(id, seat);
            game.add_card(card, lane, seat, true);
        }
    }
}

// CollapsedMineBehavior::FillWithRocks @ 0x7d49cfc and
// DestroyRocksWhenSkip @ 0x7d4a2c8, GetTargets @ 0x7d4a180.
fn collapsed_mine_fill(game: &mut Game<'_>, lane: usize) {
    for seat in [
        game.reveal_priority.unwrap_or(0),
        1 - game.reveal_priority.unwrap_or(0),
    ] {
        while game.has_space(lane, seat, 0) {
            let card = game.create_card("Rock", seat);
            if !game.add_card(card, lane, seat, true) {
                break;
            }
        }
    }
}
fn collapsed_mine_skip(game: &mut Game<'_>, _: usize) {
    for seat in [
        game.reveal_priority.unwrap_or(0),
        1 - game.reveal_priority.unwrap_or(0),
    ] {
        if game.players[seat].non_play_action_turn == Some(game.turn)
            || game.players[seat]
                .staged_history
                .iter()
                .any(|play| play.turn == game.turn)
        {
            continue;
        }
        for card in game.revealed_cards_in_resolution_order() {
            if game.cards[card].owner == seat && game.cards[card].def.id == "Rock" {
                game.destroy_card(card);
            }
        }
    }
}

// BifrostBehavior::MoveAllCards @ 0x7d3fda4 visits locations right-to-left
// and moves revealed cards only; this prevents moving one card twice.
fn bifrost(game: &mut Game<'_>, _: usize) {
    if game.turn != 4 {
        return;
    }
    for lane in (0..2).rev() {
        for card in game.lanes[lane].cards.clone() {
            game.move_card(card, lane + 1);
        }
    }
}

// BoardwalkBehavior::BoostLowestPowerCards @ 0x7d412e8, predicates
// 0x7d419f0 and 0x7d41a20: all tied lowest-power revealed characters.
fn boardwalk(game: &mut Game<'_>, lane: usize) {
    let cards: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&card| game.cards[card].revealed && game.is_character(card))
        .collect();
    if let Some(lowest) = cards.iter().map(|&card| game.cards[card].power()).min() {
        for card in cards {
            if game.cards[card].power() == lowest {
                game.add_power(card, 1, game.cards[card].owner);
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
    fn place(game: &mut Game<'_>, id: &str, seat: usize, lane: usize) -> usize {
        let card = game.create_card(id, seat);
        assert!(game.add_card(card, lane, seat, false));
        card
    }
    #[test]
    fn cave_observed_choices_preserve_two_random_draws() {
        let mut rng = PythonRandom::seed(18);
        let mut control_rng = PythonRandom::seed(18);
        let control = game(&mut control_rng);
        let mut game = game(&mut rng);
        game.turn = 4;
        game.lanes[0].state = Some(Box::new([Some("IronMan"), Some("BlueMarvel")]));
        cave_of_the_dragon(&mut game, 0);
        assert!(
            game.lanes[0]
                .cards
                .iter()
                .any(|&id| game.cards[id].def.id == "IronMan")
        );
        assert!(
            game.lanes[0]
                .cards
                .iter()
                .any(|&id| game.cards[id].def.id == "BlueMarvel")
        );
        assert!(control.random_card(cave_candidate).is_some());
        assert!(control.random_card(cave_candidate).is_some());
        assert_eq!(
            game.rng.borrow_mut().randbelow(10000),
            control.rng.borrow_mut().randbelow(10000)
        );
    }
    #[test]
    fn dark_dimension_holds_only_its_lane_until_game_end() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        game.lanes[0].def = &DARK_DIMENSION;
        game.lanes[0].revealed = true;
        let hidden = place(&mut game, "Hawkeye", 0, 0);
        let ordinary = place(&mut game, "Hawkeye", 1, 1);
        game.cards[hidden].revealed = false;
        game.cards[ordinary].revealed = false;
        game.reveal(hidden);
        game.reveal(ordinary);
        assert!(!game.cards[hidden].revealed);
        assert!(game.cards[ordinary].revealed);
        game.reveal(hidden);
        assert_eq!(
            game.delayed_reveals[0]
                .iter()
                .map(|entry| entry.card)
                .collect::<Vec<_>>(),
            vec![hidden]
        );
        game.finish_reveals();
        assert!(game.cards[hidden].revealed);
    }
    #[test]
    fn bifrost_moves_right_to_left_and_leaves_hidden_cards() {
        let mut rng = PythonRandom::seed(2);
        let mut game = game(&mut rng);
        let a = place(&mut game, "Hawkeye", 0, 0);
        let b = place(&mut game, "Hawkeye", 0, 1);
        let hidden = place(&mut game, "Hawkeye", 1, 0);
        game.cards[hidden].revealed = false;
        game.turn = 4;
        bifrost(&mut game, 0);
        assert_eq!(game.cards[a].lane, Some(1));
        assert_eq!(game.cards[b].lane, Some(2));
        assert_eq!(game.cards[hidden].lane, Some(0));
    }
    #[test]
    fn boardwalk_boosts_all_lowest_ties() {
        let mut rng = PythonRandom::seed(3);
        let mut game = game(&mut rng);
        let a = place(&mut game, "Hawkeye", 0, 0);
        let b = place(&mut game, "Hawkeye", 1, 0);
        let c = place(&mut game, "Hawkeye", 1, 0);
        game.cards[c].permanent += 1;
        let before = game.cards[a].power();
        boardwalk(&mut game, 0);
        assert_eq!(game.cards[a].power(), before + 1);
        assert_eq!(game.cards[b].power(), before + 1);
        assert_eq!(game.cards[c].power(), before + 1);
    }
    #[test]
    fn collapsed_mine_skipping_destroys_rocks_across_lanes_but_actions_prevent_it() {
        let mut rng = PythonRandom::seed(4);
        let mut game = game(&mut rng);
        let rock = place(&mut game, "Rock", 0, 2);
        let enemy = place(&mut game, "Rock", 1, 1);
        game.players[0].non_play_action_turn = Some(game.turn);
        collapsed_mine_skip(&mut game, 0);
        assert_eq!(game.cards[rock].lane, Some(2));
        assert_eq!(game.cards[enemy].lane, None);
        game.turn += 1;
        collapsed_mine_skip(&mut game, 0);
        assert_eq!(game.cards[rock].lane, None);
    }
    #[test]
    fn vormir_consumes_once_per_player_even_when_destroy_blocked() {
        let mut rng = PythonRandom::seed(5);
        let mut game = game(&mut rng);
        game.lanes[0].def = &VORMIR;
        game.lanes[0].revealed = true;
        game.lanes[0].state = VORMIR.initial_state.map(|f| f());
        let first = place(&mut game, "Hawkeye", 0, 1);
        let second = place(&mut game, "Hawkeye", 0, 0);
        let enemy = place(&mut game, "Hawkeye", 1, 0);
        for card in [first, second, enemy] {
            game.cards[card].turn_played = 1;
        }
        game.played[0].extend([(first, 0), (second, 0)]);
        game.played[1].push((enemy, 0));
        // A played card may have moved out before PostCardResolved.
        game.lanes[1].def = crate::locations::get("Wakanda").unwrap();
        game.lanes[1].revealed = true;
        vormir(&mut game, 0, Event::CardResolved { card: first });
        vormir(&mut game, 0, Event::CardResolved { card: second });
        vormir(&mut game, 0, Event::CardResolved { card: enemy });
        assert_eq!(game.cards[first].lane, Some(1));
        assert_eq!(game.cards[second].lane, Some(0));
        assert_eq!(game.cards[enemy].lane, None);
    }
}
