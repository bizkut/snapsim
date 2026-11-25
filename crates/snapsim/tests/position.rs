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
fn known_opponent_play_is_staged_and_not_sampled_in_hand() {
    let mut p = position(json!([
        {"id":"Ruins", "mine":[], "theirs":[], "known_opponent_plays":["Hulk"]},
        {"id":"Ruins", "mine":[], "theirs":[]},
        {"id":"Ruins", "mine":[], "theirs":[]}
    ]));
    p.opponent.hand = 1;
    p.opponent.pool = Some(vec!["Hulk".into(), "AntMan".into()]);
    let mut rng = PythonRandom::seed(1);
    let mut game = build_game(&p, &mut rng).unwrap();
    assert_eq!(game.players[1].hand.len(), 1);
    assert_eq!(game.cards[game.players[1].hand[0]].def.id, "AntMan");
    let staged = game.lanes[0].cards[0];
    assert_eq!(game.cards[staged].def.id, "Hulk");
    assert!(!game.cards[staged].revealed);
    assert_eq!(game.played[1], vec![(staged, 0)]);
    game.resolve_position_turn(&[], snapsim::policy::Policy::Spread, Some(1));
    assert!(game.cards[staged].revealed);
    assert_eq!(game.players[1].hand.len(), 1);
}

#[test]
fn post_galactus_position_builds_destroyed_lanes() {
    let p = position(json!([
        {"id":null, "destroyed":true, "mine":[], "theirs":[]},
        {"id":"Ruins", "mine":[{"id":"Galactus"}], "theirs":[]},
        {"id":null, "destroyed":true, "mine":[], "theirs":[]}
    ]));
    let mut rng = PythonRandom::seed(1);
    let game = build_game(&p, &mut rng).unwrap();
    assert!(game.lanes[0].destroyed);
    assert!(!game.lanes[1].destroyed);
    assert!(game.lanes[2].destroyed);
    assert!(!game.lanes[0].revealed);
    assert!(game.lanes[0].cards.is_empty());
}

#[test]
fn opponent_known_is_forced_hidden_without_resampling_seen() {
    let mut p = position(json!([
        {"id":"Xandar", "mine":[], "theirs":[]},
        {"id":"OttosLab", "mine":[], "theirs":[]},
        {"id":"KamarTaj", "mine":[], "theirs":[]}
    ]));
    p.opponent.hand = 1;
    p.opponent.deck = 1;
    p.opponent.seen = vec!["Hulk".into()];
    p.opponent.known = vec!["Hulk".into(), "AntMan".into(), "AntMan".into()];
    p.opponent.pool = Some(vec!["Hulk".into(), "AntMan".into(), "Cyclops".into()]);
    let mut rng = PythonRandom::seed(33);
    let game = build_game(&p, &mut rng).unwrap();
    let ids: Vec<_> = game.players[1]
        .hand
        .iter()
        .chain(&game.players[1].deck)
        .map(|&i| game.cards[i].def.id)
        .collect();
    assert!(ids.contains(&"AntMan"));
    assert!(ids.contains(&"Cyclops"));
    assert!(!ids.contains(&"Hulk"));
}

#[test]
fn opponent_known_rejects_impossible_hidden_state() {
    let mut p = position(json!([
        {"id":"Xandar", "mine":[], "theirs":[]},
        {"id":"OttosLab", "mine":[], "theirs":[]},
        {"id":"KamarTaj", "mine":[], "theirs":[]}
    ]));
    p.opponent.known = vec!["Hulk".into()];
    let mut rng = PythonRandom::seed(34);
    assert!(build_game(&p, &mut rng).is_err());
    p.opponent.hand = 1;
    p.opponent.known = vec!["Rock".into()];
    assert!(build_game(&p, &mut rng).is_err());
}

#[test]
fn opponent_known_changes_recruit_sampling_prior() {
    let mut p = position(json!([
        {"id":"Xandar", "mine":[], "theirs":[]},
        {"id":"OttosLab", "mine":[], "theirs":[]},
        {"id":"KamarTaj", "mine":[], "theirs":[]}
    ]));
    p.mode = Some("recruit".into());
    let recruit = snapsim::position::recruit_cards();
    let id = snapsim::cards::deck_eligible_ids()
        .iter()
        .find(|id| !recruit.iter().any(|card| card == **id))
        .unwrap();
    p.opponent.known.push((*id).to_string());
    let prior = snapsim::coverage::sampling_priors(&p);
    assert_eq!(prior["opponent_cards"]["source"], "all-supported");
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
    assert_eq!(game.cards[hawkeye].watch_contexts(), vec![(3, 0), (3, 0)]);
    assert!(game.cards[jessica].watch_contexts().is_empty());
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
            .all(|entry| game.cards[entry.card].turn_played == 2)
    );
    assert_eq!(
        game.lanes[1]
            .state
            .as_deref()
            .unwrap()
            .as_any()
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

#[test]
fn observed_quickdraw_uses_last_matching_entry() {
    let p: Position = serde_json::from_value(json!({
        "turn":4,"energy":4,"deck":[],
        "hand":[
            {"id":"Hulk","turns_drawn":[[1,false],[4,false]]},
            {"id":"IronMan","turns_drawn":[[3,true]]},
            {"id":"Medusa","turns_drawn":[[3,false]]},
            {"id":"Cyclops","turns_drawn":[[4,true],[3,false]]}
        ],
        "locations":[{"id":"Xandar","mine":[],"theirs":[]},{"id":"OttosLab","mine":[],"theirs":[]},{"id":"KamarTaj","mine":[],"theirs":[]}],
        "opponent":{"hand":0,"deck":0}
    })).unwrap();
    let mut rng = PythonRandom::seed(7);
    let game = build_game(&p, &mut rng).unwrap();
    let hand = &game.players[0].hand;
    assert!(game.is_quickdraw_active(hand[0]));
    assert!(game.is_quickdraw_active(hand[1]));
    assert!(!game.is_quickdraw_active(hand[2]));
    assert!(game.is_quickdraw_active(hand[3]));
    assert_eq!(game.players[0].draws.len(), 6);
}

#[test]
fn snap_observed_ledger_and_resolution_are_hydrated() {
    let card = |id: &str, entity: i32, power: i32, order: u32| {
        json!({
            "card_id":id,"entity_id":entity,"owner_entity_id":10,"cost":2,"power":power,
            "revealed":true,"zone_position":entity,"turn_revealed":2,"latest_resolution_order":order,
            "turns_drawn":[[1,false],[3,true]],"activated":true,"moves_per_turn":{"2":1,"3":2},
            "created":true,"created_by_card_id":"Agent13","created_by_player_entity_id":20,
            "stat_updates":[{"stat":"Power","update":"Add","value":1,"player_source_entity_id":10,
                "updater_card_id":"BlueMarvel","updater_location_id":null},
                {"stat":"Power","update":"Add","value":-2,"player_source_entity_id":20,
                "updater_card_id":"Scorpion","updater_location_id":null}]
        })
    };
    let mut hulk = card("Hulk", 11, 13, 8);
    hulk["merged_to_entity_id"] = json!(12);
    hulk["stat_updates"].as_array_mut().unwrap().splice(0..0, [
        json!({"stat":"Power","update":"Add","value":1,"player_source_entity_id":10,"updater_card_id":"BlueMarvel","updater_faction_id":"Avengers"}),
        json!({"stat":"Power","update":"Add","value":1,"player_source_entity_id":10,"updater_card_id":"BlueMarvel","updater_augment_id":"AugmentA"}),
        json!({"stat":"Power","update":"Add","value":1,"player_source_entity_id":10,"updater_card_id":"BlueMarvel","updater_league_id":"Ranked"}),
    ]);
    hulk["stat_updates"].as_array_mut().unwrap().extend([
        json!({"stat":"Cost","update":"Set","value":2,"player_source_entity_id":null}),
        json!({"stat":"Power","update":"Multiply","value":2,"player_source_entity_id":10}),
        json!({"stat":"CanBeDestroyed","update":"Set","value":0,"player_source_entity_id":10}),
    ]);
    let mut blue = card("BlueMarvel", 12, 3, 2);
    blue["stat_updates"] = json!([]);
    let mine_grave = card("Wolverine", 13, 4, 1);
    let mine_banished = card("Medusa", 14, 3, 0);
    let mut their_grave = card("IronMan", 21, 0, 3);
    their_grave["owner_entity_id"] = json!(20);
    let mut their_banished = card("Cyclops", 22, 4, 0);
    their_banished["owner_entity_id"] = json!(20);
    let observation = json!({"schema_version":1,"source":{},"game":{"turn":3,"total_turns":6,
        "players":[{"slot":"local","entity_id":10,"energy":3,"ended_turn":2,"zones":{"hand":{"count":0,"cards":[]},"deck":{"count":0},"graveyard":{"cards":[mine_grave]},"banished":{"cards":[mine_banished]}}},
                   {"slot":"opponent","entity_id":20,"zones":{"hand":{"count":0},"deck":{"count":0},"graveyard":{"cards":[their_grave]},"banished":{"cards":[their_banished]}}}],
        "locations":[{"location_id":"Ruins","cards":[hulk,blue]},{"location_id":"Ruins","cards":[]},{"location_id":"Ruins","cards":[]}]}});
    let value = snapsim::snap::position_from_snap(&observation, &json!({})).unwrap();
    assert_eq!(value["locations"][0]["mine"][0]["created_by_player"], 1);
    assert_eq!(
        value["locations"][0]["mine"][0]["stat_updates"][4]["source"],
        1
    );
    assert_eq!(
        value["locations"][0]["mine"][0]["stat_updates"][0]["updater_faction_id"],
        "Avengers"
    );
    assert_eq!(
        value["locations"][0]["mine"][0]["stat_updates"][1]["updater_augment_id"],
        "AugmentA"
    );
    assert_eq!(
        value["locations"][0]["mine"][0]["stat_updates"][2]["updater_league_id"],
        "Ranked"
    );
    assert_eq!(value["graveyard"][0]["id"], "Wolverine");
    assert_eq!(value["opponent"]["graveyard"][0]["id"], "IronMan");
    assert_eq!(value["opponent"]["banished"][0]["id"], "Cyclops");
    let p = Position::from_json(&value.to_string()).unwrap();
    let mut rng = PythonRandom::seed(8);
    let game = build_game(&p, &mut rng).unwrap();
    let id = game.lanes[0].cards[0];
    let target = &game.cards[id];
    assert!(target.created && target.activated);
    assert_eq!(target.created_by_card_id.as_deref(), Some("Agent13"));
    assert_eq!(target.created_by_player, Some(1));
    assert_eq!(target.observed_entity_id, Some(11));
    assert_eq!(target.merged_to_entity_id, Some(12));
    assert_eq!(target.merged_to, Some(game.lanes[0].cards[1]));
    assert_eq!(game.players[0].graveyard[0].1, None);
    assert_eq!(
        game.cards[game.players[0].graveyard[0].0].def.id,
        "Wolverine"
    );
    assert_eq!(game.cards[game.players[1].graveyard[0].0].def.id, "IronMan");
    assert_eq!(game.players[1].graveyard[0].1, None);
    assert_eq!(game.cards[game.players[0].banished[0]].def.id, "Medusa");
    assert_eq!(game.cards[game.players[1].banished[0]].def.id, "Cyclops");
    assert_eq!(target.latest_resolution_order, 8);
    assert_eq!(game.next_resolution_order, 9);
    assert_eq!(target.player_moves_used, 3);
    assert_eq!(target.move_turns, [2, 3, 3]);
    assert!(game.is_quickdraw_active(id));
    assert_eq!(target.power(), 13);
    assert_eq!(target.stat_updates.len(), 7);
    assert_eq!(target.stat_updates.iter().filter(|u| u.ongoing).count(), 1);
    assert_eq!(
        target
            .stat_updates
            .iter()
            .filter(|u| !u.ongoing && u.value == 1 && u.stat == snapsim::game::Stat::Power)
            .count(),
        3
    );
    assert!(
        target
            .stat_updates
            .iter()
            .any(|u| u.stat == snapsim::game::Stat::Cost
                && u.operation == snapsim::game::StatOperation::Set
                && u.source == usize::MAX)
    );
    assert!(
        target
            .stat_updates
            .iter()
            .any(|u| u.operation == snapsim::game::StatOperation::Multiply && u.source == 0)
    );
    assert!(
        target
            .stat_updates
            .iter()
            .any(|u| u.source == 1 && u.value == -2)
    );
}

#[test]
fn known_out_of_play_cards_are_not_sampled_into_the_local_deck() {
    let p: Position = serde_json::from_value(json!({
        "turn":3,"energy":3,"hand":[],"deck":[],"deck_count":30,
        "graveyard":[{"id":"Hulk"}],"banished":[{"id":"Medusa"}],
        "locations":[{"id":"Ruins","mine":[],"theirs":[]},{"id":"Ruins","mine":[],"theirs":[]},{"id":"Ruins","mine":[],"theirs":[]}],
        "opponent":{"hand":0,"deck":0}
    })).unwrap();
    let mut rng = PythonRandom::seed(9);
    let game = build_game(&p, &mut rng).unwrap();
    assert!(
        game.players[0]
            .deck
            .iter()
            .all(|&id| !matches!(game.cards[id].def.id, "Hulk" | "Medusa"))
    );
}

#[test]
fn snap_cards_missing_from_the_option_table_cannot_move() {
    // Turn 6 of a live match: one Nightcrawler already used its one move (the reader's
    // moves_per_turn stays empty), so the game lists it no longer or only as an invalid option.
    let nightcrawler = |entity: i32| {
        json!({"card_id":"Nightcrawler","entity_id":entity,"owner_entity_id":10,"cost":1,"power":2,
            "revealed":true,"zone_position":0,"turn_revealed":3,"moves_per_turn":{}})
    };
    let observation = json!({"schema_version":1,"source":{},"game":{"turn":6,"total_turns":6,
        "players":[{"slot":"local","entity_id":10,"energy":3,"ended_turn":5,"zones":{"hand":{"count":0,"cards":[]},"deck":{"count":0}}},
                   {"slot":"opponent","entity_id":20,"zones":{"hand":{"count":0},"deck":{"count":0}}}],
        "locations":[{"entity_id":15,"location_id":"Ruins","cards":[nightcrawler(27)]},
                     {"entity_id":16,"location_id":"Ruins","cards":[nightcrawler(26)]},
                     {"entity_id":17,"location_id":"Ruins","cards":[nightcrawler(50)]}],
        "legal_actions":[
            {"entity_id":26,"target_entity_ids":[17],"can_activate":false,"valid":false,"no_stage_reason":"NotPlayable"},
            {"entity_id":50,"target_entity_ids":[15],"can_activate":false,"valid":true,"no_stage_reason":"None"}]}});
    let value = snapsim::snap::position_from_snap(&observation, &json!({})).unwrap();
    for (lane, targets) in [(0, json!([])), (1, json!([])), (2, json!([0]))] {
        assert_eq!(
            value["locations"][lane]["mine"][0]["move_targets"], targets,
            "lane {lane}"
        );
    }
    assert_eq!(value["locations"][0]["mine"][0]["can_activate"], false);
    let p = Position::from_json(&value.to_string()).unwrap();
    let mut rng = PythonRandom::seed(1);
    let game = build_game(&p, &mut rng).unwrap();
    let (missing, invalid, free) = (
        game.lanes[0].cards[0],
        game.lanes[1].cards[0],
        game.lanes[2].cards[0],
    );
    for destination in 0..3 {
        assert!(
            !game.can_player_move(missing, destination, 0)
                && !game.can_player_move(invalid, destination, 0)
        );
    }
    assert!(game.can_player_move(free, 0, 0) && !game.can_player_move(free, 1, 0));
}
