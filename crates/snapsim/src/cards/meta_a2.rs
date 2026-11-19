//! Meta-deck cards, batch A2: rules from the 59.15.1 client that need no new engine mechanics.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(anti_polar_magneto),
        ..CardDef::vanilla("AntiPolarMagneto")
    },
    CardDef {
        reveal: Some(arnim_zola),
        ..CardDef::vanilla("ArnimZola")
    },
    CardDef {
        reveal: Some(dragon_lord),
        ..CardDef::vanilla("DragonLord")
    },
    CardDef {
        reveal: Some(laufey),
        ..CardDef::vanilla("Laufey")
    },
    CardDef {
        ongoing: Some(mordred),
        ..CardDef::vanilla("Mordred")
    },
    CardDef {
        reveal: Some(mother_askani),
        ..CardDef::vanilla("MotherAskani")
    },
    CardDef {
        on_destroy: Some(nimrod),
        ..CardDef::vanilla("Nimrod")
    },
    CardDef {
        reveal: Some(tombstone),
        ..CardDef::vanilla("Tombstone")
    },
    CardDef {
        reveal: Some(venom),
        ..CardDef::vanilla("Venom")
    },
];

pub(crate) static TOKENS: &[&str] = &[];

fn anti_polar_magneto(game: &mut Game<'_>, id: usize) {
    let Some(origin) = game.cards[id].lane else {
        return;
    };
    let owner = game.cards[id].owner;
    let mut candidates: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&card| matches!(game.printed_cost(card), 3 | 4))
        .collect();
    // AddCardsFromHand (0x7e669ec) shuffles once, then scans for cards still in hand. The
    // generic Shuffle (0x4b12f30) runs forward: for each i < n - 1, swap i with Roll(i, n).
    let count = candidates.len();
    for index in 0..count.saturating_sub(1) {
        let selected = index + game.rng.borrow_mut().randbelow(count - index);
        candidates.swap(index, selected);
    }
    for lane in 0..3 {
        if lane == origin {
            continue;
        }
        let Some(&card) = candidates
            .iter()
            .find(|&&card| game.players[owner].hand.contains(&card))
        else {
            break;
        };
        game.move_hand_card_to_location(card, lane);
    }
}

fn arnim_zola(game: &mut Game<'_>, id: usize) {
    let Some(origin) = game.cards[id].lane else {
        return;
    };
    let owner = game.cards[id].owner;
    let targets: Vec<_> = game.lanes[origin]
        .cards
        .iter()
        .copied()
        .filter(|&card| card != id && game.cards[card].revealed && game.cards[card].owner == owner)
        .collect();
    if targets.is_empty() {
        return;
    }
    let selected = game.rng.borrow_mut().randbelow(targets.len());
    let target = targets[selected];
    // DestroyCard (0x7e6b298) creates its snapshot before destruction triggers.
    let snapshot = game.copy_card(target, owner);
    if game.destroy_card(target) {
        for lane in 0..3 {
            if lane != origin {
                game.copy_card_to_location(snapshot, lane, owner);
            }
        }
        game.resolve_deferred_cards();
    }
}

fn dragon_lord(game: &mut Game<'_>, id: usize) {
    let Some(lane) = game.cards[id].lane else {
        return;
    };
    let owner = game.cards[id].owner;
    let hand = &game.players[owner].hand;
    if hand.is_empty() {
        return;
    }
    // PlayCard (0x7e91938) selects before checking whether the lane is full.
    let choice = game.rng.borrow_mut().randbelow(hand.len());
    let card = hand[choice];
    game.move_hand_card_to_location(card, lane);
}

fn mother_askani(game: &mut Game<'_>, id: usize) {
    let owner = game.cards[id].owner;
    let Some(&first) = game.players[owner].hand.first() else {
        return;
    };
    if let Some(copy) = game.copy_card_to_hand(first, owner) {
        gain_power(game, copy, 2, id);
        change_cost(game, copy, -1, id);
        game.notify_hand_added(owner);
    }
}

fn nimrod(game: &mut Game<'_>, id: usize) {
    let Some(origin) = game.cards[id].last_lane else {
        return;
    };
    let owner = game.cards[id].owner;
    for lane in 0..3 {
        if lane != origin {
            game.copy_card_to_location(id, lane, owner);
        }
    }
    game.resolve_deferred_cards();
}

fn gain_power(game: &mut Game<'_>, card: usize, amount: i32, source: usize) {
    game.add_power(card, amount, source);
}

fn change_cost(game: &mut Game<'_>, card: usize, amount: i32, source: usize) {
    game.add_cost(card, amount, source);
}

fn steal_power(game: &mut Game<'_>, victims: &[usize], card: usize, amount: i32) {
    game.steal_power_many(victims, card, amount, card);
}

fn ongoing_power(game: &mut Game<'_>, card: usize, amount: i32) {
    game.add_ongoing_power(card, amount, card);
}

fn laufey(game: &mut Game<'_>, id: usize) {
    let Some(lane) = game.cards[id].lane else {
        return;
    };
    let victims: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&other| other != id && game.cards[other].revealed)
        .collect();
    // StealPowerPermanently (0x7cb796c) batches the increase into one event.
    steal_power(game, &victims, id, 1);
}

fn mordred(game: &mut Game<'_>, id: usize) {
    let Some(lane) = game.cards[id].lane else {
        return;
    };
    let cost = game.printed_cost(id);
    // GetTargetCount (0x7f02318) counts revealed cards of either player;
    // its predicate (0x7f02768) compares DefCost (get_DefCost 0x7cac794).
    let count = game.lanes[lane]
        .cards
        .iter()
        .filter(|&&other| game.cards[other].revealed && game.printed_cost(other) > cost)
        .count() as i32;
    ongoing_power(game, id, 2 * count);
}

fn tombstone(game: &mut Game<'_>, id: usize) {
    let Some(origin) = game.cards[id].lane else {
        return;
    };
    let owner = game.cards[id].owner;
    let mut victims = Vec::new();
    for lane in 0..3 {
        if lane == origin {
            continue;
        }
        let eligible: Vec<_> = game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&card| game.cards[card].revealed && game.cards[card].owner == owner)
            .collect();
        let Some(lowest) = eligible.iter().map(|&card| game.cards[card].power()).min() else {
            continue;
        };
        let tied: Vec<_> = eligible
            .into_iter()
            .filter(|&card| game.cards[card].power() == lowest)
            .collect();
        // GetLowestPowerCardForPlayer (0x7ddf014) rolls even for one candidate.
        let index = game.rng.borrow_mut().randbelow(tied.len());
        victims.push(tied[index]);
    }
    // DestroyCards (0x7cdf6a0) selects both victims before destroying either.
    for victim in victims {
        game.destroy_card(victim);
    }
}

fn venom(game: &mut Game<'_>, id: usize) {
    let Some(lane) = game.cards[id].lane else {
        return;
    };
    let owner = game.cards[id].owner;
    let victims: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&other| {
            other != id && game.cards[other].revealed && game.cards[other].owner == owner
        })
        .collect();
    let mut total = 0;
    for victim in victims {
        // EatCards (0x7ceab98) snapshots each target immediately before destroying it.
        let power = game.cards[victim].power();
        if game.destroy_card(victim) {
            total += power;
            game.ongoing();
        }
    }
    if total != 0 {
        gain_power(game, id, total, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    fn game(rng: &mut PythonRandom) -> Game<'_> {
        let deck: Vec<String> = [
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
        .into_iter()
        .map(str::to_owned)
        .collect();
        let names: Vec<String> = ["Ruins", "Xandar", "Wakanda"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut game = Game::new([&deck, &deck], &names, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game
    }

    fn place(game: &mut Game<'_>, name: &str, owner: usize, lane: usize, revealed: bool) -> usize {
        let id = game.create_card(name, owner);
        game.cards[id].lane = Some(lane);
        game.cards[id].revealed = revealed;
        game.lanes[lane].cards.push(id);
        id
    }

    #[test]
    fn laufey_steals_from_revealed_both_sides_even_at_zero_power() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let card = place(&mut g, "Laufey", 0, 0, true);
        let ally = place(&mut g, "Wasp", 0, 0, true);
        let enemy = place(&mut g, "Wasp", 1, 0, true);
        let hidden = place(&mut g, "Wasp", 1, 0, false);
        g.cards[ally].permanent = 0;
        g.cards[enemy].permanent = 0;
        g.cards[hidden].permanent = 0;
        let original = g.cards[card].permanent;
        let ally_power = g.cards[ally].permanent;
        laufey(&mut g, card);
        assert_eq!(g.cards[card].permanent, original + 2);
        assert_eq!(g.cards[ally].permanent, ally_power - 1);
        assert_eq!(g.cards[enemy].permanent, ally_power - 1);
        assert_eq!(g.cards[hidden].permanent, ally_power);
    }

    #[test]
    fn mordred_counts_strictly_higher_revealed_cost_on_either_side() {
        let mut rng = PythonRandom::seed(2);
        let mut g = game(&mut rng);
        let card = place(&mut g, "Mordred", 0, 0, true);
        let ally = place(&mut g, "Cyclops", 0, 0, true);
        let enemy = place(&mut g, "Hulk", 1, 0, true);
        let hidden = place(&mut g, "Hulk", 1, 0, false);
        g.cards[ally].cost = 6;
        g.cards[enemy].cost = 0;
        g.ongoing();
        assert_eq!(g.cards[card].ongoing, 2);
        g.cards[hidden].revealed = true;
        g.ongoing();
        assert_eq!(g.cards[card].ongoing, 4);
    }

    #[test]
    fn tombstone_ignores_unrevealed_and_destroys_lowest_at_other_lanes() {
        let mut rng = PythonRandom::seed(3);
        let mut g = game(&mut rng);
        let card = place(&mut g, "Tombstone", 0, 1, true);
        let low = place(&mut g, "Wasp", 0, 0, true);
        let high = place(&mut g, "Hulk", 0, 0, true);
        let hidden = place(&mut g, "Wasp", 0, 2, false);
        let enemy = place(&mut g, "Wasp", 1, 2, true);
        tombstone(&mut g, card);
        assert_eq!(g.cards[low].lane, None);
        assert_eq!(g.cards[high].lane, Some(0));
        assert_eq!(g.cards[hidden].lane, Some(2));
        assert_eq!(g.cards[enemy].lane, Some(2));
    }

    #[test]
    fn venom_absorbs_only_successfully_destroyed_revealed_allies() {
        let mut rng = PythonRandom::seed(4);
        let mut g = game(&mut rng);
        let card = place(&mut g, "Venom", 0, 0, true);
        let ally = place(&mut g, "Hulk", 0, 0, true);
        let hidden = place(&mut g, "Hulk", 0, 0, false);
        let enemy = place(&mut g, "Hulk", 1, 0, true);
        let expected = g.cards[card].permanent + g.cards[ally].power();
        venom(&mut g, card);
        assert_eq!(g.cards[card].permanent, expected);
        assert_eq!(g.cards[ally].lane, None);
        assert_eq!(g.cards[hidden].lane, Some(0));
        assert_eq!(g.cards[enemy].lane, Some(0));
    }

    #[test]
    fn anti_polar_magneto_skips_full_lane_and_moves_only_matching_costs() {
        let mut rng = PythonRandom::seed(5);
        let mut g = game(&mut rng);
        let card = place(&mut g, "AntiPolarMagneto", 0, 1, true);
        for _ in 0..4 {
            place(&mut g, "Wasp", 0, 0, true);
        }
        g.players[0].hand.clear();
        let eligible = g.create_card("TheThing", 0);
        g.cards[eligible].cost = 1;
        let ineligible = g.create_card("Wasp", 0);
        g.players[0].hand.extend([eligible, ineligible]);
        anti_polar_magneto(&mut g, card);
        assert_eq!(g.cards[eligible].lane, Some(2));
        assert!(g.players[0].hand.contains(&ineligible));
        assert_eq!(g.cards[eligible].turn_played, 0);
    }

    #[test]
    fn zola_copies_destroyed_ally_with_its_permanent_power() {
        let mut rng = PythonRandom::seed(6);
        let mut g = game(&mut rng);
        let card = place(&mut g, "ArnimZola", 0, 1, true);
        let target = place(&mut g, "Hulk", 0, 1, true);
        g.add_power(target, 3, card);
        let power = g.cards[target].permanent;
        arnim_zola(&mut g, card);
        assert_eq!(g.cards[target].lane, None);
        for lane in [0, 2] {
            let copy = g.lanes[lane].cards[0];
            assert_eq!(g.cards[copy].def.id, "Hulk");
            assert_eq!(g.cards[copy].permanent, power);
        }
    }

    #[test]
    fn dragon_lord_moves_a_hand_card_without_recording_a_play() {
        let mut rng = PythonRandom::seed(7);
        let mut g = game(&mut rng);
        let card = place(&mut g, "DragonLord", 0, 1, true);
        g.players[0].hand.clear();
        let target = g.create_card("Hulk", 0);
        g.players[0].hand.push(target);
        dragon_lord(&mut g, card);
        assert_eq!(g.cards[target].lane, Some(1));
        assert!(g.cards[target].revealed);
        assert_eq!(g.cards[target].turn_played, 0);
        assert!(g.players[0].hand.is_empty());
    }

    #[test]
    fn askani_copies_leftmost_with_modifiers_and_respects_hand_limit() {
        let mut rng = PythonRandom::seed(8);
        let mut g = game(&mut rng);
        let card = place(&mut g, "MotherAskani", 0, 1, true);
        g.players[0].hand.clear();
        let first = g.create_card("Hulk", 0);
        g.add_power(first, 3, card);
        g.players[0].hand.push(first);
        mother_askani(&mut g, card);
        let copy = g.players[0].hand[1];
        assert_eq!(g.cards[copy].permanent, g.cards[first].permanent + 2);
        assert_eq!(g.cards[copy].cost, g.cards[first].cost - 1);
        while g.players[0].hand.len() < 7 {
            let extra = g.create_card("Wasp", 0);
            g.players[0].hand.push(extra);
        }
        mother_askani(&mut g, card);
        assert_eq!(g.players[0].hand.len(), 7);
    }

    #[test]
    fn nimrod_copies_its_modified_state_to_other_locations() {
        let mut rng = PythonRandom::seed(9);
        let mut g = game(&mut rng);
        let card = place(&mut g, "Nimrod", 0, 1, true);
        g.add_power(card, 3, card);
        let power = g.cards[card].permanent;
        assert!(g.destroy_card(card));
        for lane in [0, 2] {
            let copy = g.lanes[lane].cards[0];
            assert_eq!(g.cards[copy].def.id, "Nimrod");
            assert_eq!(g.cards[copy].permanent, power);
        }
    }

    #[test]
    fn destroy_immunity_prevents_venom_power_and_zola_copies() {
        let mut rng = PythonRandom::seed(10);
        let mut g = game(&mut rng);
        g.lanes[1].def = locations::get("Wakanda").unwrap();
        let venom_card = place(&mut g, "Venom", 0, 1, true);
        let target = place(&mut g, "Hulk", 0, 1, true);
        let before = g.cards[venom_card].permanent;
        venom(&mut g, venom_card);
        assert_eq!(g.cards[venom_card].permanent, before);
        assert_eq!(g.cards[target].lane, Some(1));
        g.lanes[1].cards.retain(|&id| id != venom_card);
        g.cards[venom_card].lane = None;
        let zola = place(&mut g, "ArnimZola", 0, 1, true);
        arnim_zola(&mut g, zola);
        assert_eq!(g.cards[target].lane, Some(1));
        assert!(g.lanes[0].cards.is_empty());
        assert!(g.lanes[2].cards.is_empty());
    }
}
