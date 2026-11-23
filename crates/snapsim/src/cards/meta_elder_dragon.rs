use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[CardDef {
    conditional_in_hand_ongoing: Some(discount),
    ..CardDef::vanilla("ElderDragonOfTheMoon")
}];

// DiscountSelf 0x7e9dfe4: IsInHand and revealed allied cards satisfying
// WithIncreasedPower 0x7d29874; ConditionalLocation priority.
fn discount(game: &mut Game<'_>, card: usize) -> bool {
    let owner = game.cards[card].owner;
    if !game.players[owner].hand.contains(&card) {
        return false;
    }
    let count = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&target| {
            target != card
                && game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.has_increased_power(target)
        })
        .count() as i32;
    if count == 0 {
        return false;
    }
    game.add_ongoing_cost(card, -count, owner);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn counts_revealed_increased_allies_only_while_in_hand() {
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
        let dragon = game.create_card("ElderDragonOfTheMoon", 0);
        game.players[0].hand.push(dragon);
        let ally = game.create_card("Hulk", 0);
        game.add_card(ally, 0, 0, true);
        game.add_power(ally, 2, dragon);
        let enemy = game.create_card("Hulk", 1);
        game.add_card(enemy, 1, 1, true);
        game.add_power(enemy, 2, dragon);
        game.cards[dragon].ongoing_cost = 0;
        assert!(discount(&mut game, dragon));
        assert_eq!(game.cards[dragon].ongoing_cost, -1);
        game.players[0].hand.clear();
        assert!(!discount(&mut game, dragon));
    }
}
