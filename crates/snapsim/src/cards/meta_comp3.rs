//! Start-of-game text copying (client 59.15.1).
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        pre_game_start: Some(kang),
        ..CardDef::vanilla("Kang")
    },
    CardDef {
        pre_game_start: Some(hulkling),
        ..CardDef::vanilla("Hulkling")
    },
];

// KangBehavior::ReplaceKang 0x7ed8ce4, predicate 0x7eda270. The client
// samples four distinct definitions, excluding its static list; sampling is
// restricted to implemented deck cards, an approximation of the client pool.
fn kang(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(index) = game.players[owner].deck.iter().position(|&id| id == card) else {
        return;
    };
    game.take_deck(owner, index);
    game.push_set_aside(owner, card);
    let mut selected = Vec::with_capacity(4);
    for _ in 0..4 {
        let Some(id) = game.random_card(|row| {
            row.cost >= 3
                && !selected.contains(&row.carddefid.as_str())
                && !matches!(
                    row.carddefid.as_str(),
                    "Arishem"
                        | "Thanos"
                        | "Kang"
                        | "Agamotto"
                        | "AgathaHarkness"
                        | "Dormammu"
                        | "HighEvolutionary"
                        | "Hulkling"
                        | "SamWilson"
                        | "Uatu"
                        | "MistyKnight"
                        | "Shocker"
                        | "Cyclops"
                        | "TheThing"
                        | "Abomination"
                        | "Hulk"
                )
                && super::no_text::has_no_text(&row.carddefid) == Some(false)
        }) else {
            break;
        };
        selected.push(id);
    }
    for id in selected {
        let source = game.create_card(id, owner);
        game.push_set_aside(owner, source);
        let replacement = game.create_card("Kang", owner);
        game.copy_text(replacement, source, false);
        game.push_deck(owner, replacement);
    }
    game.shuffle_deck(owner);
}

// HulklingBehavior::CopyAtGameStart 0x7ec7510, predicate 0x7ec7d94.
// The draw-time attached trigger is visual only; the copied text is persistent.
// The random pool is restricted to implemented cards.
fn hulkling(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if let Some(id) = game.random_card(|row| {
        row.cost == 6
            && !matches!(
                row.carddefid.as_str(),
                "AgathaHarkness" | "Thanos" | "Hulkling" | "SinisterSix" | "Hulk"
            )
    }) {
        let source = game.create_card(id, owner);
        game.push_set_aside(owner, source);
        game.copy_text(card, source, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;
    use std::collections::HashSet;

    fn game(rng: &mut PythonRandom) -> Game<'_> {
        let deck = [
            "Wasp",
            "Hulk",
            "IronMan",
            "Hawkeye",
            "Colossus",
            "AntMan",
            "Medusa",
            "Sentinel",
            "Cyclops",
            "TheThing",
            "Shocker",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "KamarTaj"].map(str::to_owned);
        Game::new([&deck, &deck], &lanes, rng).unwrap()
    }

    #[test]
    fn kang_replaces_original_with_four_distinct_copied_texts() {
        let mut rng = PythonRandom::seed(31);
        let mut game = game(&mut rng);
        let original = game.create_card("Kang", 0);
        game.push_deck(0, original);
        let before = game.players[0].deck.len();
        kang(&mut game, original);
        assert_eq!(game.players[0].deck.len(), before + 3);
        assert!(game.players[0].set_aside.contains(&original));
        let copies: Vec<_> = game.players[0]
            .deck
            .iter()
            .copied()
            .filter(|&id| game.cards[id].def.id == "Kang")
            .collect();
        assert_eq!(copies.len(), 4);
        let texts: HashSet<_> = copies
            .iter()
            .map(|&id| game.cards[id].default_ability().id)
            .collect();
        assert_eq!(texts.len(), 4);
        for id in copies {
            assert_eq!(game.cards[id].cost, game.cards[original].cost);
            assert_eq!(game.cards[id].permanent, game.cards[original].permanent);
            assert!(
                crate::data::cards().iter().any(|row| row.carddefid
                    == game.cards[id].default_ability().id
                    && row.cost >= 3)
            );
        }
    }

    #[test]
    fn hulkling_copies_six_cost_text_without_changing_identity_or_stats() {
        let mut rng = PythonRandom::seed(32);
        let mut game = game(&mut rng);
        let card = game.create_card("Hulkling", 0);
        let cost = game.cards[card].cost;
        let power = game.cards[card].permanent;
        hulkling(&mut game, card);
        let copied = game.cards[card].default_ability().id;
        assert_ne!(copied, "Hulkling");
        assert!(
            crate::data::cards()
                .iter()
                .any(|row| row.carddefid == copied && row.cost == 6)
        );
        assert_eq!(game.cards[card].def.id, "Hulkling");
        assert_eq!(game.cards[card].cost, cost);
        assert_eq!(game.cards[card].permanent, power);
    }
}
