use crate::game::{Action, MoveGrant};
use crate::random::PythonRandom;

#[test]
fn destination_grant_limits_direct_and_staged_moves() {
    let mut rng = PythonRandom::seed(49);
    let mut game = super::test_game(&mut rng);
    let source = game.create_card("Wasp", 0);
    let card = game.create_card("Hulk", 0);
    game.add_card(card, 0, 0, false);
    game.grant_move(MoveGrant {
        seat: 0,
        source,
        owner_def_id: None,
        card: Some(card),
        lane: None,
        destination: Some(1),
        start_turn: 1,
        end_turn: 1,
        remaining: 1,
        unlimited: false,
        condition: None,
        on_move: None,
    });
    game.turn = 1;
    assert!(!game.can_player_move(card, 2, 0));
    assert!(game.can_player_move(card, 1, 0));
    let mut fork_rng = game.fork_rng();
    let mut fork = game.fork(fork_rng.as_mut());
    assert!(fork.move_player_card(card, 1, 0));
    assert!(game.stage_action(0, Action::Move { card, lane: 1 }).is_ok());
    assert_eq!(game.players[0].move_grants[0].remaining, 0);
}

#[test]
fn unlimited_destination_grant_covers_new_cards_and_follows_location() {
    let mut rng = PythonRandom::seed(52);
    let mut game = super::test_game(&mut rng);
    let source = game.create_card("Wasp", 0);
    game.grant_move(MoveGrant {
        seat: 0,
        source,
        owner_def_id: None,
        card: None,
        lane: None,
        destination: Some(1),
        start_turn: 1,
        end_turn: 1,
        remaining: 0,
        unlimited: true,
        condition: None,
        on_move: None,
    });
    let first = game.create_card("Hulk", 0);
    let second = game.create_card("Wasp", 0);
    game.add_card(first, 0, 0, false);
    game.add_card(second, 0, 0, false);
    game.turn = 1;
    game.swap_location_objects(1, 2);
    assert!(!game.can_player_move(first, 1, 0));
    assert!(
        game.stage_action(
            0,
            Action::Move {
                card: first,
                lane: 2
            }
        )
        .is_ok()
    );
    assert!(
        game.stage_action(
            0,
            Action::Move {
                card: second,
                lane: 2
            }
        )
        .is_ok()
    );
}

#[test]
fn move_grant_condition_records_only_when_executed() {
    let mut rng = PythonRandom::seed(53);
    let mut game = super::test_game(&mut rng);
    let source = game.create_card("Wasp", 0);
    let card = game.create_card("Hulk", 0);
    game.add_card(card, 0, 0, false);
    game.replay_hooks = Some(std::cell::RefCell::new(Default::default()));
    game.grant_move(MoveGrant {
        seat: 0,
        source,
        owner_def_id: None,
        card: Some(card),
        lane: None,
        destination: None,
        start_turn: 2,
        end_turn: 2,
        remaining: 1,
        unlimited: false,
        condition: Some(|_, _, _| true),
        on_move: None,
    });
    game.turn = 1;
    assert!(!game.can_player_move(card, 1, 0));
    assert!(
        game.replay_hooks
            .as_ref()
            .unwrap()
            .borrow()
            .cards
            .is_empty()
    );
    game.turn = 2;
    assert!(game.can_player_move(card, 1, 0));
    assert!(
        game.replay_hooks
            .as_ref()
            .unwrap()
            .borrow()
            .cards
            .contains(&("Wasp", "move_grant_condition"))
    );
}

#[test]
fn move_capacity_counts_hidden_and_created_cards_and_tracks_space_throne() {
    let mut rng = PythonRandom::seed(55);
    let mut game = super::test_game(&mut rng);
    game.lanes[1].def = crate::locations::get("SpaceThrone").unwrap();
    game.lanes[1].revealed = true;
    let mover = game.create_card("Nightcrawler", 0);
    game.add_card(mover, 0, 0, false);
    let occupant = game.create_card("Wasp", 0);
    game.add_card(occupant, 1, 0, false);
    game.cards[occupant].revealed = false;
    game.cards[occupant].created = true;
    assert!(!game.can_move_card(mover, 1));
    game.silence_locations_through_next_turn(0);
    assert!(game.can_move_card(mover, 1));
    for _ in 0..3 {
        let card = game.create_card("Wasp", 0);
        game.add_card(card, 1, 0, false);
    }
    assert!(!game.can_move_card(mover, 1));
}

#[test]
fn jeff_ignore_move_restrictions_requires_hidden_ongoing() {
    let mut rng = PythonRandom::seed(56);
    let mut game = super::test_game(&mut rng);
    game.turn = 4;
    game.lanes[1].def = crate::locations::get("MiniaturizedLab").unwrap();
    game.lanes[1].revealed = true;
    let jeff = game.create_card("JeffTheBabyLandShark", 0);
    game.add_card(jeff, 0, 0, false);
    game.ongoing();
    assert!(game.can_move_card(jeff, 1));
    assert!(game.can_player_move(jeff, 1, 0));
    game.lanes[2].def = crate::locations::get("SpaceThrone").unwrap();
    game.lanes[2].revealed = true;
    let occupant = game.create_card("Wasp", 0);
    game.add_card(occupant, 2, 0, false);
    assert!(game.can_move_card(jeff, 2));
    assert!(game.can_player_move(jeff, 2, 0));
    let mut fork_rng = game.fork_rng();
    let mut fork = game.fork(fork_rng.as_mut());
    assert!(fork.move_player_card(jeff, 2, 0));
    for _ in 0..3 {
        let card = game.create_card("Wasp", 0);
        game.cards[card].lane = Some(2);
        game.push_location(2, card);
    }
    assert!(!game.can_move_card(jeff, 2));
    game.lanes[0].def = crate::locations::get("IslandPrison").unwrap();
    game.lanes[0].revealed = true;
    assert!(game.can_move_card(jeff, 1));
    game.lanes[0].def = crate::locations::get("DeepSpace").unwrap();
    game.lanes[0].revealed = true;
    game.ongoing();
    assert!(!game.can_move_card(jeff, 1));
    game.lanes[0].def = crate::locations::get("Ruins").unwrap();
    game.ongoing();
    assert!(game.can_move_card(jeff, 1));
    game.cards[jeff].move_staged = true;
    assert!(!game.can_move_card(jeff, 1));
    game.cards[jeff].move_staged = false;
    game.cards[jeff].abilities_removed = true;
    assert!(!game.can_move_card(jeff, 1));
}

#[test]
fn staged_move_cannot_be_staged_twice_and_undo_targets_origin() {
    use super::autoplay::Undo;
    let mut rng = PythonRandom::seed(57);
    let mut game = super::test_game(&mut rng);
    let card = game.create_card("Nightcrawler", 0);
    game.add_card(card, 0, 0, false);
    game.stage_action(0, Action::Move { card, lane: 1 })
        .unwrap();
    assert!(!game.can_player_move(card, 2, 0));
    assert_eq!(
        game.staged_undo_option(card, 0),
        Some(Undo::Move { lane: 0 })
    );
    game.is_wait_for_staging = true;
    for _ in 0..4 {
        let occupant = game.create_card("Wasp", 0);
        game.add_card(occupant, 0, 0, false);
    }
    assert!(game.can_move_card(card, 0));
    game.is_wait_for_staging = false;
    assert!(!game.can_move_card(card, 0));
    assert!(game.can_move_card(card, 2));
    let played = game.create_card("Wasp", 0);
    game.add_hand(0, played, false);
    game.stage_action(
        0,
        Action::Play {
            card: played,
            lane: 2,
        },
    )
    .unwrap();
    assert!(!game.cards[played].revealed);
    assert!(!game.can_player_move(played, 1, 0));
    assert!(game.can_move_card(played, 1));
    assert_eq!(game.staged_undo_option(card, 0), None);
}
