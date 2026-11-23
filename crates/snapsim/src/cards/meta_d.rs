//! Meta-deck cards, batch D: Objectives; rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{Event, Game, Stat, StatOperation};

#[derive(Clone)]
struct Objective {
    progress: i32,
    goal: i32,
    ready: bool,
    completed: bool,
}

fn objective() -> Box<dyn crate::game::CardState> {
    Box::new(Objective {
        progress: 0,
        goal: 0,
        ready: false,
        completed: false,
    })
}

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        initial_state: Some(objective),
        objective_update: Some(update),
        objective_freeze: Some(freeze),
        on_event: Some(complete),
        ..CardDef::vanilla("PolarisHorseman")
    },
    CardDef {
        initial_state: Some(objective),
        objective_update: Some(update),
        objective_freeze: Some(freeze),
        on_event: Some(complete),
        ..CardDef::vanilla("CalibanHorseman")
    },
    CardDef {
        initial_state: Some(objective),
        objective_update: Some(update),
        objective_freeze: Some(freeze),
        on_event: Some(complete),
        ..CardDef::vanilla("EnSabahNur")
    },
    CardDef {
        initial_state: Some(objective),
        objective_update: Some(update),
        objective_freeze: Some(freeze),
        on_event: Some(complete),
        ..CardDef::vanilla("GambitHorseman")
    },
    CardDef {
        initial_state: Some(objective),
        objective_update: Some(update),
        objective_freeze: Some(freeze),
        on_event: Some(complete),
        ..CardDef::vanilla("WolverineHorseman")
    },
    CardDef {
        reveal: Some(apocalypse),
        ..CardDef::vanilla("ApocalypseCelestialsChosen")
    },
];

pub(crate) static TOKENS: &[&str] = &["ApocalypseCelestialsChosen"];

fn board<'a>(game: &'a Game<'_>, seat: usize) -> impl Iterator<Item = usize> + 'a {
    game.lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(move |&id| game.cards[id].owner == seat && game.cards[id].revealed)
}

fn update(game: &mut Game<'_>, card: usize) {
    let seat = game.cards[card].owner;
    // The client clamps progress, but freezes it after CompleteObjective (0x7c9f714).
    let (progress, goal) = match game.cards[card].abilities().next().unwrap().def.id {
        "PolarisHorseman" => {
            // GetValidCount 0x7f166d8: sum all negative additive Power ledger entries.
            let sum: i32 = game
                .lanes
                .iter()
                .flat_map(|lane| lane.cards.iter().copied())
                .filter(|&id| {
                    game.cards[id].revealed
                        || (game.is_wait_for_staging
                            && game.cards[id].owner == seat
                            && game.cards[id].turn_played == game.turn
                            && game.turn > 0)
                })
                .flat_map(|id| &game.cards[id].stat_updates)
                .filter(|u| {
                    u.stat == Stat::Power && u.operation == StatOperation::Add && u.value < 0
                })
                .map(|u| -u.value)
                .sum();
            (sum.min(5), 5)
        }
        "CalibanHorseman" => {
            // GetValidCount 0x7e79b80 uses DefCost, including staged allies.
            let count = game
                .lanes
                .iter()
                .flat_map(|lane| lane.cards.iter().copied())
                .filter(|&id| {
                    game.cards[id].owner == seat
                        && (game.cards[id].revealed
                            || (game.is_wait_for_staging
                                && game.cards[id].turn_played == game.turn
                                && game.turn > 0))
                        && game.printed_cost(id) >= 4
                })
                .count();
            ((count as i32).min(3), 3)
        }
        "EnSabahNur" => {
            // GetFrontRowCounts 0x7ea13e8 counts occupied slots, not full locations.
            let count = game
                .lanes
                .iter()
                .flat_map(|lane| lane.cards.iter().copied())
                .filter(|&id| game.cards[id].owner == seat && game.is_front_row(id))
                .count();
            (count as i32, 6)
        }
        "GambitHorseman" => {
            // GetValidCount 0x7eb11e8 uses printed DefCost for created plays.
            let count: i32 = game.players[seat]
                .staged_history
                .iter()
                .filter(|play| {
                    play.created
                        && (play.resolved || (game.is_wait_for_staging && play.turn == game.turn))
                })
                .map(|play| play.printed_cost)
                .sum();
            (count.min(6), 6)
        }
        "WolverineHorseman" => (game.cards[card].power().min(6), 6),
        _ => return,
    };
    let state = game.cards[card]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Objective>()
        .unwrap();
    if !state.completed && !state.ready {
        state.progress = progress;
        state.goal = goal;
    }
}

fn freeze(game: &mut Game<'_>, card: usize) {
    let state = game.cards[card]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Objective>()
        .unwrap();
    if !state.completed && state.goal > 0 && state.progress >= state.goal {
        state.ready = true;
    }
}

fn complete(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::AfterTurnEnd) || game.cards[card].abilities_removed {
        return;
    }
    let state = game.cards[card]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Objective>()
        .unwrap();
    if state.completed || !state.ready {
        return;
    }
    // CompleteObjective 0x7e0d9ac marks completed before running the action.
    state.completed = true;
    let seat = game.cards[card].owner;
    let ability_id = game.cards[card].abilities().next().unwrap().def.id;
    match ability_id {
        "PolarisHorseman" => {
            let Some(lane) = game.cards[card].lane else {
                return;
            };
            // PullCard 0x7f16aa0 rolls before checking location capacity.
            if !game.players[seat].hand.is_empty() {
                let pick = game
                    .rng
                    .borrow_mut()
                    .randbelow(game.players[seat].hand.len());
                let target = game.players[seat].hand[pick];
                game.move_hand_card_to_location(target, lane);
            }
        }
        "CalibanHorseman" => {
            // BigAfflict 0x7e79d70 rolls among highest-DefCost ties with power.
            let enemies: Vec<_> = board(game, 1 - seat)
                .filter(|&id| game.is_character(id))
                .collect();
            if let Some(max) = enemies.iter().map(|&id| game.printed_cost(id)).max() {
                let tied: Vec<_> = enemies
                    .into_iter()
                    .filter(|&id| game.printed_cost(id) == max)
                    .collect();
                let pick = game.rng.borrow_mut().randbelow(tied.len());
                game.add_power(tied[pick], -6, seat);
            }
        }
        "EnSabahNur" => {
            game.transform_card(card, "ApocalypseCelestialsChosen");
        }
        "GambitHorseman" => {
            // CheckSpendToDestroy 0x7eb194c selects all targets before destroying;
            // each roll is over remaining enemies affordable with the budget.
            let mut chosen = Vec::new();
            let mut spent = 0;
            while spent < 3 {
                let candidates: Vec<_> = board(game, 1 - seat)
                    .filter(|&id| !chosen.contains(&id) && game.printed_cost(id) <= 3 - spent)
                    .collect();
                if candidates.is_empty() {
                    break;
                }
                let pick = game.rng.borrow_mut().randbelow(candidates.len());
                let target = candidates[pick];
                spent += game.printed_cost(target);
                chosen.push(target);
            }
            for target in chosen {
                game.destroy_card(target);
            }
        }
        "WolverineHorseman" => {
            // Buff 0x7cf522c rolls once per nonempty location, including itself.
            for lane in 0..game.lanes.len() {
                let allies: Vec<_> = game.lanes[lane]
                    .cards
                    .iter()
                    .copied()
                    .filter(|&id| game.cards[id].owner == seat && game.cards[id].revealed)
                    .collect();
                if !allies.is_empty() {
                    let pick = game.rng.borrow_mut().randbelow(allies.len());
                    game.add_power(allies[pick], 4, seat);
                }
            }
        }
        _ => {}
    }
}

fn apocalypse(game: &mut Game<'_>, card: usize) {
    let seat = game.cards[card].owner;
    // SwapCard 0x7e68670: choose a lowest-Power revealed front-row character,
    // then a deck character, move both and shuffle the deck.
    let fronts: Vec<_> = board(game, seat)
        .filter(|&id| game.is_front_row(id) && game.is_character(id))
        .collect();
    let Some(min) = fronts.iter().map(|&id| game.cards[id].power()).min() else {
        return;
    };
    let tied: Vec<_> = fronts
        .into_iter()
        .filter(|&id| game.cards[id].power() == min)
        .collect();
    let pick = game.rng.borrow_mut().randbelow(tied.len());
    let target = tied[pick];
    let candidates: Vec<_> = game.players[seat]
        .deck
        .iter()
        .copied()
        .filter(|&id| game.is_character(id))
        .collect();
    if candidates.is_empty() {
        return;
    }
    let pick = game.rng.borrow_mut().randbelow(candidates.len());
    let replacement = candidates[pick];
    let lane = game.cards[target].lane.unwrap();
    if !game.take_from_board(target) {
        return;
    }
    while let Some(slot) = game.players[seat]
        .deck
        .iter()
        .position(|&id| id == replacement)
    {
        game.take_deck(seat, slot);
    }
    game.push_deck(seat, target);
    if !game.add_card(replacement, lane, seat, true) {
        game.push_deck(seat, replacement);
    }
    for i in 0..game.players[seat].deck.len().saturating_sub(1) {
        let j = i + game
            .rng
            .borrow_mut()
            .randbelow(game.players[seat].deck.len() - i);
        game.swap_deck(seat, i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::StagedPlay;
    use crate::position::{Position, build_game};
    use crate::random::PythonRandom;
    use serde_json::json;

    fn position(id: &str, allies: serde_json::Value, enemies: serde_json::Value) -> Position {
        let mut mine = vec![json!({"id": id})];
        mine.extend(allies.as_array().unwrap().iter().cloned());
        serde_json::from_value(json!({
            "turn": 3, "energy": 3, "hand": [{"id": "Hulk"}], "deck": ["Hulk"],
            "locations": [
                {"id": null, "mine": mine, "theirs": enemies},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap()
    }

    fn progress(game: &Game<'_>, card: usize) -> (i32, i32, bool) {
        let state = game.cards[card]
            .state
            .as_deref()
            .unwrap()
            .as_any()
            .downcast_ref::<Objective>()
            .unwrap();
        (state.progress, state.goal, state.completed)
    }

    #[test]
    fn polaris_counts_negative_ledger_and_completes_once() {
        let p = position("PolarisHorseman", json!([]), json!([{"id":"Hulk"}]));
        let mut rng = PythonRandom::seed(1);
        let mut g = build_game(&p, &mut rng).unwrap();
        let card = g.lanes[0].cards[0];
        let enemy = g.lanes[0].cards[1];
        g.add_power(enemy, -4, 0);
        update(&mut g, card);
        assert_eq!(progress(&g, card).0, 4);
        complete(&mut g, card, Event::AfterTurnEnd);
        assert!(!progress(&g, card).2);
        g.add_power(enemy, -1, 0);
        update(&mut g, card);
        complete(&mut g, card, Event::TurnEnd);
        assert_eq!(g.players[0].hand.len(), 1);
        freeze(&mut g, card);
        complete(&mut g, card, Event::AfterTurnEnd);
        assert_eq!(g.players[0].hand.len(), 0);
        assert!(progress(&g, card).2);
        complete(&mut g, card, Event::AfterTurnEnd);
        assert_eq!(g.players[0].hand.len(), 0);
    }

    #[test]
    fn caliban_counts_printed_cost_and_afflicts_once() {
        let p = position(
            "CalibanHorseman",
            json!([{"id":"Hulk"}, {"id":"Hulk"}]),
            json!([{"id":"Hulk"}]),
        );
        let mut rng = PythonRandom::seed(2);
        let mut g = build_game(&p, &mut rng).unwrap();
        let card = g.lanes[0].cards[0];
        let enemy = *g.lanes[0].cards.last().unwrap();
        update(&mut g, card);
        assert_eq!(progress(&g, card).0, 2);
        let third = g.create_card("JessicaJones", 0);
        g.add_card(third, 1, 0, false);
        update(&mut g, card);
        assert_eq!(progress(&g, card).0, 3);
        complete(&mut g, card, Event::TurnEnd);
        assert_eq!(g.cards[enemy].power(), 14);
        freeze(&mut g, card);
        complete(&mut g, card, Event::AfterTurnEnd);
        let power = g.cards[enemy].power();
        assert_eq!(power, 8);
        complete(&mut g, card, Event::AfterTurnEnd);
        assert_eq!(g.cards[enemy].power(), power);
    }

    #[test]
    fn wolverine_freezes_progress_and_buffs_once() {
        let p = position("WolverineHorseman", json!([]), json!([]));
        let mut rng = PythonRandom::seed(3);
        let mut g = build_game(&p, &mut rng).unwrap();
        let card = g.lanes[0].cards[0];
        g.set_power(card, 5, 0);
        update(&mut g, card);
        assert_eq!(progress(&g, card).0, 5);
        g.add_power(card, 1, 0);
        update(&mut g, card);
        complete(&mut g, card, Event::TurnEnd);
        assert_eq!(g.cards[card].power(), 6);
        freeze(&mut g, card);
        complete(&mut g, card, Event::AfterTurnEnd);
        assert_eq!(g.cards[card].power(), 10);
        g.set_power(card, 1, 0);
        update(&mut g, card);
        assert_eq!(progress(&g, card), (6, 6, true));
        complete(&mut g, card, Event::AfterTurnEnd);
        assert_eq!(g.cards[card].power(), 1);
    }

    #[test]
    fn gambit_uses_captured_printed_cost_and_budget() {
        let p = position("GambitHorseman", json!([]), json!([{"id":"Hawkeye"}]));
        let mut rng = PythonRandom::seed(4);
        let mut g = build_game(&p, &mut rng).unwrap();
        let card = g.lanes[0].cards[0];
        let enemy = g.lanes[0].cards[1];
        g.players[0].staged_history.push(StagedPlay {
            turn: 1,
            card,
            def_id: "Hulk",
            printed_cost: 5,
            created: true,
            resolved: true,
        });
        update(&mut g, card);
        assert_eq!(progress(&g, card).0, 5);
        g.players[0].staged_history[0].printed_cost = 6;
        update(&mut g, card);
        complete(&mut g, card, Event::TurnEnd);
        assert!(g.cards[enemy].lane.is_some());
        freeze(&mut g, card);
        complete(&mut g, card, Event::AfterTurnEnd);
        assert!(g.cards[enemy].lane.is_none());
        assert!(progress(&g, card).2);
        complete(&mut g, card, Event::AfterTurnEnd);
        assert_eq!(g.players[1].graveyard.len(), 1);
    }

    #[test]
    fn en_sabah_counts_front_slots_and_transforms_after_turn() {
        let p = position("EnSabahNur", json!([{"id":"Hulk"}]), json!([]));
        let mut rng = PythonRandom::seed(5);
        let mut g = build_game(&p, &mut rng).unwrap();
        let card = g.lanes[0].cards[0];
        for lane in 1..3 {
            let ally = g.create_card("Hulk", 0);
            g.add_card(ally, lane, 0, false);
        }
        update(&mut g, card);
        assert_eq!(progress(&g, card).0, 4);
        for lane in 1..3 {
            let ally = g.create_card("Hulk", 0);
            g.add_card(ally, lane, 0, false);
        }
        update(&mut g, card);
        assert_eq!(progress(&g, card).0, 6);
        complete(&mut g, card, Event::TurnEnd);
        assert_eq!(g.cards[card].def.id, "EnSabahNur");
        freeze(&mut g, card);
        complete(&mut g, card, Event::AfterTurnEnd);
        assert_eq!(g.cards[card].def.id, "ApocalypseCelestialsChosen");
        assert_eq!(g.cards[card].power(), 8);
        assert!(g.cards[card].def.on_event.is_none());
    }
}
