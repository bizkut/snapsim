//! Meta-deck cards, batch H1: rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{CardState, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(drax),
        ..CardDef::vanilla("Drax")
    },
    CardDef {
        initial_state: Some(torch_state),
        end_turn: Some(torch),
        end_turn_trigger: true,
        ..CardDef::vanilla("HumanTorchFirstSteps")
    },
    CardDef {
        ongoing_ability: true,
        ..CardDef::vanilla("LukeCage")
    },
    CardDef {
        ongoing: Some(power_stone),
        ..CardDef::vanilla("PowerStone")
    },
    CardDef {
        reveal: Some(silver_sable),
        ..CardDef::vanilla("SilverSable")
    },
    CardDef {
        reveal: Some(reality_stone),
        ..CardDef::vanilla("RealityStone")
    },
    CardDef {
        reveal: Some(time_stone),
        ..CardDef::vanilla("TimeStone")
    },
];

pub(crate) static TOKENS: &[&str] = &["PowerStone", "RealityStone", "TimeStone"];

// DraxBehavior::BuffThis (0x7e93500): staged plays count regardless of reveal order.
fn drax(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.cards[card]
        .lane
        .is_some_and(|lane| game.played_at(1 - owner, lane))
    {
        game.add_power(card, 4, owner);
    }
}

fn torch_state() -> Box<dyn CardState> {
    Box::new(false)
}

// HumanTorchFirstStepsBehavior::BuffCard (0x7ec812c).
fn torch(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let full = game.lanes[lane]
        .cards
        .iter()
        .filter(|&&id| game.cards[id].owner == owner)
        .count()
        >= 4;
    let used = game.cards[card]
        .state
        .as_deref_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<bool>())
        .expect("Torch state");
    if full && !*used {
        *used = true;
        game.multiply_power(card, 2, owner);
    } else {
        game.add_power(card, 1, owner);
    }
}

// PowerStoneBehavior::CheckStonesPlayed (0x7f179b4), BuffThanos (0x7f17568),
// and GetAllThanos (0x7f173d4): distinct IDs, not surviving stones.
fn power_stone(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let stones = [
        "MindStone",
        "PowerStone",
        "RealityStone",
        "SoulStone",
        "SpaceStone",
        "TimeStone",
    ];
    if !stones.iter().all(|id| {
        game.players[owner]
            .play_history
            .iter()
            .any(|&(_, played)| game.cards[played].def.id == *id)
    }) {
        return;
    }
    for target in 0..game.cards.len() {
        if game.cards[target].owner == owner
            && matches!(game.cards[target].def.id, "Thanos" | "ThanosChampion")
            && (game.cards[target].lane.is_some()
                || game.players[owner].hand.contains(&target)
                || game.players[owner].deck.contains(&target))
        {
            game.add_ongoing_power(target, 10, owner);
        }
    }
}

// SilverSableBehavior::DrainTopCard (0x7f3cb7c): first character in deck order.
fn silver_sable(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if let Some(target) = game.players[1 - owner]
        .deck
        .iter()
        .copied()
        .find(|&id| game.is_character(id))
    {
        game.steal_power(target, card, 2, owner);
    }
}

// RealityStoneBehavior::DoAction (0x7f238dc) calls GetRandomLocationDefId.
fn reality_stone(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let choices: Vec<_> = game
        .location_pool
        .iter()
        .filter(|location| !game.lanes.iter().any(|lane| lane.def.id == location.id))
        .map(|location| location.id)
        .collect();
    if !choices.is_empty() {
        let index = game.rng.borrow_mut().randbelow(choices.len());
        game.replace_location(lane, choices[index]);
    }
}

// TimeStoneBehavior::DrawCardAndReduceThanosCost (0x7cde6b8) draws first,
// then reduces every matching owned card, in any zone.
fn time_stone(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.draw(owner);
    for target in 0..game.cards.len() {
        if game.cards[target].owner == owner
            && matches!(game.cards[target].def.id, "Thanos" | "ThanosChampion")
        {
            game.add_cost(target, -1, owner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{Position, build_game};
    use crate::random::PythonRandom;
    use serde_json::json;

    fn position(id: &str) -> Position {
        serde_json::from_value(json!({
            "turn": 3, "energy": 3, "hand": [], "deck": [],
            "locations": [
                {"id": null, "mine": [{"id": id}], "theirs": []},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap()
    }

    #[test]
    fn drax_counts_staged_enemy_only_at_play_lane() {
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&position("Drax"), &mut rng).unwrap();
        let card = game.lanes[0].cards[0];
        let enemy = game.create_card("Hulk", 1);
        game.cards[enemy].turn_played = 3;
        game.pending_plays[1].push((enemy, 1));
        drax(&mut game, card);
        assert_eq!(game.cards[card].power(), 6);
        game.pending_plays[1][0].1 = 0;
        drax(&mut game, card);
        assert_eq!(game.cards[card].power(), 10);
    }

    #[test]
    fn torch_doubles_only_once() {
        let mut rng = PythonRandom::seed(2);
        let mut game = build_game(&position("HumanTorchFirstSteps"), &mut rng).unwrap();
        let card = game.lanes[0].cards[0];
        torch(&mut game, card);
        assert_eq!(game.cards[card].power(), 3);
        for _ in 0..3 {
            let ally = game.create_card("Hulk", 0);
            game.add_card(ally, 0, 0, true);
        }
        torch(&mut game, card);
        assert_eq!(game.cards[card].power(), 6);
        torch(&mut game, card);
        assert_eq!(game.cards[card].power(), 7);
    }

    #[test]
    fn sable_steals_from_top_character_and_handles_empty_deck() {
        let mut rng = PythonRandom::seed(3);
        let mut game = build_game(&position("SilverSable"), &mut rng).unwrap();
        let card = game.lanes[0].cards[0];
        silver_sable(&mut game, card);
        let enemy = game.create_card("Hulk", 1);
        game.players[1].deck.push(enemy);
        silver_sable(&mut game, card);
        assert_eq!(game.cards[card].power(), 2);
        assert_eq!(game.cards[enemy].power(), 12);
    }

    #[test]
    fn luke_protects_front_row_and_restores_prior_debuff() {
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&position("LukeCage"), &mut rng).unwrap();
        let luke = game.lanes[0].cards[0];
        let ally = game.create_card("Hulk", 0);
        game.add_power(ally, -2, 1);
        game.add_card(ally, 1, 0, true);
        game.ongoing();
        assert_eq!(game.cards[ally].power(), 14);
        game.add_power(ally, -2, 1);
        assert_eq!(game.cards[ally].power(), 14);
        let enemy = game.create_card("SilverSable", 1);
        game.steal_power(ally, enemy, 2, 1);
        assert_eq!(game.cards[enemy].power(), 0);
        assert_eq!(game.cards[ally].power(), 14);
        game.cards[luke].abilities_removed = true;
        game.ongoing();
        assert_eq!(game.cards[ally].power(), 10);
    }

    #[test]
    fn power_stone_requires_six_distinct_plays_even_if_gone() {
        let mut rng = PythonRandom::seed(5);
        let mut game = build_game(&position("PowerStone"), &mut rng).unwrap();
        let stone = game.lanes[0].cards[0];
        let thanos = game.create_card("Thanos", 0);
        game.players[0].hand.push(thanos);
        for id in [
            "PowerStone",
            "MindStone",
            "RealityStone",
            "SoulStone",
            "SpaceStone",
        ] {
            let played = game.create_card(id, 0);
            game.players[0].play_history.push((1, played));
        }
        power_stone(&mut game, stone);
        assert_eq!(game.cards[thanos].ongoing, 0);
        let time = game.create_card("TimeStone", 0);
        game.players[0].play_history.push((2, time));
        power_stone(&mut game, stone);
        assert_eq!(game.cards[thanos].ongoing, 10);
    }

    #[test]
    fn time_stone_draws_before_discounting_thanos() {
        let mut rng = PythonRandom::seed(6);
        let mut game = build_game(&position("TimeStone"), &mut rng).unwrap();
        let stone = game.lanes[0].cards[0];
        let thanos = game.create_card("Thanos", 0);
        game.players[0].deck.push(thanos);
        let cost = game.cards[thanos].cost;
        time_stone(&mut game, stone);
        assert_eq!(game.players[0].hand, [thanos]);
        assert_eq!(game.cards[thanos].cost, cost - 1);
    }

    #[test]
    fn torch_coverage_only_flags_earlier_reveals() {
        let mut position = position("HumanTorchFirstSteps");
        position.locations[0].mine[0].played_turn = Some(position.turn);
        assert!(
            !crate::coverage::coverage(&position)["state"]
                .to_string()
                .contains("doubling")
        );
        position.locations[0].mine[0].played_turn = Some(position.turn - 1);
        assert!(
            crate::coverage::coverage(&position)["state"]
                .to_string()
                .contains("doubling")
        );
    }
}
