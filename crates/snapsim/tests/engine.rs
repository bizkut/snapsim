use snapsim::{game::Game, locations, random::PythonRandom};

fn game(rng: &mut PythonRandom) -> Game<'_> {
    let ids = [
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
    ];
    let deck: Vec<String> = ids.into_iter().map(str::to_owned).collect();
    let locations: Vec<String> = ["Ruins", "Xandar", "KamarTaj"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut result = Game::new([&deck, &deck], &locations, rng).unwrap();
    for (lane, name) in ["Ruins", "KamarTaj", "Xandar"].into_iter().enumerate() {
        result.lanes[lane].def = locations::get(name).unwrap();
        result.lanes[lane].revealed = true;
    }
    result
}
fn play(g: &mut Game, name: &str, owner: usize, lane: usize) -> usize {
    let id = g.create_card(name, owner);
    g.cards[id].lane = Some(lane);
    g.cards[id].turn_played = g.turn;
    g.lanes[lane].cards.push(id);
    g.played[owner].push((id, lane));
    g.reveal(id);
    id
}
#[test]
fn repeated_reveal_blocking_and_ongoing() {
    let mut rng = PythonRandom::seed(1);
    let mut g = game(&mut rng);
    let medusa = play(&mut g, "Medusa", 0, 1);
    assert_eq!(g.cards[medusa].permanent, 8);
    let cosmo = play(&mut g, "Cosmo", 1, 1);
    let second = play(&mut g, "Medusa", 0, 1);
    assert_eq!(g.cards[second].permanent, 2);
    assert!(g.cards[cosmo].revealed);
    let iron = play(&mut g, "IronMan", 0, 2);
    let misty = play(&mut g, "MistyKnight", 0, 2);
    assert_eq!(
        g.powers()[2][0],
        (g.cards[iron].power() + g.cards[misty].power()) * 2
    );
}
#[test]
fn move_watch_and_destroy_hooks() {
    let mut rng = PythonRandom::seed(1);
    let mut g = game(&mut rng);
    let kraven = play(&mut g, "Kraven", 0, 0);
    let moving = play(&mut g, "MistyKnight", 1, 2);
    let before = g.cards[kraven].permanent;
    assert!(g.move_card(moving, 0));
    assert_eq!(g.cards[kraven].permanent, before + 2);
    let hawkeye = play(&mut g, "Hawkeye", 0, 1);
    assert_eq!(g.cards[hawkeye].watches.len(), 2);
    g.turn += 1;
    g.played = Default::default();
    play(&mut g, "Wasp", 0, 1);
    let before = g.cards[hawkeye].permanent;
    (g.cards[hawkeye].def.end_turn.unwrap())(&mut g, hawkeye);
    assert_eq!(g.cards[hawkeye].permanent, before + 8);
    let ally = play(&mut g, "Shocker", 0, 2);
    let carnage = play(&mut g, "Carnage", 0, 2);
    assert!(g.cards[ally].lane.is_none());
    assert_eq!(g.cards[carnage].permanent, 4);
}
