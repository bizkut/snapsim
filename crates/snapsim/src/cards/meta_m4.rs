//! Card rules traced from the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        on_activate: Some(warlock),
        ..CardDef::vanilla("Warlock")
    },
    CardDef {
        reveal: Some(lady_bullseye),
        ..CardDef::vanilla("LadyBullseye")
    },
    CardDef {
        end_turn: Some(thaddeus_ross),
        end_turn_trigger: true,
        ..CardDef::vanilla("ThaddeusRoss")
    },
];

// WarlockBehavior::ReduceCosts 0x7cedb5c, ThisCardActivatedTrigger;
// its candidate predicate at 0x7cee094 tests the current cost is positive.
fn warlock(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut targets = Vec::with_capacity(game.players[owner].hand.len());
    for _ in 0..game.players[owner].energy.max(0) {
        targets.clear();
        targets.extend(
            game.players[owner]
                .hand
                .iter()
                .copied()
                .filter(|&target| game.card_cost(target) > 0),
        );
        if targets.is_empty() {
            break;
        }
        let index = game.rng.borrow_mut().randbelow(targets.len());
        game.add_cost(targets[index], -1, card);
    }
}

// LadyBullseyeBehavior::AfflictEnemies 0x7ee2c8c: shuffle all revealed
// enemies, then take one plus the owner's unspent energy.
fn lady_bullseye(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&target| game.cards[target].owner != owner && game.cards[target].revealed)
        .collect();
    for i in 0..targets.len().saturating_sub(1) {
        let j = i + game.rng.borrow_mut().randbelow(targets.len() - i);
        targets.swap(i, j);
    }
    let count = (game.players[owner].energy.max(0) as usize).saturating_add(1);
    for target in targets.into_iter().take(count) {
        game.add_power(target, -2, card);
    }
}

// ThaddeusRossBehavior::DrawStrongCard 0x7cd58c8; the predicate at
// 0x7cd5db4 compares the card's current power with AssignedPower (10).
fn thaddeus_ross(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[1 - owner].energy == 0 {
        return;
    }
    if let Some(target) = game.players[owner]
        .deck
        .iter()
        .copied()
        .find(|&target| game.cards[target].power() >= 10)
    {
        game.draw_specific(owner, target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn warlock_discounts_once_per_unspent_energy_and_skips_zero_cost() {
        let mut rng = PythonRandom::seed(1);
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
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Warlock", 0);
        let target = game.create_card("Hulk", 0);
        let free = game.create_card("Wasp", 0);
        game.players[0].hand = vec![target, free];
        game.players[0].energy = 3;
        warlock(&mut game, source);
        assert_eq!(game.card_cost(target), 3);
        assert_eq!(game.card_cost(free), 0);
        assert_eq!(game.players[0].energy, 3);
    }

    #[test]
    fn lady_bullseye_afflicts_distinct_revealed_enemies() {
        let mut rng = PythonRandom::seed(4);
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
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("LadyBullseye", 0);
        let a = game.create_card("Hulk", 1);
        let b = game.create_card("Hulk", 1);
        let c = game.create_card("Hulk", 1);
        for target in [a, b, c] {
            game.cards[target].lane = Some(0);
            game.cards[target].revealed = true;
            game.lanes[0].cards.push(target);
        }
        game.players[0].energy = 1;
        lady_bullseye(&mut game, source);
        assert_eq!(
            [a, b, c]
                .iter()
                .filter(|&&id| game.cards[id].permanent == 12)
                .count(),
            2
        );
    }

    #[test]
    fn thaddeus_draws_first_strong_card_only_if_opponent_has_energy() {
        let mut rng = PythonRandom::seed(5);
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
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("ThaddeusRoss", 0);
        let weak = game.create_card("Wasp", 0);
        let strong = game.create_card("Hulk", 0);
        game.players[0].hand.clear();
        game.players[0].deck = vec![weak, strong];
        game.players[1].energy = 0;
        thaddeus_ross(&mut game, source);
        assert!(game.players[0].hand.is_empty());
        game.players[1].energy = 1;
        thaddeus_ross(&mut game, source);
        assert_eq!(game.players[0].hand, vec![strong]);
        assert_eq!(game.players[0].deck, vec![weak]);
    }
}
