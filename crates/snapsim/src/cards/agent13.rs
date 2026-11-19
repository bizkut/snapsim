use super::{CardDef, deck_eligible_ids};
use crate::game::Game;

pub const AGENT_13: CardDef = CardDef {
    reveal: Some(reveal),
    ..CardDef::vanilla("Agent13")
};

// Reading (reference): the card is drawn uniformly from the implemented,
// deck-eligible cards, so this matches Python only with the full registry.
fn reveal(game: &mut Game<'_>, id: usize) {
    let owner = game.cards[id].owner;
    if game.players[owner].hand.len() < 7 {
        let ids = deck_eligible_ids();
        let choice = game.rng.borrow_mut().randbelow(ids.len());
        let card = game.create_card(ids[choice], owner);
        game.add_hand(owner, card, false);
    }
}
