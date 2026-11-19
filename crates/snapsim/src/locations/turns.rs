use super::{LocationDef, location};
use crate::game::Game;

pub const MUIR_ISLAND: LocationDef = LocationDef {
    on_turn_end: Some(muir_island),
    ..location("MuirIsland", 0, 0, 1, false)
};

fn muir_island(game: &mut Game<'_>, lane: usize) {
    for &card in &game.lanes[lane].cards {
        game.cards[card].permanent += 1;
    }
}
