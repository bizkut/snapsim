//! Locations, batch D3: rules from the 59.15.1 client.

use super::{LocationDef, location};
use crate::game::{CardState, Event, Game};

pub const CHRONOSPHERE_SPHINX: LocationDef = LocationDef {
    on_reveal: Some(chronosphere_sphinx),
    ..location("ChronosphereSphinx", 0, 0, 1, false)
};
pub const CRIMSON_CITADEL: LocationDef = LocationDef {
    initial_state: Some(|| Box::new([false; 2]) as Box<dyn CardState>),
    on_event: Some(crimson_citadel),
    ..location("CrimsonCitadel", 0, 0, 1, false)
};
pub const HALA: LocationDef = LocationDef {
    on_turn_end: Some(hala),
    ..location("Hala", 0, 0, 1, false)
};
pub const JOSIES_BAR: LocationDef = LocationDef {
    on_staging_end: Some(josies_bar),
    ..location("JosiesBar", 0, 0, 1, false)
};
pub const KLYNTAR: LocationDef = LocationDef {
    on_turn_end: Some(klyntar),
    ..location("Klyntar", 0, 0, 1, false)
};
pub const TA_LO: LocationDef = LocationDef {
    on_turn_end: Some(ta_lo),
    ..location("TaLo", 0, 0, 1, false)
};

// ChronosphereSphinxBehavior::CopyCard 0x7d478e4 and its cost selector
// 0x7d481cc: one roll over the tied highest-cost deck cards per player.
fn chronosphere_sphinx(game: &mut Game<'_>, _: usize) {
    for seat in 0..2 {
        let deck = &game.players[seat].deck;
        let Some(highest) = deck.iter().map(|&id| game.printed_cost(id)).max() else {
            continue;
        };
        let choices: Vec<_> = deck
            .iter()
            .copied()
            .filter(|&id| game.printed_cost(id) == highest)
            .collect();
        let slot = game.rng.borrow_mut().randbelow(choices.len());
        if game.players[seat].hand.len() < 7 {
            let copy = game.copy_card(choices[slot], seat);
            game.add_hand(seat, copy, false);
        }
    }
}

// CrimsonCitadelBehavior::StealForNextPlay 0x7d4b3c0 spends the trigger
// even when no enemy can be selected.
fn crimson_citadel(game: &mut Game<'_>, lane: usize, event: Event) {
    let Event::CardResolved { card } = event else {
        return;
    };
    let seat = game.cards[card].owner;
    if !game.played[seat]
        .iter()
        .any(|&(id, played_lane)| id == card && played_lane == lane)
    {
        return;
    }
    let Some(spent) = game.lanes[lane]
        .state
        .as_deref_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<[bool; 2]>())
    else {
        return;
    };
    if spent[seat] {
        return;
    }
    spent[seat] = true;
    let choices: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|l| l.cards.iter().copied())
        .filter(|&id| {
            game.cards[id].owner != seat && game.cards[id].revealed && game.is_character(id)
        })
        .collect();
    if !choices.is_empty() {
        let index = game.rng.borrow_mut().randbelow(choices.len());
        game.steal_power(choices[index], card, 2, seat);
    }
}

// HalaBehavior::DestroyCards 0x7d57c9c checks the losing side at turn 4.
fn hala(game: &mut Game<'_>, lane: usize) {
    if game.turn != 4 {
        return;
    }
    let power = game.powers()[lane];
    if power[0] == power[1] {
        return;
    }
    let loser = usize::from(power[1] < power[0]);
    let victims: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].owner == loser && game.cards[id].revealed)
        .collect();
    for victim in victims {
        game.destroy_card(victim);
    }
}

// JosiesBarBehavior::BuffUnrevealedCards 0x7d5b898: exactly two
// staged plays, excluding the turn in which the location revealed.
fn josies_bar(game: &mut Game<'_>, lane: usize) {
    if game.lanes[lane].revealed_turn == Some(game.turn) {
        return;
    }
    for seat in 0..2 {
        let cards: Vec<_> = game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&id| {
                game.cards[id].owner == seat
                    && !game.cards[id].revealed
                    && game.cards[id].turn_played == game.turn
            })
            .collect();
        if cards.len() == 2 {
            for id in cards {
                game.add_power(id, 2, seat);
            }
        }
    }
}

// KlyntarBehavior::SymbioteMerge 0x7d5cd5c snapshots both sides' total
// power, morphs the first revealed card and merges the rest.
fn klyntar(game: &mut Game<'_>, lane: usize) {
    if game.turn != 4 {
        return;
    }
    let cards: [Vec<usize>; 2] = std::array::from_fn(|seat| {
        game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&id| game.cards[id].owner == seat && game.cards[id].revealed)
            .collect()
    });
    let powers: [i32; 2] =
        std::array::from_fn(|seat| cards[seat].iter().map(|&id| game.cards[id].power()).sum());
    for seat in [
        game.reveal_priority.unwrap_or(0),
        1 - game.reveal_priority.unwrap_or(0),
    ] {
        let Some((&first, rest)) = cards[seat].split_first() else {
            continue;
        };
        if game.transform_card_without_modifiers(first, "Symbiote") {
            for &id in rest {
                game.merge_card(first, id);
            }
            game.set_power(first, powers[seat], seat);
        }
    }
}

// TaLoBehavior::PlayCards 0x7d87688; predicate 0x7d87e7c selects
// characters only and rolls once per nonempty eligible hand.
fn ta_lo(game: &mut Game<'_>, _: usize) {
    for seat in 0..2 {
        let choices: Vec<_> = game.players[seat]
            .hand
            .iter()
            .copied()
            .filter(|&id| game.is_character(id))
            .collect();
        if !choices.is_empty() {
            let index = game.rng.borrow_mut().randbelow(choices.len());
            game.add_power(choices[index], 1, seat);
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

    fn place(g: &mut Game<'_>, name: &str, seat: usize, lane: usize) -> usize {
        let id = g.create_card(name, seat);
        assert!(g.add_card(id, lane, seat, false));
        id
    }
    #[test]
    fn sphinx_selects_printed_cost_not_modified_cost() {
        let mut rng = PythonRandom::seed(6);
        let mut g = game(&mut rng);
        g.players[0].deck.clear();
        g.players[1].deck.clear();
        g.players[0].hand.clear();
        let low = g.create_card("Hawkeye", 0);
        let high = g.create_card("Hulk", 0);
        g.cards[low].cost = 20;
        g.players[0].deck.extend([low, high]);
        chronosphere_sphinx(&mut g, 0);
        assert_eq!(g.cards[g.players[0].hand[0]].def.id, "Hulk");
    }

    #[test]
    fn citadel_spends_once_even_without_a_target() {
        let mut rng = PythonRandom::seed(7);
        let mut g = game(&mut rng);
        g.lanes[0].def = &CRIMSON_CITADEL;
        g.lanes[0].state = CRIMSON_CITADEL.initial_state.map(|f| f());
        let first = place(&mut g, "Hawkeye", 0, 0);
        g.played[0].push((first, 0));
        crimson_citadel(&mut g, 0, Event::CardResolved { card: first });
        let enemy = place(&mut g, "Hawkeye", 1, 1);
        let second = place(&mut g, "AntMan", 0, 0);
        let power = g.cards[enemy].permanent;
        g.played[0].push((second, 0));
        crimson_citadel(&mut g, 0, Event::CardResolved { card: second });
        assert_eq!(g.cards[enemy].permanent, power);
    }

    #[test]
    fn citadel_steals_from_revealed_enemy_once() {
        let mut rng = PythonRandom::seed(8);
        let mut g = game(&mut rng);
        g.lanes[0].state = CRIMSON_CITADEL.initial_state.map(|f| f());
        let first = place(&mut g, "Hawkeye", 0, 0);
        let enemy = place(&mut g, "Hawkeye", 1, 1);
        let before = [g.cards[first].permanent, g.cards[enemy].permanent];
        g.played[0].push((first, 0));
        crimson_citadel(&mut g, 0, Event::CardResolved { card: first });
        crimson_citadel(&mut g, 0, Event::CardResolved { card: first });
        assert_eq!(
            [g.cards[first].permanent, g.cards[enemy].permanent],
            [before[0] + 2, before[1] - 2]
        );
    }

    #[test]
    fn staging_hook_buffs_exactly_two_before_reveal() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        g.lanes[0].def = &JOSIES_BAR;
        g.lanes[0].revealed = true;
        g.turn = 2;
        let a = place(&mut g, "Hawkeye", 0, 0);
        let b = place(&mut g, "AntMan", 0, 0);
        for id in [a, b] {
            g.cards[id].revealed = false;
            g.cards[id].turn_played = 2;
        }
        let before = [g.cards[a].permanent, g.cards[b].permanent];
        g.resolve_staged_actions(0);
        assert_eq!(
            [g.cards[a].permanent, g.cards[b].permanent],
            [before[0] + 2, before[1] + 2]
        );
    }

    #[test]
    fn hala_tie_and_loser() {
        let mut rng = PythonRandom::seed(2);
        let mut g = game(&mut rng);
        g.turn = 4;
        let a = place(&mut g, "Hawkeye", 0, 0);
        let b = place(&mut g, "Hawkeye", 1, 0);
        hala(&mut g, 0);
        assert!(g.cards[a].lane.is_some() && g.cards[b].lane.is_some());
        g.add_power(a, 2, 0);
        hala(&mut g, 0);
        assert!(g.cards[a].lane.is_some() && g.cards[b].lane.is_none());
    }

    #[test]
    fn klyntar_morphs_single_and_merges_multiple() {
        let mut rng = PythonRandom::seed(3);
        let mut g = game(&mut rng);
        g.turn = 4;
        let a = place(&mut g, "Hawkeye", 0, 0);
        let b = place(&mut g, "AntMan", 0, 0);
        let c = place(&mut g, "Hawkeye", 1, 0);
        let sum = g.cards[a].power() + g.cards[b].power();
        klyntar(&mut g, 0);
        assert_eq!(g.cards[a].def.id, "Symbiote");
        assert_eq!(g.cards[a].power(), sum);
        assert_eq!(g.cards[b].merged_to, Some(a));
        assert_eq!(g.cards[c].def.id, "Symbiote");
    }

    #[test]
    fn ta_lo_buffs_only_characters() {
        let mut rng = PythonRandom::seed(4);
        let mut g = game(&mut rng);
        for seat in 0..2 {
            g.players[seat].hand.clear();
            let id = g.create_card("Hawkeye", seat);
            g.players[seat].hand.push(id);
        }
        let ids = [g.players[0].hand[0], g.players[1].hand[0]];
        let before = [g.cards[ids[0]].permanent, g.cards[ids[1]].permanent];
        ta_lo(&mut g, 0);
        assert_eq!(
            [g.cards[ids[0]].permanent, g.cards[ids[1]].permanent],
            [before[0] + 1, before[1] + 1]
        );
    }
}
