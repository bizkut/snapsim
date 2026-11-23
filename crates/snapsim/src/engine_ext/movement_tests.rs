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
