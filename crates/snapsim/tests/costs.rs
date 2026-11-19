use snapsim::{game::Game, locations, random::PythonRandom};

#[test]
fn location_cost_modifiers_apply_to_hand_cards_only() {
    // Found by the full parity gate: at Elysium ("Cards cost 1 less") Elektra must
    // still see a 2-Cost card on the board as 2-Cost, as Python's energy_cost does.
    let deck: Vec<String> = [
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
    .into_iter()
    .map(str::to_owned)
    .collect();
    let pool: Vec<String> = ["Ruins", "Xandar", "KamarTaj"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut rng = PythonRandom::seed(3);
    let mut game = Game::new([&deck, &deck], &pool, &mut rng).unwrap();
    for lane in 0..3 {
        game.lanes[lane].def = locations::get(if lane == 2 { "Elysium" } else { "Ruins" }).unwrap();
        game.lanes[lane].revealed = true;
    }

    let held = game.create_card("Shocker", 1);
    assert!(game.add_hand(1, held, false));
    assert_eq!(game.card_cost(held), 1);

    let on_board = game.create_card("Shocker", 1);
    game.cards[on_board].lane = Some(2);
    game.lanes[2].cards.push(on_board);
    assert_eq!(game.card_cost(on_board), 2);
}
