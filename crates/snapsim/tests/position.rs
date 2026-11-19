use serde_json::json;
use snapsim::{
    position::{Position, build_game, state_json},
    random::PythonRandom,
};

fn position(locations: serde_json::Value) -> Position {
    serde_json::from_value(json!({"turn": 3, "energy": 3, "hand": [], "deck": [],
        "locations": locations, "opponent": {"hand": 0, "deck": 0}, "priority": "opponent"}))
    .unwrap()
}

#[test]
fn pending_watches_obey_historic_reveal_conditions() {
    let p = position(json!([
        {"id":"KamarTaj", "mine":[{"id":"Hawkeye", "played_turn":2}], "theirs":[], "revealed_turn":1},
        {"id":"Xandar", "mine":[{"id":"JessicaJones", "played_turn":2}], "theirs":[{"id":"Cosmo", "played_turn":1}]},
        {"id":null, "mine":[], "theirs":[]}
    ]));
    let mut rng = PythonRandom::seed(1);
    let game = build_game(&p, &mut rng).unwrap();
    let hawkeye = game.lanes[0].cards[0];
    let jessica = game.lanes[1].cards[0];
    assert_eq!(game.cards[hawkeye].watches, vec![(3, 0), (3, 0)]);
    assert!(game.cards[jessica].watches.is_empty());
    assert_eq!(game.reveal_priority, Some(1));
}

#[test]
fn restores_lemuria_face_down_and_lab_flags() {
    let p = position(json!([
        {"id":"Lemuria", "mine":[], "theirs":[], "theirs_face_down":2, "revealed_turn":2},
        {"id":"OttosLab", "mine":[], "theirs":[], "ottos_lab_spent":[true,false]},
        {"id":"unknown-location", "mine":[], "theirs":[]}
    ]));
    let mut rng = PythonRandom::seed(2);
    let game = build_game(&p, &mut rng).unwrap();
    assert_eq!(game.delayed_reveals[1].len(), 2);
    assert!(
        game.delayed_reveals[1]
            .iter()
            .all(|&c| game.cards[c].turn_played == 2)
    );
    assert_eq!(
        game.lanes[1]
            .state
            .as_ref()
            .unwrap()
            .downcast_ref::<[bool; 2]>(),
        Some(&[true, false])
    );
    assert_eq!(state_json(&game)["locations"][2]["id"], "unknown-location");
}

#[test]
fn hidden_deck_count_and_observed_power_are_restored() {
    let mut p = position(json!([
        {"id":"Xandar", "mine":[{"id":"Hawkeye", "power":12, "played_turn":1}], "theirs":[]},
        {"id":null, "mine":[], "theirs":[]},
        {"id":null, "mine":[], "theirs":[]}
    ]));
    p.deck = vec!["Hulk".into(), "Cyclops".into()];
    p.deck_count = Some(1);
    let mut rng = PythonRandom::seed(3);
    let game = build_game(&p, &mut rng).unwrap();
    assert_eq!(game.players[0].deck.len(), 1);
    assert_eq!(game.cards[game.lanes[0].cards[0]].power(), 12);
}

#[test]
fn current_lemuria_and_unsupported_copies() {
    let p = position(json!([
        {"id":"Lemuria", "mine":[{"id":"AbsorbingMan"}], "theirs":[], "revealed_turn":3},
        {"id":"Xandar", "mine":[], "theirs":[]},
        {"id":"OttosLab", "mine":[], "theirs":[]}
    ]));
    let mut rng = PythonRandom::seed(4);
    let mut game = build_game(&p, &mut rng).unwrap();
    assert!(game.delays_reveals);
    let original = game.lanes[0].cards[0];
    let copy = game.copy_card(original, 0);
    assert_eq!(game.cards[copy].def.id, "AbsorbingMan");
}

#[test]
fn empty_recruit_draw_does_not_advance_rng() {
    let mut p = position(json!([
        {"id":"Xandar", "mine":[], "theirs":[]},
        {"id":"OttosLab", "mine":[], "theirs":[]},
        {"id":"KamarTaj", "mine":[], "theirs":[]}
    ]));
    p.mode = Some("recruit".into());
    let mut rng = PythonRandom::seed(5);
    let before = rng.getstate();
    let game = build_game(&p, &mut rng).unwrap();
    drop(game);
    assert_eq!(rng.getstate(), before);
}

#[test]
fn next_reveal_uses_first_hidden_lane() {
    let p = position(json!([
        {"id":null, "mine":[], "theirs":[]},
        {"id":"Xandar", "mine":[], "theirs":[]},
        {"id":"OttosLab", "mine":[], "theirs":[]}
    ]));
    let mut rng = PythonRandom::seed(6);
    let mut game = build_game(&p, &mut rng).unwrap();
    game.turn = 4;
    game.start_turn();
    assert!(game.lanes[0].revealed);
    assert_eq!(game.lanes[0].revealed_turn, Some(4));
}
