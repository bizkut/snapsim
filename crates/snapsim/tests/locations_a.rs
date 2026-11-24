use snapsim::{game::Game, locations, random::PythonRandom};

fn game<'a>(rng: &'a mut PythonRandom, name: &str) -> Game<'a> {
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
    let mut game = Game::new([&deck, &deck], &pool, rng).unwrap();
    for lane in 0..3 {
        game.lanes[lane].def = locations::get(if lane == 0 { name } else { "Ruins" }).unwrap();
        game.lanes[lane].revealed = true;
    }
    game
}
fn play(game: &mut Game<'_>, name: &str, seat: usize, lane: usize) -> usize {
    let card = game.create_card(name, seat);
    game.cards[card].lane = Some(lane);
    game.cards[card].turn_played = game.turn;
    game.lanes[lane].cards.push(card);
    game.reveal(card);
    card
}
#[test]
fn atlantis_counts_unrevealed_cards() {
    let mut rng = PythonRandom::seed(1);
    let mut game = game(&mut rng, "Atlantis");
    let card = play(&mut game, "Wasp", 0, 0);
    assert_eq!(game.cards[card].location_power, 5);
    let other = game.create_card("Hulk", 0);
    game.cards[other].lane = Some(0);
    game.lanes[0].cards.push(other);
    game.ongoing();
    assert_eq!(game.cards[card].location_power, 0);
}
#[test]
fn lab_flooding_and_turn_restrictions() {
    let mut rng = PythonRandom::seed(2);
    let mut game = game(&mut rng, "MiniaturizedLab");
    let card = game.create_card("Wasp", 0);
    for turn in 1..=6 {
        game.turn = turn;
        assert_eq!(game.can_add(card, 0, 0), !(3..=5).contains(&turn));
    }
    game.lanes[0].def = locations::get("Flooding").unwrap();
    (game.lanes[0].def.on_turn_end.unwrap())(&mut game, 0);
    assert_eq!(game.lanes[0].def.id, "Flooded");
    assert!(!game.can_play(card, 0, 0, 6, 0));
}
#[test]
fn bar_sinister_does_not_repeat_reveal() {
    let mut rng = PythonRandom::seed(3);
    let mut game = game(&mut rng, "BarSinister");
    game.lanes[1].def = locations::get("BarSinister").unwrap();
    let card = play(&mut game, "Medusa", 0, 1);
    assert_eq!(game.lanes[1].cards.len(), 4);
    assert!(
        game.lanes[1]
            .cards
            .iter()
            .all(|&id| game.cards[id].revealed)
    );
    assert_eq!(game.cards[card].permanent, 5);
    assert!(
        game.lanes[1]
            .cards
            .iter()
            .all(|&id| game.cards[id].permanent == 5)
    );
}
#[test]
fn genosha_keeps_tied_highest_and_madripoor_buffs_ties() {
    let mut rng = PythonRandom::seed(4);
    let mut game = game(&mut rng, "Madripoor");
    let a = play(&mut game, "Hulk", 0, 0);
    let b = play(&mut game, "Hulk", 0, 0);
    let low = play(&mut game, "Wasp", 0, 0);
    let original = game.cards[a].permanent;
    (game.lanes[0].def.on_turn_end.unwrap())(&mut game, 0);
    assert_eq!(game.cards[a].permanent, original + 2);
    assert_eq!(game.cards[b].permanent, original + 2);
    game.lanes[0].def = locations::get("Genosha").unwrap();
    game.turn = 5;
    (game.lanes[0].def.on_turn_end.unwrap())(&mut game, 0);
    assert!(game.cards[low].lane.is_none());
    // The client keeps every card tied for the highest printed cost (no roll).
    assert!(game.cards[a].lane.is_some() && game.cards[b].lane.is_some());
}

#[test]
fn mindscape_swaps_hands_only_on_turn_six() {
    let mut rng = PythonRandom::seed(8);
    let mut game = game(&mut rng, "Mindscape");
    let before = [game.players[0].hand.clone(), game.players[1].hand.clone()];
    game.turn = 5;
    (game.lanes[0].def.on_turn_start.unwrap())(&mut game, 0);
    assert_eq!(game.players[0].hand, before[0]);
    game.turn = 6;
    (game.lanes[0].def.on_turn_start.unwrap())(&mut game, 0);
    assert_eq!(game.players[0].hand, before[1]);
    assert_eq!(game.players[1].hand, before[0]);
    assert!(
        game.players[0]
            .hand
            .iter()
            .all(|&id| game.cards[id].owner == 0)
    );
    assert!(
        game.players[1]
            .hand
            .iter()
            .all(|&id| game.cards[id].owner == 1)
    );
}

#[test]
fn central_park_reveals_tokens_on_all_lanes() {
    let mut rng = PythonRandom::seed(9);
    let mut game = game(&mut rng, "CentralPark");
    (game.lanes[0].def.on_reveal.unwrap())(&mut game, 0);
    for lane in &game.lanes {
        assert_eq!(lane.cards.len(), 2);
        assert!(
            lane.cards
                .iter()
                .all(|&id| game.cards[id].def.id == "Squirrel" && game.cards[id].revealed)
        );
        assert_ne!(
            game.cards[lane.cards[0]].owner,
            game.cards[lane.cards[1]].owner
        );
    }
}

#[test]
fn dream_and_elysium_apply_in_location_order() {
    let mut rng = PythonRandom::seed(10);
    let mut game = game(&mut rng, "DreamDimension");
    game.lanes[1].def = locations::get("Elysium").unwrap();
    // Location cost modifiers apply to hand cards, as Python's `_cost_adjustment` does.
    let card = game.create_card("Shocker", 0);
    assert!(game.add_hand(0, card, false));
    game.turn = 4;
    // StartTurn::TriggerStartOfTurn refreshes ongoing stats (0x7e4a36c).
    game.ongoing();
    assert_eq!(game.card_cost(card), 1);
    game.turn = 5;
    game.ongoing();
    assert_eq!(game.card_cost(card), 2);
    game.turn = 6;
    game.ongoing();
    assert_eq!(game.card_cost(card), 1);
}

#[test]
fn danger_room_rolls_zero_to_ninety_nine_per_play() {
    for seed in 0..20 {
        let mut first_rng = PythonRandom::seed(seed);
        let mut second_rng = PythonRandom::seed(seed);
        let mut first = game(&mut first_rng, "DangerRoom");
        let second = game(&mut second_rng, "DangerRoom");
        // Client: one integer Roll(0, 100), destroy below 25.
        let expected = second.rng.borrow_mut().randbelow(100) < 25;
        let card = first.create_card("Wasp", 0);
        first.cards[card].lane = Some(0);
        first.cards[card].revealed = true;
        first.lanes[0].cards.push(card);
        (first.lanes[0].def.after_card_play.unwrap())(&mut first, 0, card);
        assert_eq!(first.cards[card].lane.is_none(), expected);
        assert_eq!(
            first.rng.borrow_mut().random(),
            second.rng.borrow_mut().random()
        );
    }
}
