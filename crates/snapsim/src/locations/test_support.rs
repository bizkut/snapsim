//! Shared fixtures for the location RNG regression tests.
use crate::game::Game;
use crate::random::PythonRandom;

/// A game whose three lanes are Ruins, so no location rule interferes.
pub(crate) fn rng_game(rng: &mut PythonRandom) -> Game<'_> {
    rng_game_in(rng, &["Ruins", "Xandar", "Wakanda"])
}

/// Like [`rng_game`] with a larger location pool, so random location picks have candidates
/// beyond the three starting locations.
pub(crate) fn rng_game_in<'a>(rng: &'a mut PythonRandom, pool: &[&str]) -> Game<'a> {
    let deck: Vec<String> = [
        "Hulk",
        "Medusa",
        "Sentinel",
        "IronMan",
        "Hawkeye",
        "Wasp",
        "Cyclops",
        "Shocker",
        "TheThing",
        "MistyKnight",
        "Abomination",
        "Quicksilver",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let locations: Vec<String> = pool.iter().map(|id| (*id).to_owned()).collect();
    let mut game = Game::new([&deck, &deck], &locations, rng).unwrap();
    for lane in &mut game.lanes {
        lane.def = crate::locations::get("Ruins").unwrap();
        lane.revealed = true;
    }
    game
}

/// Puts `count` revealed Hulks of `seat` into `lane`.
pub(crate) fn fill(game: &mut Game<'_>, seat: usize, lane: usize, count: usize) -> Vec<usize> {
    (0..count)
        .map(|_| {
            let card = game.create_card("Hulk", seat);
            assert!(game.add_card(card, lane, seat, false));
            game.cards[card].revealed = true;
            card
        })
        .collect()
}

/// Replaces `seat`'s hand with `count` Hulks.
pub(crate) fn set_hand(game: &mut Game<'_>, seat: usize, count: usize) {
    game.players[seat].hand.clear();
    for _ in 0..count {
        let card = game.create_card("Hulk", seat);
        assert!(game.add_hand(seat, card, false));
    }
}

/// Asserts `game` consumed exactly the draws `expected` did.
pub(crate) fn assert_same_stream(game: &Game<'_>, expected: &mut dyn crate::random::GameRng) {
    assert_eq!(
        game.rng.borrow_mut().getrandbits(32),
        expected.getrandbits(32)
    );
}
