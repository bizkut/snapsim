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

#[test]
fn copied_text_keeps_identity_and_can_resolve() {
    let mut rng = PythonRandom::seed(9);
    let mut g = game(&mut rng);
    let target = play(&mut g, "MistyKnight", 0, 1);
    let source = g.create_card("Medusa", 0);
    assert!(g.copy_text(target, source, true));
    assert_eq!(g.cards[target].def.id, "MistyKnight");
    assert_eq!(g.cards[target].ability_def().id, "Medusa");
    assert_eq!(g.cards[target].permanent, 6);
    assert!(!g.copy_text(target, source, true));
}

#[test]
fn morph_retains_or_discards_permanent_modifiers() {
    let mut rng = PythonRandom::seed(10);
    let mut g = game(&mut rng);
    let card = g.create_card("MistyKnight", 0);
    g.add_power(card, 2, 0);
    g.set_power(card, 9, 0);
    g.set_cost(card, 9, 0);
    assert!(g.transform_card(card, "Hulk"));
    assert_eq!(g.cards[card].def.id, "Hulk");
    assert_eq!(g.cards[card].morphed_from, Some("MistyKnight"));
    assert_eq!(g.cards[card].permanent, 16);
    assert_eq!(g.cards[card].cost, 6);
    assert!(g.transform_card_without_modifiers(card, "Shocker"));
    assert_eq!(g.cards[card].permanent, 4);
}

#[test]
fn snap_and_extended_quickdraw_are_turn_scoped() {
    let mut rng = PythonRandom::seed(11);
    let mut g = game(&mut rng);
    let card = g.create_card("MistyKnight", 0);
    g.turn = 3;
    g.cards[card].drawn_turn = Some(2);
    assert!(!g.is_quickdraw_active(card));
    g.cards[card].draw_extends_quickdraw = true;
    assert!(g.is_quickdraw_active(card));
    g.players[1].snapped_turns.push(3);
    assert!(g.snapped_this_turn());
    g.turn = 4;
    assert!(!g.snapped_this_turn());
    assert!(!g.is_quickdraw_active(card));
}

#[test]
fn literal_deck_top_differs_from_eligible_draw() {
    let mut rng = PythonRandom::seed(12);
    let mut g = game(&mut rng);
    let domino = g.create_card("Domino", 0);
    let other = g.create_card("MistyKnight", 0);
    g.players[0].deck = vec![domino, other];
    g.turn = 1;
    assert_eq!(g.top_card(0), Some(domino));
    g.draw(0);
    assert!(g.players[0].hand.contains(&other));
    assert_eq!(g.top_card(0), Some(domino));
}

#[test]
fn fork_clones_dynamic_state_and_independent_rng() {
    let mut rng = PythonRandom::seed(13);
    let mut game = game(&mut rng);
    let card = game.create_card("MistyKnight", 0);
    game.cards[card].state = Some(Box::new(vec![1u8, 2]));
    game.lanes[0].state = Some(Box::new([true, false]));
    let mut branch_rng = game.fork_rng();
    let mut fork = game.fork(branch_rng.as_mut());
    fork.cards[card]
        .state
        .as_deref_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Vec<u8>>()
        .unwrap()
        .push(3);
    fork.lanes[0]
        .state
        .as_deref_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<[bool; 2]>()
        .unwrap()[0] = false;
    assert_eq!(
        game.cards[card]
            .state
            .as_deref()
            .unwrap()
            .as_any()
            .downcast_ref::<Vec<u8>>()
            .unwrap(),
        &[1, 2]
    );
    assert_eq!(
        game.lanes[0]
            .state
            .as_deref()
            .unwrap()
            .as_any()
            .downcast_ref::<[bool; 2]>()
            .unwrap(),
        &[true, false]
    );
    let branch_roll = fork.rng.borrow_mut().randbelow(1000);
    let original_roll = game.rng.borrow_mut().randbelow(1000);
    assert_eq!(branch_roll, original_roll);
}

#[test]
fn staged_activation_precedes_play_and_is_once_per_game() {
    use snapsim::{cards::CardDef, game::Action};
    fn activate(game: &mut Game<'_>, card: usize) {
        let seat = game.cards[card].owner;
        game.players[seat].next_play_power += 2;
    }
    static ACTIVATOR: CardDef = CardDef {
        on_activate: Some(activate),
        activate_cost: 1,
        ..CardDef::vanilla("MistyKnight")
    };
    let mut rng = PythonRandom::seed(14);
    let mut game = game(&mut rng);
    let actor = play(&mut game, "MistyKnight", 0, 0);
    game.cards[actor].def = &ACTIVATOR;
    let next = game.create_card("Wasp", 0);
    assert!(game.add_hand(0, next, false));
    game.players[0].energy = 2;
    game.stage_action(0, Action::Activate { card: actor })
        .unwrap();
    assert!(!game.can_activate(actor, 0));
    game.stage_action(
        0,
        Action::Play {
            card: next,
            lane: 0,
        },
    )
    .unwrap();
    assert_eq!(game.cards[next].permanent, 1);
    game.resolve_staged_actions(0);
    assert_eq!(game.cards[next].permanent, 3);
    assert!(game.cards[actor].activated);
    assert!(!game.can_activate(actor, 0));
}

#[test]
fn staged_move_updates_slots_but_fires_hooks_only_on_resolution() {
    use snapsim::game::Action;
    let mut rng = PythonRandom::seed(15);
    let mut game = game(&mut rng);
    let kraven = play(&mut game, "Kraven", 0, 1);
    let crawler = play(&mut game, "Nightcrawler", 0, 0);
    assert!(game.can_player_move(crawler, 1, 0));
    game.stage_action(
        0,
        Action::Move {
            card: crawler,
            lane: 1,
        },
    )
    .unwrap();
    assert_eq!(game.cards[crawler].lane, Some(1));
    assert_eq!(game.cards[kraven].permanent, 3);
    assert!(!game.can_player_move(crawler, 2, 0));
    game.resolve_staged_actions(0);
    assert_eq!(game.cards[kraven].permanent, 5);
    assert!(!game.can_player_move(crawler, 2, 0));
}

#[test]
fn lane_move_grant_is_turn_scoped_and_shared() {
    use snapsim::game::{Action, MoveGrant};
    fn buff(game: &mut Game<'_>, source: usize, moved: usize) {
        game.add_power(moved, 1, game.cards[source].owner);
    }
    let mut rng = PythonRandom::seed(16);
    let mut game = game(&mut rng);
    let source = play(&mut game, "MistyKnight", 0, 0);
    let ally = play(&mut game, "Wasp", 0, 0);
    game.grant_move(MoveGrant {
        seat: 0,
        source,
        card: None,
        lane: Some(0),
        start_turn: 2,
        end_turn: 2,
        remaining: 1,
        on_move: Some(buff),
    });
    assert!(!game.can_player_move(ally, 1, 0));
    game.turn = 2;
    assert!(game.can_player_move(ally, 1, 0));
    game.stage_action(
        0,
        Action::Move {
            card: ally,
            lane: 1,
        },
    )
    .unwrap();
    assert!(!game.can_player_move(source, 2, 0));
    assert_eq!(game.cards[ally].permanent, 1);
    game.resolve_staged_actions(0);
    assert_eq!(game.cards[ally].permanent, 2);
}
