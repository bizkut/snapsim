//! Card registrations. Add a card by defining its hooks in a group module and
//! listing its descriptor in that module's slice in `GROUPS`.
use crate::game::Game;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

mod agent13;
mod core_rules;
mod kahhori;
mod meta_a1;
mod meta_a2;
mod meta_b1;
mod meta_b2;
mod meta_b3a;
mod meta_b3b;
mod meta_c;
mod meta_d;
mod meta_e1;
mod meta_e2;
mod meta_f1;
mod meta_f2;
mod meta_f3;
mod meta_g;
mod meta_h1;
mod meta_h2;
mod meta_h3;
mod meta_i1;
mod meta_i2;
mod meta_j1;
mod meta_j2;
mod meta_k1;
mod meta_k2;
mod meta_k3;
mod meta_k4;
pub(crate) mod no_text;
mod recruit_a;
mod recruit_b;
pub(crate) mod reference_rules;
mod representative;
mod sword_master;
pub(crate) use meta_h2::restore_objective;
pub(crate) use recruit_a::angel_replace;

/// Tokens a deck may not contain: the reference's `ownable = False` cards.
const BASE_TOKENS: &[&str] = &[
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

/// Token IDs, one slice per card-group module.
static TOKEN_GROUPS: &[&[&str]] = &[
    BASE_TOKENS,
    meta_a1::TOKENS,
    meta_a2::TOKENS,
    meta_b1::TOKENS,
    meta_b2::TOKENS,
    meta_b3a::TOKENS,
    meta_b3b::TOKENS,
    meta_c::TOKENS,
    meta_d::TOKENS,
    meta_e1::TOKENS,
    meta_e2::TOKENS,
    meta_f1::TOKENS,
    meta_f2::TOKENS,
    meta_f3::TOKENS,
    meta_g::TOKENS,
    meta_h1::TOKENS,
    meta_h2::TOKENS,
    meta_h3::TOKENS,
    meta_i1::TOKENS,
    meta_k1::TOKENS,
    meta_k2::TOKENS,
    meta_k4::TOKENS,
    meta_k3::TOKENS,
];

/// Whether a card is a token, which decks may not contain.
pub fn is_token(id: &str) -> bool {
    TOKEN_GROUPS.iter().any(|group| group.contains(&id))
}

/// Registered deck cards in the reference's `deck_eligible_ids()` order: sorted IDs.
static DECK_ELIGIBLE_IDS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut ids: Vec<_> = registry()
        .map(|def| def.id)
        .filter(|id| !is_token(id))
        .collect();
    ids.sort_unstable();
    ids
});

/// The frozen Python reference's `deck_eligible_ids()`. Parity gates compare unchanged
/// content against that reference, so they pin every pool built from this list to it.
static REFERENCE_IDS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let ids: Vec<&'static str> =
        serde_json::from_str(include_str!("../../../parity/reference-pool.json"))
            .expect("invalid parity/reference-pool.json");
    assert!(
        ids.iter().all(|id| get(id).is_some()),
        "reference pool lists an unregistered card"
    );
    ids
});

static USE_REFERENCE: AtomicBool = AtomicBool::new(false);
static ENV_REFERENCE: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("SNAPSIM_REFERENCE_POOL").is_some_and(|value| value == "1"));

/// Pin `deck_eligible_ids()` to the reference list for the rest of the process.
/// Setting `SNAPSIM_REFERENCE_POOL=1` does the same for the CLI and the Python module.
pub fn use_reference_pool() {
    USE_REFERENCE.store(true, Ordering::Relaxed);
}

static USE_REFERENCE_RULES: AtomicBool = AtomicBool::new(false);
static ENV_REFERENCE_RULES: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("SNAPSIM_REFERENCE_RULES").is_some_and(|value| value == "1"));

/// For the rest of the process, existing cards whose rule was deliberately changed to match the
/// client use the frozen reference's rule instead (`cards/reference_rules.rs`). Parity gates
/// only; live play never sets it. `SNAPSIM_REFERENCE_RULES=1` does the same for the CLI and the
/// Python module.
pub fn use_reference_rules() {
    USE_REFERENCE_RULES.store(true, Ordering::Relaxed);
}

pub(crate) fn reference_rules_enabled() -> bool {
    USE_REFERENCE_RULES.load(Ordering::Relaxed) || *ENV_REFERENCE_RULES
}

/// Cards a deck may contain and random-card effects draw from: every registered non-token
/// card, or the reference list while pinned for parity gates.
pub fn deck_eligible_ids() -> &'static [&'static str] {
    if reference_pool_enabled() {
        &REFERENCE_IDS
    } else {
        &DECK_ELIGIBLE_IDS
    }
}

/// Whether pools are pinned to the frozen reference (parity gates only).
pub(crate) fn reference_pool_enabled() -> bool {
    USE_REFERENCE.load(Ordering::Relaxed) || *ENV_REFERENCE
}

/// Whether a card has rules here but not in the frozen Python reference (its deck cards and
/// base tokens). The reference treated such cards as unsupported stand-ins, so parity gates
/// report positions containing them as new content instead of comparing them.
pub fn beyond_reference(id: &str) -> bool {
    get(id).is_some() && !REFERENCE_IDS.contains(&id) && !BASE_TOKENS.contains(&id)
}

pub type CardHook = fn(&mut Game, usize);
pub type PlayHook = fn(&mut Game, usize, usize);
pub type PlayRequirement = fn(&Game<'_>, usize, usize, usize) -> bool;
pub type ActivateRequirement = fn(&Game<'_>, usize) -> bool;
/// Arguments: the guarding card, the card whose cost would change, its old and its new cost.
pub type CostGuard = fn(&Game<'_>, usize, usize, i32, i32) -> bool;

pub struct CardDef {
    pub id: &'static str,
    pub reveal: Option<CardHook>,
    pub ongoing: Option<CardHook>,
    pub in_hand_ongoing: Option<CardHook>,
    /// Refresh objective progress after all ongoing effects have run.
    pub objective_update: Option<CardHook>,
    /// Freeze readiness before any post-end-turn objective completes.
    pub objective_freeze: Option<CardHook>,
    /// Ongoing effect evaluated after normal card and location contributions.
    pub late_ongoing: Option<CardHook>,
    /// Final hand-cost adjustment after ordinary location modifiers.
    pub modify_final_cost: Option<fn(&Game<'_>, usize, i32) -> i32>,
    /// Return false to block a proposed cost transition.
    pub cost_guard: Option<CostGuard>,
    pub ongoing_ability: bool,
    pub extra_ongoing_runs: u8,
    pub reveal_repetitions: Option<fn(&Game<'_>, usize, usize) -> u8>,
    pub on_play: Option<PlayHook>,
    /// Arguments: card, destination lane, playing seat.
    pub can_play: Option<PlayRequirement>,
    /// Once-per-game player activation, and an optional additional requirement.
    pub on_activate: Option<CardHook>,
    pub can_activate: Option<ActivateRequirement>,
    pub activate_cost: i32,
    /// Total intrinsic voluntary moves available (0 means none, 255 unlimited).
    pub move_limit: u8,
    /// Intrinsic move remains available on subsequent turns.
    pub move_each_turn: bool,
    /// Ongoing source grants a voluntary move to another card in its lane.
    pub grants_move_from: Option<fn(&Game<'_>, usize, usize) -> bool>,
    pub pre_end_turn: Option<CardHook>,
    pub end_turn: Option<CardHook>,
    /// An End-of-Turn trigger implemented through the event hook.
    pub end_turn_trigger: bool,
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
    /// Source-specific ongoing protection for a destruction target.
    pub destroy_guard: Option<fn(&Game<'_>, usize, usize) -> bool>,
    /// Ongoing lane prohibition on playing cards with current cost at least four.
    pub blocks_expensive_plays: bool,
    /// Ongoing draw prohibition while this card remains in hand.
    pub blocks_draw_in_hand: bool,
    /// Ongoing prohibition on additions not caused by plays or moves.
    pub blocks_spawns: bool,
    pub locks_lane: bool,
    pub immune_destroy_move: bool,
    /// Draw this card ahead of ordinary cards during the initial hand.
    pub start_in_hand: bool,
    pub draw_turn: Option<u8>,
}

impl CardDef {
    pub const fn vanilla(id: &'static str) -> Self {
        Self {
            id,
            reveal: None,
            ongoing: None,
            in_hand_ongoing: None,
            objective_update: None,
            objective_freeze: None,
            late_ongoing: None,
            modify_final_cost: None,
            cost_guard: None,
            ongoing_ability: false,
            extra_ongoing_runs: 0,
            reveal_repetitions: None,
            on_play: None,
            can_play: None,
            on_activate: None,
            can_activate: None,
            activate_cost: 0,
            move_limit: 0,
            move_each_turn: false,
            grants_move_from: None,
            pre_end_turn: None,
            blocks_spawns: false,
            end_turn: None,
            end_turn_trigger: false,
            blocks_reveal: false,
            prevents_destroy: false,
            destroy_guard: None,
            blocks_expensive_plays: false,
            blocks_draw_in_hand: false,
            locks_lane: false,
            immune_destroy_move: false,
            start_in_hand: false,
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
static GROUPS: &[&[CardDef]] = &[
    BASE,
    meta_a1::CARDS,
    meta_a2::CARDS,
    meta_b1::CARDS,
    meta_b2::CARDS,
    meta_b3a::CARDS,
    meta_b3b::CARDS,
    meta_c::CARDS,
    meta_d::CARDS,
    meta_e1::CARDS,
    meta_e2::CARDS,
    meta_f1::CARDS,
    meta_f2::CARDS,
    meta_f3::CARDS,
    meta_g::CARDS,
    meta_h1::CARDS,
    meta_h2::CARDS,
    meta_h3::CARDS,
    meta_i1::CARDS,
    meta_i2::CARDS,
    meta_j1::CARDS,
    meta_j2::CARDS,
    meta_k3::CARDS,
    meta_k2::CARDS,
    meta_k1::CARDS,
    meta_k4::CARDS,
    kahhori::CARDS,
];

/// Every registered card rule.
pub fn registry() -> impl Iterator<Item = &'static CardDef> {
    GROUPS.iter().flat_map(|group| group.iter())
}

pub fn get(id: &str) -> Option<&'static CardDef> {
    if reference_rules_enabled()
        && let Some(entry) = reference_rules::CARDS.iter().find(|entry| entry.id == id)
    {
        return Some(entry);
    }
    registry().find(|entry| entry.id == id)
}
