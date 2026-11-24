use snapsim::{game::Game, locations, random::PythonRandom};

fn game(rng: &mut PythonRandom) -> Game<'_> {
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
    let names: Vec<String> = ["Ruins", "Xandar", "Wakanda"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut g = Game::new([&deck, &deck], &names, rng).unwrap();
    for lane in &mut g.lanes {
        lane.def = locations::get("Ruins").unwrap();
        lane.revealed = true;
    }
    g
}
fn play(g: &mut Game<'_>, name: &str, owner: usize, lane: usize) -> usize {
    let id = g.create_card(name, owner);
    g.cards[id].lane = Some(lane);
    g.cards[id].turn_played = g.turn;
    g.lanes[lane].cards.push(id);
    g.played[owner].push((id, lane));
    g.reveal(id);
    id
}
#[test]
fn discard_returns_same_apocalypse_and_stacks() {
    let mut rng = PythonRandom::seed(1);
    let mut g = game(&mut rng);
    let a = g.create_card("Apocalypse", 0);
    let power = g.cards[a].permanent;
    g.add_hand(0, a, false);
    g.discard_card(0, a);
    g.discard_card(0, a);
    assert_eq!(g.players[0].hand.last(), Some(&a));
    assert_eq!(g.cards[a].permanent, power + 8);
    assert!(!g.players[0].discard.contains(&a));
}
#[test]
fn cable_steals_top_card_and_korg_shuffles_a_rock_into_the_enemy_deck() {
    let mut rng = PythonRandom::seed(2);
    let mut g = game(&mut rng);
    let top = g.players[1].deck[0];
    play(&mut g, "Cable", 0, 0);
    assert_eq!(g.players[0].hand.last(), Some(&top));
    assert_eq!(g.cards[top].owner, 0);
    assert!(!g.players[1].deck.contains(&top));
    let before = g.players[1].deck.clone();
    play(&mut g, "Korg", 0, 1);
    let rock = *g.players[1]
        .deck
        .iter()
        .find(|&&id| g.cards[id].def.id == "Rock")
        .unwrap();
    assert_eq!(g.cards[rock].owner, 1);
    // KorgBehavior shuffles the whole deck after adding the Rock (client Game::Shuffle),
    // so only the multiset of the other cards is preserved, not their order.
    let mut after: Vec<_> = g.players[1]
        .deck
        .iter()
        .copied()
        .filter(|&id| id != rock)
        .collect();
    let mut expected = before;
    after.sort_unstable();
    expected.sort_unstable();
    assert_eq!(after, expected);
}
#[test]
fn elektra_and_deathlok_only_destroy_revealed_targets() {
    let mut rng = PythonRandom::seed(3);
    let mut g = game(&mut rng);
    let enemy = play(&mut g, "MistyKnight", 1, 0);
    let expensive = play(&mut g, "Shocker", 1, 0);
    let hidden = g.create_card("MistyKnight", 1);
    g.cards[hidden].lane = Some(0);
    g.lanes[0].cards.push(hidden);
    play(&mut g, "Elektra", 0, 0);
    assert_eq!(g.cards[enemy].lane, None);
    assert_eq!(g.cards[expensive].lane, Some(0));
    assert_eq!(g.cards[hidden].lane, Some(0));
    let ally = play(&mut g, "Wasp", 0, 1);
    let deathlok = play(&mut g, "Deathlok", 0, 1);
    assert_eq!(g.cards[ally].lane, None);
    assert_eq!(g.cards[deathlok].lane, Some(1));
}

#[test]
fn elektra_uses_one_python_random_choice_over_eligible_targets() {
    for seed in 0..10 {
        let mut rng = PythonRandom::seed(seed);
        let mut expected_rng = PythonRandom::seed(seed);
        let mut g = game(&mut rng);
        let expected_game = game(&mut expected_rng);
        let expected = expected_game.rng.borrow_mut().randbelow(2);
        let targets = [
            play(&mut g, "MistyKnight", 1, 0),
            play(&mut g, "Hawkeye", 1, 0),
        ];
        play(&mut g, "Elektra", 0, 0);
        assert_eq!(g.cards[targets[expected]].lane, None);
        assert_eq!(g.cards[targets[1 - expected]].lane, Some(0));
    }
}
#[test]
fn ongoing_counts_and_guardian_play_timing() {
    let mut rng = PythonRandom::seed(4);
    let mut g = game(&mut rng);
    play(&mut g, "MistyKnight", 1, 0);
    let groot = play(&mut g, "Groot", 0, 0);
    assert_eq!(g.cards[groot].permanent, 9);
    let ant = play(&mut g, "AntMan", 0, 1);
    play(&mut g, "Wasp", 0, 1);
    play(&mut g, "Shocker", 0, 1);
    play(&mut g, "CaptainAmerica", 0, 1);
    g.ongoing();
    assert_eq!(g.cards[ant].ongoing, 6);
}

#[test]
fn forge_buffs_the_next_revealed_play_only() {
    let mut rng = PythonRandom::seed(5);
    let mut g = game(&mut rng);
    play(&mut g, "Forge", 0, 0);
    let target = play(&mut g, "MistyKnight", 0, 1);
    let next = play(&mut g, "MistyKnight", 0, 2);
    assert_eq!(g.cards[target].permanent, 5);
    assert_eq!(g.cards[next].permanent, 3);
}

#[test]
fn angel_replaces_destroyed_ally_from_hand_before_deck() {
    let mut rng = PythonRandom::seed(6);
    let mut g = game(&mut rng);
    let from_hand = g.create_card("Angel", 0);
    let from_deck = g.create_card("Angel", 0);
    g.players[0].hand.push(from_hand);
    g.players[0].deck.push(from_deck);
    let ally = play(&mut g, "Wasp", 0, 0);
    play(&mut g, "Deathlok", 0, 0);
    assert_eq!(g.cards[ally].lane, None);
    assert_eq!(g.cards[from_hand].lane, Some(0));
    assert!(g.cards[from_hand].revealed);
    assert!(g.players[0].deck.contains(&from_deck));
}
