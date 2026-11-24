//! Location rules from the 59.15.1 client.
use super::{CostUpdate, LocationDef, location};
use crate::game::Game;

pub const JOTUNHEIM: LocationDef = LocationDef {
    on_turn_end: Some(jotunheim),
    ..location("Jotunheim", 0, 0, 1, false)
};
pub const UTOPIA: LocationDef = LocationDef {
    modify_cost: Some(utopia),
    ..location("Utopia", 0, 0, 1, false)
};
pub const CAMP_LEHIGH: LocationDef = LocationDef {
    on_reveal: Some(camp_lehigh),
    ..location("CampLehigh", 0, 0, 1, false)
};
pub const ATTILAN: LocationDef = LocationDef {
    on_turn_end: Some(attilan),
    ..location("Attilan", 0, 0, 1, false)
};

// JotunheimBehavior::ReducePower @ 0x7d5c09c: all revealed cards.
fn jotunheim(game: &mut Game<'_>, lane: usize) {
    for card in game.lanes[lane].cards.clone() {
        if game.cards[card].revealed {
            game.add_power(card, -1, game.cards[card].owner);
        }
    }
}

// UtopiaBehavior::ReduceCardCost @ 0x7d9059c checks the card's cost field.
fn utopia(game: &Game<'_>, _: usize, card: usize, _: i32) -> Option<CostUpdate> {
    matches!(game.cards[card].cost, 3 | 4).then_some(CostUpdate::Add(-1))
}

// CampLehighBehavior::AddRandomCard 0x7d42a84 calls ordered players at
// 0x7d42b70. The pool is restricted to simulator-implemented cards.
fn camp_lehigh(game: &mut Game<'_>, _: usize) {
    let first = game.reveal_priority.unwrap_or(0);
    for seat in [first, 1 - first] {
        if let Some(id) = game.random_card(|row| row.cost == 3) {
            let card = game.create_card(id, seat);
            game.add_hand(seat, card, false);
        }
    }
}

// AttilanBehavior::ShuffleAndDraw @ 0x7d3d6e0.
fn attilan(game: &mut Game<'_>, _: usize) {
    if game.turn != 3 {
        return;
    }
    for seat in 0..2 {
        game.move_hand_to_deck(seat);
        game.shuffle_deck(seat);
        for _ in 0..3 {
            game.draw(seat);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn jotunheim_afflicts_revealed_cards_only() {
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
        let mut game = Game::new(
            [&deck, &deck],
            &[
                "Ruins".to_owned(),
                "Xandar".to_owned(),
                "Wakanda".to_owned(),
            ],
            &mut rng,
        )
        .unwrap();
        let a = game.create_card("Hawkeye", 0);
        let b = game.create_card("Hawkeye", 1);
        assert!(game.add_card(a, 0, 0, false));
        assert!(game.add_card(b, 0, 1, false));
        game.cards[a].revealed = true;
        game.cards[b].revealed = false;
        let before_a = game.cards[a].power();
        let before_b = game.cards[b].power();
        jotunheim(&mut game, 0);
        assert_eq!(game.cards[a].power(), before_a - 1);
        assert_eq!(game.cards[b].power(), before_b);
    }

    #[test]
    fn utopia_checks_card_cost_not_accumulated_cost() {
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
        let card = game.create_card("Bishop", 0);
        assert_eq!(utopia(&game, 0, card, 2), Some(CostUpdate::Add(-1)));
        game.cards[card].cost = 2;
        assert_eq!(utopia(&game, 0, card, 2), None);
    }

    #[test]
    fn attilan_only_shuffles_after_turn_three() {
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
        game.turn = 2;
        let hand = game.players[0].hand.clone();
        attilan(&mut game, 0);
        assert_eq!(game.players[0].hand, hand);
        game.turn = 3;
        attilan(&mut game, 0);
        assert_eq!(game.players[0].hand.len(), 3);
        assert_eq!(game.players[1].hand.len(), 3);
    }

    #[test]
    fn camp_lehigh_adds_a_three_cost_to_both_hands() {
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
        camp_lehigh(&mut game, 0);
        for seat in 0..2 {
            let card = *game.players[seat].hand.last().unwrap();
            assert_eq!(game.cards[card].cost, 3);
            assert!(game.cards[card].created);
        }
    }

    #[test]
    fn camp_lehigh_rolls_in_initiative_order() {
        let mut rng = PythonRandom::seed(42);
        let mut oracle_rng = PythonRandom::seed(42);
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
        let oracle = Game::new([&deck, &deck], &locations, &mut oracle_rng).unwrap();
        game.reveal_priority = Some(1);
        let expected = [
            oracle.random_card(|row| row.cost == 3).unwrap(),
            oracle.random_card(|row| row.cost == 3).unwrap(),
        ];
        camp_lehigh(&mut game, 0);
        assert_eq!(
            game.cards[*game.players[1].hand.last().unwrap()].def.id,
            expected[0]
        );
        assert_eq!(
            game.cards[*game.players[0].hand.last().unwrap()].def.id,
            expected[1]
        );
    }
}
