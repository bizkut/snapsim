use super::{LocationDef, location};
use crate::game::Game;

pub const MUIR_ISLAND: LocationDef = LocationDef {
    on_turn_end: Some(muir_island),
    ..location("MuirIsland", 0, 0, 1, false)
};

fn muir_island(game: &mut Game<'_>, lane: usize) {
    for index in 0..game.lanes[lane].cards.len() {
        let card = game.lanes[lane].cards[index];
        game.add_power(card, 1, game.cards[card].owner);
    }
}
