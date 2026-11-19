//! Location descriptors. A location is registered by adding one descriptor.

use crate::game::Game;
mod locations_b;
mod locations_b_extra;
mod recruit_a;
mod turns;

pub type LocationHook = fn(&mut Game<'_>, usize);
pub type CardHook = fn(&mut Game<'_>, usize, usize);
pub type MoveHook = fn(&mut Game<'_>, usize, usize, usize);
pub type PlayPredicate = fn(&Game<'_>, usize, usize, usize, usize) -> bool;
pub type EnergyHook = fn(&Game<'_>, usize, usize) -> i32;
pub type CostHook = fn(&Game<'_>, usize, usize, i32) -> i32;

pub struct LocationDef {
    pub id: &'static str,
    pub power: i32,
    pub reveal_energy: i32,
    pub reveal_repeats: u8,
    pub ongoing_repeats: u8,
    pub no_destroy: bool,
    pub delays_reveals: bool,
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
    pub modify_cost: Option<CostHook>,
    pub minimum_turns: u8,
    pub initial_state: Option<fn() -> Box<dyn crate::game::CardState>>,
}

pub(crate) const fn location(
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
        ongoing_repeats: 1,
        no_destroy,
        delays_reveals: false,
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
        modify_cost: None,
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
    LocationDef {
        minimum_turns: 7,
        ..location("Limbo", 0, 0, 1, false)
    },
    LocationDef {
        delays_reveals: true,
        ..location("Lemuria", 0, 0, 1, false)
    },
    turns::MUIR_ISLAND,
    locations_b::MOJOWORLD,
    locations_b::MONSTER_METROPOLIS,
    locations_b::THRONE_ROOM,
    locations_b::MONSTER_ISLAND,
    locations_b::SHADOWLAND,
    locations_b::SAVAGE_LAND,
    locations_b::NOVA_ROMA,
    locations_b::OLYMPIA,
    locations_b::WAKANDAN_EMBASSY,
    locations_b::MOSH_PIT,
    locations_b::MURDERWORLD,
    locations_b::ROCKET_PAD,
    locations_b::STARK_INDUSTRIES,
    locations_b::STARK_TOWER,
    locations_b::PROSCENIUM,
    locations_b::SHURIS_LAB,
    locations_b::ORCHIS_FORGE,
    locations_b::REEDS_LAB,
    locations_b::VAULT,
    locations_b::BIG_HOUSE,
    recruit_a::ASGARD,
    recruit_a::ATLANTIS,
    recruit_a::AVENGERS_COMPOUND,
    recruit_a::BAXTER_BUILDING,
    recruit_a::CASTLE_BLACKSTONE,
    recruit_a::CENTRAL_PARK,
    recruit_a::CRIMSON_COSMOS,
    recruit_a::DANGER_ROOM,
    recruit_a::EBONSHIRE,
    recruit_a::ETERNITY_RANGE,
    recruit_a::FISK_TOWER,
    recruit_a::FLOODED,
    recruit_a::FOGWELLS_GYM,
    recruit_a::FRONTIER_OUTPOST,
    recruit_a::GENOSHA,
    recruit_a::HELLFIRE_CLUB,
    recruit_a::HELLS_KITCHEN,
    recruit_a::KUN_LUN,
    recruit_a::KYLN,
    recruit_a::LAKE_HELLAS,
    recruit_a::MADRIPOOR,
    recruit_a::MINIATURIZED_LAB,
    recruit_a::DREAM_DIMENSION,
    recruit_a::ELYSIUM,
    recruit_a::FLOODING,
    recruit_a::LOS_DIABLOS,
    recruit_a::GRAND_CENTRAL,
    recruit_a::BAR_SINISTER,
    recruit_a::CLONING_VATS,
    recruit_a::DAILY_BUGLE,
    recruit_a::MACHINEWORLD,
    recruit_a::MINDSCAPE,
    locations_b_extra::TITAN,
    locations_b_extra::SANDBAR,
    locations_b_extra::STARK_ISLAND,
    locations_b_extra::ICE_BOX,
    locations_b_extra::DEADLANDS,
    locations_b_extra::SURVIVORS_CAMP,
    locations_b_extra::UNDEAD_METROPOLIS,
    locations_b_extra::OSCORP_TOWER,
    locations_b_extra::MIRROR_DIMENSION,
    locations_b_extra::ZENN_LA,
    locations_b_extra::SAKAAR,
    locations_b_extra::OTTOS_LAB,
    locations_b_extra::STRANGE_ACADEMY,
    locations_b_extra::QUANTUM_TUNNEL,
    locations_b_extra::SINISTER_LONDON,
    locations_b_extra::WESTVIEW,
    locations_b_extra::WHITE_PALACE,
    locations_b_extra::ONSLAUGHTS_CITADEL,
    locations_b::WASHINGTON_DC,
];

pub fn get(id: &str) -> Option<&'static LocationDef> {
    REGISTRY.iter().find(|entry| entry.id == id)
}
