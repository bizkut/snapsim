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
    let locations: Vec<String> = ["Ruins", "Xandar", "KamarTaj"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut g = Game::new([&deck, &deck], &locations, rng).unwrap();
    for lane in &mut g.lanes {
        lane.def = locations::get("Ruins").unwrap();
        lane.revealed = true;
    }
    g
}
fn play(g: &mut Game, name: &str, owner: usize, lane: usize) -> usize {
    let id = g.create_card(name, owner);
    g.cards[id].lane = Some(lane);
    g.lanes[lane].cards.push(id);
    g.played[owner].push((id, lane));
    g.reveal(id);
    id
}
#[test]
fn squirrel_tokens_respect_space_and_nova_buffs_revealed_allies() {
    let mut rng = PythonRandom::seed(1);
    let mut g = game(&mut rng);
    for _ in 0..4 {
        play(&mut g, "Wasp", 0, 2);
    }
    play(&mut g, "SquirrelGirl", 0, 0);
    assert_eq!(g.lanes[1].cards.len(), 1);
    assert_eq!(g.cards[g.lanes[1].cards[0]].def.id, "Squirrel");
    assert_eq!(g.lanes[2].cards.len(), 4);
    let nova = play(&mut g, "Nova", 0, 0);
    let ally = g.lanes[1].cards[0];
    let before = g.cards[ally].permanent;
    assert!(g.destroy_card(nova));
    assert_eq!(g.cards[ally].permanent, before + 1);
}
#[test]
fn yondu_removes_only_lowest_cost_and_prediction_counts_staged_play() {
    let mut rng = PythonRandom::seed(9);
    let mut g = game(&mut rng);
    let lowest = g.players[1]
        .deck
        .iter()
        .map(|&c| g.cards[c].cost)
        .min()
        .unwrap();
    let before = g.players[1].deck.clone();
    play(&mut g, "Yondu", 0, 0);
    let removed = before
        .iter()
        .find(|c| !g.players[1].deck.contains(c))
        .unwrap();
    assert_eq!(g.cards[*removed].cost, lowest);
    let staged = g.create_card("Wasp", 1);
    g.cards[staged].turn_played = g.turn;
    g.played[1].push((staged, 1));
    let rocket = play(&mut g, "RocketRaccoon", 0, 1);
    assert_eq!(g.cards[rocket].permanent, 5);
}

#[test]
fn copies_clone_and_tokens_keep_correct_power_and_limits() {
    let mut rng = PythonRandom::seed(3);
    let mut g = game(&mut rng);
    g.players[0].hand.clear();
    let first = g.create_card("Hulk", 0);
    g.cards[first].permanent += 3;
    g.add_hand(0, first, true);
    let second = g.create_card("Wasp", 0);
    g.add_hand(0, second, true);
    play(&mut g, "MoonGirl", 0, 0);
    assert_eq!(g.players[0].hand.len(), 4);
    assert_eq!(
        g.cards[g.players[0].hand[2]].permanent,
        g.cards[first].permanent
    );
    g.players[1].hand.clear();
    let enemy = g.create_card("Hulk", 1);
    g.cards[enemy].permanent += 4;
    g.add_hand(1, enemy, true);
    play(&mut g, "WhiteQueen", 0, 0);
    let copied = *g.players[0].hand.last().unwrap();
    assert_eq!(g.cards[copied].def.id, "Hulk");
    assert_eq!(g.cards[copied].permanent, g.cards[enemy].permanent);
    let sinister = g.create_card("MrSinister", 0);
    g.cards[sinister].permanent += 4;
    g.cards[sinister].lane = Some(1);
    g.lanes[1].cards.push(sinister);
    g.reveal(sinister);
    let clone = *g.lanes[1].cards.last().unwrap();
    assert_eq!(g.cards[clone].def.id, "SinisterClone");
    assert_eq!(g.cards[clone].permanent, g.cards[sinister].permanent);
    play(&mut g, "WhiteTiger", 0, 2);
    assert!(
        g.lanes
            .iter()
            .flat_map(|l| &l.cards)
            .any(|&c| g.cards[c].def.id == "TigerSpirit")
    );
}

#[test]
fn wolverine_regenerates_and_professor_locks_nonmove_actions() {
    let mut rng = PythonRandom::seed(8);
    let mut g = game(&mut rng);
    let wolverine = g.create_card("Wolverine", 0);
    g.players[0].hand.push(wolverine);
    let power = g.cards[wolverine].permanent;
    g.discard_card(0, wolverine);
    assert!(g.cards[wolverine].lane.is_some());
    assert_eq!(g.cards[wolverine].permanent, power + 2);
    assert!(g.destroy_card(wolverine));
    assert!(g.cards[wolverine].lane.is_some());
    assert_eq!(g.cards[wolverine].permanent, power + 4);
    play(&mut g, "ProfessorX", 0, 1);
    let card = g.create_card("Wasp", 1);
    assert!(!g.can_add(card, 1, 1));
    let professor = g.lanes[1]
        .cards
        .iter()
        .copied()
        .find(|&c| g.cards[c].def.id == "ProfessorX")
        .unwrap();
    assert!(!g.destroy_card(professor));
}
