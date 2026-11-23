//! Stryfe must execute the chosen hand ability with Stryfe as its source.
use snapsim::{
    cards,
    game::{Action, Event, Game, PlayerTrigger},
    locations,
    random::PythonRandom,
};

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
    game.turn = 2;
    game
}

fn borrow(game: &mut Game<'_>, name: &str) -> (usize, usize) {
    let stryfe = game.create_card("Stryfe", 1);
    game.cards[stryfe].lane = Some(1);
    game.cards[stryfe].revealed = true;
    game.lanes[1].cards.push(stryfe);
    let target = game.create_card(name, 1);
    game.players[1].hand = vec![target];
    game.players[1].on_reveal_cards_played = 3;
    let def = cards::get("Stryfe").unwrap();
    def.objective_update.unwrap()(game, stryfe);
    def.objective_freeze.unwrap()(game, stryfe);
    def.on_event.unwrap()(game, stryfe, Event::AfterTurnEnd);
    (stryfe, target)
}

fn stage_play(game: &mut Game<'_>, name: &str, lane: usize) -> usize {
    let card = game.create_card(name, 1);
    game.players[1].hand.push(card);
    game.players[1].energy = 10;
    game.stage_action(1, Action::Play { card, lane }).unwrap();
    game.resolve_staged_actions(1);
    card
}

#[test]
fn shuri_registers_on_stryfes_seat_and_lane() {
    let mut rng = PythonRandom::seed(1);
    let mut game = game(&mut rng);
    let (source, _) = borrow(&mut game, "Shuri");
    assert!(game.players[0].pending_effects.is_empty());
    let effect = &game.players[1].pending_effects[0];
    assert_eq!(effect.source, source);
    assert_eq!(effect.lane, Some(1));
    assert_eq!(effect.trigger, PlayerTrigger::BeforeCardRevealed);
    assert!(game.destroy_card(source));
    let target = stage_play(&mut game, "Wasp", 1);
    assert_eq!(game.cards[target].power(), 2);
    assert!(game.players[1].pending_effects.is_empty());
}

#[test]
fn stick_buffs_with_stryfes_power() {
    let mut rng = PythonRandom::seed(7);
    let mut game = game(&mut rng);
    let (source, _) = borrow(&mut game, "Stick");
    assert!(game.players[0].pending_effects.is_empty());
    let effect = &game.players[1].pending_effects[0];
    assert_eq!(effect.source, source);
    assert_eq!(effect.lane, Some(1));
    assert_eq!(effect.trigger, PlayerTrigger::BeforeCardRevealed);
    let source_power = game.cards[source].power();
    let target = stage_play(&mut game, "Wasp", 0);
    assert_eq!(game.cards[target].power(), 1 + source_power);
    assert!(game.players[1].pending_effects.is_empty());
}

#[test]
fn surge_buffs_stryfes_owner_top_deck_card() {
    let mut rng = PythonRandom::seed(8);
    let mut game = game(&mut rng);
    let target = game.create_card("Hulk", 1);
    let next = game.create_card("Hulk", 1);
    game.players[1].deck = vec![target, next];
    let next_power = game.cards[next].power();
    let next_cost = game.cards[next].cost;
    let cost = game.cards[target].cost;
    let power = game.cards[target].power();
    borrow(&mut game, "Surge");
    assert_eq!(game.cards[target].cost, cost - 1);
    assert_eq!(game.cards[target].power(), power + 1);
    game.players[1].deck.remove(0);
    game.players[1].hand.push(target);
    game.players[1].energy = 10;
    game.stage_action(
        1,
        Action::Play {
            card: target,
            lane: 0,
        },
    )
    .unwrap();
    game.resolve_staged_actions(1);
    assert_eq!(game.cards[next].cost, next_cost);
    assert_eq!(game.cards[next].power(), next_power);
}

#[test]
fn grapple_arrow_registers_on_stryfes_seat_and_lane() {
    let mut rng = PythonRandom::seed(2);
    let mut game = game(&mut rng);
    let (source, _) = borrow(&mut game, "GrappleArrow");
    assert!(game.players[0].pending_effects.is_empty());
    let effect = &game.players[1].pending_effects[0];
    assert_eq!(effect.source, source);
    assert_eq!(effect.lane, Some(1));
    assert_eq!(effect.trigger, PlayerTrigger::CardResolved);
    assert!(game.destroy_card(source));
    let target = stage_play(&mut game, "Wasp", 0);
    assert_eq!(game.cards[target].lane, Some(1));
    assert!(game.players[1].pending_effects.is_empty());
}

#[test]
fn nico_spells_register_independent_player_effects() {
    for (spell, name) in [
        "Spell01NicoMinoru",
        "Spell02NicoMinoru",
        "Spell03NicoMinoru",
        "Spell04NicoMinoru",
        "Spell05NicoMinoru",
        "Spell06NicoMinoru",
        "Spell07NicoMinoru",
    ]
    .into_iter()
    .enumerate()
    {
        let mut rng = PythonRandom::seed(3);
        let mut game = game(&mut rng);
        let (source, _) = borrow(&mut game, name);
        assert!(game.players[0].pending_effects.is_empty(), "{name}");
        let effect = &game.players[1].pending_effects[0];
        assert_eq!(effect.source, source, "{name}");
        assert_eq!(effect.lane, Some(1), "{name}");
        assert_eq!(effect.trigger, PlayerTrigger::CardPlayed, "{name}");
        assert_eq!(
            effect.state.as_ref().as_any().downcast_ref::<(u8, usize)>(),
            Some(&((spell + 1) as u8, 1)),
            "{name}"
        );
        let before_power = game.cards[source].power();
        let before_hand = game.players[1].hand.len();
        let before_deck = game.players[1].deck.len();
        let target = stage_play(&mut game, "Wasp", 0);
        match spell {
            0 => assert_eq!(game.cards[target].def.id, "Demon"),
            1 => {
                assert!(game.cards[target].lane.is_none());
                assert_eq!(game.players[1].deck.len(), before_deck.saturating_sub(2));
            }
            2 => assert_eq!(game.cards[target].lane, Some(1)),
            3 => assert_eq!(game.cards[target].power(), 3),
            4 => assert_ne!(game.lanes[0].def.id, "Ruins"),
            5 => assert_eq!(game.players[1].hand.len(), before_hand + 1),
            6 => assert_eq!(game.cards[source].power(), before_power * 2),
            _ => unreachable!(),
        }
        assert!(game.players[1].pending_effects.is_empty(), "{name}");
    }
}

#[test]
fn iron_patriot_registers_delayed_effect_for_stryfe() {
    let mut rng = PythonRandom::seed(4);
    let mut game = game(&mut rng);
    let (source, _) = borrow(&mut game, "IronPatriot");
    assert!(game.players[0].pending_effects.is_empty());
    let effect = &game.players[1].pending_effects[0];
    assert_eq!(effect.source, source);
    assert_eq!(effect.lane, Some(1));
    assert_eq!(effect.trigger, PlayerTrigger::EndTurn);
    let target = *effect
        .state
        .as_ref()
        .as_any()
        .downcast_ref::<usize>()
        .unwrap();
    let cost = game.cards[target].cost;
    game.end_turn_effects();
    assert_eq!(game.cards[target].cost, cost);
    game.turn = 3;
    game.end_turn_effects();
    assert_eq!(game.cards[target].cost, cost - 3);
    assert!(game.players[1].pending_effects.is_empty());
}

#[test]
fn borrowed_movement_grants_capture_stryfes_location_and_seat() {
    for name in ["Cloak", "Hellion", "WebSling"] {
        let mut rng = PythonRandom::seed(5);
        let mut game = game(&mut rng);
        let (source, _) = borrow(&mut game, name);
        let grant = &game.players[1].move_grants[0];
        assert_eq!(grant.source, source, "{name}");
        assert_eq!(grant.seat, 1, "{name}");
        assert_eq!(grant.start_turn, 3, "{name}");
        assert_eq!(grant.end_turn, 3, "{name}");
        if name == "WebSling" {
            assert_eq!(grant.lane, Some(1));
            assert_eq!(grant.destination, None);
        } else {
            assert_eq!(grant.destination, Some(1));
        }
        assert_eq!(
            !game.players[0].move_grants.is_empty(),
            name == "Cloak",
            "{name}"
        );
        let ally = game.create_card("Wasp", 1);
        let lane = if name == "WebSling" { 1 } else { 0 };
        game.cards[ally].lane = Some(lane);
        game.cards[ally].revealed = true;
        game.lanes[lane].cards.push(ally);
        let destination = if name == "WebSling" { 0 } else { 1 };
        assert!(!game.can_player_move(ally, destination, 1), "{name}");
        game.turn = 3;
        assert!(game.can_player_move(ally, destination, 1), "{name}");
        game.stage_action(
            1,
            Action::Move {
                card: ally,
                lane: destination,
            },
        )
        .unwrap();
        game.resolve_staged_actions(1);
        assert_eq!(game.cards[ally].lane, Some(destination), "{name}");
        if name == "WebSling" {
            assert_eq!(game.cards[ally].power(), 2);
        }
        game.turn = 4;
        assert!(!game.can_player_move(ally, lane, 1), "{name}");
    }
}

#[test]
fn borrowed_cloak_uses_cloaks_priority_not_stryfes() {
    let mut rng = PythonRandom::seed(5);
    let mut game = game(&mut rng);
    let (source, _) = borrow(&mut game, "Cloak");
    assert_eq!(game.players[1].move_grants[0].source, source);
    assert_eq!(game.players[1].move_grants[0].owner_def_id, Some("Cloak"));
    let crawler = game.create_card("Nightcrawler", 1);
    game.cards[crawler].lane = Some(0);
    game.cards[crawler].revealed = true;
    game.lanes[0].cards.push(crawler);
    game.turn = 3;
    game.stage_action(
        1,
        Action::Move {
            card: crawler,
            lane: 1,
        },
    )
    .unwrap();
    assert_eq!(game.last_move_source, Some("Cloak"));
    assert_eq!(game.cards[crawler].player_moves_used, 0);
    assert!(
        game.stage_action(
            1,
            Action::Move {
                card: crawler,
                lane: 2
            }
        )
        .is_err()
    );
    assert_eq!(game.last_move_source, None);
}

#[test]
fn mysterio_adds_illusion_for_stryfes_owner_elsewhere() {
    let mut rng = PythonRandom::seed(6);
    let mut game = game(&mut rng);
    borrow(&mut game, "Mysterio");
    let illusions: Vec<_> = game
        .lanes
        .iter()
        .enumerate()
        .flat_map(|(lane, data)| data.cards.iter().copied().map(move |card| (lane, card)))
        .filter(|&(_, card)| game.cards[card].def.id == "MysterioIllusion")
        .collect();
    assert_eq!(illusions.len(), 1);
    assert_ne!(illusions[0].0, 1);
    assert_eq!(game.cards[illusions[0].1].owner, 1);
}

#[test]
fn borrowed_watches_attach_to_stryfe_at_his_lane() {
    use snapsim::game::AttachedAbilityKind;
    for (name, expected_power, expected_play) in [
        ("Rescue", 5, true),
        ("Hawkeye", 4, true),
        ("JessicaJones", 5, false),
        ("BasicArrow", 4, true),
    ] {
        let mut rng = PythonRandom::seed(9);
        let mut game = game(&mut rng);
        let (source, selected) = borrow(&mut game, name);
        let watch = game.cards[source]
            .attached_abilities
            .iter()
            .find(|ability| matches!(ability.kind, AttachedAbilityKind::Watch { .. }))
            .expect(name);
        assert_eq!(watch.origin, Some(source), "{name}");
        assert!(
            matches!(watch.kind, AttachedAbilityKind::Watch {
            turn: 3, lane: 1, expect, power, ..
        } if expect == expected_play && power == expected_power),
            "{name}"
        );
        assert!(game.cards[selected].attached_abilities.is_empty(), "{name}");
        let before = game.cards[source].power();
        game.turn = 3;
        if expected_play {
            let played = game.create_card("Wasp", 1);
            game.cards[played].turn_played = 3;
            game.played[1].push((played, 1));
        }
        game.resolve_attached_watches(source);
        assert_eq!(
            game.cards[source].power(),
            before + expected_power,
            "{name}"
        );
        assert!(game.cards[source].watch_contexts().is_empty(), "{name}");
    }
}

#[test]
fn zero_registers_for_stryfes_seat() {
    let mut rng = PythonRandom::seed(10);
    let mut game = game(&mut rng);
    let (source, _) = borrow(&mut game, "Zero");
    assert!(game.players[0].pending_effects.is_empty());
    assert_eq!(game.players[1].pending_effects[0].source, source);
    assert!(game.destroy_card(source));
    let target = stage_play(&mut game, "Hawkeye", 0);
    assert!(game.cards[target].abilities_removed);
    assert!(game.players[1].pending_effects.is_empty());
}

#[test]
fn wave_registers_for_both_players() {
    use snapsim::game::PlayerCostEffectKind;
    let mut rng = PythonRandom::seed(11);
    let mut game = game(&mut rng);
    borrow(&mut game, "Wave");
    for seat in 0..2 {
        assert!(
            game.players[seat]
                .cost_effects
                .iter()
                .any(|effect| matches!(effect.kind, PlayerCostEffectKind::Wave { until: 3 }))
        );
    }
    let cards = [game.create_card("Hulk", 0), game.create_card("Hulk", 1)];
    for (seat, &card) in cards.iter().enumerate() {
        game.players[seat].hand.push(card);
    }
    game.ongoing();
    for &card in &cards {
        assert_eq!(game.card_cost(card), 4);
    }
    game.turn = 3;
    game.ongoing();
    for &card in &cards {
        assert_eq!(game.card_cost(card), 4);
    }
    game.turn = 4;
    game.ongoing();
    for &card in &cards {
        assert_eq!(game.card_cost(card), 6);
    }
}

#[test]
fn baron_mordo_targets_stryfes_opponents_top_card() {
    use snapsim::game::PlayerCostEffectKind;
    let mut rng = PythonRandom::seed(12);
    let mut game = game(&mut rng);
    let top = game.create_card("Wasp", 0);
    game.players[0].deck = vec![top];
    borrow(&mut game, "BaronMordo");
    assert!(game.players[0].cost_effects.iter().any(|effect|
        matches!(effect.kind, PlayerCostEffectKind::BaronMordo { card, until: 6, value: 6 } if card == top)));
    assert!(game.players[1].cost_effects.is_empty());
    game.players[0].hand.push(top);
    game.ongoing();
    assert_eq!(game.card_cost(top), 6);
    game.turn = 7;
    game.ongoing();
    assert_eq!(game.card_cost(top), 0);
}

#[test]
fn space_shot_buffs_stryfe_and_grants_move_to_owned_thanos() {
    let mut rng = PythonRandom::seed(13);
    let mut game = game(&mut rng);
    let thanos = game.create_card("ThanosFracturedFrontier", 1);
    game.cards[thanos].lane = Some(0);
    game.cards[thanos].revealed = true;
    game.lanes[0].cards.push(thanos);
    let (source, _) = borrow(&mut game, "SpaceShot");
    assert_eq!(game.cards[source].power(), 7);
    assert!(
        game.players[1]
            .move_grants
            .iter()
            .any(|grant| grant.source == source
                && grant.card == Some(thanos)
                && grant.start_turn == 3)
    );
    assert!(game.players[0].move_grants.is_empty());
    assert!(!game.can_player_move(thanos, 2, 1));
    game.turn = 3;
    assert!(game.can_player_move(thanos, 2, 1));
    game.stage_action(
        1,
        Action::Move {
            card: thanos,
            lane: 2,
        },
    )
    .unwrap();
    game.resolve_staged_actions(1);
    assert_eq!(game.cards[thanos].lane, Some(2));
    assert!(!game.can_player_move(thanos, 0, 1));
}

#[test]
fn loki_replaces_stryfes_deck_with_opponent_starting_deck() {
    let mut rng = PythonRandom::seed(14);
    let mut game = game(&mut rng);
    game.players[0].starting_deck = Some(vec!["Hulk"]);
    let opponent_deck = game.players[0].deck.clone();
    borrow(&mut game, "Loki");
    assert_eq!(game.players[0].deck, opponent_deck);
    assert_eq!(game.players[1].deck.len(), 1);
    let replacement = game.players[1].deck[0];
    assert_eq!(game.cards[replacement].def.id, "Hulk");
    assert_eq!(game.cards[replacement].owner, 1);
    assert_eq!(game.cards[replacement].cost, 5);
}
