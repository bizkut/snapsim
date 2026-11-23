//! Meta-deck cards, batch F2: rules from the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    // JJonahJamesonBehavior::RudeEmotes (0x7ed4778) is cosmetic, but is Ongoing.
    CardDef {
        ongoing_ability: true,
        ..CardDef::vanilla("JJonahJameson")
    },
    CardDef {
        reveal: Some(anti_venom),
        ..CardDef::vanilla("AntiVenom")
    },
    CardDef {
        on_activate: Some(magus),
        ..CardDef::vanilla("Magus")
    },
    CardDef {
        reveal: Some(selene),
        ..CardDef::vanilla("Selene")
    },
    CardDef {
        reveal: Some(ten_rings),
        start_in_hand: true,
        ..CardDef::vanilla("TenRings")
    },
];

pub(crate) static TOKENS: &[&str] = &["TenRings"];

// AntiVenomBehavior::SetTopCard (0x7e67390).
fn anti_venom(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if let Some(target) = game.top_card(owner) {
        game.set_power(target, 0, owner);
        game.set_cost(target, 0, owner);
    }
}

// MagusBehavior::CopyForUnspentEnergy (0x7eef1d4), predicate 0x7eef634.
fn magus(game: &mut Game<'_>, card: usize) {
    if game.cards[card].lane.is_none() {
        return;
    }
    let owner = game.cards[card].owner;
    let energy = game.players[owner].energy;
    if energy < 0 {
        return;
    }
    let target = game.players[owner]
        .deck
        .iter()
        .copied()
        .find(|&target| game.is_character(target) && game.printed_cost(target) == energy);
    if let Some(target) = target {
        game.copy_text(card, target, false);
    }
}

// SeleneBehavior::ReduceLowestPower (0x7f33b6c).
fn selene(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for seat in 0..2 {
        let hand = &game.players[seat].hand;
        let Some(lowest) = hand
            .iter()
            .filter(|&&id| game.is_character(id))
            .map(|&id| game.cards[id].power())
            .min()
        else {
            continue;
        };
        let candidates = || {
            hand.iter()
                .copied()
                .filter(|&id| game.is_character(id) && game.cards[id].power() == lowest)
        };
        let count = candidates().count();
        let pick = game.rng.borrow_mut().randbelow(count);
        let target = candidates().nth(pick).unwrap();
        game.add_power(target, -3, owner);
    }
}

// TenRingsBehavior::AddPower (0x7cd45d0): revealed other allies, one roll.
fn ten_rings(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut candidates = [0; 4];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if id != card && game.cards[id].owner == owner && game.cards[id].revealed {
            candidates[count] = id;
            count += 1;
        }
    }
    // Location::GetAllRevealedCards orders its result by resolution order.
    candidates[..count].sort_by_key(|&id| game.cards[id].latest_resolution_order);
    if count != 0 {
        let pick = game.rng.borrow_mut().randbelow(count);
        game.add_power(candidates[pick], 1, owner);
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
    fn anti_venom_sets_only_the_literal_top_card() {
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&position("AntiVenom"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let top = game.create_card("Hulk", 0);
        let second = game.create_card("Cyclops", 0);
        game.players[0].deck.extend([top, second]);
        anti_venom(&mut game, source);
        assert_eq!((game.cards[top].power(), game.cards[top].cost), (0, 0));
        assert_eq!(
            (game.cards[second].power(), game.cards[second].cost),
            (5, 3)
        );
    }

    #[test]
    fn magus_selects_first_printed_cost_without_revealing() {
        let mut rng = PythonRandom::seed(2);
        let mut game = build_game(&position("Magus"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let iron = game.create_card("IronMan", 0);
        let medusa = game.create_card("Medusa", 0);
        game.set_cost(iron, 2, 0);
        game.players[0].energy = 2;
        game.players[0].deck.extend([iron, medusa]);
        magus(&mut game, source);
        assert_eq!(
            game.cards[source].abilities().next().unwrap().def.id,
            "Medusa"
        );
        assert_eq!(game.cards[source].power(), 4);
    }

    #[test]
    fn selene_afflicts_a_lowest_card_in_each_nonempty_hand() {
        let mut rng = PythonRandom::seed(3);
        let mut game = build_game(&position("Selene"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let low = game.create_card("IronMan", 0);
        let high = game.create_card("Hulk", 0);
        let enemy = game.create_card("Cyclops", 1);
        game.players[0].hand.extend([high, low]);
        game.players[1].hand.push(enemy);
        selene(&mut game, source);
        assert_eq!(game.cards[low].power(), -3);
        assert_eq!(game.cards[high].power(), 14);
        assert_eq!(game.cards[enemy].power(), 2);
    }

    #[test]
    fn rings_affect_only_revealed_other_allies() {
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&position("TenRings"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let ally = game.create_card("Hulk", 0);
        let hidden = game.create_card("Cyclops", 0);
        let enemy = game.create_card("Hulk", 1);
        for id in [ally, hidden, enemy] {
            game.cards[id].lane = Some(0);
            game.lanes[0].cards.push(id);
        }
        game.cards[ally].revealed = true;
        game.cards[enemy].revealed = true;
        ten_rings(&mut game, source);
        assert_eq!(game.cards[ally].power(), 15);
        assert_eq!(game.cards[hidden].power(), 5);
        assert_eq!(game.cards[enemy].power(), 14);
    }

    #[test]
    fn jameson_is_an_ongoing_card() {
        let mut rng = PythonRandom::seed(5);
        let game = build_game(&position("JJonahJameson"), &mut rng).unwrap();
        assert!(game.cards[game.lanes[0].cards[0]].has_ongoing(false));
    }
}
