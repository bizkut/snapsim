//! Thanos Fractured Frontier and its Infinity Shots, client 59.15.1.
use super::CardDef;
use crate::game::{Game, MoveGrant};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        pre_game_start: Some(start),
        reveal: Some(fire),
        ..CardDef::vanilla("ThanosFracturedFrontier")
    },
    CardDef {
        reveal: Some(space),
        ..CardDef::vanilla("SpaceShot")
    },
    CardDef {
        reveal: Some(reality),
        ..CardDef::vanilla("RealityShot")
    },
    CardDef {
        reveal: Some(time),
        ..CardDef::vanilla("TimeShot")
    },
    CardDef {
        reveal: Some(mind),
        ..CardDef::vanilla("MindShot")
    },
    CardDef {
        reveal: Some(power),
        ..CardDef::vanilla("PowerShot")
    },
    CardDef {
        reveal: Some(soul),
        ..CardDef::vanilla("SoulShot")
    },
];
pub(crate) static TOKENS: &[&str] = &[
    "SpaceShot",
    "RealityShot",
    "TimeShot",
    "MindShot",
    "PowerShot",
    "SoulShot",
];

fn start(game: &mut Game<'_>, card: usize) {
    crate::engine_ext::quickdraw::install(game, card);
}
fn fire(game: &mut Game<'_>, card: usize) {
    game.fire_quickdraw_shots(card);
}

// SpaceShotBehavior::AddPowerAndAbility 0x7f4338c.
fn space(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.add_power(card, 1, owner);
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            game.cards[id].owner == owner
                && (game.cards[id].revealed || id == card)
                && game.cards[id].def.id == "ThanosFracturedFrontier"
        })
        .collect();
    let turn = game.turn.saturating_add(1);
    for target in targets {
        game.grant_move(MoveGrant {
            seat: owner,
            source: card,
            owner_def_id: None,
            card: Some(target),
            lane: None,
            destination: None,
            start_turn: turn,
            end_turn: turn,
            remaining: 1,
            unlimited: false,
            condition: Some(space_move_active),
            on_move: None,
        });
    }
}

fn space_move_active(game: &Game<'_>, source: usize, target: usize) -> bool {
    game.cards[source].lane.is_some()
        && game.cards[target].lane.is_some()
        && game.cards[source].owner == game.cards[target].owner
}

// RealityShotBehavior::MakeCopy 0x7f23170 selects lowest cost, rolling ties.
fn reality(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if !game.has_space(lane, owner, 0) {
        return;
    }
    let mut targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| id != card && game.cards[id].owner == owner && game.cards[id].revealed)
        .collect();
    let Some(cost) = targets.iter().map(|&id| game.card_cost(id)).min() else {
        return;
    };
    targets.retain(|&id| game.card_cost(id) == cost);
    let index = game.rng.borrow_mut().randbelow(targets.len());
    if game
        .copy_card_to_location(targets[index], lane, owner)
        .is_some()
    {
        game.resolve_deferred_cards();
    }
}

// TimeShotBehavior::ReduceCost 0x7cde0c8.
fn time(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for target in game.players[owner].hand.clone() {
        game.add_cost(target, -1, owner);
    }
}
// MindShotBehavior::DrawCards 0x7ef8fb0.
fn mind(game: &mut Game<'_>, card: usize) {
    game.draw(game.cards[card].owner);
}
// PowerShot's AddPowerTriggerAction in the client definition.
fn power(game: &mut Game<'_>, card: usize) {
    game.add_power(card, 3, game.cards[card].owner);
}
// SoulShotBehavior::DestroyCard 0x7f426b4; lowest-power tie rolls.
fn soul(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].owner != owner && game.cards[id].revealed)
        .collect();
    let Some(lowest) = targets.iter().map(|&id| game.cards[id].power()).min() else {
        return;
    };
    targets.retain(|&id| game.cards[id].power() == lowest);
    let index = game.rng.borrow_mut().randbelow(targets.len());
    game.destroy_card(targets[index]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;
    fn test_game(rng: &mut PythonRandom) -> Game<'_> {
        let deck = [
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
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "KamarTaj"].map(str::to_owned);
        Game::new([&deck, &deck], &lanes, rng).unwrap()
    }

    #[test]
    fn space_adds_power_and_next_turn_move() {
        let mut rng = PythonRandom::seed(1);
        let mut game = test_game(&mut rng);
        let c = game.create_card("ThanosFracturedFrontier", 0);
        game.add_card(c, 0, 0, false);
        let before = game.cards[c].permanent;
        space(&mut game, c);
        assert_eq!(game.cards[c].permanent, before + 1);
        assert!(
            game.players[0]
                .move_grants
                .iter()
                .any(|g| g.card == Some(c) && g.start_turn == game.turn + 1)
        );
    }
    #[test]
    fn space_move_requires_source_remain_in_play() {
        let mut rng = PythonRandom::seed(7);
        let mut game = test_game(&mut rng);
        let source = game.create_card("ThanosFracturedFrontier", 0);
        let target = game.create_card("ThanosFracturedFrontier", 0);
        game.add_card(source, 0, 0, false);
        game.add_card(target, 0, 0, false);
        space(&mut game, source);
        game.turn += 1;
        assert!(game.can_player_move(target, 1, 0));
        game.destroy_card(source);
        assert!(!game.can_player_move(target, 1, 0));
    }
    #[test]
    fn reality_copies_lowest_cost_other_ally() {
        let mut rng = PythonRandom::seed(2);
        let mut game = test_game(&mut rng);
        let c = game.create_card("ThanosFracturedFrontier", 0);
        let ally = game.create_card("Wasp", 0);
        game.add_card(c, 0, 0, false);
        game.add_card(ally, 0, 0, false);
        game.cards[ally].revealed = true;
        reality(&mut game, c);
        assert_eq!(
            game.lanes[0]
                .cards
                .iter()
                .filter(|&&id| game.cards[id].def.id == "Wasp")
                .count(),
            2
        );
    }
    #[test]
    fn time_reduces_all_hand_costs() {
        let mut rng = PythonRandom::seed(3);
        let mut game = test_game(&mut rng);
        let c = game.create_card("ThanosFracturedFrontier", 0);
        let hand = game.players[0].hand.clone();
        let before: Vec<_> = hand.iter().map(|&id| game.cards[id].cost).collect();
        time(&mut game, c);
        for (id, old) in hand.into_iter().zip(before) {
            assert_eq!(game.cards[id].cost, old - 1);
        }
    }
    #[test]
    fn mind_draws_one() {
        let mut rng = PythonRandom::seed(4);
        let mut game = test_game(&mut rng);
        let c = game.create_card("ThanosFracturedFrontier", 0);
        let before = game.players[0].hand.len();
        mind(&mut game, c);
        assert_eq!(game.players[0].hand.len(), before + 1);
    }
    #[test]
    fn power_adds_three() {
        let mut rng = PythonRandom::seed(5);
        let mut game = test_game(&mut rng);
        let c = game.create_card("ThanosFracturedFrontier", 0);
        let before = game.cards[c].permanent;
        power(&mut game, c);
        assert_eq!(game.cards[c].permanent, before + 3);
    }
    #[test]
    fn soul_destroys_lowest_enemy() {
        let mut rng = PythonRandom::seed(6);
        let mut game = test_game(&mut rng);
        let c = game.create_card("ThanosFracturedFrontier", 0);
        let weak = game.create_card("Wasp", 1);
        let strong = game.create_card("Hulk", 1);
        game.add_card(c, 0, 0, false);
        game.add_card(weak, 0, 1, false);
        game.add_card(strong, 0, 1, false);
        game.cards[weak].revealed = true;
        game.cards[strong].revealed = true;
        soul(&mut game, c);
        assert!(game.cards[weak].lane.is_none());
        assert_eq!(game.cards[strong].lane, Some(0));
    }
}
