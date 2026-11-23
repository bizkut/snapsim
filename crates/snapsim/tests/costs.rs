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

    // Card::AddCost (0x7cb69e4) clamps each update to zero, not just the
    // eventual displayed cost: a later increase starts from that floor.
    game.add_ongoing_cost(held, -10, 1);
    assert_eq!(
        game.cards[held].stat_updates.last().unwrap().operation,
        snapsim::game::StatOperation::Set
    );
    game.add_ongoing_cost(held, 1, 1);
    assert_eq!(game.card_cost(held), 1);

    game.lanes[2].def = locations::get("Ruins").unwrap();
    let low = game.create_card("Wasp", 0);
    let high = game.create_card("Hulk", 0);
    assert!(game.add_hand(0, low, false));
    assert!(game.add_hand(0, high, false));
    game.turn = 3;
    game.players[0].attach_cost_effect(snapsim::game::PlayerCostEffectKind::Wave { until: 3 });
    game.ongoing();
    assert_eq!(game.card_cost(low), 0);
    assert!(
        !game.cards[low]
            .stat_updates
            .iter()
            .any(|update| update.stat == snapsim::game::Stat::Cost)
    );
    assert_eq!(game.card_cost(high), 4);

    game.turn = 4;
    game.players[0].attach_cost_effect(snapsim::game::PlayerCostEffectKind::Sandman { turn: 4 });
    game.ongoing();
    assert_eq!(game.card_cost(high), 6);
    assert_eq!(
        game.cards[high].stat_updates.last().unwrap().operation,
        snapsim::game::StatOperation::Set
    );
    assert_eq!(game.card_cost(low), 1);
    assert_eq!(
        game.cards[low].stat_updates.last().unwrap().operation,
        snapsim::game::StatOperation::Add
    );
}

#[test]
fn sandbar_updates_only_created_cards_and_preserves_cap_operation() {
    use snapsim::game::{Stat, StatOperation};

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
    .map(str::to_owned);
    let lanes = ["TheSandbar", "Ruins", "Xandar"].map(str::to_owned);
    let mut rng = PythonRandom::seed(4);
    let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
    for lane in 0..3 {
        game.lanes[lane].def =
            locations::get(if lane == 0 { "TheSandbar" } else { "Ruins" }).unwrap();
        game.lanes[lane].revealed = true;
    }
    game.players[0].hand.clear();
    let cards = [(3, true), (6, true), (7, true), (3, false)].map(|(cost, created)| {
        let card = game.create_card("Hulk", 0);
        game.cards[card].cost = cost;
        game.cards[card].created = created;
        game.players[0].hand.push(card);
        card
    });
    game.ongoing();
    for (card, cost, operation) in [
        (cards[0], 4, Some(StatOperation::Add)),
        (cards[1], 6, Some(StatOperation::Set)),
        (cards[2], 7, Some(StatOperation::Set)),
        (cards[3], 3, None),
    ] {
        assert_eq!(game.card_cost(card), cost);
        let updates: Vec<_> = game.cards[card]
            .stat_updates
            .iter()
            .filter(|update| update.stat == Stat::Cost && update.ongoing)
            .map(|update| update.operation)
            .collect();
        assert_eq!(updates, operation.into_iter().collect::<Vec<_>>());
    }
}
