//! High Evolutionary's separate client card definitions, client 59.15.1.
use super::CardDef;
use crate::game::{Game, StatOperation};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        in_hand_ongoing: Some(abomination),
        ..CardDef::vanilla("EvolvedAbomination")
    },
    CardDef {
        end_turn: Some(cyclops),
        end_turn_trigger: true,
        ..CardDef::vanilla("EvolvedCyclops")
    },
    CardDef {
        end_turn: Some(hulk),
        end_turn_trigger: true,
        end_turn_in_hand: true,
        ..CardDef::vanilla("EvolvedHulk")
    },
    CardDef {
        end_turn: Some(misty),
        end_turn_trigger: true,
        ..CardDef::vanilla("EvolvedMistyKnight")
    },
    CardDef {
        reveal: Some(shocker),
        ..CardDef::vanilla("EvolvedShocker")
    },
    CardDef {
        reveal: Some(thing),
        ..CardDef::vanilla("EvolvedTheThing")
    },
    CardDef {
        reveal: Some(wasp),
        ..CardDef::vanilla("EvolvedWasp")
    },
];
pub(crate) static TOKENS: &[&str] = &[
    "EvolvedAbomination",
    "EvolvedCyclops",
    "EvolvedHulk",
    "EvolvedMistyKnight",
    "EvolvedShocker",
    "EvolvedTheThing",
    "EvolvedWasp",
];

// DiscountSelf 0x7ea2c94 counts each revealed opponent once if either
// modifier or stat-update ledger contains a negative additive power entry.
fn abomination(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game
        .lanes
        .iter()
        .flat_map(|lane| &lane.cards)
        .filter(|&&id| {
            let target = &game.cards[id];
            target.owner != owner
                && target.revealed
                && game.can_reduce_card_power(id)
                && (game.is_afflicted(id)
                    || target
                        .power_modifiers
                        .iter()
                        .any(|m| m.operation == StatOperation::Add && m.value < 0))
        })
        .count() as i32;
    if count > 0 {
        // EvolvedAbominationBehavior::DiscountSelf (0x7ea2c94) sets
        // the reduced current cost, clamped to zero.
        let cost = game.cards[card].cost + game.cards[card].ongoing_cost;
        game.set_ongoing_cost(card, (cost - count).max(0), owner);
    }
}

// ReduceEnemyCardsPower 0x7ea3464 shuffles revealed enemies before taking two.
fn cyclops(game: &mut Game<'_>, card: usize) {
    if game.players[game.cards[card].owner].energy > 0 {
        afflict_shuffled(game, card, 2, false);
    }
}
// BuffSelf 0x7ea3b6c triggers in hand or on the board.
fn hulk(game: &mut Game<'_>, card: usize) {
    if game.players[game.cards[card].owner].energy > 0 {
        game.add_power(card, 2, card);
    }
}
// BuffAlly 0x7ea3f34 shuffles all other revealed allies across locations.
fn misty(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].energy <= 0 {
        return;
    }
    let mut targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| id != card && game.cards[id].owner == owner && game.cards[id].revealed)
        .collect();
    shuffle(game, &mut targets);
    if let Some(&target) = targets.first() {
        game.add_power(target, 1, card);
    }
}
// ReduceCost 0x7ea447c targets Hand::GetTopCard, the leftmost card.
fn shocker(game: &mut Game<'_>, card: usize) {
    if let Some(&target) = game.players[game.cards[card].owner].hand.first() {
        game.add_cost(target, -1, card);
    }
}
// SubtractPower 0x7ea47cc filters enemies able to lose power, shuffles,
// then takes three distinct targets.
fn thing(game: &mut Game<'_>, card: usize) {
    afflict_shuffled(game, card, 3, true);
}
// SubtractPower 0x7ea5058 rolls once among revealed enemies at this location.
fn wasp(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].owner != owner && game.cards[id].revealed)
        .collect();
    if !targets.is_empty() {
        let index = game.rng.borrow_mut().randbelow(targets.len());
        game.add_power(targets[index], -1, card);
    }
}
fn shuffle(game: &mut Game<'_>, targets: &mut [usize]) {
    for i in 0..targets.len().saturating_sub(1) {
        let j = i + game.rng.borrow_mut().randbelow(targets.len() - i);
        targets.swap(i, j);
    }
}
fn afflict_shuffled(game: &mut Game<'_>, card: usize, count: usize, only_reducible: bool) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            game.cards[id].owner != owner
                && game.cards[id].revealed
                && (!only_reducible || game.can_reduce_card_power(id))
        })
        .collect();
    shuffle(game, &mut targets);
    for target in targets.into_iter().take(count) {
        game.add_power(target, -1, card);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    fn setup<'a>(rng: &'a mut PythonRandom) -> Game<'a> {
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
        Game::new([&deck, &deck], &lanes, rng).unwrap()
    }
    fn put(game: &mut Game<'_>, id: &str, owner: usize, lane: usize) -> usize {
        let card = game.create_card(id, owner);
        game.add_card(card, lane, owner, true);
        card
    }
    #[test]
    fn evolved_abomination_counts_afflicted_enemies_once() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("EvolvedAbomination", 0);
        let enemy = put(&mut game, "Hulk", 1, 0);
        game.add_power(enemy, -1, card);
        game.add_power(enemy, -1, card);
        abomination(&mut game, card);
        assert_eq!(game.cards[card].ongoing_cost, -1);
        assert_eq!(
            game.cards[card].stat_updates.last().unwrap().operation,
            StatOperation::Set
        );
    }
    #[test]
    fn evolved_cyclops_needs_energy_and_hits_two() {
        let mut rng = PythonRandom::seed(2);
        let mut game = setup(&mut rng);
        let card = put(&mut game, "EvolvedCyclops", 0, 0);
        let enemies = [
            put(&mut game, "Hulk", 1, 0),
            put(&mut game, "Hulk", 1, 0),
            put(&mut game, "Hulk", 1, 0),
        ];
        game.players[0].energy = 0;
        cyclops(&mut game, card);
        assert_eq!(
            enemies
                .iter()
                .filter(|&&id| game.cards[id].permanent == 13)
                .count(),
            0
        );
        game.players[0].energy = 1;
        cyclops(&mut game, card);
        assert_eq!(
            enemies
                .iter()
                .filter(|&&id| game.cards[id].permanent == 13)
                .count(),
            2
        );
    }
    #[test]
    fn evolved_hulk_buffs_with_unspent_energy() {
        let mut rng = PythonRandom::seed(3);
        let mut game = setup(&mut rng);
        let card = game.create_card("EvolvedHulk", 0);
        game.players[0].energy = 0;
        hulk(&mut game, card);
        assert_eq!(game.cards[card].permanent, 14);
        game.players[0].energy = 1;
        hulk(&mut game, card);
        assert_eq!(game.cards[card].permanent, 16);
    }
    #[test]
    fn evolved_misty_buffs_another_ally() {
        let mut rng = PythonRandom::seed(4);
        let mut game = setup(&mut rng);
        let card = put(&mut game, "EvolvedMistyKnight", 0, 0);
        let ally = put(&mut game, "Hulk", 0, 1);
        game.players[0].energy = 1;
        misty(&mut game, card);
        assert_eq!(game.cards[ally].permanent, 15);
        assert_eq!(game.cards[card].permanent, 3);
    }
    #[test]
    fn evolved_shocker_discounts_leftmost() {
        let mut rng = PythonRandom::seed(5);
        let mut game = setup(&mut rng);
        let card = game.create_card("EvolvedShocker", 0);
        let target = game.players[0].hand[0];
        let cost = game.cards[target].cost;
        shocker(&mut game, card);
        assert_eq!(game.cards[target].cost, cost - 1);
    }
    #[test]
    fn evolved_thing_afflicts_three_distinct_enemies() {
        let mut rng = PythonRandom::seed(6);
        let mut game = setup(&mut rng);
        let card = put(&mut game, "EvolvedTheThing", 0, 0);
        let enemies = [
            put(&mut game, "Hulk", 1, 0),
            put(&mut game, "Hulk", 1, 0),
            put(&mut game, "Hulk", 1, 0),
            put(&mut game, "Hulk", 1, 0),
        ];
        thing(&mut game, card);
        assert_eq!(
            enemies
                .iter()
                .filter(|&&id| game.cards[id].permanent == 13)
                .count(),
            3
        );
    }
    #[test]
    fn evolved_wasp_afflicts_one_enemy() {
        let mut rng = PythonRandom::seed(7);
        let mut game = setup(&mut rng);
        let card = put(&mut game, "EvolvedWasp", 0, 0);
        let enemy = put(&mut game, "Hulk", 1, 0);
        wasp(&mut game, card);
        assert_eq!(game.cards[enemy].permanent, 13);
    }
}
