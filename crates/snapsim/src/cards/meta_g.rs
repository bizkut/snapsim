//! Meta-deck cards, batch G: rules from the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(mind_stone),
        ..CardDef::vanilla("MindStone")
    },
    CardDef {
        reveal: Some(polaris),
        ..CardDef::vanilla("Polaris")
    },
    CardDef {
        reveal: Some(soul_stone),
        ongoing_ability: true,
        ..CardDef::vanilla("SoulStone")
    },
    CardDef {
        reveal: Some(space_stone),
        ongoing_ability: true,
        ..CardDef::vanilla("SpaceStone")
    },
    CardDef::vanilla("Symbiote"),
];

pub(crate) static TOKENS: &[&str] = &["MindStone", "SoulStone", "SpaceStone", "Symbiote"];

fn draw_selected(game: &mut Game<'_>, owner: usize, card: usize) {
    if game.players[owner].hand.len() >= 7 {
        return;
    }
    if let Some(slot) = game.players[owner].deck.iter().position(|&id| id == card) {
        game.take_deck(owner, slot);
        game.add_hand(owner, card, true);
    }
}

// MindStoneBehavior::DrawTwoStones (0x7ef92f4) shuffles and draws two.
// Its predicate (0x7ef9a58) tests membership in Thanos.Card_AllTokens,
// whose client definition contains exactly these six Stone IDs.
fn mind_stone(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut stones: Vec<_> = game.players[owner]
        .deck
        .iter()
        .copied()
        .filter(|&id| {
            matches!(
                game.cards[id].def.id,
                "MindStone"
                    | "SoulStone"
                    | "SpaceStone"
                    | "PowerStone"
                    | "RealityStone"
                    | "TimeStone"
            )
        })
        .collect();
    for i in (1..stones.len()).rev() {
        let j = game.rng.borrow_mut().randbelow(i + 1);
        stones.swap(i, j);
    }
    for target in stones.into_iter().take(2) {
        draw_selected(game, owner, target);
    }
}

// PolarisBehavior::MoveRandomCard (0x7f15f74), cost predicate 0x7f164f0.
fn polaris(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let candidates: Vec<_> = game
        .lanes
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != lane)
        .flat_map(|(_, location)| location.cards.iter().copied())
        .filter(|&id| {
            game.cards[id].owner != owner
                && game.cards[id].revealed
                && matches!(game.card_cost(id), 1 | 2)
        })
        .collect();
    if !candidates.is_empty() {
        let pick = game.rng.borrow_mut().randbelow(candidates.len());
        game.move_card(candidates[pick], lane);
    }
}

// SoulStoneBehavior::DrawCard (0x7f42a5c).
fn soul_stone(game: &mut Game<'_>, card: usize) {
    game.draw(game.cards[card].owner);
}

// SpaceStoneBehavior::DrawThanos (0x7f44084): choose one matching deck card.
fn space_stone(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let candidates: Vec<_> = game.players[owner]
        .deck
        .iter()
        .copied()
        .filter(|&id| matches!(game.cards[id].def.id, "Thanos" | "ThanosChampion"))
        .collect();
    if !candidates.is_empty() {
        let pick = game.rng.borrow_mut().randbelow(candidates.len());
        draw_selected(game, owner, candidates[pick]);
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
    fn mind_draws_only_stones() {
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&position("MindStone"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let a = game.create_card("SpaceStone", 0);
        let b = game.create_card("SoulStone", 0);
        let other = game.create_card("Hulk", 0);
        game.players[0].deck.extend([other, a, b]);
        mind_stone(&mut game, source);
        assert_eq!(game.players[0].deck, [other]);
        assert!(game.players[0].hand.contains(&a));
        assert!(game.players[0].hand.contains(&b));
    }

    #[test]
    fn space_draws_only_thanos() {
        let mut rng = PythonRandom::seed(2);
        let mut game = build_game(&position("SpaceStone"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let other = game.create_card("Hulk", 0);
        let thanos = game.cards.len();
        game.cards
            .push(crate::position::make_card("Thanos", 0).unwrap());
        game.players[0].deck.extend([other, thanos]);
        space_stone(&mut game, source);
        assert_eq!(game.players[0].deck, [other]);
        assert_eq!(game.players[0].hand, [thanos]);
    }

    #[test]
    fn polaris_targets_revealed_enemy_with_current_cost() {
        let mut rng = PythonRandom::seed(3);
        let mut game = build_game(&position("Polaris"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let enemy = game.create_card("Hulk", 1);
        game.cards[enemy].cost = 2;
        game.cards[enemy].lane = Some(1);
        game.cards[enemy].revealed = true;
        game.lanes[1].cards.push(enemy);
        polaris(&mut game, source);
        assert_eq!(game.cards[enemy].lane, Some(0));
    }

    #[test]
    fn soul_protects_thanos_in_hand_and_board_only_while_active() {
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&position("SoulStone"), &mut rng).unwrap();
        let stone = game.lanes[0].cards[0];
        let thanos = game.cards.len();
        game.cards
            .push(crate::position::make_card("Thanos", 0).unwrap());
        game.players[0].hand.push(thanos);
        game.discard_card(0, thanos);
        assert!(game.players[0].hand.contains(&thanos));
        game.players[0].hand.clear();
        game.add_card(thanos, 1, 0, true);
        assert!(!game.destroy_card(thanos));
        game.cards[stone].abilities_removed = true;
        assert!(game.destroy_card(thanos));
    }

    #[test]
    fn space_overrides_restrictions_but_not_energy_or_capacity() {
        let mut rng = PythonRandom::seed(5);
        let mut game = build_game(&position("SpaceStone"), &mut rng).unwrap();
        let thanos = game.cards.len();
        game.cards
            .push(crate::position::make_card("Thanos", 0).unwrap());
        game.players[0].hand.push(thanos);
        game.lanes[1].def = crate::locations::get("Flooded").unwrap();
        game.lanes[1].revealed = true;
        assert!(game.can_play(thanos, 1, 0, 10, 0));
        assert!(!game.can_play(thanos, 1, 0, 0, 0));
        assert!(!game.can_play(thanos, 1, 0, 10, 4));
        let stone = game.lanes[0].cards[0];
        game.cards[stone].abilities_removed = true;
        assert!(!game.can_play(thanos, 1, 0, 10, 0));
    }
}
