mod draw;
mod energy;
mod hand;
mod location;
#[cfg(test)]
mod movement_tests;
mod resurrect;
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
