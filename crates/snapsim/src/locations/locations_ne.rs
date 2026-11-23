//! Location lifecycle rules traced from the 59.15.1 client.
use super::{LocationDef, location};
use crate::game::Game;

pub const WORLDSHIP: LocationDef = LocationDef {
    on_reveal: Some(worldship),
    ..location("Worldship", 0, 0, 1, false)
};

pub const X_MANSION: LocationDef = LocationDef {
    on_turn_end: Some(x_mansion),
    ..location("XMansion", 0, 0, 1, false)
};

pub const MOUNT_VESUVIUS: LocationDef = LocationDef {
    prevents_retreat: Some(vesuvius),
    ..location("MountVesuvius", 0, 0, 1, false)
};

// MountVesuviusBehavior::EnablePreventRetreating (0x7d6863c).
fn vesuvius(game: &Game<'_>, end_turn: bool) -> bool {
    game.turn > 5 || (game.turn == 5 && end_turn)
}

// WorldshipBehavior::DestroyOtherLocations (0x7d99ac4) calls the same
// Game::DestroyLocation (0x7daeaf4) as Galactus for each other location.
fn worldship(game: &mut Game<'_>, lane: usize) {
    for other in 0..3 {
        if other != lane {
            game.destroy_location(other);
        }
    }
}

// XMansionBehavior::AddRandomCards (0x7d9a2f0) uses resolution order,
// chooses once per player excluding Morph and Iron Lad, creates then stages.
// As elsewhere, random definitions are limited to implemented cards.
fn x_mansion(game: &mut Game<'_>, lane: usize) {
    if game.turn != 3 {
        return;
    }
    let first = game.reveal_priority.unwrap_or_else(|| {
        let winner = game.winner();
        if winner == 2 {
            game.rng.borrow_mut().randbelow(2)
        } else {
            winner as usize
        }
    });
    let start = game.post_resolve_plays.len();
    for owner in [first, 1 - first] {
        if let Some(id) = game.random_card(|row| {
            row.card_type == "Character" && !matches!(row.carddefid.as_str(), "Morph" | "IronLad")
        }) {
            let card = game.create_card(id, owner);
            game.stage_effect_play(card, lane);
        }
    }
    game.resolve_effect_plays(start);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn worldship_removes_other_lanes_and_force_destroys_their_cards() {
        let mut rng = PythonRandom::seed(9);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Wakanda", "Worldship", "Ruins"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.lanes[0].revealed = true;
        let protected = game.create_card("Wasp", 0);
        let hidden = game.create_card("Hulk", 1);
        let survivor = game.create_card("Wasp", 0);
        game.add_card(protected, 0, 0, false);
        game.add_card(hidden, 2, 1, false);
        game.cards[hidden].revealed = false;
        game.add_card(survivor, 1, 0, true);
        game.replace_location_def(1, crate::locations::get("Worldship").unwrap(), None);
        assert!(game.lanes[0].destroyed && game.lanes[2].destroyed);
        assert!(!game.lanes[1].destroyed);
        assert!(game.lanes[0].cards.is_empty() && game.lanes[2].cards.is_empty());
        assert!(game.cards[protected].revealed && game.cards[hidden].revealed);
        assert!(
            game.players[0]
                .graveyard
                .iter()
                .any(|&(id, _)| id == protected)
        );
        assert!(
            game.players[1]
                .graveyard
                .iter()
                .any(|&(id, _)| id == hidden)
        );
        assert_eq!(game.powers()[0], [0, 0]);
        assert_eq!(game.powers()[2], [0, 0]);
        assert_eq!(game.lane_winner(0), None);
        let hand = game.create_card("Wasp", 0);
        assert!(!game.can_play(hand, 0, 0, 10, 0));
        assert!(!game.can_add(hand, 2, 0));
        assert!(!game.can_move_card(survivor, 0));
    }

    #[test]
    fn x_mansion_stages_created_cards_even_with_full_hands() {
        let mut rng = PythonRandom::seed(11);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "XMansion", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        for seat in 0..2 {
            while game.players[seat].hand.len() < 7 {
                let filler = game.create_card("Wasp", seat);
                game.players[seat].hand.push(filler);
            }
        }
        game.reveal_priority = Some(1);
        game.turn = 2;
        x_mansion(&mut game, 1);
        assert!(game.players.iter().all(|p| p.staged_history.is_empty()));
        game.turn = 3;
        x_mansion(&mut game, 1);
        for player in &game.players {
            assert!(player.staged_history.iter().any(|play| {
                play.turn == 3 && play.created && !matches!(play.def_id, "Morph" | "IronLad")
            }));
        }
    }

    #[test]
    fn vesuvius_blocks_retreat_after_turn_five_and_during_its_end_phase() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "MountVesuvius", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.lanes[1].def = &MOUNT_VESUVIUS;
        game.lanes[1].revealed = false;
        game.turn = 5;
        assert!(game.can_retreat(false));
        game.lanes[1].revealed = true;
        assert!(game.can_retreat(false));
        assert!(!game.can_retreat(true));
        game.turn = 6;
        assert!(!game.can_retreat(false));
        game.lanes[1].revealed = false;
        assert!(game.can_retreat(false));
    }
}
