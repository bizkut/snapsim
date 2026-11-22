//! Client 59.15.1 card rules, batch J2.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[CardDef {
    reveal: Some(silver_surfer),
    ..CardDef::vanilla("SilverSurfer")
}];

// TriggerAbilityDef::ExecuteTriggerAction (0x7e0f6e4), with
// RevealedPlayerCardsSelector excluding self and ByDefCost (0x7d29ad8).
fn silver_surfer(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            id != card
                && game.cards[id].owner == owner
                && game.cards[id].revealed
                && game.printed_cost(id) == 3
        })
        .collect();
    for target in targets {
        game.add_power(target, 2, owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    #[test]
    fn surfer_buffs_only_other_revealed_allied_printed_three_costs() {
        let mut rng = PythonRandom::seed(1);
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
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        let mut place = |name, owner, lane, revealed| {
            let id = game.create_card(name, owner);
            game.cards[id].lane = Some(lane);
            game.cards[id].revealed = revealed;
            game.lanes[lane].cards.push(id);
            id
        };
        let surfer = place("SilverSurfer", 0, 0, true);
        let ally = place("MrFantastic", 0, 1, true);
        let hidden = place("MrFantastic", 0, 2, false);
        let enemy = place("MrFantastic", 1, 0, true);
        let different = place("Hulk", 0, 0, true);
        game.cards[ally].cost = 5;
        game.cards[different].cost = 3;
        let before: Vec<_> = [surfer, ally, hidden, enemy, different]
            .map(|id| game.cards[id].power())
            .into();
        silver_surfer(&mut game, surfer);
        assert_eq!(game.cards[ally].power(), before[1] + 2);
        for (index, id) in [surfer, hidden, enemy, different].into_iter().enumerate() {
            assert_eq!(game.cards[id].power(), before[[0, 2, 3, 4][index]]);
        }
    }
}
