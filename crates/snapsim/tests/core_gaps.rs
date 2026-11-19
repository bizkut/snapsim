use snapsim::{game::Game, locations, random::PythonRandom};

fn game(rng: &mut PythonRandom) -> Game<'_> {
    let deck: Vec<_> = [
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
    let lanes: Vec<_> = ["Ruins", "Xandar", "KamarTaj"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut game = Game::new([&deck, &deck], &lanes, rng).unwrap();
    for lane in &mut game.lanes {
        lane.def = locations::get("Ruins").unwrap();
        lane.revealed = true;
    }
    game
}

fn play(game: &mut Game<'_>, name: &str, lane: usize, seat: usize) -> usize {
    let card = game.create_card(name, seat);
    game.cards[card].turn_played = game.turn;
    game.cards[card].lane = Some(lane);
    game.lanes[lane].cards.push(card);
    game.reveal(card);
    card
}

#[test]
fn enchantress_removes_active_ongoing_and_armor_protection() {
    let mut rng = PythonRandom::seed(12);
    let mut game = game(&mut rng);
    let armor = play(&mut game, "Armor", 0, 0);
    let ally = play(&mut game, "MistyKnight", 0, 0);
    assert!(!game.destroy_card(ally));
    play(&mut game, "Enchantress", 0, 1);
    assert!(!game.cards[armor].has_ongoing());
    assert!(game.destroy_card(ally));
}

#[test]
fn colossus_resists_reduction_destruction_and_movement() {
    let mut rng = PythonRandom::seed(17);
    let mut game = game(&mut rng);
    let card = play(&mut game, "Colossus", 0, 0);
    let power = game.cards[card].power();
    game.cards[card].reduce_power(3);
    assert_eq!(game.cards[card].power(), power);
    assert!(!game.move_card(card, 1));
    assert!(!game.destroy_card(card));
    game.replace_location(0, "NegativeZone");
    assert_eq!(game.cards[card].power(), power);
}

#[test]
fn limbo_replacement_and_ongoing_repetition() {
    let mut rng = PythonRandom::seed(19);
    let mut game = game(&mut rng);
    play(&mut game, "MistyKnight", 0, 0);
    play(&mut game, "IronMan", 0, 0);
    let before = game.powers()[0][0];
    play(&mut game, "Onslaught", 0, 0);
    assert!(game.powers()[0][0] > before);
    play(&mut game, "Magik", 1, 0);
    assert_eq!(game.lanes[1].def.id, "Limbo");
    assert_eq!(game.total_turns, 7);
}

#[test]
fn delayed_card_is_not_revealed_until_delay_clears() {
    let mut rng = PythonRandom::seed(20);
    let mut game = game(&mut rng);
    game.replace_location(0, "Lemuria");
    let card = play(&mut game, "Medusa", 1, 0);
    assert!(!game.cards[card].revealed);
    assert_eq!(game.delayed_reveals[0], vec![card]);
    game.delays_reveals = false;
    game.reveal(card);
    assert!(game.cards[card].revealed);
}

#[test]
fn copies_preserve_custom_state_but_clear_hawkeye_watches() {
    let mut rng = PythonRandom::seed(21);
    let mut game = game(&mut rng);
    let hawkeye = play(&mut game, "Hawkeye", 0, 0);
    assert!(!game.cards[hawkeye].watch_contexts().is_empty());
    game.cards[hawkeye].state = Some(Box::new(vec![3_i32, 5]));
    let copy = game.copy_card(hawkeye, 1);
    assert!(game.cards[copy].watch_contexts().is_empty());
    assert_eq!(
        game.cards[copy]
            .state
            .as_deref()
            .unwrap()
            .as_any()
            .downcast_ref::<Vec<i32>>()
            .unwrap(),
        &[3, 5]
    );
    game.cards[copy]
        .state
        .as_deref_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Vec<i32>>()
        .unwrap()
        .push(7);
    assert_eq!(
        game.cards[hawkeye]
            .state
            .as_deref()
            .unwrap()
            .as_any()
            .downcast_ref::<Vec<i32>>()
            .unwrap(),
        &[3, 5]
    );
}

#[test]
fn iron_fist_moves_next_play_and_multiple_man_leaves_copy() {
    let mut rng = PythonRandom::seed(22);
    let mut game = game(&mut rng);
    let kraven = play(&mut game, "Kraven", 0, 0);
    play(&mut game, "IronFist", 1, 0);
    let multiple = play(&mut game, "MultipleMan", 1, 0);
    assert_eq!(game.cards[multiple].lane, Some(0));
    assert_eq!(game.cards[kraven].permanent, 5);
    assert!(
        game.lanes[1]
            .cards
            .iter()
            .any(|&id| game.cards[id].def.id == "MultipleMan")
    );
}

#[test]
fn joaquin_and_odin_repeat_eligible_reveals() {
    let mut rng = PythonRandom::seed(23);
    let mut game = game(&mut rng);
    play(&mut game, "JoaquinTorres", 0, 0);
    let hawkeye = play(&mut game, "Hawkeye", 0, 0);
    assert_eq!(game.cards[hawkeye].watch_contexts().len(), 2);
    play(&mut game, "Odin", 0, 0);
    assert_eq!(game.cards[hawkeye].watch_contexts().len(), 4);
}
