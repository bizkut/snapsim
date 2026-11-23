//! Champion skills, from the 59.15.1 client.
use super::CardDef;
use crate::game::{EffectLifetime, Game, PendingEffect, PlayerTrigger};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(thor_skill),
        ..CardDef::vanilla("Skill01ThorChampion")
    },
    CardDef {
        reveal: Some(thing_skill),
        in_hand_ongoing: Some(thing_discount),
        ..CardDef::vanilla("Skill01TheThingFirstStepsChampion")
    },
    CardDef {
        reveal: Some(thanos_skill),
        ..CardDef::vanilla("Skill01ThanosChampion")
    },
    CardDef {
        reveal: Some(weapon_x_skill),
        ..CardDef::vanilla("Skill01WeaponXWolverineChampion")
    },
    CardDef {
        reveal: Some(magneto_skill),
        ..CardDef::vanilla("Skill01MagnetoChampion")
    },
    CardDef {
        reveal: Some(iron_man_skill),
        ..CardDef::vanilla("Skill01IronManChampion")
    },
    CardDef {
        reveal: Some(fantastic_skill),
        ..CardDef::vanilla("Skill01MrFantasticChampion")
    },
    CardDef {
        reveal: Some(rocket_skill),
        ..CardDef::vanilla("Skill01RocketRaccoonChampion")
    },
    CardDef {
        reveal: Some(sentinel_skill),
        ..CardDef::vanilla("Skill01SentinelChampion")
    },
];

pub(crate) static TOKENS: &[&str] = &[
    "Skill01ThorChampion",
    "Skill01TheThingFirstStepsChampion",
    "Skill01ThanosChampion",
    "Skill01WeaponXWolverineChampion",
    "Skill01MagnetoChampion",
    "Skill01IronManChampion",
    "Skill01MrFantasticChampion",
    "Skill01RocketRaccoonChampion",
    "Skill01SentinelChampion",
];

// AddMjolnirDraw 0x7d116b8 creates Mjolnir, shuffles, then draws.
fn thor_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mjolnir = game.create_card("Mjolnir", owner);
    game.push_deck(owner, mjolnir);
    game.shuffle_deck(owner);
    game.draw(owner);
}

// DiscountSelf 0x7d10e40 counts distinct printed costs of revealed characters.
fn thing_discount(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut costs = [0; 12];
    let mut count = 0;
    for lane in &game.lanes {
        for &id in &lane.cards {
            if game.cards[id].owner == owner && game.cards[id].revealed && game.is_character(id) {
                let cost = game.printed_cost(id);
                if !costs[..count].contains(&cost) {
                    costs[count] = cost;
                    count += 1;
                }
            }
        }
    }
    game.add_ongoing_cost(card, -(count as i32), owner);
}

// AddPower 0x7d11194 looks up the player's champion in any zone.
fn thing_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if let Some(champion) = game.player_champion(owner) {
        game.add_power(champion, 8, owner);
    }
}

// DestroyIfHighChampionPower 0x7d108c4 includes unrevealed front-row enemies.
fn thanos_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let Some(champion) = game.player_champion(owner) else {
        return;
    };
    if !game.cards[champion].revealed
        || game.cards[champion].lane.is_none()
        || game.cards[champion].power() < 20
    {
        return;
    }
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].owner != owner && game.is_front_row(id))
        .collect();
    for target in targets {
        game.destroy_card(target);
    }
}

// DestroyAndAddCopy 0x7d11a8c compares each revealed power-bearing ally
// with the champion's current power, then copies this skill to hand.
fn weapon_x_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let Some(champion) = game.player_champion(owner) else {
        return;
    };
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            game.cards[id].owner == owner && game.cards[id].revealed && game.is_character(id)
        })
        .collect();
    for target in targets {
        if game.cards[target].power() < game.cards[champion].power() {
            game.destroy_card(target);
        }
    }
    if let Some(copy) = game.copy_card_to_hand(card, owner) {
        game.add_cost(copy, 1, owner);
        game.notify_hand_added(owner);
    }
}

// MoveCards 0x7d0c92c shuffles the targets before attempting moves.
fn magneto_skill(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let opponent = 1 - owner;
    let mut targets = [0; 8];
    let mut count = 0;
    for other in 0..3 {
        if other == lane {
            continue;
        }
        for &target in &game.lanes[other].cards {
            if game.cards[target].owner == opponent
                && game.cards[target].revealed
                && matches!(game.card_cost(target), 3 | 4)
            {
                targets[count] = target;
                count += 1;
            }
        }
    }
    for i in 0..count.saturating_sub(1) {
        let j = i + game.rng.borrow_mut().randbelow(count - i);
        targets.swap(i, j);
    }
    for &target in &targets[..count] {
        if !game.has_space(lane, opponent, 0) {
            break;
        }
        game.move_card(target, lane);
    }
    if let Some(champion) = game.player_champion(owner)
        && (game.cards[champion].lane.is_some() || game.players[owner].hand.contains(&champion))
        && (!game.has_space(lane, owner, 0) || !game.has_space(lane, opponent, 0))
    {
        game.multiply_power(champion, 2, owner);
    }
}

// AddBuffCardTriggerToPlayer 0x7d0c2c4 installs a one-shot end-turn
// player trigger; GrantPowerForWinning 0x7d0c3e8 counts won locations.
fn iron_man_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: None,
            created_turn: game.turn,
            trigger: PlayerTrigger::EndTurn,
            lifetime: EffectLifetime::ThisTurn,
            state: Box::new(owner),
            apply: iron_man_end_turn,
        },
    );
}

fn iron_man_end_turn(game: &mut Game<'_>, effect: &mut PendingEffect, _: Option<usize>) -> bool {
    let owner = *effect.state.as_any().downcast_ref::<usize>().unwrap();
    if let Some(champion) = game.player_champion(owner) {
        let won = (0..3)
            .filter(|&lane| game.lane_winner(lane) == Some(owner))
            .count();
        if won != 0 {
            game.add_power(champion, 2 * won as i32, owner);
        }
    }
    true
}

// ReduceCostOngoing 0x7d0db9c selects a random ongoing card in hand.
fn fantastic_select(game: &mut Game<'_>, owner: usize) -> Option<usize> {
    let candidates: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.cards[id].has_ongoing(false))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let target = candidates[game.rng.borrow_mut().randbelow(candidates.len())];
    game.add_cost(target, -1, owner);
    game.add_power(target, 1, owner);
    Some(target)
}

fn fantastic_skill(game: &mut Game<'_>, card: usize) {
    if game.cards[card].lane.is_none() {
        return;
    }
    let owner = game.cards[card].owner;
    if let Some(target) = fantastic_select(game, owner) {
        game.add_player_effect(
            owner,
            PendingEffect {
                source: card,
                lane: None,
                created_turn: game.turn,
                trigger: PlayerTrigger::CardResolved,
                lifetime: EffectLifetime::UntilTriggered,
                state: Box::new((owner, target)),
                apply: fantastic_repeat,
            },
        );
    }
}

// ReturnOnOngoingPlay 0x7d0e09c waits for the selected entity to resolve.
fn fantastic_repeat(
    game: &mut Game<'_>,
    effect: &mut PendingEffect,
    target: Option<usize>,
) -> bool {
    let &(owner, selected) = effect
        .state
        .as_any()
        .downcast_ref::<(usize, usize)>()
        .unwrap();
    if target != Some(selected) {
        return false;
    }
    if let Some(next) = fantastic_select(game, owner) {
        effect.state = Box::new((owner, next));
        false
    } else {
        true
    }
}

// AddMoveAndOnRevealTrigger 0x7d0f468 installs a player-owned
// PostCardBanishedTrigger, or copies immediately when no champion exists.
fn rocket_skill(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if game.player_champion(owner).is_none() {
        if game.copy_card_to_hand(card, owner).is_some() {
            game.notify_hand_added(owner);
        }
        return;
    }
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: Some(lane),
            created_turn: game.turn,
            trigger: PlayerTrigger::CardBanished,
            lifetime: EffectLifetime::UntilTriggered,
            state: Box::new(owner),
            apply: rocket_banished,
        },
    );
}

// MoveChampionRepeatOnReveal 0x7d0f6d4 repeats even if the move is blocked.
fn rocket_banished(game: &mut Game<'_>, effect: &mut PendingEffect, target: Option<usize>) -> bool {
    if target != Some(effect.source) {
        return false;
    }
    let owner = *effect.state.as_any().downcast_ref::<usize>().unwrap();
    if let Some(champion) = game.player_champion(owner) {
        if game.cards[champion].revealed && game.cards[champion].lane.is_some() {
            game.move_card(champion, effect.lane.unwrap());
            game.ongoing();
            game.trigger_on_reveal(champion);
        }
        if game.copy_card_to_hand(effect.source, owner).is_some() {
            game.notify_hand_added(owner);
        }
    }
    true
}

// GetPowerBuffAmount 0x7d0ffe4 counts prior created non-Sentinel plays
// and adds the current skill itself if created. GivePower 0x7d0fc60
// buffs revealed Sentinels in locations, not those in hand or deck.
fn sentinel_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game.players[owner]
        .staged_history
        .iter()
        .filter(|play| {
            play.resolved && play.created && !matches!(play.def_id, "Sentinel" | "SentinelChampion")
        })
        .count() as i32
        + i32::from(game.cards[card].created);
    if count == 0 {
        return;
    }
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            game.cards[id].owner == owner
                && game.cards[id].revealed
                && matches!(game.cards[id].def.id, "Sentinel" | "SentinelChampion")
        })
        .collect();
    for target in targets {
        game.add_power(target, count, owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{Position, build_game};
    use crate::random::PythonRandom;
    use serde_json::json;

    fn position(mine: serde_json::Value, theirs: serde_json::Value) -> Position {
        serde_json::from_value(json!({
            "turn": 3, "energy": 3, "hand": [], "deck": [],
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
    fn thing_buffs_champion_and_discounts_distinct_costs() {
        let p = position(
            json!([{"id":"TheThingFirstStepsChampion"},{"id":"AntMan"},{"id":"Hawkeye"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(2);
        let mut game = build_game(&p, &mut rng).unwrap();
        let champion = game.lanes[0].cards[0];
        game.players[0].starting_deck_instances = Some(vec![champion]);
        let skill = game.create_card("Skill01TheThingFirstStepsChampion", 0);
        let power = game.cards[champion].power();
        thing_skill(&mut game, skill);
        assert_eq!(game.cards[champion].power(), power + 8);
        thing_discount(&mut game, skill);
        assert_eq!(game.card_cost(skill), 4);
    }

    #[test]
    fn thanos_destroys_front_row_at_threshold() {
        let p = position(
            json!([{"id":"ThanosChampion"},{"id":"Skill01ThanosChampion"}]),
            json!([{"id":"Hawkeye"}]),
        );
        let mut rng = PythonRandom::seed(3);
        let mut game = build_game(&p, &mut rng).unwrap();
        let champion = game.lanes[0].cards[0];
        let skill = game.lanes[0].cards[1];
        let enemy = game.lanes[0].cards[2];
        game.players[0].starting_deck_instances = Some(vec![champion]);
        let power = game.cards[champion].power();
        game.add_power(champion, 20 - power, 0);
        thanos_skill(&mut game, skill);
        assert!(game.cards[enemy].lane.is_none());
    }

    #[test]
    fn weapon_x_destroys_weaker_character_and_copies_skill() {
        let p = position(
            json!([{"id":"WeaponXWolverineChampion"},{"id":"Skill01WeaponXWolverineChampion"},{"id":"Hawkeye"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&p, &mut rng).unwrap();
        let champion = game.lanes[0].cards[0];
        let skill = game.lanes[0].cards[1];
        let ally = game.lanes[0].cards[2];
        game.players[0].starting_deck_instances = Some(vec![champion]);
        game.add_power(champion, 20, 0);
        weapon_x_skill(&mut game, skill);
        assert!(game.cards[ally].lane.is_none());
        let copy = game.players[0].hand[0];
        assert_eq!(game.card_cost(copy), 3);
    }

    #[test]
    fn magneto_doubles_champion_when_either_side_is_full() {
        let p = position(
            json!([{"id":"MagnetoChampion"},{"id":"Skill01MagnetoChampion"},{"id":"Hawkeye"},{"id":"AntMan"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(5);
        let mut game = build_game(&p, &mut rng).unwrap();
        let champion = game.lanes[0].cards[0];
        let skill = game.lanes[0].cards[1];
        game.players[0].starting_deck_instances = Some(vec![champion]);
        let power = game.cards[champion].power();
        magneto_skill(&mut game, skill);
        assert_eq!(game.cards[champion].power(), power * 2);
    }

    #[test]
    fn iron_man_schedules_one_end_turn_buff() {
        let p = position(
            json!([{"id":"IronManChampion"},{"id":"Skill01IronManChampion"},{"id":"Hawkeye"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(6);
        let mut game = build_game(&p, &mut rng).unwrap();
        let champion = game.lanes[0].cards[0];
        let skill = game.lanes[0].cards[1];
        game.players[0].starting_deck_instances = Some(vec![champion]);
        let power = game.cards[champion].power();
        iron_man_skill(&mut game, skill);
        game.end_turn_effects();
        assert_eq!(game.cards[champion].power(), power + 2);
        game.end_turn_effects();
        assert_eq!(game.cards[champion].power(), power + 2);
    }

    #[test]
    fn fantastic_chains_after_target_resolves() {
        let p = position(json!([{"id":"Skill01MrFantasticChampion"}]), json!([]));
        let mut rng = PythonRandom::seed(7);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[0];
        let first = game.create_card("IronMan", 0);
        game.add_hand(0, first, false);
        fantastic_skill(&mut game, skill);
        assert_eq!(game.card_cost(first), 4);
        game.players[0].hand.clear();
        let second = game.create_card("AntMan", 0);
        game.add_hand(0, second, false);
        let mut effect = game.players[0].pending_effects.pop().unwrap();
        assert!(!fantastic_repeat(&mut game, &mut effect, Some(first)));
        assert_eq!(game.card_cost(second), 0);
    }

    #[test]
    fn rocket_banish_dispatch_moves_and_copies() {
        let p = position(
            json!([{"id":"RocketRaccoonChampion"},{"id":"Skill01RocketRaccoonChampion"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(8);
        let mut game = build_game(&p, &mut rng).unwrap();
        let champion = game.lanes[0].cards[0];
        let skill = game.lanes[0].cards[1];
        game.players[0].starting_deck_instances = Some(vec![champion]);
        game.cards[skill].revealed = false;
        game.cards[skill].turn_played = game.turn;
        game.reveal_with_staging(
            skill,
            Some(crate::game::StagingContext { seat: 0, lane: 0 }),
        );
        assert!(game.players[0].banished.contains(&skill));
        assert_eq!(
            game.cards[game.players[0].hand[0]].def.id,
            "Skill01RocketRaccoonChampion"
        );
        assert!(game.players[0].pending_effects.is_empty());
    }

    #[test]
    fn sentinel_counts_created_skill_and_prior_created_plays() {
        let p = position(
            json!([{"id":"Sentinel"},{"id":"Skill01SentinelChampion"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(9);
        let mut game = build_game(&p, &mut rng).unwrap();
        let sentinel = game.lanes[0].cards[0];
        let skill = game.lanes[0].cards[1];
        game.cards[skill].created = true;
        let power = game.cards[sentinel].power();
        sentinel_skill(&mut game, skill);
        assert_eq!(game.cards[sentinel].power(), power + 1);
    }

    #[test]
    fn thor_shuffles_mjolnir_then_draws() {
        let position: Position = serde_json::from_value(json!({
            "turn": 3, "energy": 3, "hand": [], "deck": [],
            "locations": [
                {"id": null, "mine": [{"id":"Skill01ThorChampion"}], "theirs": []},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap();
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&position, &mut rng).unwrap();
        let skill = game.lanes[0].cards[0];
        thor_skill(&mut game, skill);
        assert!(game.players[0].deck.is_empty());
        assert_eq!(game.players[0].hand.len(), 1);
        assert_eq!(game.cards[game.players[0].hand[0]].def.id, "Mjolnir");
    }
}
