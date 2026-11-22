//! Kahhori's 59.15.1 reveal rule.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[CardDef {
    reveal: Some(kahhori),
    ..CardDef::vanilla("Kahhori")
}];

// BuffForEachCard (0x7ed8498) rolls with replacement for each hand card,
// aggregates by target, then adds Power once per selected target. Both filters
// (0x7ed8c84, 0x7ed8cb4) exclude the source card by Card::Equals.
fn kahhori(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game.players[owner]
        .hand
        .iter()
        .filter(|&&id| id != card)
        .count();
    let mut targets = [0usize; 12];
    let mut amounts = [0i32; 12];
    let mut len = 0;
    for lane in &game.lanes {
        for &id in &lane.cards {
            if id != card && game.cards[id].owner == owner && game.cards[id].revealed {
                targets[len] = id;
                len += 1;
            }
        }
    }
    if len == 0 {
        return;
    }
    for _ in 0..count {
        let index = game.rng.borrow_mut().randbelow(len);
        amounts[index] += 1;
    }
    for index in 0..len {
        if amounts[index] != 0 {
            game.add_power(targets[index], amounts[index], owner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn buffs_only_other_revealed_allies_once_per_hand_card() {
        let mut rng = PythonRandom::seed(7);
        let deck = [
            "Wasp",
            "MistyKnight",
            "Hulk",
            "Cyclops",
            "Shocker",
            "TheThing",
            "Abomination",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut g = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = g.create_card("Kahhori", 0);
        let ally = g.create_card("Hulk", 0);
        let enemy = g.create_card("Hulk", 1);
        let hidden = g.create_card("Hulk", 0);
        for (id, revealed) in [(source, true), (ally, true), (enemy, true), (hidden, false)] {
            g.cards[id].lane = Some(0);
            g.cards[id].revealed = revealed;
            g.lanes[0].cards.push(id);
        }
        g.players[0].hand.clear();
        let first = g.create_card("Wasp", 0);
        let second = g.create_card("Wasp", 0);
        g.players[0].hand.extend([source, first, second]);
        let before = [ally, enemy, hidden, source].map(|id| g.cards[id].permanent);
        kahhori(&mut g, source);
        assert_eq!(g.cards[ally].permanent, before[0] + 2);
        assert_eq!(g.cards[enemy].permanent, before[1]);
        assert_eq!(g.cards[hidden].permanent, before[2]);
        assert_eq!(g.cards[source].permanent, before[3]);
    }
}
