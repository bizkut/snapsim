//! Shang-Chi and the Ten Rings, client 59.15.1.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        pre_game_start: Some(add_rings),
        reveal: Some(upgrade_rings),
        ..CardDef::vanilla("ShangChiMasterOfTheRings")
    },
    CardDef {
        start_in_hand: true,
        reveal: Some(rings_reveal),
        ..CardDef::vanilla("TenRings")
    },
    CardDef {
        end_turn: Some(rings_end_turn),
        end_turn_trigger: true,
        ..CardDef::vanilla("TenRingsUpgrade")
    },
];
pub(crate) static TOKENS: &[&str] = &["TenRings", "TenRingsUpgrade"];

// AddRingsToHand 0x7f39228 creates the StartInHand token in the deck and
// shuffles; DrawSelf 0x7cc1158 selects it during the opening draw.
fn add_rings(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let rings = game.create_card("TenRings", owner);
    game.cards[rings].created_by_card_id = Some(game.cards[card].def.id.to_owned());
    game.cards[rings].created_by_player = Some(owner);
    game.finish_creation_zone(rings, false, owner);
    game.push_deck(owner, rings);
    game.shuffle_deck(owner);
}

// UpgradeAndDrawRings 0x7f39378 snapshots hand, deck, locations and graveyard;
// predicate 0x7f39b3c selects the TenRings definition, not creator identity.
fn upgrade_rings(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.players[owner]
        .hand
        .iter()
        .chain(&game.players[owner].deck)
        .chain(
            game.lanes
                .iter()
                .flat_map(|lane| lane.cards.iter())
                .filter(|&&id| game.cards[id].owner == owner),
        )
        .chain(&game.players[owner].graveyard_order)
        .copied()
        .filter(|&id| game.cards[id].def.id == "TenRings")
        .collect();
    for target in targets {
        game.transform_card(target, "TenRingsUpgrade");
    }
}

// TenRingsBehavior::AddPower 0x7cd45d0 selects one other revealed ally.
fn rings_reveal(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| id != card && game.cards[id].owner == owner && game.cards[id].revealed)
        .collect();
    if !targets.is_empty() {
        let index = game.rng.borrow_mut().randbelow(targets.len());
        game.add_power(targets[index], 1, card);
    }
}

// TenRingsUpgradeBehavior::AddPower 0x7cd4b3c buffs each other revealed ally.
fn rings_end_turn(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| id != card && game.cards[id].owner == owner && game.cards[id].revealed)
        .collect();
    for target in targets {
        game.add_power(target, 1, card);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;
    #[test]
    fn rings_start_in_hand_and_upgrade_existing_entity() {
        let mut rng = PythonRandom::seed(2);
        let deck = [
            "ShangChiMasterOfTheRings",
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
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let rings = *game.players[0]
            .hand
            .iter()
            .find(|&&id| game.cards[id].def.id == "TenRings")
            .unwrap();
        let source = (0..game.cards.len())
            .find(|&id| {
                game.cards[id].owner == 0 && game.cards[id].def.id == "ShangChiMasterOfTheRings"
            })
            .unwrap();
        upgrade_rings(&mut game, source);
        assert_eq!(game.cards[rings].def.id, "TenRingsUpgrade");
        assert!(game.players[0].hand.contains(&rings));
    }
    #[test]
    fn rings_effects_buff_only_other_revealed_allies() {
        let mut rng = PythonRandom::seed(3);
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
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let ally = game.create_card("Hulk", 0);
        let rings = game.create_card("TenRings", 0);
        game.add_card(ally, 0, 0, true);
        game.add_card(rings, 0, 0, true);
        assert_eq!(game.cards[ally].permanent, 15);
        game.transform_card(rings, "TenRingsUpgrade");
        rings_end_turn(&mut game, rings);
        assert_eq!(game.cards[ally].permanent, 16);
    }
}
