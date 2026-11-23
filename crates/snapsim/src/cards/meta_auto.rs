//! Autonomous staging cards.
use crate::cards::CardDef;
use crate::game::Game;

// AgathaHarknessBehavior::StartOfGameAbilities (0x7e60570) draws this
// particular card from its owner's deck before the normal opening draw.
fn agatha_game_start(game: &mut Game<'_>, card: usize) {
    let seat = game.cards[card].owner;
    game.draw_specific(seat, card);
}

pub static CARDS: &[CardDef] = &[CardDef {
    pre_game_start: Some(agatha_game_start),
    ..CardDef::vanilla("AgathaHarkness")
}];

#[cfg(test)]
mod tests {
    use crate::game::Game;
    use crate::random::PythonRandom;

    #[test]
    fn agatha_draws_before_three_regular_opening_cards() {
        let deck = [
            "AgathaHarkness",
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "KamarTaj"].map(str::to_owned);
        let mut rng = PythonRandom::seed(1);
        let game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        for player in &game.players {
            assert_eq!(player.hand.len(), 4);
            assert!(
                player
                    .hand
                    .iter()
                    .any(|&card| game.cards[card].def.id == "AgathaHarkness")
            );
        }
    }
}
