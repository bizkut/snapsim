use snapsim::{game::Game, locations, random::PythonRandom};

#[test]
fn conditional_location_actions_report_retry_or_completion() {
    let mut rng = PythonRandom::seed(1);
    let deck = [
        "Hawkeye",
        "Medusa",
        "Wasp",
        "Hulk",
        "IronMan",
        "Cyclops",
        "AntMan",
        "Shocker",
        "Sentinel",
        "TheThing",
        "Quicksilver",
        "Abomination",
    ]
    .map(str::to_owned)
    .to_vec();
    let location_ids = ["Ruins", "Xandar", "KamarTaj"].map(str::to_owned).to_vec();
    let mut game = Game::new([&deck, &deck], &location_ids, &mut rng).unwrap();
    for name in ["MonsterMetropolis", "WakandanThroneRoom"] {
        let hook = locations::get(name).unwrap().conditional_ongoing.unwrap();
        assert!(!hook(&mut game, 0), "{name} with no targets");
        let card = game.create_card("Hawkeye", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        assert!(hook(&mut game, 0), "{name} with a revealed card");
        game.lanes[0].cards.clear();
    }
    let crown = locations::get("CrownCity")
        .unwrap()
        .conditional_ongoing
        .unwrap();
    assert!(
        crown(&mut game, 1),
        "a tie still succeeds when neighbors exist"
    );
    game.lanes[0].destroyed = true;
    game.lanes[2].destroyed = true;
    assert!(!crown(&mut game, 1), "no extant neighbors requires a retry");
    for name in ["BaxterBuilding", "Hospice"] {
        assert!(
            locations::get(name).unwrap().conditional_ongoing.unwrap()(&mut game, 1),
            "{name} succeeds without targets"
        );
    }
}
