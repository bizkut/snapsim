//! The frozen Python reference's version of each existing card whose rule was deliberately
//! changed to match the 59.15.1 client. Only parity gates use these (`cards::use_reference_rules`
//! or `SNAPSIM_REFERENCE_RULES=1`), so the gates keep checking all other content exactly while
//! live play uses the client rule. Each entry names the client rule it differs from.
use super::CardDef;
use crate::game::Game;

// Hawkeye/Jessica's pending watches now survive copied-text replacement only
// when externally originated (CopyText 0x7cbadb4; predicate 0x7cbe110).
// The frozen reference has no merge-and-copy or text-replacement cards, so
// ordinary reference matches need no alternate descriptors for this interaction.

pub(crate) static CARDS: &[CardDef] = &[CardDef {
    reveal: Some(wolfsbane),
    ..CardDef::vanilla("Wolfsbane")
}];

/// Reference rule: counts only revealed allies. The client (recruit_b.rs `wolfsbane`) also
/// counts face-down ones.
fn wolfsbane(g: &mut Game, id: usize) {
    let card = &g.cards[id];
    let count = g.lanes[card.lane.unwrap()]
        .cards
        .iter()
        .filter(|&&other| {
            other != id && g.cards[other].revealed && g.cards[other].owner == card.owner
        })
        .count();
    g.add_power(id, 2 * count as i32, g.cards[id].owner);
}

/// The frozen Zenn-La rule uses effective cost and does not trigger On Reveal.
pub(crate) fn zenn_la(game: &mut Game<'_>, lane: usize) {
    for seat in 0..2 {
        let slot = game.players[seat]
            .deck
            .iter()
            .position(|&id| game.card_cost(id) == 3);
        if let Some(slot) = slot {
            let id = game.players[seat].deck[slot];
            if game.add_card(id, lane, seat, false) {
                game.players[seat].deck.remove(slot);
            }
        }
    }
}
