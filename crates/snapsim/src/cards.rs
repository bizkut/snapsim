//! Card registrations. Add a card by defining its hooks in a group module and
//! listing its descriptor in that module's slice in `GROUPS`.
use crate::game::Game;

mod agent13;
mod core_rules;
mod recruit_a;
mod recruit_b;
mod representative;
mod sword_master;
pub(crate) use recruit_a::angel_replace;

/// Tokens a deck may not contain: the reference's `ownable = False` cards.
pub const NON_OWNABLE: [&str; 10] = [
    "TigerSpirit",
    "Rock",
    "Squirrel",
    "Drone",
    "Ninja",
    "Monster",
    "Raptor",
    "SinisterClone",
    "Demon",
    "ZombieHorde",
];

/// Registered deck cards in the reference's `deck_eligible_ids()` order: sorted IDs.
static DECK_ELIGIBLE_IDS: std::sync::LazyLock<Vec<&'static str>> = std::sync::LazyLock::new(|| {
    let mut ids: Vec<_> = registry()
        .map(|def| def.id)
        .filter(|id| !NON_OWNABLE.contains(id))
        .collect();
    ids.sort_unstable();
    ids
});

pub fn deck_eligible_ids() -> &'static [&'static str] {
    &DECK_ELIGIBLE_IDS
}

pub type CardHook = fn(&mut Game, usize);
pub type PlayHook = fn(&mut Game, usize, usize);

pub struct CardDef {
    pub id: &'static str,
    pub reveal: Option<CardHook>,
    pub ongoing: Option<CardHook>,
    pub ongoing_ability: bool,
    pub extra_ongoing_runs: u8,
    pub reveal_repetitions: Option<fn(&Game<'_>, usize, usize) -> u8>,
    pub on_play: Option<PlayHook>,
    pub end_turn: Option<CardHook>,
    pub on_move_here: Option<PlayHook>,
    pub on_move: Option<PlayHook>,
    pub on_destroy: Option<CardHook>,
    pub on_discard: Option<CardHook>,
    pub on_hand_added: Option<PlayHook>,
    pub start_turn: Option<CardHook>,
    pub on_event: Option<fn(&mut Game<'_>, usize, crate::game::Event)>,
    pub on_added: Option<CardHook>,
    pub initial_state: Option<fn() -> Box<dyn crate::game::CardState>>,
    pub clear_reveal_state: Option<fn(&mut crate::game::Card)>,
    pub blocks_reveal: bool,
    pub prevents_destroy: bool,
    pub locks_lane: bool,
    pub immune_destroy_move: bool,
    pub draw_turn: Option<u8>,
}

impl CardDef {
    pub const fn vanilla(id: &'static str) -> Self {
        Self {
            id,
            reveal: None,
            ongoing: None,
            ongoing_ability: false,
            extra_ongoing_runs: 0,
            reveal_repetitions: None,
            on_play: None,
            end_turn: None,
            blocks_reveal: false,
            prevents_destroy: false,
            locks_lane: false,
            immune_destroy_move: false,
            draw_turn: None,
            on_move_here: None,
            on_move: None,
            on_destroy: None,
            on_discard: None,
            on_hand_added: None,
            start_turn: None,
            on_event: None,
            on_added: None,
            initial_state: None,
            clear_reveal_state: None,
        }
    }
}

static BASE: &[CardDef] = &[
    agent13::AGENT_13,
    CardDef::vanilla("Abomination"),
    CardDef::vanilla("Cyclops"),
    CardDef::vanilla("Hulk"),
    CardDef::vanilla("MistyKnight"),
    CardDef::vanilla("Shocker"),
    CardDef::vanilla("TheThing"),
    CardDef::vanilla("Wasp"),
    CardDef::vanilla("Quicksilver"),
    CardDef::vanilla("Drone"),
    CardDef::vanilla("TigerSpirit"),
    CardDef::vanilla("Rock"),
    CardDef::vanilla("Squirrel"),
    CardDef::vanilla("Ninja"),
    CardDef::vanilla("Monster"),
    CardDef::vanilla("Raptor"),
    CardDef::vanilla("SinisterClone"),
    CardDef::vanilla("Demon"),
    CardDef::vanilla("ZombieHorde"),
    representative::HAWKEYE,
    representative::JESSICA_JONES,
    representative::MEDUSA,
    representative::SENTINEL,
    representative::ULTRON,
    representative::IRON_MAN,
    representative::KAZAR,
    representative::KLAW,
    representative::MISTER_FANTASTIC,
    representative::BLUE_MARVEL,
    representative::ANGELA,
    representative::BISHOP,
    representative::KRAVEN,
    representative::CARNAGE,
    representative::BLADE,
    sword_master::SWORD_MASTER,
    representative::COSMO,
    CardDef::vanilla("Uatu"),
    recruit_b::NAMOR,
    recruit_b::NOVA,
    recruit_b::PUNISHER,
    recruit_b::ROCKET,
    recruit_b::STARLORD,
    recruit_b::SPECTRUM,
    recruit_b::SPIDER_WOMAN,
    recruit_b::SQUIRREL_GIRL,
    recruit_b::STRONG_GUY,
    recruit_b::COLLECTOR,
    recruit_b::WOLFSBANE,
    recruit_b::YELLOWJACKET,
    recruit_b::YONDU,
    recruit_b::MANTIS,
    recruit_b::MOON_GIRL,
    recruit_b::MR_SINISTER,
    recruit_b::PROFESSOR_X,
    recruit_b::WHITE_QUEEN,
    recruit_b::WHITE_TIGER,
    recruit_b::WOLVERINE,
    recruit_b::ZOMBIE_GIANT_MAN,
    recruit_b::ZOMBIE_BLADE,
    recruit_a::AMERICA_CHAVEZ,
    recruit_a::ANGEL,
    recruit_a::ANT_MAN,
    recruit_a::APOCALYPSE,
    recruit_a::BLACK_PANTHER,
    recruit_a::CABLE,
    recruit_a::CAPTAIN_AMERICA,
    recruit_a::DEATHLOK,
    recruit_a::DEVIL_DINOSAUR,
    recruit_a::ELEKTRA,
    recruit_a::GAMORA,
    recruit_a::GROOT,
    recruit_a::HOOD,
    recruit_a::IRONHEART,
    recruit_a::KORG,
    recruit_a::LADY_SIF,
    recruit_a::LIZARD,
    recruit_a::ICEMAN,
    recruit_a::FORGE,
    core_rules::ENCHANTRESS,
    core_rules::ARMOR,
    core_rules::COLOSSUS,
    core_rules::DOMINO,
    core_rules::ONSLAUGHT,
    core_rules::MAGIK,
    core_rules::IRON_FIST,
    core_rules::HEIMDALL,
    core_rules::DOCTOR_STRANGE,
    core_rules::MULTIPLE_MAN,
    core_rules::NIGHTCRAWLER,
    core_rules::ODIN,
    core_rules::JOAQUIN,
    core_rules::HULKBUSTER,
    core_rules::MORPH,
    core_rules::SCARLET_WITCH,
];

/// Rule descriptors, one slice per card-group module.
static GROUPS: &[&[CardDef]] = &[BASE];

/// Every registered card rule.
pub fn registry() -> impl Iterator<Item = &'static CardDef> {
    GROUPS.iter().flat_map(|group| group.iter())
}

pub fn get(id: &str) -> Option<&'static CardDef> {
    registry().find(|entry| entry.id == id)
}
