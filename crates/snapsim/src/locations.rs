//! Location descriptors. A location is registered by adding one descriptor.

use crate::game::Game;
mod turns;

pub type LocationHook = fn(&mut Game<'_>, usize);
pub type CardHook = fn(&mut Game<'_>, usize, usize);
pub type MoveHook = fn(&mut Game<'_>, usize, usize, usize);
pub type PlayPredicate = fn(&Game<'_>, usize, usize, usize, usize) -> bool;
pub type EnergyHook = fn(&Game<'_>, usize, usize) -> i32;

pub struct LocationDef {
    pub id: &'static str,
    pub power: i32,
    pub reveal_energy: i32,
    pub reveal_repeats: u8,
    pub no_destroy: bool,
    pub on_reveal: Option<LocationHook>,
    pub on_turn_start: Option<LocationHook>,
    pub on_turn_end: Option<LocationHook>,
    pub ongoing: Option<LocationHook>,
    pub before_card_reveal: Option<CardHook>,
    pub after_card_play: Option<CardHook>,
    pub on_card_moved: Option<MoveHook>,
    pub on_card_added: Option<CardHook>,
    pub on_event: Option<fn(&mut Game<'_>, usize, crate::game::Event)>,
    pub allows_play: Option<PlayPredicate>,
    pub allows_play_elsewhere: Option<PlayPredicate>,
    pub allows_addition: Option<PlayPredicate>,
    pub energy_bonus: Option<EnergyHook>,
    pub minimum_turns: u8,
    pub initial_state: Option<fn() -> Box<dyn std::any::Any>>,
}

const fn location(
    id: &'static str,
    power: i32,
    reveal_energy: i32,
    reveal_repeats: u8,
    no_destroy: bool,
) -> LocationDef {
    LocationDef {
        id,
        power,
        reveal_energy,
        reveal_repeats,
        no_destroy,
        on_reveal: None,
        on_turn_start: None,
        on_turn_end: None,
        ongoing: None,
        before_card_reveal: None,
        after_card_play: None,
        on_card_moved: None,
        on_card_added: None,
        on_event: None,
        allows_play: None,
        allows_play_elsewhere: None,
        allows_addition: None,
        energy_bonus: None,
        minimum_turns: 0,
        initial_state: None,
    }
}

pub static REGISTRY: &[LocationDef] = &[
    location("Xandar", 1, 0, 1, false),
    location("NegativeZone", -3, 0, 1, false),
    location("Necrosha", -2, 0, 1, false),
    location("Nidavellir", 5, 0, 1, false),
    location("SewerSystem", -1, 0, 1, false),
    location("Ruins", 0, 0, 1, false),
    location("Wakanda", 0, 0, 1, true),
    location("KamarTaj", 0, 0, 2, false),
    location("TinkerersWorkshop", 0, 1, 1, false),
    location("ProjectPegasus", 0, 5, 1, false),
    turns::MUIR_ISLAND,
];

pub fn get(id: &str) -> Option<&'static LocationDef> {
    REGISTRY.iter().find(|entry| entry.id == id)
}
