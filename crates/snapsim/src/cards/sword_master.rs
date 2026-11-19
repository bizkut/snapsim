use super::CardDef;
use crate::game::Game;

pub const SWORD_MASTER: CardDef = CardDef {
    reveal: Some(reveal),
    ..CardDef::vanilla("SwordMaster")
};

fn reveal(game: &mut Game<'_>, id: usize) {
    let owner = game.cards[id].owner;
    let eligible: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&card| game.cards[card].cost % 2 != 0)
        .collect();
    if !eligible.is_empty() {
        let chosen = game.rng.borrow_mut().randbelow(eligible.len());
        game.discard_card(owner, eligible[chosen]);
    }
}
