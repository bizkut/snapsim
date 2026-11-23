pub(crate) mod abilities;
pub(crate) mod autoplay;
mod draw;
mod energy;
mod hand;
mod location;
pub(crate) mod merge;
#[cfg(test)]
mod movement_tests;
pub(crate) mod quickdraw;
mod resurrect;
pub(crate) mod sinister;
mod transfer;

#[cfg(test)]
fn test_game(rng: &mut crate::random::PythonRandom) -> crate::game::Game<'_> {
    let deck = [
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
    .map(str::to_owned)
    .to_vec();
    let locations = ["Ruins", "Xandar", "KamarTaj"].map(str::to_owned).to_vec();
    crate::game::Game::new([&deck, &deck], &locations, rng).unwrap()
}
