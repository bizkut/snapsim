//! Location descriptors. A location is registered by adding one descriptor.

use crate::game::Game;
mod locations_b;
mod locations_b_extra;
mod locations_c;
mod locations_d1;
mod locations_d2;
mod locations_d3;
mod locations_e;
mod locations_f;
mod locations_g;
mod locations_h;
mod locations_na;
mod locations_nd;
mod locations_ne;
mod locations_si;
mod recruit_a;
mod turns;

pub(crate) use locations_d2::cave_candidate;

pub type LocationHook = fn(&mut Game<'_>, usize);
pub type ConditionalLocationHook = fn(&mut Game<'_>, usize) -> bool;
pub type CardHook = fn(&mut Game<'_>, usize, usize);
pub type MoveHook = fn(&mut Game<'_>, usize, usize, usize);
pub type PlayPredicate = fn(&Game<'_>, usize, usize, usize, usize) -> bool;
pub type GlobalCountPlayPredicate = fn(&Game<'_>, usize, usize, usize) -> bool;
pub type EnergyHook = fn(&Game<'_>, usize, usize) -> i32;
pub type CostHook = fn(&Game<'_>, usize, usize, i32) -> i32;
pub type DrawHook = fn(&mut Game<'_>, usize, usize) -> Option<usize>;

pub struct LocationDef {
    pub id: &'static str,
    pub power: i32,
    pub reveal_energy: i32,
    pub reveal_repeats: u8,
    pub ongoing_repeats: u8,
    pub no_destroy: bool,
    pub blocks_activate: bool,
    pub delays_reveals: bool,
    pub delays_reveals_until_end: bool,
    pub disables_ongoing: bool,
    pub disables_on_reveal: bool,
    pub on_reveal: Option<LocationHook>,
    pub on_turn_start: Option<LocationHook>,
    pub on_staging_end: Option<LocationHook>,
    pub on_turn_end: Option<LocationHook>,
    pub ongoing: Option<LocationHook>,
    pub conditional_ongoing: Option<ConditionalLocationHook>,
    pub before_card_reveal: Option<CardHook>,
    pub after_card_play: Option<CardHook>,
    pub on_card_moved: Option<MoveHook>,
    pub on_card_added: Option<CardHook>,
    pub on_event: Option<fn(&mut Game<'_>, usize, crate::game::Event)>,
    pub allows_play: Option<PlayPredicate>,
    pub allows_play_with_count: Option<fn(&Game<'_>, usize, usize) -> bool>,
    pub allows_play_globally_with_count: Option<GlobalCountPlayPredicate>,
    pub allows_play_elsewhere: Option<PlayPredicate>,
    pub allows_addition: Option<PlayPredicate>,
    pub allows_move_from: Option<PlayPredicate>,
    pub before_draw: Option<DrawHook>,
    pub grants_move_to: Option<PlayPredicate>,
    pub energy_bonus: Option<EnergyHook>,
    pub modify_cost: Option<CostHook>,
    pub prevents_retreat: Option<fn(&Game<'_>, bool) -> bool>,
    pub minimum_turns: u8,
    pub maximum_turns: u8,
    pub contributes_to_other_lanes: bool,
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
        blocks_activate: false,
        delays_reveals: false,
        delays_reveals_until_end: false,
        disables_ongoing: false,
        disables_on_reveal: false,
        on_reveal: None,
        on_turn_start: None,
        on_staging_end: None,
        on_turn_end: None,
        ongoing: None,
        conditional_ongoing: None,
        before_card_reveal: None,
        after_card_play: None,
        on_card_moved: None,
        on_card_added: None,
        on_event: None,
        allows_play: None,
        allows_play_with_count: None,
        allows_play_globally_with_count: None,
        allows_play_elsewhere: None,
        allows_addition: None,
        allows_move_from: None,
        before_draw: None,
        grants_move_to: None,
        energy_bonus: None,
        modify_cost: None,
        prevents_retreat: None,
        minimum_turns: 0,
        maximum_turns: 0,
        contributes_to_other_lanes: false,
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
    locations_c::STAR_BRAND_CRATER,
    locations_c::CROWN_CITY,
    locations_c::NEW_YORK,
    locations_c::PYRAMID_OF_RAMA_TUT,
    locations_c::ASTEROID_M,
    // locations_d1 (append below)
    locations_d1::CELESTIAL_CIRCUIT,
    locations_d1::DEATHS_DOMAIN,
    locations_d1::ISLAND_PRISON,
    locations_d1::ISLE_OF_SILENCE,
    locations_d1::QUANTUM_REALM,
    locations_d1::SUBTERRANEA_TUTORIAL,
    locations_d1::TIME_THEATER,
    // locations_d2 (append below)
    locations_d2::VORMIR,
    locations_d2::WORLD_FORGE,
    locations_d2::CAVE_OF_THE_DRAGON,
    locations_d2::COLLAPSED_MINE,
    locations_d2::DARK_DIMENSION,
    locations_d2::BIFROST,
    locations_d2::BOARDWALK,
    // locations_d3 (append below)
    locations_d3::CHRONOSPHERE_SPHINX,
    locations_d3::CRIMSON_CITADEL,
    locations_d3::HALA,
    locations_d3::JOSIES_BAR,
    locations_d3::KLYNTAR,
    locations_d3::TA_LO,
    // locations_e (append below)
    locations_e::KNOWHERE,
    locations_e::MORAG,
    locations_e::SAKAAR_TUTORIAL,
    locations_e::SANCTUM_SANCTORUM,
    locations_e::PIT_OF_EXILE,
    locations_e::PET_AVENGERS_MANSION,
    locations_e::SPIDER_ISLAND,
    locations_e::YGGDRASIL,
    locations_e::THUNDERBOLTS_TOWER,
    locations_e::THE_ABBEY,
    locations_e::RAFT,
    locations_f::JOTUNHEIM,
    locations_f::UTOPIA,
    locations_f::CAMP_LEHIGH,
    locations_f::ATTILAN,
    locations_g::CAMELOT,
    locations_g::WARRIOR_FALLS,
    locations_g::RICKETY_BRIDGE,
    locations_g::SANDCASTLE,
    locations_g::TVA,
    locations_g::THE_NEXUS,
    locations_g::VALHALLA,
    locations_g::SUPERFLOW,
    locations_g::EMPIRE_STATE_UNIVERSITY,
    locations_g::CLUBHOUSE,
    locations_g::ESSEXS_LAB,
    locations_g::FEAST,
    locations_g::ALTAR_OF_DEATH,
    locations_g::SMITHSONIAN_MUSEUM,
    locations_g::PARKER_INDUSTRIES,
    locations_g::FRACTURED_FRONTIER,
    locations_g::VALLEY_OF_THE_HAND,
    locations_g::SAKAAR_GRAND_PRIX,
    locations_g::WHITE_HOT_ROOM,
    locations_g::WEAPON_X_FACILITY,
    locations_h::SOKOVIA,
    locations_h::THE_HUB,
    locations_h::GREAT_PORTAL,
    locations_h::MILANO,
    locations_h::SUBTERRANEA,
    locations_h::WEIRDWORLD,
    locations_h::LECHUGUILLA,
    locations_h::LUKES_BAR,
    locations_h::UTNAPISHTIMS_ARK,
    locations_h::SPACE_THRONE,
    locations_h::DEEP_SPACE,
    locations_h::STARLIGHT_CITADEL,
    locations_h::PANOPTICHRON,
    locations_h::GREAT_WEB,
    locations_h::BAR_WITH_NO_NAME,
    locations_h::CANCUN,
    locations_h::NOOR_DIMENSION,
    locations_h::PIZZA_PLACE,
    locations_h::TACO_TRUCK,
    locations_h::DESTROYED_MANSION,
    locations_h::VIBRANIUM_MINES,
    locations_h::GAMMA_LAB,
    locations_h::BLACK_VORTEX,
    locations_h::ASGARD_BESIEGED,
    locations_h::HOTEL_INFERNO,
    locations_h::KRAKOA,
    locations_h::THE_PEAK,
    locations_h::TRISKELION,
    locations_h::TARNAX,
    locations_h::CELESTIAL_BURIAL_GROUND,
    locations_h::AUNT_MAYS,
    locations_h::SACRED_TIMELINE,
    locations_h::ASTRAL_PLANE,
    locations_h::CASTLE_ZEMO,
    locations_h::KVCH,
    locations_ne::WORLDSHIP,
    locations_ne::X_MANSION,
    locations_ne::MOUNT_VESUVIUS,
    locations_na::BROOKLYN_BRIDGE,
    locations_nd::HOSPICE,
    locations_si::SANCTUM_INFINITUM,
    location("Ego", 0, 0, 1, false),
];

pub fn get(id: &str) -> Option<&'static LocationDef> {
    REGISTRY.iter().find(|entry| entry.id == id)
}

/// Priority of a location-owned ongoing stat ability in the 59.15.1 client.
/// The fixed-power locations and the first group use the default priority.
/// Constructor RVAs: Atlantis 0x7d3d4a0, Baxter 0x7d3fb60,
/// Crown City 0x7d4c498, Monster Metropolis 0x7d67480,
/// Wakandan Throne Room 0x7d95318, Hospice 0x7d59744, Nexus 0x7d8b588.
/// Cost constructor RVAs: Dream Dimension 0x7d4fd68, Elysium 0x7d50f18,
/// Fractured Frontier 0x7d54638, Sandbar 0x7d8c8b8, Titan 0x7d8e4d0,
/// Utopia 0x7d90820; all use the default priority.
/// Returns priority, active-while-staging, active-when-staged-for-play,
/// and active-when-staged-for-move. The client constructors leave all three
/// staging flags false for these location-owned effects.
pub fn ongoing_stat_schedule(id: &str) -> Option<(u8, bool, bool, bool)> {
    let priority = match id {
        "Xandar" | "NegativeZone" | "Necrosha" | "Nidavellir" | "SewerSystem" | "WashingtonDC"
        | "Mojoworld" | "Atlantis" | "LakeHellas" | "Panoptichron" | "SmithsonianMuseum"
        | "DreamDimension" | "Elysium" | "FracturedFrontier" | "TheSandbar" | "Titan"
        | "Utopia" => 0,
        "MonsterMetropolis" | "WakandanThroneRoom" => 2,
        "BaxterBuilding" | "CrownCity" | "Hospice" | "TheNexus" => 3,
        _ => return None,
    };
    Some((priority, false, false, false))
}

#[cfg(test)]
mod priority_tests {
    use super::{REGISTRY, ongoing_stat_schedule};

    #[test]
    fn every_ongoing_stat_location_has_a_client_priority() {
        for location in REGISTRY {
            if location.power != 0
                || location.ongoing.is_some()
                || location.conditional_ongoing.is_some()
                || location.modify_cost.is_some()
                || location.contributes_to_other_lanes
            {
                assert!(
                    ongoing_stat_schedule(location.id).is_some(),
                    "missing priority for {}",
                    location.id
                );
            }
        }
    }
}
