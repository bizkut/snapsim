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

#[test]
fn stat_updates_and_reset() {
    let mut rng = PythonRandom::seed(3);
    let mut g = game(&mut rng);
    let a = g.create_card("MistyKnight", 0);
    let b = g.create_card("Shocker", 1);
    g.add_power(a, 4, 0);
    g.add_power(a, -2, 0);
    assert!(g.has_increased_power(a));
    // The client predicate also counts reductions from a card's own player.
    assert!(g.is_afflicted(a));
    g.steal_power(a, b, 1, 1);
    assert_eq!(g.cards[a].permanent, 4);
    assert_eq!(g.cards[b].permanent, 5);
    g.reset_power(a);
    assert_eq!(g.cards[a].permanent, 3);
    assert!(!g.is_afflicted(a));
    assert!(!g.has_increased_power(a));
    g.add_cost(b, -1, 1);
    g.set_cost(b, 4, 1);
    assert_eq!(g.cards[b].cost, 4);
}

#[test]
fn graveyard_tracks_last_location_and_resurrection() {
    let mut rng = PythonRandom::seed(4);
    let mut g = game(&mut rng);
    let id = play(&mut g, "MistyKnight", 0, 0);
    assert!(g.move_card(id, 2));
    assert_eq!(g.cards[id].last_lane, Some(0));
    assert!(g.destroy_card(id));
    assert_eq!(g.cards[id].last_lane, Some(2));
    assert_eq!(g.players[0].graveyard, vec![(id, 2)]);
    assert!(g.add_card(id, 0, 0, false));
    assert!(g.players[0].graveyard.is_empty());
}

#[test]
fn skill_remains_until_play_observers_have_run() {
    let mut rng = PythonRandom::seed(5);
    let mut g = game(&mut rng);
    let angela = play(&mut g, "Angela", 0, 0);
    let skill = play(&mut g, "AdamantiumInfusion", 0, 0);
    assert_eq!(g.cards[angela].permanent, 4);
    assert_eq!(g.cards[skill].lane, None);
    assert!(g.players[0].banished.contains(&skill));
}

#[test]
fn rows_follow_each_players_slots_and_shift_on_removal() {
    let mut rng = PythonRandom::seed(6);
    let mut g = game(&mut rng);
    let first = play(&mut g, "MistyKnight", 0, 0);
    let other = play(&mut g, "Shocker", 1, 0);
    let second = play(&mut g, "Wasp", 0, 0);
    let third = play(&mut g, "Cyclops", 0, 0);
    assert!(g.is_front_row(other));
    assert!(g.is_front_row(second));
    assert!(g.is_back_row(third));
    assert!(g.destroy_card(first));
    assert!(g.is_front_row(third));
}

#[test]
fn player_effect_survives_source_and_triggers_once() {
    use snapsim::game::{EffectLifetime, PendingEffect, PlayerTrigger};
    fn apply(g: &mut Game<'_>, effect: &mut PendingEffect, played: Option<usize>) -> bool {
        if let Some(card) = played {
            g.add_power(card, 2, g.cards[effect.source].owner);
        }
        true
    }
    let mut rng = PythonRandom::seed(7);
    let mut g = game(&mut rng);
    let source = play(&mut g, "MistyKnight", 0, 0);
    g.add_player_effect(
        0,
        PendingEffect {
            source,
            lane: Some(0),
            created_turn: g.turn,
            trigger: PlayerTrigger::BeforeCardRevealed,
            lifetime: EffectLifetime::NextCard,
            state: Box::new(()),
            apply,
        },
    );
    assert!(g.destroy_card(source));
    let first = play(&mut g, "Shocker", 0, 0);
    let second = play(&mut g, "Cyclops", 0, 0);
    assert_eq!(g.cards[first].permanent, 6);
    assert_eq!(g.cards[second].permanent, 5);
    assert!(g.players[0].pending_effects.is_empty());
}

#[test]
fn doubling_uses_individual_stat_changes_not_net_power() {
    let mut rng = PythonRandom::seed(8);
    let mut g = game(&mut rng);
    let source = play(&mut g, "MistyKnight", 0, 0);
    let enemy = play(&mut g, "Shocker", 1, 1);
    g.add_power(enemy, 4, 1);
    g.add_power(enemy, -2, 0);
    g.double_power_increases(enemy);
    assert_eq!(g.cards[enemy].power(), 10);
    g.double_afflictions(source);
    assert_eq!(g.cards[enemy].power(), 8);
    g.add_cost(enemy, -1, 0);
    assert_eq!(g.double_cost_reductions(enemy, 1), 0);
}
