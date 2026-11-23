//! Conservative admission check for masks reconstructed without the opponent's hidden state.
//!
//! This does not reconstruct the recorded player's state. The caller must separately
//! establish that its energy, cards, counters and other state are observable.
use crate::cards::{self, CardDef};
pub use crate::engine_ext::abilities::AbilityKind;
use crate::game::{AttachedAbilityKind, CardCostEffectKind, PlayerCostEffectKind};

/// Values must be proven complete, not merely the subset observed in a recording.
#[derive(Clone, Copy, Debug)]
pub enum Known<'a, T> {
    Known(&'a [T]),
    Unknown,
}

/// Every active effect must retain an identified source; a missing source fails closed.
pub struct Sourced<T> {
    pub source: Option<String>,
    pub effect: T,
}

/// Other stateful effects which can affect the recorded player's current actions.
#[derive(Clone, Copy, Debug)]
pub enum StateEffectKind {
    PlayLock,
    LocationSilence,
    MoveGrant,
    PendingEffect,
    EnergyEffect,
    GlobalEffect,
}

pub struct Inputs<'a> {
    /// Complete active location list; callbacks may inspect hidden cards.
    pub active_locations: Known<'a, &'a str>,
    /// Complete recorded-seat hand and visible board card IDs, on both sides.
    /// Their hooks can read hidden identities; unreviewed cards fail closed.
    pub visible_cards: Known<'a, &'a str>,
    /// Union of all possible opponent hand, deck and face-down card identities.
    pub hidden_cards: Known<'a, &'a str>,
    /// Includes copied/composed abilities, including their definition IDs.
    pub copied_abilities: Known<'a, (&'a str, AbilityKind)>,
    pub attached_abilities: Known<'a, AttachedAbilityKind>,
    pub card_cost_effects: Known<'a, Sourced<CardCostEffectKind>>,
    pub player_cost_effects: Known<'a, Sourced<PlayerCostEffectKind>>,
    /// Includes play locks, location silence, move grants and other pending/global effects.
    pub state_effects: Known<'a, Sourced<StateEffectKind>>,
    /// Complete movable component source identities and creation order.
    /// The client breaks priority ties with MovableByPlayerComponent.CreatedTimestamp.
    pub move_source_provenance: Known<'a, ()>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Result {
    Valid,
    Invalid { reasons: Vec<String> },
}

// No wildcard arms: new engine variants must be reviewed at compile time.
fn card_cost_may_affect(kind: CardCostEffectKind) -> bool {
    match kind {
        CardCostEffectKind::NextTurnDelta { .. }
        | CardCostEffectKind::TemporarySet { .. }
        | CardCostEffectKind::WhiteQueenWinning { .. } => true,
    }
}
fn player_cost_may_affect(kind: PlayerCostEffectKind) -> bool {
    match kind {
        PlayerCostEffectKind::Sandman { .. }
        | PlayerCostEffectKind::BaronMordo { .. }
        | PlayerCostEffectKind::BlackSwan { .. }
        | PlayerCostEffectKind::Wave { .. }
        | PlayerCostEffectKind::ChampionDoom { .. } => true,
    }
}
fn attached_may_affect(kind: AttachedAbilityKind) -> bool {
    match kind {
        AttachedAbilityKind::Watch { .. }
        | AttachedAbilityKind::MovePower { .. }
        | AttachedAbilityKind::NicoPrepare { .. }
        | AttachedAbilityKind::RevealAtGameEnd => true,
    }
}
fn ability_may_affect(kind: AbilityKind) -> bool {
    match kind {
        AbilityKind::Default
        | AbilityKind::Ongoing
        | AbilityKind::Reveal
        | AbilityKind::Activate
        | AbilityKind::Trigger => true,
    }
}
fn state_may_affect(kind: StateEffectKind) -> bool {
    match kind {
        StateEffectKind::PlayLock
        | StateEffectKind::LocationSilence
        | StateEffectKind::MoveGrant
        | StateEffectKind::PendingEffect
        | StateEffectKind::EnergyEffect
        | StateEffectKind::GlobalEffect => true,
    }
}

// Unknown and unreviewed hooks fail closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Classification {
    Safe,
    MayAffect,
}
use Classification::{MayAffect, Safe};

const HIDDEN_HOOK_CARDS: &[(&str, Classification, &str)] = &[
    (
        "Marrow",
        MayAffect,
        "meta_m6::marrow_discard afflicts opposing revealed cards",
    ),
    (
        "WidowsKiss",
        Safe,
        "meta_k3::widows_kiss requires a lane and a non-Empower event",
    ),
    (
        "Makkari",
        MayAffect,
        "meta_ne::makkari moves from hand to a location",
    ),
    (
        "Clea",
        MayAffect,
        "meta_ne::clea adds a hand card, triggering revealed hand-added observers",
    ),
    (
        "Arishem",
        MayAffect,
        "meta_ne::arishem_start inserts random cards whose hidden start hooks may affect legality",
    ),
    (
        "EzekielSims",
        Safe,
        "meta_m2::ezekiel_sims requires a lane for its reaction",
    ),
    (
        "Mockingbird",
        Safe,
        "meta_h3::mockingbird changes only its own cost",
    ),
    (
        "Venus",
        MayAffect,
        "meta_b1::venus empowers allies from a lane, including face-down during Empower",
    ),
    (
        "PolarisHorseman",
        Safe,
        "meta_d::complete requires a lane for Polaris",
    ),
    (
        "CalibanHorseman",
        MayAffect,
        "meta_d::complete can afflict opposing cards after a stored objective completes",
    ),
    (
        "EnSabahNur",
        MayAffect,
        "meta_d::complete transforms its card when a stored objective completes",
    ),
    (
        "GambitHorseman",
        MayAffect,
        "meta_d::complete destroys opposing cards after a stored objective completes",
    ),
    (
        "WolverineHorseman",
        MayAffect,
        "meta_d::complete buffs revealed allies on objective completion",
    ),
    ("Stature", Safe, "meta_na::stature discounts itself"),
    (
        "SheHulk",
        Safe,
        "meta_na::she_hulk_event stores owner energy; she_hulk_cost changes only its own cost",
    ),
    (
        "Galacta",
        Safe,
        "meta_na::galacta requires a lane for its reaction",
    ),
    (
        "StormHorseman",
        MayAffect,
        "meta_na::storm_horseman empowers its owner's hand and revealed allies",
    ),
    (
        "EvolvedAbomination",
        Safe,
        "meta_evolved::abomination discounts itself",
    ),
    (
        "EvolvedHulk",
        Safe,
        "meta_evolved::hulk buffs itself in hand; empower observers do not affect other cards there",
    ),
    (
        "Silk",
        Safe,
        "meta_l2::silk responds to CardResolved, not dispatched to hidden cards",
    ),
    (
        "ElsaBloodstone",
        MayAffect,
        "meta_l2::elsa_bloodstone buffs a revealed ally, potentially changing opposing Wilson Fisk cost",
    ),
    (
        "MBaku",
        MayAffect,
        "meta_l2::mbaku moves from deck to a location at game end",
    ),
    (
        "Xorn",
        Safe,
        "meta_l2::xorn responds to CardResolved, not dispatched to hidden cards",
    ),
    (
        "Hercules",
        Safe,
        "meta_l2::hercules requires a lane for its reaction",
    ),
    (
        "Batroc",
        Safe,
        "meta_l2::batroc requires a lane for its reaction",
    ),
    (
        "ProximaMidnight",
        MayAffect,
        "meta_l2::proxima_midnight resurrects onto the board",
    ),
    ("Miek", Safe, "meta_l2::miek requires a lane"),
    (
        "ZombieBlackCat",
        MayAffect,
        "meta_l2::zombie_black_cat discards another trigger card and resurrects",
    ),
    ("Martyr", Safe, "meta_l2::martyr requires a lane"),
    (
        "RocketAndGroot",
        Safe,
        "meta_m3::rocket_and_groot requires a lane",
    ),
    (
        "SpiderPunk",
        Safe,
        "meta_m3::spider_punk responds to CardMoved, not dispatched to hidden cards",
    ),
    (
        "SeleneHorseman",
        Safe,
        "meta_m3::selene_complete requires a lane to steal",
    ),
    (
        "JuggernautHorseman",
        Safe,
        "meta_h2::objective_complete only buffs itself",
    ),
    (
        "Apocalypse",
        MayAffect,
        "recruit_a::apocalypse returns to hand, triggering revealed hand-added observers",
    ),
    (
        "ColonelAmerica",
        Safe,
        "meta_l3::colonel_discard records only its own discard state",
    ),
    (
        "Khonshu",
        MayAffect,
        "meta_l3::khonshu_discard dispatches hand-added hooks",
    ),
    (
        "KhonshuWaxing",
        MayAffect,
        "meta_l3::khonshu_discard dispatches hand-added hooks",
    ),
    (
        "ArchangelHorseman",
        Safe,
        "meta_l3::archangel_complete requires a lane before resurrecting",
    ),
    (
        "ZombieVenom",
        Safe,
        "meta_l3::zombie_venom_event responds to CardDestroyed, not dispatched to hidden cards",
    ),
    (
        "BlackKnight",
        MayAffect,
        "meta_l3::black_knight adds Ebony Blade and triggers hand-added observers",
    ),
    (
        "NegasonicTeenageWarhead",
        Safe,
        "meta_l3::negasonic requires a lane",
    ),
    (
        "Dracula",
        MayAffect,
        "meta_l3::dracula discards a hand card, invoking its discard hook",
    ),
    (
        "WeaponH",
        MayAffect,
        "meta_l3::weapon_h returns to hand through discarded_to_hand_add_cost and its observers",
    ),
    (
        "Scorn",
        MayAffect,
        "meta_l3::scorn buffs a revealed ally and returns to hand",
    ),
    ("Death", Safe, "meta_l3::death discounts itself"),
    (
        "Swarm",
        MayAffect,
        "meta_k2::swarm creates hand copies and notifies hand-added observers",
    ),
    (
        "WidowsBite",
        MayAffect,
        "meta_k2::WidowsBite blocks owner draws and their card-drawn observers",
    ),
    (
        "Thanos",
        MayAffect,
        "meta_nd::thanos creates and shuffles six stones at game start",
    ),
    (
        "ShangChiMasterOfTheRings",
        MayAffect,
        "meta_nd_rings::add_rings changes deck size, affecting opposing Darkhawk",
    ),
    (
        "HighEvolutionary",
        MayAffect,
        "meta_comp2::high_evolutionary copies evolved text into deck cards and recalculates ongoing effects",
    ),
    (
        "Firehair",
        Safe,
        "meta_comp::firehair requires a lane before borrowing a resolve ability",
    ),
    (
        "Kang",
        MayAffect,
        "meta_comp3::kang copies text from a broad card pool",
    ),
    (
        "Hulkling",
        MayAffect,
        "meta_comp3::hulkling copies a six-cost card's text",
    ),
    (
        "CaptainMarvel",
        Safe,
        "meta_b3b::captain_marvel requires a lane before moving",
    ),
    (
        "ZombieGalacti",
        Safe,
        "meta_b3b::zombie_event stores selections from its owner's deck",
    ),
    (
        "ElderDragonOfTheMoon",
        Safe,
        "meta_elder_dragon::discount discounts itself",
    ),
    (
        "Skill01TheThingFirstStepsChampion",
        Safe,
        "meta_skills_b::thing_discount discounts itself",
    ),
    ("Sasquatch", Safe, "meta_b3a::sasquatch discounts itself"),
    (
        "JubileeSilverSurfer",
        Safe,
        "meta_b3a::jubilee requires a lane",
    ),
    (
        "ShadowlandsDaredevil",
        Safe,
        "meta_b3a::daredevil_draw requires a lane",
    ),
    (
        "GhostThunderbolts",
        Safe,
        "meta_variants_b::ghost discounts itself",
    ),
    (
        "AntiVenomWebSlingers",
        Safe,
        "meta_variants_b::anti_venom_event requires a lane before triggering reveal",
    ),
    (
        "AgonyWebSlingers",
        Safe,
        "meta_variants_b::agony requires a lane before merging",
    ),
    (
        "ThanosFracturedFrontier",
        MayAffect,
        "meta_tff::start installs quickdraw player effects",
    ),
    (
        "Skill01GhostRiderChampion",
        MayAffect,
        "meta_skills_a::rider_discard adds the card to a location",
    ),
    (
        "AgathaHarkness",
        MayAffect,
        "meta_auto::agatha_game_start draws through location and card draw observers",
    ),
    (
        "VampireDeaconFrost",
        MayAffect,
        "meta_zd2::objective_event transforms the card on completion",
    ),
    (
        "PunisherWarMachine",
        Safe,
        "meta_zd2::objective_event marks its own objective",
    ),
    (
        "TheCollector",
        Safe,
        "game::notify_hand_added dispatches recruit_b::collector only to revealed board cards",
    ),
    (
        "Wolverine",
        MayAffect,
        "recruit_b::discard_wolverine and regenerate add the card to a location",
    ),
    (
        "ManSpider",
        MayAffect,
        "meta_nc::man_spider_event buffs the merge receiver",
    ),
    (
        "PeniParker",
        Safe,
        "meta_nc::peni_event schedules energy for its owner only",
    ),
    (
        "SinisterSix",
        MayAffect,
        "meta_nc::sinister_start installs player-owned draw and resolution effects",
    ),
    (
        "StrangeSupreme",
        Safe,
        "meta_nc::strange_event requires a lane and a merge event",
    ),
    ("Namorita", Safe, "meta_i1::namorita requires a lane"),
    (
        "Topaz",
        Safe,
        "meta_i1::topaz responds to CardResolved, not dispatched to hidden cards",
    ),
    (
        "BlackCat",
        MayAffect,
        "meta_k1::black_cat discards itself, enabling the opposing Stature discount",
    ),
    (
        "SebastianShaw",
        Safe,
        "meta_j1::sebastian_shaw silently buffs itself",
    ),
    (
        "SpiderSense",
        Safe,
        "meta_e2::spider_sense_discount discounts itself",
    ),
    ("MonstroOctopus", Safe, "meta_l4::monstro discounts itself"),
    (
        "MuramasaShard",
        MayAffect,
        "meta_l4::muramasa buffs allied Daken on discard",
    ),
    (
        "KittyPryde",
        Safe,
        "meta_l4::kitty_event buffs itself on return",
    ),
    (
        "DoctorOctopus",
        Safe,
        "meta_l4::octopus_event invokes doctor_octopus, which requires a lane",
    ),
    (
        "TheGrandDesign",
        Safe,
        "meta_l4::grand_discount discounts itself",
    ),
    (
        "Agamotto",
        MayAffect,
        "meta_l4::agamotto_start adds deck cards, changing opposing Darkhawk and potentially Wilson Fisk cost",
    ),
    (
        "Dormammu",
        MayAffect,
        "meta_l4::dormammu_start adds a hand card, triggering revealed hand-added observers",
    ),
    (
        "GalactusFirstSteps",
        Safe,
        "meta_l4::galactus_event records its herald and buffs itself while in hand",
    ),
    (
        "Stryfe",
        MayAffect,
        "meta_l4::stryfe_complete borrows a hand card's reveal on AfterTurnEnd",
    ),
    (
        "X23",
        MayAffect,
        "meta_m5::x23_discard resurrects the card and schedules energy",
    ),
    (
        "Helicarrier",
        MayAffect,
        "meta_m5::helicarrier creates hand cards and triggers revealed hand-added observers",
    ),
    (
        "SubMariner",
        MayAffect,
        "meta_m5::sub_mariner creates a random card and adds it to hand",
    ),
    (
        "Boomerang",
        MayAffect,
        "meta_m5::boomerang_returned steals opposing hand power",
    ),
    (
        "JeffTheBabyDolphin",
        Safe,
        "meta_c::jeff_event requires a lane",
    ),
    ("MilesMorales", Safe, "meta_k4::miles discounts itself"),
    ("Scream", Safe, "meta_m1::scream requires a lane"),
    (
        "Copycat",
        MayAffect,
        "meta_m1::copycat copies opposing deck text when drawn",
    ),
    (
        "WeaponXWolverine",
        Safe,
        "meta_m1::weapon_x responds to CardDestroyed, not dispatched to hidden cards",
    ),
    ("Skaar", Safe, "meta_l1::skaar discounts itself"),
    ("RedHulk", Safe, "meta_l1::red_hulk buffs itself in hand"),
    ("HopeSummers", Safe, "meta_l1::hope_summers requires a lane"),
    (
        "FrankieRaye",
        Safe,
        "meta_l1::frankie_raye handles CardResolved, which Game::emit does not dispatch to hand or deck",
    ),
    (
        "Surtur",
        Safe,
        "meta_l1::surtur responds to CardResolved, not dispatched to hidden cards",
    ),
    (
        "WilsonFisk",
        Safe,
        "meta_l1::wilson_fisk changes only its own hand cost",
    ),
    (
        "Titania",
        Safe,
        "meta_l1::titania responds to CardResolved, not dispatched to hidden cards",
    ),
    (
        "Agony",
        Safe,
        "meta_f3::agony requires a lane before merging",
    ),
    (
        "ThanosChampion",
        MayAffect,
        "meta_nd::thanos creates and shuffles six stones at game start",
    ),
    (
        "WeaponXWolverineChampion",
        Safe,
        "meta_m1::weapon_x responds to CardDestroyed, not dispatched to hidden cards",
    ),
];

// Ordinary in-play ongoing dispatch checks `revealed` (Game::ongoing in
// game.rs); Game::emit filters face-down event observers and separately visits
// hand/deck observers (game.rs, Game::emit). Borrowed reveals are *not*
// dismissed by that rule: the copier and copied-ability inputs fail closed.
fn has_hidden_hooks(def: &CardDef) -> bool {
    def.pre_game_start.is_some()
        || def.in_hand_ongoing.is_some()
        || def.late_in_hand_ongoing.is_some()
        || def.conditional_in_hand_ongoing.is_some()
        || def.end_turn_in_hand
        || def.end_turn_trigger_hidden
        || def.on_event.is_some()
        || def.on_discard.is_some()
        || def.on_hand_added.is_some()
        || def.on_added.is_some()
        || def.blocks_draw_in_hand
        // Review these explicitly even if their present descriptor changes.
        || matches!(def.id, "Boomerang" | "Stryfe" | "Copycat")
}

fn hidden_hook_may_affect(def: &CardDef) -> bool {
    if !has_hidden_hooks(def) {
        return false;
    }
    let Some((_, classification, reason)) =
        HIDDEN_HOOK_CARDS.iter().find(|(id, _, _)| *id == def.id)
    else {
        return true;
    };
    if *reason == "unreviewed" {
        return true;
    }
    // At a private decision no event or game-start hook is firing. Its
    // surviving effects must instead be included in the complete effect and
    // copied-ability inventories supplied to check_hidden_pool.
    if def.in_hand_ongoing.is_none()
        && def.late_in_hand_ongoing.is_none()
        && def.conditional_in_hand_ongoing.is_none()
    {
        return false;
    }
    *classification != Safe
}
// These reviews concern the already-restored private decision, not subsequent
// reveals or turn transitions. Effects produced earlier must be hydrated and
// independently pass the effect/provenance inventories below.
const VISIBLE_CARD_HOOKS: &[(&str, Classification)] = &[
    ("AntMan", Safe),
    ("Hawkeye", Safe),
    ("KaZar", Safe),
    ("Klaw", Safe),
    ("SquirrelGirl", Safe),
    ("JessicaJones", Safe),
    ("Medusa", Safe),
    ("Squirrel", Safe),
    ("Wolfsbane", Safe),
    // Morph replaces its own definition with the copied card; an observed
    // Morph alone does not prove what ability composition survived.
    ("Morph", MayAffect),
    ("Bishop", Safe),
    ("Groot", Safe),
    ("Ironheart", Safe),
    ("Starlord", Safe),
    // Movement, activation or current cost depends on unexported provenance.
    ("Electro", MayAffect),
    ("JenniferKale", MayAffect),
    ("KarimaShapandar", MayAffect),
    ("MadameWeb", MayAffect),
    ("Magus", MayAffect),
    ("Maverick", MayAffect),
    ("Mockingbird", MayAffect),
    ("Nightcrawler", MayAffect),
    ("Sasquatch", MayAffect),
    ("SpiderManBrandNewDay", MayAffect),
    ("Vision", MayAffect),
    ("ZombieCaptainMarvel", MayAffect),
    // The following hooks have not been proven independent of all hydrated
    // power, cost, move and composed-ability inputs.
    ("Agony", MayAffect),
    ("Angela", Safe), // on_play runs only after a card resolves
    ("Armor", Safe),  // visible destruction protection
    ("Attuma", Safe), // end_turn only
    // Ongoing power uses the visible board and active abilities.
    ("BlueMarvel", Safe),
    ("CaptainAmerica", Safe),
    ("CaptainCarter", MayAffect),
    ("Colossus", Safe), // visible protection flag
    ("Cosmo", Safe),    // visible reveal suppression
    ("Dagger", MayAffect),
    ("Daredevil", MayAffect),
    ("DevilDinosaur", MayAffect),
    ("DraxAvatarOfLife", Safe), // end_turn only
    ("EzekielSims", MayAffect),
    ("Galacta", MayAffect),
    ("Giganto", MayAffect),
    ("Goliath", Safe),
    ("HighEvolutionary", MayAffect),
    ("HumanTorchFirstSteps", MayAffect),
    ("IronMan", Safe),
    ("Isca", Safe),          // end_turn only
    ("JJonahJameson", Safe), // cosmetic ongoing
    ("JoaquinTorres", MayAffect),
    ("JubileeSilverSurfer", MayAffect),
    ("JuggernautHorseman", MayAffect),
    ("Kang", MayAffect),
    ("Knull", MayAffect),
    ("Lizard", Safe),
    ("LukeCage", MayAffect),
    ("MrFantastic", Safe),
    ("MrFantasticFirstSteps", Safe), // end_turn only
    ("Namor", Safe),
    ("Namorita", MayAffect),
    ("Nebula", MayAffect),
    ("NicholasScratch", Safe), // on_play runs at resolution
    // Late ongoing depends on lane-power resolution.
    ("OmegaRed", MayAffect),
    ("Onslaught", Safe), // multiplier derives from visible ongoing sources
    ("PowerStone", MayAffect),
    ("ProfessorX", Safe),
    ("Punisher", Safe),
    ("Quicksilver", Safe),
    ("ScarletWitchQueenOfChaos", Safe), // start_turn only
    ("SebastianShaw", MayAffect),
    // A clipped discount can hide the underlying permanent cost. Moving or
    // suppressing Sera may expose it during this decision.
    ("Sera", MayAffect),
    ("ShangChiMasterOfTheRings", MayAffect),
    ("SoulStone", Safe),  // active visible stone controls protection
    ("SpaceStone", Safe), // active visible stone controls Thanos permission
    ("Stardust", Safe),
    ("SuperiorSpiderMan", Safe), // start_turn only
    ("Surge", Safe),             // reveal and on_play run at resolution
    ("TenRings", Safe),          // starting hand and reveal
    ("TenRingsUpgrade", MayAffect),
    ("Thanos", MayAffect),
    ("ThanosFracturedFrontier", Safe), // game start and reveal
    ("TheCollector", MayAffect),
    ("TheThingFirstSteps", MayAffect),
    ("Topaz", MayAffect),
    ("VampireEnchantress", MayAffect),
    ("VampireKillmonger", Safe), // end_turn only
    ("Venus", MayAffect),
    ("WinterSoldierThunderbolts", MayAffect),
    ("Wolverine", MayAffect),
    ("Wong", MayAffect),
    ("ZombieBlade", MayAffect),
    ("ZombieScarletWitch", MayAffect),
    // Vanilla cards have no dispatched legality hook.
    ("Broodling", Safe),
    ("Cyclops", Safe),
    ("Demon", Safe),
    ("Drone", Safe),
    ("Hulk", Safe),
    ("MistyKnight", Safe),
    ("Monster", Safe),
    ("NicoMinoru", Safe),
    ("Raptor", Safe),
    ("Rock", Safe),
    ("Shocker", Safe),
    ("SinisterClone", Safe),
    ("Symbiote", Safe),
    ("TheThing", Safe),
    ("TigerSpirit", Safe),
    ("ZombieHorde", Safe),
    // Reveals happen at resolution; resulting board and hand are hydrated.
    ("Agent13", Safe),
    ("AntiVenom", Safe),
    ("Mirage", Safe),
    ("AmericaChavez", Safe),
    ("BlackPanther", Safe),
    ("Blade", Safe),
    ("Brood", Safe),
    ("Cable", Safe),
    ("Carnage", Safe),
    ("Drax", Safe),
    ("Elektra", Safe),
    ("Elixir", Safe),
    ("Forge", Safe),
    ("Gamora", Safe),
    ("GreenGoblin", Safe),
    ("Hobgoblin", Safe),
    ("Hood", Safe),
    ("Iceman", Safe),
    ("Kahhori", Safe),
    ("KnightsVow", Safe),
    ("Magik", Safe),
    ("MajesticWingbeat", Safe),
    ("MindStone", Safe),
    ("MrSinister", Safe),
    ("Nakia", Safe),
    ("Odin", Safe),
    ("Phastos", Safe),
    ("Polaris", Safe),
    ("Polymorph", Safe),
    ("RemoteMines", Safe),
    ("Selene", Safe),
    ("Sentinel", Safe),
    ("SilverSable", Safe),
    ("SilverSurfer", Safe),
    ("Spectrum", Safe),
    ("Spell03Agamotto", Safe),
    ("SpiderMan", Safe),
    ("Stick", Safe),
    ("SwordMaster", Safe),
    ("TheHunger", Safe),
    ("VampiricSpear", Safe),
    ("VibroShockGauntlets", Safe),
    ("WebSling", Safe),
    ("WhiteTiger", Safe),
    ("ZombieGiantMan", Safe),
    ("Ultron", Safe),
];

const LOCATION_HOOKS: &[(&str, Classification)] = &[
    // Energy at this decision is hydrated; this hook runs at turn start.
    ("ReedsLab", Safe),
    // The copy is produced on a later play, not at this decision.
    ("Machineworld", Safe),
    // A move can fire BeforeCardMoved and award a card; the consumed flag is
    // not present in the restored position.
    ("Raft", MayAffect),
    // Current-turn, card-cost/power, or unconditional predicates only.
    ("AvengersCompound", Safe),
    ("CrimsonCosmos", Safe),
    ("HellfireClub", Safe),
    ("IslandPrison", Safe),
    ("Kyln", Safe),
    ("MiniaturizedLab", Safe),
    ("JeanGreySchool", Safe),
    ("NewYork", Safe),
    ("PitOfExile", Safe),
    ("SanctumSanctorum", Safe),
    ("TheBigHouse", Safe),
    ("Vault", Safe),
    ("DreamDimension", Safe),
    ("Elysium", Safe),
    ("Titan", Safe),
    ("Utopia", Safe),
    // Sandbar reads card.created, which need not be exported.
    ("TheSandbar", MayAffect),
    ("Morag", MayAffect),
    ("PetAvengersMansion", MayAffect),
    ("TheAbbey", MayAffect),
    // Remaining present hooks are retained conservatively until their full
    // interactions with action staging and hydrated inputs are established.
    ("AsteroidM", MayAffect),
    ("Atlantis", MayAffect),
    ("BarSinister", MayAffect),
    ("BaxterBuilding", MayAffect),
    ("CloningVats", MayAffect),
    ("CrimsonCitadel", MayAffect),
    ("CrownCity", MayAffect),
    ("DarkDimension", MayAffect),
    ("DeathsDomain", MayAffect),
    ("FiskTower", MayAffect),
    ("IsleOfSilence", MayAffect),
    ("JosiesBar", MayAffect),
    ("KamarTaj", MayAffect),
    ("Knowhere", MayAffect),
    ("KunLun", MayAffect),
    ("LakeHellas", MayAffect),
    ("Lemuria", MayAffect),
    ("Limbo", MayAffect),
    ("MonsterMetropolis", MayAffect),
    ("OnslaughtsCitadel", MayAffect),
    ("OrchisForge", MayAffect),
    ("OttosLab", MayAffect),
    ("Proscenium", MayAffect),
    ("QuantumRealm", MayAffect),
    ("QuantumTunnel", MayAffect),
    ("ShurisLab", MayAffect),
    ("SinisterLondon", MayAffect),
    ("SpiderIsland", MayAffect),
    ("TimeTheater", MayAffect),
    ("TinkerersWorkshop", MayAffect),
    ("Vormir", MayAffect),
    ("Wakanda", MayAffect),
    ("WashingtonDC", MayAffect),
    // Static power only, or no effects.
    ("Xandar", Safe),
    ("NegativeZone", Safe),
    ("Necrosha", Safe),
    ("SewerSystem", Safe),
    ("Ruins", Safe),
    // These only dispatch on location reveal or at turn boundaries.
    ("Asgard", Safe),
    ("Attilan", Safe),
    ("Bifrost", Safe),
    ("Boardwalk", Safe),
    ("CampLehigh", Safe),
    ("CaveOfTheDragon", Safe),
    ("CelestialCircuit", Safe),
    ("CentralPark", Safe),
    ("ChronosphereSphinx", Safe),
    ("CollapsedMine", Safe),
    ("Ebonshire", Safe),
    ("EternityRange", Safe),
    ("Genosha", Safe),
    ("GrandCentral", Safe),
    ("GreatPortal", Safe),
    ("Hala", Safe),
    ("HellsKitchen", Safe),
    ("Jotunheim", Safe),
    ("Klyntar", Safe),
    ("Milano", Safe),
    ("MirrorDimension", Safe),
    ("MonsterIsland", Safe),
    ("MuirIsland", Safe),
    ("Murderworld", Safe),
    ("Olympia", Safe),
    ("OscorpTower", Safe),
    ("PyramidOfRamaTut", Safe),
    ("RocketPad", Safe),
    ("SakaarTutorial", Safe),
    ("SavageLand", Safe),
    ("StarBrandCrater", Safe),
    ("StarkIndustries", Safe),
    ("StarkTower", Safe),
    ("StrangeAcademy", Safe),
    ("SubterraneaTutorial", Safe),
    ("TaLo", Safe),
    ("TheDeadlands", Safe),
    ("TheIceBox", Safe),
    ("ThunderboltsTower", Safe),
    ("WakandanEmbassy", Safe),
    ("Weirdworld", Safe),
    ("WhitePalace", Safe),
    ("WorldForge", Safe),
    ("Yggdrasil", Safe),
    ("ZennLa", Safe),
];

fn reviewed_may_affect(id: &str, table: &[(&str, Classification)]) -> bool {
    !matches!(table.iter().find(|(name, _)| *name == id), Some((_, Safe)))
}

fn visible_card_may_affect(id: &str) -> bool {
    cards::get(id).is_none() || reviewed_may_affect(id, VISIBLE_CARD_HOOKS)
}

fn location_may_affect(id: &str) -> bool {
    crate::locations::get(id).is_none() || reviewed_may_affect(id, LOCATION_HOOKS)
}

/// Explain unresolved ledger entries without treating a sampled ledger as a
/// complete inventory. In particular, a creator definition is not an effect
/// implementation or proof that a historical effect is still active.
pub fn provenance_reasons(provenance: &crate::position::Provenance) -> Vec<String> {
    if provenance.schema_version != 2 {
        return vec!["provenance schema is not version 2".into()];
    }
    let Some(ledger) = &provenance.ledger else {
        return vec!["provenance ledger missing".into()];
    };
    let mut reasons = Vec::new();
    if !ledger.complete_for_legality {
        if ledger.legality_reasons.is_empty() {
            reasons.push("provenance ledger incomplete".into());
        } else {
            reasons.extend(
                ledger
                    .legality_reasons
                    .iter()
                    .map(|reason| format!("provenance ledger: {reason}")),
            );
        }
    }
    for item in &ledger.items {
        if item.active == Some(false) || item.legality_relevant == Some(false) {
            continue;
        }
        if item.attribution != "exact" {
            reasons.push(format!(
                "provenance {}: attribution ambiguous",
                item.effect_key
            ));
        } else {
            reasons.push(format!(
                "provenance {}: effect mapping unverified",
                item.effect_key
            ));
        }
        reasons.extend(
            item.reasons
                .iter()
                .map(|reason| format!("provenance {}: {reason}", item.effect_key)),
        );
    }
    reasons
}

/// Return invalid for unknown, unreviewed, or potentially affecting hidden sources.
pub fn check_hidden_pool(inputs: &Inputs<'_>) -> Result {
    let mut reasons = Vec::new();
    match inputs.visible_cards {
        Known::Unknown => reasons.push("visible card pool is unknown".into()),
        Known::Known(ids) => {
            for id in ids {
                if visible_card_may_affect(id) {
                    reasons.push(format!("unknown or unreviewed visible card: {id}"));
                }
            }
        }
    }
    match inputs.active_locations {
        Known::Unknown => reasons.push("active locations are unknown".into()),
        Known::Known(ids) => {
            for id in ids {
                if location_may_affect(id) {
                    reasons.push(format!("unknown or unreviewed active location: {id}"));
                }
            }
        }
    }
    match inputs.hidden_cards {
        Known::Unknown => reasons.push("hidden card pool is unknown".into()),
        Known::Known(ids) => {
            for id in ids {
                match cards::get(id) {
                    None => reasons.push(format!("unknown hidden card: {id}")),
                    Some(def) if hidden_hook_may_affect(def) => reasons.push(format!(
                        "hidden card has affecting or unreviewed hooks: {id}"
                    )),
                    Some(_) => {}
                }
            }
        }
    }
    match inputs.copied_abilities {
        Known::Unknown => reasons.push("copied ability pool is unknown".into()),
        Known::Known(abilities) => {
            for (id, kind) in abilities {
                if ability_may_affect(*kind) {
                    reasons.push(format!(
                        "copied ability has unreviewed hooks: {id} ({kind:?})"
                    ));
                }
            }
        }
    }
    match inputs.attached_abilities {
        Known::Unknown => reasons.push("attached abilities are unknown".into()),
        Known::Known(abilities) => {
            for kind in abilities {
                if attached_may_affect(*kind) {
                    reasons.push("attached ability may affect legality".into());
                }
            }
        }
    }
    match inputs.card_cost_effects {
        Known::Unknown => reasons.push("card cost effects are unknown".into()),
        Known::Known(effects) => {
            for item in effects {
                if item.source.is_none() || card_cost_may_affect(item.effect) {
                    reasons.push(format!(
                        "card cost effect may affect legality; source: {:?}",
                        item.source
                    ));
                }
            }
        }
    }
    match inputs.player_cost_effects {
        Known::Unknown => reasons.push("player cost effects are unknown".into()),
        Known::Known(effects) => {
            for item in effects {
                if item.source.is_none() || player_cost_may_affect(item.effect) {
                    reasons.push(format!(
                        "player cost effect may affect legality; source: {:?}",
                        item.source
                    ));
                }
            }
        }
    }
    match inputs.state_effects {
        Known::Unknown => reasons.push("player/global effects are unknown".into()),
        Known::Known(effects) => {
            for item in effects {
                if item.source.is_none() || state_may_affect(item.effect) {
                    reasons.push(format!(
                        "player/global effect may affect legality; source: {:?}",
                        item.source
                    ));
                }
            }
        }
    }
    if matches!(inputs.move_source_provenance, Known::Unknown) {
        reasons.push("move source provenance is unknown".into());
    }
    if reasons.is_empty() {
        Result::Valid
    } else {
        Result::Invalid { reasons }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn provenance_items_remain_fail_closed_without_a_reviewed_mapping() {
        let provenance: crate::position::Provenance = serde_json::from_value(serde_json::json!({
            "schema_version": 2, "entities": [],
            "ledger": {"complete": false, "match_start_seen": false,
                "reasons": ["sampled_effects_unobservable"], "legality_reasons": ["sampled_effects_unobservable"], "items": [
                    {"effect_key": "a", "kind": "power", "target_entity_id": 1,
                     "creator_entity_id": 2, "creator_def_id": "Scorpion",
                     "created_turn": 2, "expiry_turn": null, "attribution": "exact",
                     "reasons": [], "active": true},
                    {"effect_key": "b", "kind": "cost", "target_entity_id": 1,
                     "creator_entity_id": null, "creator_def_id": "Beast",
                     "created_turn": null, "expiry_turn": null, "attribution": "ambiguous",
                     "reasons": ["multiple_sources"], "active": null},
                    {"effect_key": "c", "kind": "move_grant", "target_entity_id": 1,
                     "creator_entity_id": null, "creator_def_id": "Example",
                     "created_turn": null, "expiry_turn": null, "attribution": "ambiguous",
                     "reasons": [], "active": false}
                ]}
        }))
        .unwrap();
        let reasons = super::provenance_reasons(&provenance);
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("sampled_effects_unobservable"))
        );
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("a: effect mapping unverified"))
        );
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("b: attribution ambiguous"))
        );
        assert!(reasons.iter().any(|r| r.contains("b: multiple_sources")));
        assert!(!reasons.iter().any(|r| r.contains("c:")));
    }

    use super::*;
    #[test]
    fn reviewed_visible_hook_inventory_does_not_drift() {
        for &(id, classification) in VISIBLE_CARD_HOOKS {
            let d = cards::get(id).unwrap();
            if classification != Safe {
                continue;
            }
            assert_eq!(
                (d.reveal.is_some(), d.ongoing.is_some(), d.on_play.is_some()),
                match id {
                    "AntMan" | "KaZar" | "Klaw" | "BlueMarvel" | "CaptainAmerica" | "Goliath"
                    | "IronMan" | "Lizard" | "MrFantastic" | "Namor" | "ProfessorX"
                    | "Punisher" => (false, true, false),
                    "Squirrel"
                    | "Broodling"
                    | "Cyclops"
                    | "Demon"
                    | "Drone"
                    | "Hulk"
                    | "MistyKnight"
                    | "Monster"
                    | "NicoMinoru"
                    | "Raptor"
                    | "Rock"
                    | "Shocker"
                    | "SinisterClone"
                    | "Symbiote"
                    | "TheThing"
                    | "TigerSpirit"
                    | "ZombieHorde"
                    | "Quicksilver"
                    | "Stardust"
                    | "Attuma"
                    | "DraxAvatarOfLife"
                    | "Isca"
                    | "MrFantasticFirstSteps"
                    | "VampireKillmonger"
                    | "ScarletWitchQueenOfChaos"
                    | "SuperiorSpiderMan"
                    | "Armor"
                    | "Colossus"
                    | "Cosmo"
                    | "JJonahJameson"
                    | "Onslaught" => (false, false, false),
                    "Bishop" | "Angela" | "NicholasScratch" => (false, false, true),
                    "Surge" => (true, false, true),
                    _ => (true, false, false),
                },
                "{id}"
            );
            if classification == Safe {
                assert!(
                    d.can_play.is_none()
                        && d.ongoing_play_guard.is_none()
                        && d.global_play_guard.is_none()
                        && d.modify_final_cost.is_none()
                        && d.cost_guard.is_none()
                        && !d.grants_unrestricted_hand_plays
                        && d.grants_move_from.is_none()
                        && d.grants_move_globally.is_none()
                        && d.blocks_player_move.is_none()
                        && d.move_limit == 0
                        && d.on_activate.is_none()
                        && !d.blocks_expensive_plays
                        && d.blocks_spawns == (id == "Stardust")
                        && d.locks_lane == (id == "ProfessorX"),
                    "{id}"
                );
                assert!(
                    d.pre_game_start.is_some() == (id == "ThanosFracturedFrontier")
                        && d.in_hand_ongoing.is_none()
                        && d.late_in_hand_ongoing.is_none()
                        && d.conditional_in_hand_ongoing.is_none()
                        && d.objective_update.is_none()
                        && d.objective_freeze.is_none()
                        && d.late_ongoing.is_none()
                        && d.ongoing_ability
                            == matches!(
                                id,
                                "Colossus"
                                    | "JJonahJameson"
                                    | "Onslaught"
                                    | "SoulStone"
                                    | "SpaceStone"
                            )
                        && d.extra_ongoing_runs == u8::from(id == "Onslaught")
                        && d.reveal_repetitions.is_none()
                        && d.can_activate.is_none()
                        && d.activate_cost == 0
                        && !d.move_each_turn
                        && d.pre_end_turn.is_none()
                        && d.end_turn.is_some()
                            == matches!(
                                id,
                                "Attuma"
                                    | "DraxAvatarOfLife"
                                    | "Isca"
                                    | "MrFantasticFirstSteps"
                                    | "VampireKillmonger"
                            )
                        && !d.end_turn_in_hand
                        && d.end_turn_trigger == d.end_turn.is_some()
                        && !d.end_turn_trigger_hidden
                        && d.on_move_here.is_none()
                        && d.on_move.is_none()
                        && d.on_destroy.is_none()
                        && d.on_discard.is_none()
                        && d.on_hand_added.is_none()
                        && d.start_turn.is_some()
                            == matches!(id, "ScarletWitchQueenOfChaos" | "SuperiorSpiderMan")
                        && d.post_start_turn.is_none()
                        && d.on_event.is_none()
                        && !d.event_active_when_unrevealed
                        && d.on_added.is_none()
                        && d.initial_state.is_some()
                            == matches!(id, "ScarletWitchQueenOfChaos" | "Surge")
                        && d.clear_reveal_state.is_none()
                        && d.blocks_reveal == (id == "Cosmo")
                        && !d.conceals_owner_cards
                        && d.prevents_destroy == (id == "Armor")
                        && d.destroy_guard.is_none()
                        && !d.blocks_draw_in_hand
                        && d.immune_destroy_move == (id == "Colossus")
                        && d.start_in_hand == (id == "TenRings")
                        && d.draw_turn.is_none(),
                    "{id}"
                );
            }
        }
        for (index, (id, _)) in VISIBLE_CARD_HOOKS.iter().enumerate() {
            assert!(
                !VISIBLE_CARD_HOOKS[..index]
                    .iter()
                    .any(|(other, _)| other == id)
            );
        }
        for (index, (id, classification)) in LOCATION_HOOKS.iter().enumerate() {
            let d = crate::locations::get(id).unwrap();
            assert!(!LOCATION_HOOKS[..index].iter().any(|(other, _)| other == id));
            if *classification == Safe {
                assert!(
                    d.on_staging_end.is_none()
                        && d.on_event.is_none()
                        && d.on_card_moved.is_none()
                        && d.on_card_added.is_none()
                        && d.allows_play_with_count.is_none()
                        && d.allows_play_globally_with_count.is_none()
                        && !d.blocks_activate,
                    "{id}"
                );
                assert_eq!(
                    d.allows_play.is_some(),
                    matches!(
                        *id,
                        "CrimsonCosmos"
                            | "HellfireClub"
                            | "Kyln"
                            | "PitOfExile"
                            | "SanctumSanctorum"
                            | "TheBigHouse"
                            | "Vault"
                    ),
                    "{id}"
                );
                assert_eq!(
                    d.allows_play_elsewhere.is_some(),
                    *id == "AvengersCompound",
                    "{id}"
                );
                assert_eq!(
                    d.allows_addition.is_some(),
                    *id == "MiniaturizedLab",
                    "{id}"
                );
                assert_eq!(d.allows_move_from.is_some(), *id == "IslandPrison", "{id}");
                assert_eq!(d.grants_move_to.is_some(), *id == "NewYork", "{id}");
                assert_eq!(
                    d.grants_move_from.is_some(),
                    *id == "JeanGreySchool",
                    "{id}"
                );
                assert_eq!(
                    d.modify_cost.is_some(),
                    matches!(*id, "DreamDimension" | "Elysium" | "Titan" | "Utopia"),
                    "{id}"
                );
                assert_eq!(d.after_card_play.is_some(), *id == "Machineworld", "{id}");
                assert_eq!(d.energy_bonus.is_some(), *id == "ReedsLab", "{id}");
                assert!(
                    d.ongoing.is_none()
                        && d.conditional_ongoing.is_none()
                        && d.before_card_reveal.is_none()
                        && d.before_draw.is_none()
                        && d.prevents_retreat.is_none()
                        && d.initial_state.is_none()
                        && !d.no_destroy
                        && !d.delays_reveals
                        && !d.delays_reveals_until_end
                        && !d.disables_ongoing
                        && !d.disables_on_reveal
                        && d.reveal_energy == 0
                        && d.reveal_repeats == 1
                        && d.ongoing_repeats == 1
                        && d.minimum_turns == 0
                        && d.maximum_turns == 0
                        && !d.contributes_to_other_lanes,
                    "{id}"
                );
                assert_eq!(
                    d.on_turn_start.is_some(),
                    matches!(*id, "MirrorDimension" | "StarBrandCrater" | "WorldForge"),
                    "{id}"
                );
                assert_eq!(
                    d.on_reveal.is_some(),
                    matches!(
                        *id,
                        "CampLehigh"
                            | "CentralPark"
                            | "ChronosphereSphinx"
                            | "CollapsedMine"
                            | "Ebonshire"
                            | "GreatPortal"
                            | "HellsKitchen"
                            | "Milano"
                            | "MonsterIsland"
                            | "Olympia"
                            | "SakaarTutorial"
                            | "SavageLand"
                            | "SubterraneaTutorial"
                            | "TheIceBox"
                            | "WakandanEmbassy"
                            | "Weirdworld"
                            | "WhitePalace"
                            | "ZennLa"
                    ),
                    "{id}"
                );
                assert_eq!(
                    d.on_turn_end.is_some(),
                    matches!(
                        *id,
                        "Asgard"
                            | "Attilan"
                            | "Bifrost"
                            | "Boardwalk"
                            | "CaveOfTheDragon"
                            | "CelestialCircuit"
                            | "CollapsedMine"
                            | "EternityRange"
                            | "Genosha"
                            | "GrandCentral"
                            | "Hala"
                            | "Jotunheim"
                            | "Klyntar"
                            | "MuirIsland"
                            | "Murderworld"
                            | "OscorpTower"
                            | "PyramidOfRamaTut"
                            | "RocketPad"
                            | "StarkIndustries"
                            | "StarkTower"
                            | "StrangeAcademy"
                            | "TaLo"
                            | "TheDeadlands"
                            | "ThunderboltsTower"
                            | "Yggdrasil"
                    ),
                    "{id}"
                );
            }
        }
        assert!(location_may_affect("Raft"));
        assert!(visible_card_may_affect("Morph"));
        // Power Stone uses unexported play history, Sera can conceal permanent
        // cost under its floor, and movement/activation counters need provenance.
        for id in [
            "PowerStone",
            "Sera",
            "Nightcrawler",
            "SpiderManBrandNewDay",
            "JenniferKale",
            "KarimaShapandar",
        ] {
            assert!(visible_card_may_affect(id), "{id}");
        }
        assert!(location_may_affect("not-a-location"));
        assert!(visible_card_may_affect("not-a-card"));
    }

    #[test]
    fn every_hidden_hook_card_is_in_review_table() {
        let missing: Vec<_> = cards::registry()
            .filter(|def| {
                has_hidden_hooks(def) && !HIDDEN_HOOK_CARDS.iter().any(|(id, _, _)| *id == def.id)
            })
            .map(|def| def.id)
            .collect();
        assert!(missing.is_empty(), "unclassified hidden hooks: {missing:?}");
    }

    #[test]
    fn reviewed_classifications_control_hidden_admission() {
        for id in [
            "SheHulk",
            "Silk",
            "WilsonFisk",
            "Boomerang",
            "Wolverine",
            "Apocalypse",
            "Agamotto",
            "Stryfe",
            "Copycat",
        ] {
            assert!(!hidden_hook_may_affect(cards::get(id).unwrap()), "{id}");
        }
        for (index, (id, _, reason)) in HIDDEN_HOOK_CARDS.iter().enumerate() {
            assert!(!reason.is_empty(), "missing reason for {id}");
            assert!(
                !HIDDEN_HOOK_CARDS[..index]
                    .iter()
                    .any(|(other, _, _)| other == id),
                "duplicate {id}"
            );
        }
    }

    // Per-kind snapshots catch new hook fields on an already inventoried card.
    #[test]
    fn hidden_hook_fields_do_not_drift() {
        type HookGroup = (&'static str, fn(&CardDef) -> bool, &'static str);
        let groups: &[HookGroup] = &[
            (
                "pre_game_start",
                |d| d.pre_game_start.is_some(),
                "Arishem,ShangChiMasterOfTheRings,HighEvolutionary,Kang,Hulkling,ThanosFracturedFrontier,AgathaHarkness",
            ),
            (
                "in_hand_ongoing",
                |d| d.in_hand_ongoing.is_some(),
                "Mockingbird,Stature,SheHulk,EvolvedAbomination,Death,Skill01TheThingFirstStepsChampion,Sasquatch,GhostThunderbolts,SpiderSense,MonstroOctopus,TheGrandDesign,MilesMorales",
            ),
            (
                "late_in_hand_ongoing",
                |d| d.late_in_hand_ongoing.is_some(),
                "WilsonFisk",
            ),
            (
                "conditional_in_hand_ongoing",
                |d| d.conditional_in_hand_ongoing.is_some(),
                "ElderDragonOfTheMoon,Skaar",
            ),
            (
                "end_turn_in_hand",
                |d| d.end_turn_in_hand,
                "Makkari,EvolvedHulk",
            ),
            (
                "end_turn_trigger_hidden",
                |d| d.end_turn_trigger_hidden,
                "SheHulk",
            ),
            (
                "on_event",
                |d| d.on_event.is_some(),
                "WidowsKiss,Clea,EzekielSims,Venus,PolarisHorseman,CalibanHorseman,EnSabahNur,GambitHorseman,WolverineHorseman,SheHulk,Galacta,StormHorseman,Silk,ElsaBloodstone,MBaku,Xorn,Hercules,Batroc,Miek,Martyr,RocketAndGroot,SpiderPunk,SeleneHorseman,JuggernautHorseman,ColonelAmerica,ArchangelHorseman,ZombieVenom,BlackKnight,NegasonicTeenageWarhead,Dracula,Thanos,Firehair,CaptainMarvel,ZombieGalacti,JubileeSilverSurfer,ShadowlandsDaredevil,AntiVenomWebSlingers,AgonyWebSlingers,VampireDeaconFrost,PunisherWarMachine,ThanosChampion,WeaponXWolverineChampion,ManSpider,PeniParker,SinisterSix,StrangeSupreme,Namorita,Topaz,BlackCat,SebastianShaw,KittyPryde,DoctorOctopus,Agamotto,Dormammu,GalactusFirstSteps,Stryfe,SubMariner,Boomerang,JeffTheBabyDolphin,Scream,Copycat,WeaponXWolverine,RedHulk,HopeSummers,FrankieRaye,Surtur,Titania,Agony",
            ),
            (
                "on_discard",
                |d| d.on_discard.is_some(),
                "Marrow,ProximaMidnight,ZombieBlackCat,Apocalypse,Khonshu,KhonshuWaxing,WeaponH,Scorn,Swarm,Skill01GhostRiderChampion,Wolverine,MuramasaShard,X23,Helicarrier",
            ),
            (
                "on_hand_added",
                |d| d.on_hand_added.is_some(),
                "TheCollector",
            ),
            ("on_added", |d| d.on_added.is_some(), ""),
            (
                "blocks_draw_in_hand",
                |d| d.blocks_draw_in_hand,
                "WidowsBite",
            ),
        ];
        for (name, has_hook, expected) in groups {
            let mut actual: Vec<_> = cards::registry()
                .filter(|d| has_hook(d))
                .map(|d| d.id)
                .collect();
            let mut expected: Vec<_> = expected.split(',').filter(|id| !id.is_empty()).collect();
            actual.sort_unstable();
            expected.sort_unstable();
            assert_eq!(actual, expected, "hook inventory changed: {name}");
        }
    }

    // Adding a descriptor field requires an explicit hidden-dispatch review.
    #[test]
    fn descriptor_fields_require_review() {
        for def in cards::registry() {
            let CardDef {
                id: _,
                pre_game_start: _,
                reveal: _,
                ongoing: _,
                in_hand_ongoing: _,
                late_in_hand_ongoing: _,
                conditional_in_hand_ongoing: _,
                objective_update: _,
                objective_freeze: _,
                late_ongoing: _,
                modify_final_cost: _,
                cost_guard: _,
                ongoing_ability: _,
                grants_unrestricted_hand_plays: _,
                extra_ongoing_runs: _,
                reveal_repetitions: _,
                on_play: _,
                can_play: _,
                ongoing_play_guard: _,
                global_play_guard: _,
                on_activate: _,
                can_activate: _,
                activate_cost: _,
                move_limit: _,
                move_each_turn: _,
                grants_move_from: _,
                grants_move_globally: _,
                blocks_player_move: _,
                pre_end_turn: _,
                end_turn: _,
                end_turn_in_hand: _,
                end_turn_trigger: _,
                end_turn_trigger_hidden: _,
                on_move_here: _,
                on_move: _,
                on_destroy: _,
                on_discard: _,
                on_hand_added: _,
                start_turn: _,
                post_start_turn: _,
                on_event: _,
                event_active_when_unrevealed: _,
                on_added: _,
                initial_state: _,
                clear_reveal_state: _,
                blocks_reveal: _,
                conceals_owner_cards: _,
                prevents_destroy: _,
                destroy_guard: _,
                blocks_expensive_plays: _,
                blocks_draw_in_hand: _,
                blocks_spawns: _,
                locks_lane: _,
                immune_destroy_move: _,
                start_in_hand: _,
                draw_turn: _,
            } = def;
        }
        for def in crate::locations::REGISTRY {
            let crate::locations::LocationDef {
                id: _,
                power: _,
                reveal_energy: _,
                reveal_repeats: _,
                ongoing_repeats: _,
                no_destroy: _,
                blocks_activate: _,
                delays_reveals: _,
                delays_reveals_until_end: _,
                disables_ongoing: _,
                disables_on_reveal: _,
                on_reveal: _,
                on_turn_start: _,
                on_staging_end: _,
                on_turn_end: _,
                ongoing: _,
                conditional_ongoing: _,
                before_card_reveal: _,
                after_card_play: _,
                on_card_moved: _,
                on_card_added: _,
                on_event: _,
                allows_play: _,
                allows_play_with_count: _,
                allows_play_globally_with_count: _,
                allows_play_elsewhere: _,
                allows_addition: _,
                allows_move_from: _,
                before_draw: _,
                grants_move_to: _,
                grants_move_from: _,
                energy_bonus: _,
                modify_cost: _,
                prevents_retreat: _,
                minimum_turns: _,
                maximum_turns: _,
                contributes_to_other_lanes: _,
                initial_state: _,
            } = def;
        }
    }

    #[test]
    fn unknown_inputs_and_unreviewed_hooks_fail_closed() {
        let inputs = Inputs {
            active_locations: Known::Known(&[]),
            visible_cards: Known::Known(&[]),
            hidden_cards: Known::Known(&["Copycat"]),
            copied_abilities: Known::Known(&[("Copycat", AbilityKind::Ongoing)]),
            attached_abilities: Known::Known(&[]),
            card_cost_effects: Known::Known(&[]),
            player_cost_effects: Known::Known(&[]),
            state_effects: Known::Known(&[]),
            move_source_provenance: Known::Known(&[]),
        };
        assert!(matches!(check_hidden_pool(&inputs), Result::Invalid { .. }));
        let unknown = Inputs {
            hidden_cards: Known::Unknown,
            ..inputs
        };
        assert!(matches!(
            check_hidden_pool(&unknown),
            Result::Invalid { .. }
        ));
    }

    #[test]
    fn visible_cards_require_complete_reviewed_inventory() {
        let mut inputs = Inputs {
            active_locations: Known::Known(&[]),
            visible_cards: Known::Known(&[]),
            hidden_cards: Known::Known(&[]),
            copied_abilities: Known::Known(&[]),
            attached_abilities: Known::Known(&[]),
            card_cost_effects: Known::Known(&[]),
            player_cost_effects: Known::Known(&[]),
            state_effects: Known::Known(&[]),
            move_source_provenance: Known::Known(&[]),
        };
        assert_eq!(check_hidden_pool(&inputs), Result::Valid);
        inputs.visible_cards = Known::Known(&["AntMan", "Hawkeye", "Squirrel"]);
        inputs.active_locations = Known::Known(&["ReedsLab", "Machineworld"]);
        assert_eq!(check_hidden_pool(&inputs), Result::Valid);
        inputs.active_locations = Known::Known(&["Raft"]);
        assert!(matches!(check_hidden_pool(&inputs), Result::Invalid { .. }));
        inputs.active_locations = Known::Known(&[]);
        inputs.move_source_provenance = Known::Unknown;
        assert!(matches!(check_hidden_pool(&inputs), Result::Invalid { .. }));
        inputs.move_source_provenance = Known::Known(&[]);
        inputs.visible_cards = Known::Unknown;
        assert!(matches!(check_hidden_pool(&inputs), Result::Invalid { .. }));
        inputs.visible_cards = Known::Known(&["Wasp"]);
        assert!(matches!(check_hidden_pool(&inputs), Result::Invalid { .. }));
        inputs.visible_cards = Known::Known(&["not-a-card"]);
        assert!(matches!(check_hidden_pool(&inputs), Result::Invalid { .. }));
    }
}
