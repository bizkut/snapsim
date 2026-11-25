use snapsim::{game::Game, locations, random::PythonRandom};

fn game<'a>(rng: &'a mut PythonRandom, location: &str) -> Game<'a> {
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
    let pool: Vec<String> = [location, "Ruins", "Xandar"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut game = Game::new([&deck, &deck], &pool, rng).unwrap();
    game.lanes[0].def = locations::get(location).unwrap();
    game.lanes[0].revealed = true;
    game.lanes[0].revealed_turn = Some(1);
    game
}
fn play(game: &mut Game<'_>, name: &str, seat: usize) -> usize {
    let id = game.create_card(name, seat);
    game.cards[id].lane = Some(0);
    game.cards[id].turn_played = game.turn;
    game.lanes[0].cards.push(id);
    game.played[seat].push((id, 0));
    game.reveal(id);
    id
}
#[test]
fn proscenium_and_shuri_trigger_at_reveal() {
    let mut rng = PythonRandom::seed(1);
    let mut g = game(&mut rng, "Proscenium");
    let card = play(&mut g, "Medusa", 0);
    assert_eq!(g.cards[card].permanent, 4);
    g.lanes[0].def = locations::get("ShurisLab").unwrap();
    let card = play(&mut g, "MistyKnight", 0);
    assert_eq!(g.cards[card].permanent, 6);
}

#[test]
fn shuris_lab_uses_play_location_when_spider_island_moves_a_card() {
    let mut rng = PythonRandom::seed(2);
    let mut g = game(&mut rng, "ShurisLab");
    g.lanes[1].def = locations::get("SpiderIsland").unwrap();
    g.lanes[1].revealed = true;
    for _ in 0..4 {
        let blocker = g.create_card("Rock", 0);
        assert!(g.add_card(blocker, 2, 0, false));
    }
    let card = g.create_card("KaZar", 0);
    assert!(g.add_card(card, 1, 0, false));
    g.cards[card].revealed = false;
    g.cards[card].turn_played = g.turn;
    g.played[0].push((card, 1));
    g.reveal(card);
    assert_eq!(g.cards[card].lane, Some(0));
    assert_eq!(g.cards[card].permanent, 6);

    let card = play(&mut g, "MistyKnight", 0);
    assert_eq!(g.cards[card].permanent, 6); // 3 base × 2
    assert_eq!(g.cards[card].power(), 7); // Ka-Zar's Ongoing +1 follows the multiplier
}
#[test]
fn mosh_pit_and_stark_tower_respect_order_and_turn() {
    let mut rng = PythonRandom::seed(2);
    let mut g = game(&mut rng, "MoshPit");
    let cards: Vec<_> = (0..3).map(|_| play(&mut g, "MistyKnight", 0)).collect();
    (g.lanes[0].def.on_turn_end.unwrap())(&mut g, 0);
    assert_eq!(
        cards
            .iter()
            .map(|&id| g.cards[id].permanent)
            .collect::<Vec<_>>(),
        [4, 4, 3]
    );
    g.lanes[0].def = locations::get("StarkTower").unwrap();
    g.turn = 4;
    (g.lanes[0].def.on_turn_end.unwrap())(&mut g, 0);
    assert_eq!(g.cards[cards[0]].permanent, 4);
    g.turn = 5;
    (g.lanes[0].def.on_turn_end.unwrap())(&mut g, 0);
    assert_eq!(g.cards[cards[0]].permanent, 6);
}
#[test]
fn sakaar_uses_reveal_turn_and_does_not_trigger_reveal() {
    let mut rng = PythonRandom::seed(4);
    let mut g = game(&mut rng, "Sakaar");
    g.lanes[0].revealed_turn = Some(4);
    g.turn = 3;
    (g.lanes[0].def.on_turn_end.unwrap())(&mut g, 0);
    assert!(g.lanes[0].cards.is_empty());
    g.turn = 4;
    (g.lanes[0].def.on_turn_end.unwrap())(&mut g, 0);
    assert_eq!(g.lanes[0].cards.len(), 2);
    assert!(
        g.lanes[0]
            .cards
            .iter()
            .all(|&id| g.cards[id].revealed && g.cards[id].turn_played == 0)
    );
}
#[test]
fn ottos_lab_is_spent_per_player() {
    let mut rng = PythonRandom::seed(5);
    let mut g = game(&mut rng, "OttosLab");
    g.lanes[0].state = g.lanes[0].def.initial_state.map(|f| f());
    let n = g.players[1].hand.len();
    play(&mut g, "MistyKnight", 0);
    assert_eq!(g.players[1].hand.len(), n - 1);
    play(&mut g, "MistyKnight", 0);
    assert_eq!(g.players[1].hand.len(), n - 1);
}

#[test]
fn quantum_tunnel_replaces_played_card_without_reveal() {
    let mut rng = PythonRandom::seed(6);
    let mut g = game(&mut rng, "QuantumTunnel");
    g.players[0].deck.clear();
    let replacement = g.create_card("Medusa", 0);
    g.players[0].deck.push(replacement);
    let played = play(&mut g, "MistyKnight", 0);
    assert_eq!(g.lanes[0].cards, [replacement]);
    assert_eq!(g.cards[replacement].permanent, 2);
    assert_eq!(g.players[0].deck, [played]);
}

#[test]
fn citadel_repeats_ongoing_effects() {
    let mut rng = PythonRandom::seed(8);
    let mut g = game(&mut rng, "OnslaughtsCitadel");
    let ally = play(&mut g, "MistyKnight", 0);
    play(&mut g, "KaZar", 0);
    assert_eq!(g.cards[ally].power(), 5);
}

#[test]
fn washington_dc_only_buffs_cards_without_text() {
    let mut rng = PythonRandom::seed(9);
    let mut g = game(&mut rng, "WashingtonDC");
    let vanilla = play(&mut g, "MistyKnight", 0);
    let ability = play(&mut g, "Hawkeye", 0);
    assert_eq!(g.cards[vanilla].power(), 6);
    assert_eq!(g.cards[ability].power(), 1);
}
// StarkIndustriesBehavior::BuffCards (0x7d82ee8): eight independent picks with
// replacement over revealed cards only; the reference splits 8 / n evenly.
#[test]
fn stark_industries_makes_eight_independent_picks_over_revealed_cards() {
    let (mut saw_none, mut saw_many) = (false, false);
    for seed in 0..60 {
        let mut rng = PythonRandom::seed(seed);
        let mut g = game(&mut rng, "StarkIndustries");
        g.turn = 4;
        let cards: Vec<_> = (0..3).map(|_| play(&mut g, "MistyKnight", 0)).collect();
        let hidden = play(&mut g, "MistyKnight", 1);
        g.cards[hidden].revealed = false;
        let before: Vec<_> = cards.iter().map(|&id| g.cards[id].permanent).collect();
        let hidden_before = g.cards[hidden].permanent;
        (g.lanes[0].def.on_turn_end.unwrap())(&mut g, 0);
        let gains: Vec<_> = cards
            .iter()
            .zip(&before)
            .map(|(&id, &b)| g.cards[id].permanent - b)
            .collect();
        assert_eq!(gains.iter().sum::<i32>(), 8);
        assert_eq!(g.cards[hidden].permanent, hidden_before);
        saw_none |= gains.contains(&0);
        saw_many |= gains.iter().any(|&x| x >= 4);
    }
    assert!(saw_none && saw_many);
}
