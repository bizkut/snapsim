//! Card rules from the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[CardDef {
    ongoing: Some(sera),
    ..CardDef::vanilla("Sera")
}];

// SeraBehavior::ReduceCardCost (0x7f369b0): AddCostWithMin(-1, min(1, current)).
fn sera(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for index in 0..game.players[owner].hand.len() {
        let target = game.players[owner].hand[index];
        if game.cards[target].cost + game.cards[target].ongoing_cost > 1 {
            game.add_ongoing_cost(target, -1, owner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    #[test]
    fn sera_discounts_hand_without_raising_zero_or_lowering_one() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "MistyKnight",
            "Hulk",
            "Cyclops",
            "Shocker",
            "TheThing",
            "Abomination",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        let source = game.create_card("Sera", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let zero = game.create_card("Wasp", 0);
        let one = game.create_card("MistyKnight", 0);
        let six = game.create_card("Hulk", 0);
        game.players[0].hand.clear();
        game.players[0].hand.extend([zero, one, six]);
        game.ongoing();
        assert_eq!(
            [
                game.card_cost(zero),
                game.card_cost(one),
                game.card_cost(six)
            ],
            [0, 1, 5]
        );
        game.cards[source].abilities_removed = true;
        game.ongoing();
        assert_eq!(game.card_cost(six), 6);
    }
}
