//! Card registrations. Add a card by defining its hooks in a group module and
//! adding its descriptor to `REGISTRY`.
use crate::game::Game;

mod representative;
mod sword_master;

pub type CardHook = fn(&mut Game, usize);
pub type PlayHook = fn(&mut Game, usize, usize);

pub struct CardDef {
    pub id: &'static str,
    pub reveal: Option<CardHook>,
    pub ongoing: Option<CardHook>,
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
    pub initial_state: Option<fn() -> Box<dyn std::any::Any>>,
    pub blocks_reveal: bool,
    pub prevents_destroy: bool,
    pub draw_turn: Option<u8>,
}

impl CardDef {
    pub const fn vanilla(id: &'static str) -> Self {
        Self {
            id,
            reveal: None,
            ongoing: None,
            on_play: None,
            end_turn: None,
            blocks_reveal: false,
            prevents_destroy: false,
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
        }
    }
}

pub static REGISTRY: &[CardDef] = &[
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
];

pub fn get(id: &str) -> Option<&'static CardDef> {
    REGISTRY.iter().find(|entry| entry.id == id)
}
