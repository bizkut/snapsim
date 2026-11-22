//! Meta-deck cards, batch F1: rules from the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    // MovableOngoingAbilityDef::ExecuteOngoingAction (0x7e0e6f4).
    CardDef {
        move_each_turn: true,
        ..CardDef::vanilla("Vision")
    },
    CardDef {
        late_ongoing: Some(captain_carter),
        ..CardDef::vanilla("CaptainCarter")
    },
    CardDef {
        end_turn: Some(isca),
        end_turn_trigger: true,
        ..CardDef::vanilla("Isca")
    },
    CardDef {
        end_turn: Some(drax),
        end_turn_trigger: true,
        ..CardDef::vanilla("DraxAvatarOfLife")
    },
    CardDef {
        reveal: Some(spider_man),
        ..CardDef::vanilla("SpiderMan")
    },
];

// SpiderManBehavior::MoveWithEnemy (0x7f4c418); destination predicates
// 0x7f4cc70, 0x7f4ccc8 and 0x7f4ccec.
fn spider_man(game: &mut Game<'_>, card: usize) {
    let Some(origin) = game.cards[card].lane else {
        return;
    };
    // Card::ResolveForPlay (0x7cb9d90) sets revealed before its trigger.
    game.cards[card].revealed = true;
    let owner = game.cards[card].owner;
    let enemies: Vec<_> = game.lanes[origin]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].owner != owner && game.cards[id].revealed)
        .collect();
    let enemy = if enemies.is_empty() {
        None
    } else {
        let pick = game.rng.borrow_mut().randbelow(enemies.len());
        Some(enemies[pick])
    };
    let enemy_can_move = |lane| enemy.is_some_and(|id| game.can_move_card(id, lane));
    let mut destinations: Vec<_> = (0..3)
        .filter(|&lane| game.can_move_card(card, lane) && enemy_can_move(lane))
        .collect();
    let any_enemy_destination = (0..3).any(enemy_can_move);
    if destinations.is_empty() {
        destinations = (0..3)
            .filter(|&lane| game.can_move_card(card, lane))
            .collect();
    }
    if destinations.is_empty() {
        return;
    }
    let pick = game.rng.borrow_mut().randbelow(destinations.len());
    let destination = destinations[pick];
    game.move_card(card, destination);
    if let Some(enemy) = enemy
        && any_enemy_destination
    {
        game.move_card(enemy, destination);
    }
}

// BuffAndMove (0x7e929bc), predicates 0x7e933d8/0x7e934d0 and
// WouldBeWinningAtLocationAfterMove (0x7e93048).
fn drax(game: &mut Game<'_>, card: usize) {
    let Some(origin) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let candidates: Vec<usize> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&enemy| {
            game.cards[enemy].owner != owner
                && game.cards[enemy].revealed
                && game.is_character(enemy)
                && game.cards[enemy].turn_played == game.turn
                && game.cards[enemy].power() > game.cards[card].power()
                && game.pending_plays[1 - owner]
                    .iter()
                    .chain(&game.played[1 - owner])
                    .any(|&(played, _)| played == enemy)
        })
        .collect();
    if candidates.is_empty() {
        return;
    }
    game.add_power(card, 3, owner);
    game.ongoing();
    let mut valid = Vec::new();
    for enemy in candidates {
        let Some(lane) = game.cards[enemy].lane else {
            continue;
        };
        if lane == origin {
            continue;
        }
        if !game.can_move_card(card, lane) {
            valid.push(lane);
            continue;
        }
        let mut rng = game.fork_rng();
        let mut simulation = game.fork(rng.as_mut());
        simulation.move_card(card, lane);
        if simulation.lane_winner(lane) == Some(owner) {
            valid.push(lane);
        }
    }
    if !valid.is_empty() {
        let selected = game.rng.borrow_mut().randbelow(valid.len());
        game.move_card(card, valid[selected]);
    }
}

// CaptainCarterBehavior::BuffCards (0x7e7d22c).
fn captain_carter(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let power = game.cards[card].power();
    for index in 0..game.lanes[lane].cards.len() {
        let target = game.lanes[lane].cards[index];
        if target != card
            && game.cards[target].owner == owner
            && game.cards[target].revealed
            && game.is_back_row(target)
        {
            game.add_ongoing_power(target, power, owner);
        }
    }
}

// IscaBehavior::DoublePower (0x7ed08d4).
fn isca(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if game.lane_winner(lane) == Some(1 - owner) {
        game.multiply_power(card, 2, owner);
    }
}

pub(crate) static TOKENS: &[&str] = &[];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{Position, build_game};
    use crate::random::PythonRandom;
    use serde_json::json;

    fn position(mine: serde_json::Value, theirs: serde_json::Value) -> Position {
        serde_json::from_value(json!({
            "turn": 3, "energy": 3, "hand": [], "deck": ["Hulk"],
            "locations": [
                {"id": "Ruins", "mine": mine, "theirs": theirs},
                {"id": "Ruins", "mine": [], "theirs": []},
                {"id": "Ruins", "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap()
    }

    #[test]
    fn carter_buffs_only_other_revealed_back_row_allies() {
        let p = position(
            json!([{"id":"CaptainCarter"}, {"id":"Wasp"},
            {"id":"MistyKnight"}, {"id":"Hulk"}]),
            json!([{"id":"Hulk"}]),
        );
        let mut rng = PythonRandom::seed(2);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        game.ongoing();
        assert_eq!(game.cards[ids[0]].power(), 2);
        assert_eq!(game.cards[ids[1]].power(), 1);
        assert_eq!(game.cards[ids[2]].power(), 5);
        assert_eq!(game.cards[ids[3]].power(), 16);
        let owner = game.cards[ids[0]].owner;
        game.add_power(ids[0], 3, owner);
        game.ongoing();
        assert_eq!(game.cards[ids[2]].power(), 8);
    }

    #[test]
    fn isca_doubles_only_when_strictly_losing() {
        let p = position(json!([{"id":"Isca"}]), json!([{"id":"Hulk", "power":8}]));
        let mut rng = PythonRandom::seed(3);
        let mut game = build_game(&p, &mut rng).unwrap();
        let card = game.lanes[0].cards[0];
        isca(&mut game, card);
        assert_eq!(game.cards[card].power(), 4);
        isca(&mut game, card);
        assert_eq!(game.cards[card].power(), 8);
        isca(&mut game, card);
        assert_eq!(game.cards[card].power(), 8);
    }

    #[test]
    fn spider_man_pulls_revealed_enemy_to_its_destination() {
        let p = position(json!([{"id":"SpiderMan"}]), json!([{"id":"Hulk"}]));
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&p, &mut rng).unwrap();
        let spider = game.lanes[0].cards[0];
        let enemy = game.lanes[0].cards[1];
        game.cards[spider].revealed = false;
        game.cards[spider].turn_played = game.turn;
        game.reveal(spider);
        assert_ne!(game.cards[spider].lane, Some(0));
        assert_eq!(game.cards[spider].lane, game.cards[enemy].lane);
    }

    #[test]
    fn drax_buffs_for_greater_play_and_can_move_to_win() {
        let mut p = position(json!([{"id":"DraxAvatarOfLife"}]), json!([]));
        p.locations[1] = serde_json::from_value(json!({
            "id":"Ruins", "mine":[], "theirs":[{"id":"Hulk", "power":5}]
        }))
        .unwrap();
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&p, &mut rng).unwrap();
        let card = game.lanes[0].cards[0];
        let enemy = game.lanes[1].cards[0];
        game.cards[enemy].turn_played = game.turn;
        game.played[1].push((enemy, 1));
        drax(&mut game, card);
        assert_eq!(game.cards[card].power(), 6);
        assert_eq!(game.cards[card].lane, Some(1));
    }
}
