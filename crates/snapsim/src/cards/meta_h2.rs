//! Meta-deck cards, batch H2: rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{CardState, Event, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        on_move: Some(dagger),
        ..CardDef::vanilla("Dagger")
    },
    CardDef {
        start_turn: Some(daredevil),
        ..CardDef::vanilla("Daredevil")
    },
    CardDef {
        reveal: Some(green_goblin),
        ..CardDef::vanilla("GreenGoblin")
    },
    CardDef {
        on_activate: Some(jennifer_kale),
        ..CardDef::vanilla("JenniferKale")
    },
    CardDef {
        initial_state: Some(objective_state),
        objective_update: Some(objective_update),
        objective_freeze: Some(objective_freeze),
        on_event: Some(objective_complete),
        ..CardDef::vanilla("JuggernautHorseman")
    },
];

// DaredevilBehavior::ForceOpponentToStageFirst @ 0x7e87de0.
fn daredevil(game: &mut Game<'_>, card: usize) {
    if game.turn == 5 && game.cards[card].lane.is_some() {
        let owner = game.cards[card].owner;
        if let Some(queue) = &mut game.staging_order {
            queue.extend([1 - owner, owner]);
        }
    }
}

// DaggerBehavior::BuffOthers @ 0x7e86cb4 counts all opposing cards, including
// face-down cards, at the destination.
fn dagger(game: &mut Game<'_>, card: usize, _source: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let count = game.lanes[lane]
        .cards
        .iter()
        .filter(|&&other| game.cards[other].owner != owner)
        .count() as i32;
    if count > 0 {
        game.add_power(card, count * 2, owner);
    }
}

// GreenGoblinBehavior::SwitchSide @ 0x7eb9c3c calls
// Location::ChangeOwnerOfCard @ 0x7ddfadc, not a move.
fn green_goblin(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let other = 1 - game.cards[card].owner;
    if game.has_space(lane, other, 0) {
        game.cards[card].owner = other;
    }
}

// JenniferKaleBehavior::AddEnergyTriggerToPlayer @ 0x7ed3a0c creates a
// player-owned next-turn effect, independent of her subsequent presence.
fn jennifer_kale(game: &mut Game<'_>, card: usize) {
    game.schedule_energy(game.cards[card].owner, game.turn + 1, 1);
}

#[derive(Clone, Default)]
struct Objective {
    progress: usize,
    ready: bool,
    completed: bool,
}

pub(crate) fn restore_objective(card: &mut crate::game::Card, progress: usize, completed: bool) {
    if let Some(state) = card
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<Objective>())
    {
        state.progress = progress;
        state.ready = false;
        state.completed = completed;
    }
}

fn objective_state() -> Box<dyn CardState> {
    Box::new(Objective::default())
}

// JuggernautHorsemanBehavior::CalculateObjectiveProgress @ 0x7ed73fc and
// GetValidCount @ 0x7ed7510 count distinct current powers of other allies.
fn objective_update(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut powers = [0; 12];
    let mut count = 0;
    for id in game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
    {
        let other = &game.cards[id];
        if id != card
            && other.owner == owner
            && game.is_character(id)
            && (other.revealed
                || (game.is_wait_for_staging && other.turn_played == game.turn && game.turn > 0))
            && !powers[..count].contains(&other.power())
        {
            powers[count] = other.power();
            count += 1;
        }
    }
    let state = game.cards[card]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Objective>()
        .unwrap();
    if !state.completed && !state.ready {
        state.progress = count.min(6);
    }
}

fn objective_freeze(game: &mut Game<'_>, card: usize) {
    let state = game.cards[card]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Objective>()
        .unwrap();
    if !state.completed && state.progress >= 6 {
        state.ready = true;
    }
}

fn objective_complete(game: &mut Game<'_>, card: usize, event: Event) {
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
    state.completed = true;
    // JuggernautHorsemanBehavior::BuffSelf @ 0x7ed7814.
    game.add_power(card, 6, game.cards[card].owner);
}

pub(crate) static TOKENS: &[&str] = &[];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Action;
    use crate::position::{Position, build_game};
    use crate::random::PythonRandom;
    use serde_json::{Value, json};

    fn position(mine: Value, theirs: Value) -> Position {
        serde_json::from_value(json!({
            "turn": 5, "energy": 5, "hand": [], "deck": [],
            "locations": [
                {"id": null, "mine": mine, "theirs": theirs},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap()
    }

    #[test]
    fn dagger_counts_face_down_enemies_and_each_move() {
        let p = position(
            json!([{"id":"Dagger"}]),
            json!([
                {"id":"Hulk"}, {"id":"Wasp", "revealed":false, "played_turn":5}
            ]),
        );
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let card = game.lanes[0].cards[0];
        let before = game.cards[card].power();
        dagger(&mut game, card, 1);
        dagger(&mut game, card, 1);
        assert_eq!(game.cards[card].power(), before + 8);
    }

    #[test]
    fn goblin_switches_only_if_space_and_can_switch_back() {
        let p = position(json!([{"id":"GreenGoblin"}]), json!([]));
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let card = game.lanes[0].cards[0];
        green_goblin(&mut game, card);
        assert_eq!(game.cards[card].owner, 1);
        green_goblin(&mut game, card);
        assert_eq!(game.cards[card].owner, 0);
        for _ in 0..4 {
            let id = game.create_card("Wasp", 1);
            assert!(game.add_card(id, 0, 1, false));
        }
        green_goblin(&mut game, card);
        assert_eq!(game.cards[card].owner, 0);
    }

    #[test]
    fn goblin_play_history_stays_with_staging_player() {
        let p = position(json!([]), json!([]));
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let card = game.create_card("GreenGoblin", 0);
        game.players[0].hand.push(card);
        game.stage_action(0, Action::Play { card, lane: 0 })
            .unwrap();
        game.resolve_staged_actions(0);
        assert_eq!(game.cards[card].owner, 1);
        assert_eq!(game.players[0].play_history, vec![(5, card)]);
        assert!(game.players[0].staged_history[0].resolved);
        assert!(game.players[1].play_history.is_empty());
    }

    #[test]
    fn jennifer_schedules_owner_energy_even_after_leaving() {
        let p = position(json!([{"id":"JenniferKale"}]), json!([]));
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let card = game.lanes[0].cards[0];
        jennifer_kale(&mut game, card);
        assert!(game.take_from_board(card));
        assert_eq!(game.players[0].scheduled_energy, vec![(6, 1)]);
    }

    #[test]
    fn juggernaut_distinct_power_objective_completes_once() {
        let p = position(json!([{"id":"JuggernautHorseman"}]), json!([]));
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let card = game.lanes[0].cards[0];
        let mut allies = Vec::new();
        for n in 0..6 {
            let id = game.create_card("Wasp", 0);
            game.cards[id].permanent = n.min(4);
            assert!(game.add_card(id, if n < 3 { 1 } else { 2 }, 0, false));
            allies.push(id);
        }
        objective_update(&mut game, card);
        objective_freeze(&mut game, card);
        let before = game.cards[card].power();
        objective_complete(&mut game, card, Event::AfterTurnEnd);
        assert_eq!(game.cards[card].power(), before);
        game.cards[allies[5]].permanent = 5;
        objective_update(&mut game, card);
        objective_freeze(&mut game, card);
        objective_complete(&mut game, card, Event::AfterTurnEnd);
        objective_complete(&mut game, card, Event::AfterTurnEnd);
        assert_eq!(game.cards[card].power(), before + 6);
    }

    #[test]
    fn daredevil_plans_after_visible_opponent_staging() {
        let mut p = position(json!([{"id":"Daredevil"}]), json!([]));
        // Turn execution must not reveal a randomly sampled location.
        for lane in &mut p.locations {
            lane.id = Some("Ruins".into());
        }
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let enemy = game.create_card("Wasp", 1);
        game.players[1].hand.push(enemy);
        let mut order = Vec::new();
        game.play_turn_with_actions(|view, seat, _| {
            order.push(seat);
            if seat == 1 {
                Ok(vec![Action::Play {
                    card: enemy,
                    lane: 1,
                }])
            } else {
                assert!(view.lanes[1].cards.contains(&enemy));
                assert!(!view.cards[enemy].revealed);
                Ok(vec![])
            }
        })
        .unwrap();
        assert_eq!(order, vec![1, 0]);
    }

    #[test]
    fn daredevil_live_coverage_is_fail_closed_only_for_unshown_turn_five_plays() {
        let mut p = position(json!([{"id":"Daredevil"}]), json!([]));
        let reason = json!("Daredevil-visible opponent plays not exported");
        let flagged = |p: &Position| {
            crate::coverage::coverage(p)["state"]
                .as_array()
                .unwrap()
                .contains(&reason)
        };
        // No face-down opponent play: nothing is missing.
        assert!(!flagged(&p));
        // A face-down opponent play the reader did not mark as shown by Daredevil.
        p.locations[0].theirs_face_down = 1;
        assert!(flagged(&p));
        p.turn = 4;
        assert!(!flagged(&p));
    }
}
