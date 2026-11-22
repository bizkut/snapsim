//! Coverage and hidden-information sampling metadata for public positions.
use crate::{
    cards, data, locations,
    position::{self, Position},
};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashSet};
use std::sync::LazyLock;

static RECRUIT_LOCATIONS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    data::locations()
        .into_iter()
        .filter(|d| {
            !d.carddefid.is_empty()
                && d.status == "released"
                && matches!(d.difficulty.as_str(), "Basic" | "Standard")
        })
        .map(|d| d.carddefid)
        .collect()
});

pub fn supported_cards() -> Vec<&'static str> {
    let mut ids: Vec<_> = cards::registry().map(|d| d.id).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}
pub fn supported_locations() -> Vec<&'static str> {
    let mut ids: Vec<_> = locations::REGISTRY.iter().map(|d| d.id).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}
pub fn supported() -> Value {
    json!({"cards": supported_cards(), "locations": supported_locations()})
}

fn opponent_candidates(position: &Position) -> (&'static str, Vec<String>) {
    let opponent = &position.opponent;
    let (source, pool) = if let Some(pool) = &opponent.deck_list {
        ("deck_list", pool.clone())
    } else if let Some(pool) = &opponent.pool {
        ("pool", pool.clone())
    } else if position.mode.as_deref() == Some("recruit")
        && !position::outside(opponent.seen.iter().chain(&opponent.known))
    {
        ("mode:recruit", position::recruit_cards())
    } else {
        ("all-supported", position::all_cards())
    };
    let excluded: HashSet<_> = opponent.seen.iter().chain(&opponent.known).collect();
    let mut remaining: Vec<_> = pool
        .into_iter()
        .filter(|id| !excluded.contains(id))
        .collect();
    remaining.sort();
    (source, remaining)
}

pub fn sampling_priors(position: &Position) -> Value {
    let (source, cards) = opponent_candidates(position);
    let revealed: HashSet<_> = position
        .locations
        .iter()
        .filter_map(|l| l.id.as_ref())
        .collect();
    let recruit = position.mode.as_deref() == Some("recruit")
        && revealed.iter().all(|id| RECRUIT_LOCATIONS.contains(*id));
    let loc_source = if recruit {
        "mode:recruit"
    } else {
        "all-supported"
    };
    let loc_count = position::all_locations(recruit)
        .iter()
        .filter(|id| !revealed.contains(id))
        .count();
    let missing = position
        .deck_count
        .unwrap_or(position.deck.len())
        .saturating_sub(position.deck.len());
    let (deck_source, deck_size) = if missing == 0 {
        ("listed", position.deck.len())
    } else {
        let known: HashSet<_> = position
            .hand
            .iter()
            .map(|c| &c.id)
            .chain(position.deck.iter())
            .chain(
                position
                    .locations
                    .iter()
                    .flat_map(|l| l.mine.iter().map(|c| &c.id)),
            )
            .collect();
        let recruit = position.mode.as_deref() == Some("recruit")
            && !position::outside(known.iter().copied());
        let pool = if recruit {
            position::recruit_cards()
        } else {
            position::all_cards()
        };
        (
            if recruit {
                "mode:recruit"
            } else {
                "all-supported"
            },
            pool.iter().filter(|id| !known.contains(id)).count(),
        )
    };
    json!({"opponent_cards": {"source":source,"size":cards.len(),"weighting":if source == "mode:recruit" && position.opponent.prior.as_deref() != Some("uniform") {"weighted"} else {"uniform"}}, "hidden_locations":{"source":loc_source,"size":loc_count}, "own_deck":{"source":deck_source,"size":deck_size}})
}

// Player-owned triggers survive their source. A card's staging turn is not a
// reveal turn (e.g. Dark Dimension), so only resolved action history can
// establish expiration or consumption.
fn installed_effect_possible(position: &Position, id: &str) -> bool {
    let actions = position
        .history
        .as_ref()
        .and_then(|history| history.actions.as_ref());
    let active = |entity: Option<i64>| {
        if entity.is_none() {
            return true;
        }
        let Some(action) = actions.and_then(|actions| {
            actions.iter().find(|action| {
                action.card_id == id
                    && action.kind == "play"
                    && action.resolved
                    && entity.is_none_or(|entity| action.card_entity_id == entity)
            })
        }) else {
            return true;
        };
        // A later re-reveal is not an action history entry. Only use the
        // original play's time if the observed latest resolution still agrees.
        let latest = position
            .locations
            .iter()
            .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
            .chain(position.graveyard.iter().flatten())
            .chain(position.banished.iter().flatten())
            .chain(position.opponent.graveyard.iter().flatten())
            .chain(position.opponent.banished.iter().flatten())
            .find(|card| card.entity_id == entity)
            .and_then(|card| card.latest_resolution_order);
        if latest.is_none() || latest != action.resolution_order {
            return true;
        }
        let Some(turn) = action.turn_resolved else {
            return true;
        };
        if matches!(id, "Wave" | "Psylocke") {
            return turn.saturating_add(1) >= position.turn;
        }
        let seat = if action.owner == "local" { 0 } else { 1 };
        let Some(history) = position.complete_past_actions(seat) else {
            return true;
        };
        let Some(order) = action.resolution_order else {
            return true;
        };
        !history.iter().any(|later| {
            later.kind == "play"
                && later.resolved
                && later
                    .turn_resolved
                    .zip(later.resolution_order)
                    .is_some_and(|(t, o)| (t, o) > (turn, order))
        })
    };
    let visible = position
        .locations
        .iter()
        .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
        .filter(|card| card.revealed && card.id == id)
        .chain(
            position
                .graveyard
                .iter()
                .flatten()
                .filter(|card| card.id == id),
        )
        .chain(
            position
                .banished
                .iter()
                .flatten()
                .filter(|card| card.id == id),
        )
        .chain(
            position
                .opponent
                .graveyard
                .iter()
                .flatten()
                .filter(|card| card.id == id),
        )
        .chain(
            position
                .opponent
                .banished
                .iter()
                .flatten()
                .filter(|card| card.id == id),
        );
    if visible.into_iter().any(|card| active(card.entity_id)) {
        return true;
    }
    if let Some(actions) = actions
        && actions
            .iter()
            .filter(|action| action.card_id == id && action.kind == "play" && action.resolved)
            .any(|action| active(Some(action.card_entity_id)))
    {
        return true;
    }
    position.opponent.seen.iter().any(|seen| seen == id)
        && !actions.is_some_and(|actions| {
            actions
                .iter()
                .any(|action| action.card_id == id && action.kind == "play" && action.resolved)
        })
        && !position
            .locations
            .iter()
            .flat_map(|lane| &lane.theirs)
            .any(|card| card.id == id)
}

// These registered effects can morph locations. The observation currently has
// no location-change event history or previous location identity.
fn location_change_evidence(position: &Position) -> bool {
    let changes = |id: &str| {
        matches!(
            id,
            "Magik" | "ScarletWitch" | "RealityStone" | "NicoMinoru" | "OmniversalPresence"
        )
    };
    position.locations.iter().any(|lane| {
        lane.mine
            .iter()
            .chain(&lane.theirs)
            .any(|card| card.revealed && changes(&card.id))
            || lane
                .id
                .as_deref()
                .is_some_and(|id| matches!(id, "WorldForge" | "LosDiablos"))
    }) || position
        .graveyard
        .iter()
        .flatten()
        .chain(position.banished.iter().flatten())
        .chain(position.opponent.graveyard.iter().flatten())
        .chain(position.opponent.banished.iter().flatten())
        .any(|card| changes(&card.id))
        || position.opponent.seen.iter().any(|id| changes(id))
        || position
            .history
            .as_ref()
            .and_then(|history| history.actions.as_ref())
            .is_some_and(|actions| {
                actions
                    .iter()
                    .any(|action| action.resolved && changes(&action.card_id))
            })
}

fn text_change_evidence(position: &Position) -> bool {
    let changes = |id: &str| {
        matches!(
            id,
            "Enchantress"
                | "Echo"
                | "DeafeningChord"
                | "IronLad"
                | "NicoMinoru"
                | "SymbioteSpiderMan"
                | "TaoMandala"
                | "AncientOne"
                | "AstralProjection"
                | "TechnoOrganicVirus"
                | "Magus"
        )
    };
    position.locations.iter().any(|lane| {
        lane.mine
            .iter()
            .chain(&lane.theirs)
            .any(|card| card.revealed && changes(&card.id))
    }) || position
        .graveyard
        .iter()
        .flatten()
        .chain(position.banished.iter().flatten())
        .chain(position.opponent.graveyard.iter().flatten())
        .chain(position.opponent.banished.iter().flatten())
        .any(|card| changes(&card.id))
        || position.opponent.seen.iter().any(|id| changes(id))
        || position
            .history
            .as_ref()
            .and_then(|history| history.actions.as_ref())
            .is_some_and(|actions| {
                actions
                    .iter()
                    .any(|action| action.resolved && changes(&action.card_id))
            })
}
// Persistent effects survive their source and may only be evidenced by history.
fn resolved_source_possible(position: &Position, id: &str) -> bool {
    position
        .locations
        .iter()
        .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
        .any(|card| card.id == id && card.revealed)
        || position
            .graveyard
            .iter()
            .flatten()
            .chain(position.banished.iter().flatten())
            .chain(position.opponent.graveyard.iter().flatten())
            .chain(position.opponent.banished.iter().flatten())
            .any(|card| card.id == id)
        || position.opponent.seen.iter().any(|seen| seen == id)
        || position
            .history
            .as_ref()
            .and_then(|history| history.actions.as_ref())
            .is_some_and(|actions| {
                actions
                    .iter()
                    .any(|action| action.card_id == id && action.kind == "play" && action.resolved)
            })
}

pub fn coverage(position: &Position) -> Value {
    let mut card_ids: BTreeSet<&str> = position
        .hand
        .iter()
        .map(|c| c.id.as_str())
        .chain(position.deck.iter().map(String::as_str))
        .chain(
            position
                .locations
                .iter()
                .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
                .map(|card| card.id.as_str()),
        )
        .chain(position.opponent.seen.iter().map(String::as_str))
        .chain(position.opponent.known.iter().map(String::as_str))
        .chain(
            position
                .opponent
                .deck_list
                .iter()
                .flatten()
                .map(String::as_str),
        )
        .chain(position.opponent.pool.iter().flatten().map(String::as_str))
        .collect();
    let mut loc_ids = BTreeSet::new();
    let mut state = BTreeSet::new();
    let mut extra_state = BTreeSet::new();
    // TheGrandDesignBehavior::DiscountSelf 0x7d1800c counts historic
    // played cards by creation provenance, absent from game.history.
    if !cards::reference_rules_enabled()
        && position.turn > 1
        && (position.hand.iter().any(|card| card.id == "TheGrandDesign")
            || position.deck.iter().any(|id| id == "TheGrandDesign")
            || position
                .opponent
                .known
                .iter()
                .chain(&position.opponent.seen)
                .chain(position.opponent.deck_list.iter().flatten())
                .chain(position.opponent.pool.iter().flatten())
                .any(|id| id == "TheGrandDesign"))
    {
        extra_state.insert("The Grand Design created-card play history not exported".to_owned());
    }
    if !cards::reference_rules_enabled()
        && position.turn > 1
        && ["GhostRider", "Firelord", "FenrisWolf"]
            .iter()
            .any(|id| card_ids.contains(id))
    {
        // GhostRider 0x7eb3e70, Firelord 0x7ea9c7c and FenrisWolf
        // 0x7ea8928 select from discarded/destroyed zone inventories.
        extra_state.insert(
            "complete discarded and destroyed card zone inventories not exported".to_owned(),
        );
    }
    if !cards::reference_rules_enabled() {
        if position.turn > 1
            && ["Khonshu", "KhonshuWaxing", "KhonshuFull"]
                .iter()
                .any(|id| card_ids.contains(id))
        {
            extra_state
                .insert("Khonshu complete discarded-card zone inventory not exported".to_owned());
        }
        // Client Location::CanCardChangeOwner (0x7ddc3b4) consults
        // CanChangeCardOwnerHere; its active modifier state is not observed.
        for id in [
            "Viper",
            "Annihilus",
            "GreenGoblin",
            "Hobgoblin",
            "AcidArrow",
        ] {
            if card_ids.contains(id) {
                extra_state.insert(format!(
                    "{id} CanChangeCardOwnerHere restriction state not exported"
                ));
            }
        }
        for id in ["Electro", "Wiccan", "Arishem"] {
            if position.turn > 1 && resolved_source_possible(position, id) {
                extra_state.insert(format!("{id} prior permanent energy update not exported"));
            }
        }
        if position.turn > 1 && card_ids.contains("HydraStomper") {
            extra_state
                .insert("Hydra Stomper player movement count history not exported".to_owned());
        }
        if position.turn > 1 && card_ids.contains("Stryfe") {
            extra_state.insert("Stryfe play-time On Reveal history not exported".to_owned());
        }
        if position
            .locations
            .iter()
            .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
            .any(|card| card.revealed && card.id == "Batroc")
        {
            extra_state
                .insert("Batroc current-turn consumed trigger state not exported".to_owned());
        }
        if position.turn > 1 && resolved_source_possible(position, "Arana") {
            extra_state.insert("Arana pending next-card move trigger not exported".to_owned());
        }
        if resolved_source_possible(position, "Miek") {
            extra_state.insert("Miek attached movement grant not exported".to_owned());
        }
        if card_ids.contains("AuntMay") {
            extra_state
                .insert("Aunt May current-turn resolved movement history not exported".to_owned());
        }
        if resolved_source_possible(position, "Mysterio") {
            extra_state.insert("Mysterio linked illusion entity not exported".to_owned());
        }
        if resolved_source_possible(position, "Redwing") {
            extra_state.insert("Redwing consumed movement trigger not exported".to_owned());
        }
        if resolved_source_possible(position, "Sparky") {
            extra_state
                .insert("Sparky last moved entity and previous zone not exported".to_owned());
        }
        if card_ids.contains("ArchangelHorseman") && position.turn > 1 {
            extra_state.insert("Archangel Horseman objective state not restored".to_owned());
        }
        for (id, name) in [("Cloak", "Cloak"), ("Hellion", "Hellion")] {
            // Cloak 0x7e81254 and Hellion 0x7ec0a74 install player-owned grants.
            if position.turn > 1 && resolved_source_possible(position, id) {
                extra_state.insert(format!("{name} prior player movement grant not exported"));
            }
        }
        if resolved_source_possible(position, "Quinjet")
            && position.hand.iter().any(|card| card.created.is_none())
        {
            // QuinjetBehavior::NewRecruitsCostOneLess 0x7f1e884.
            extra_state.insert("Quinjet hand card creation provenance not exported".to_owned());
        }
        if position.turn > 1 && resolved_source_possible(position, "CorvusGlaive") {
            // CorvusGlaiveBehavior::DiscardCards 0x7e83c18.
            extra_state
                .insert("Corvus Glaive prior permanent energy grant not exported".to_owned());
        }
        if position.turn > 1 && resolved_source_possible(position, "FallenOne") {
            // FallenOneBehavior::SetMaxEnergy 0x7ea68d4.
            extra_state.insert("Fallen One prior permanent energy set not exported".to_owned());
        }
        if position.turn > 1 && resolved_source_possible(position, "Havok") {
            // HavokBehavior 0x7ebaf68 permanently reduces maximum energy.
            extra_state.insert("Havok prior permanent energy reductions not exported".to_owned());
        }
        if resolved_source_possible(position, "SpiderManNoir") {
            // SpiderManNoirBehavior::SetTopCardPowerAndCheckForMove 0x7f4d494.
            extra_state.insert(
                "Spider-Man Noir affected deck card identity and power not exported".to_owned(),
            );
        }
        if position.turn <= 6 && resolved_source_possible(position, "BaronMordo") {
            // BaronMordoBehavior::AddCostSetAbilityToPlayer 0x7e6eaac.
            extra_state.insert("Baron Mordo affected deck card identity not exported".to_owned());
        }
        if resolved_source_possible(position, "ToxieDoxie") && position.turn > 1 {
            // ToxieDoxieBehavior::BuffCardsAndLockOut 0x7ce066c.
            extra_state.insert("Toxie Doxie locked hand card identities not exported".to_owned());
        }
        if resolved_source_possible(position, "BlackSwan") && position.turn > 1 {
            extra_state.insert("Black Swan player-owned hand cost effect not exported".to_owned());
        }
        if position.turn > 1 && resolved_source_possible(position, "Malekith") {
            extra_state.insert(
                "Malekith hidden target and game-end reveal attachment not exported".to_owned(),
            );
        }
        if position.turn > 1 && resolved_source_possible(position, "Lockjaw") {
            extra_state.insert("Lockjaw pending next-resolved-play swap not exported".to_owned());
        }
        if [
            "SummoningRitual01Dormammu",
            "SummoningRitual02Dormammu",
            "SummoningRitual03Dormammu",
        ]
        .iter()
        .any(|id| card_ids.contains(id))
            || position.turn > 1 && resolved_source_possible(position, "Dormammu")
        {
            extra_state
                .insert("Dormammu ritual source entity and merged state not exported".to_owned());
        }
        if position.turn > 1
            && ["GrappleArrow", "BasicArrow", "KateBishop"]
                .iter()
                .any(|id| resolved_source_possible(position, id))
        {
            extra_state.insert(
                "Kate Bishop pending arrow effects or hidden opponent arrows not exported"
                    .to_owned(),
            );
        }
        if card_ids.contains("Agamotto") && position.turn > 1 {
            // AgamottoBehavior::AddSpells 0x7e5ff44.
            extra_state.insert("Agamotto shuffled spell deck composition not exported".to_owned());
        }
        if position.turn > 1 && card_ids.contains("GalactusFirstSteps") {
            // GalactusFirstStepsBehavior::ChooseHerald 0x7eaebcc.
            extra_state.insert(
                "Galactus First Steps herald entity and trigger state not exported".to_owned(),
            );
        }
        if position.turn > 1 && card_ids.contains("Stryfe") {
            // StryfeBehavior::CalculateObjectiveProgress 0x7cca080.
            extra_state.insert("Stryfe objective progress and completion not restored".to_owned());
        }
        if position
            .locations
            .iter()
            .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
            .any(|card| card.id == "Hercules" && card.revealed)
        {
            // HerculesBehavior::MoveAfterMove 0x7ec3380.
            extra_state.insert("Hercules current-turn trigger consumption not exported".to_owned());
        }
        if position
            .locations
            .iter()
            .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
            .any(|card| {
                card.id == "Quicksand"
                    && card.revealed
                    && card.played_turn.is_none_or(|turn| turn < position.turn)
            })
        {
            // QuicksandBehavior 0x7f1dfa8 stores whether its trigger was used.
            extra_state.insert("Quicksand prior trigger consumption not exported".to_owned());
        }
        let own_loki = position.hand.iter().any(|card| card.id == "Loki")
            || position.deck.iter().any(|id| id == "Loki");
        let opponent_loki = position
            .opponent
            .known
            .iter()
            .chain(&position.opponent.seen)
            .chain(position.opponent.deck_list.iter().flatten())
            .chain(position.opponent.pool.iter().flatten())
            .any(|id| id == "Loki");
        if (own_loki && position.opponent.deck_list.is_none()) || opponent_loki {
            extra_state.insert("Loki target starting deck identities not exported".to_owned());
        }
        if resolved_source_possible(position, "Loki") {
            extra_state
                .insert("Loki replaced deck composition and card costs not exported".to_owned());
        }
        let victoria = resolved_source_possible(position, "VictoriaHand")
            || position.hand.iter().any(|card| card.id == "VictoriaHand")
            || position.deck.iter().any(|id| id == "VictoriaHand");
        if victoria && position.turn > 1 {
            extra_state.insert("Victoria Hand created card creation zone not exported".to_owned());
        }
        if card_ids.contains("MaryJane") {
            // MaryJaneBehavior::BoostCards 0x7ef4a34 inspects all moved entities.
            for seat in 0..2 {
                if position.current_action_present(seat) != Some(false) {
                    extra_state.insert(format!(
                        "Mary Jane seat {seat} current-turn moved card history unknown"
                    ));
                }
            }
        }
    }
    if !cards::reference_rules_enabled() && position.turn > 1 && location_change_evidence(position)
    {
        extra_state.insert(
            "a location may have been replaced (pending location grants unknown)".to_owned(),
        );
    }
    if !cards::reference_rules_enabled() && position.turn > 1 {
        let previous = position.previous_turn_locations.as_ref();
        let grant_location = |id: &str| matches!(id, "Feast" | "AltarOfDeath");
        let previously_present = previous
            .is_some_and(|history| history.ids.iter().flatten().any(|id| grant_location(id)));
        let visible = position
            .locations
            .iter()
            .any(|lane| lane.id.as_deref().is_some_and(grant_location));
        if previously_present
            || previous.is_none()
            || (visible && previous.is_some_and(|h| !h.complete))
        {
            extra_state.insert(
                "Feast/AltarOfDeath prior-turn pending player energy grants unknown".to_owned(),
            );
        }
    }
    // These client abilities query historical events, not the mutable zones:
    // Morbius 0x7f01d3c, Colonel America 0x7e82934,
    // Archangel Horseman 0x7e697a0, and Death 0x7e89764.
    if !cards::reference_rules_enabled() && position.turn > 1 {
        let relevant = |id: &str| {
            card_ids.contains(id)
                || position.locations.iter().any(|lane| {
                    lane.mine
                        .iter()
                        .chain(&lane.theirs)
                        .any(|card| card.id == id)
                })
        };
        if ["Morbius", "ColonelAmerica", "WildChild"]
            .iter()
            .any(|id| relevant(id))
        {
            extra_state.insert("complete discard event history not exported".to_owned());
        }
        if ["ArchangelHorseman", "Death", "WildChild"]
            .iter()
            .any(|id| relevant(id))
        {
            extra_state.insert("complete destruction event history not exported".to_owned());
        }
    }
    // AgentVenomBehavior::SetPowerOfDeck 0x7e61d30 changes individual
    // deck-card power, which the observation's deck ID list does not retain.
    if !cards::reference_rules_enabled()
        && (position
            .locations
            .iter()
            .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
            .any(|card| card.id == "AgentVenom" && card.revealed)
            || position
                .graveyard
                .iter()
                .flatten()
                .chain(position.banished.iter().flatten())
                .chain(position.opponent.graveyard.iter().flatten())
                .chain(position.opponent.banished.iter().flatten())
                .any(|card| card.id == "AgentVenom")
            || position.opponent.seen.iter().any(|id| id == "AgentVenom")
            || position
                .history
                .as_ref()
                .and_then(|history| history.actions.as_ref())
                .is_some_and(|actions| {
                    actions
                        .iter()
                        .any(|action| action.card_id == "AgentVenom" && action.resolved)
                }))
    {
        extra_state.insert("Agent Venom deck card power modifications not exported".to_owned());
    }
    // HydroManBehavior::SetTopCardCost 0x7ec93cc mutates both hidden deck tops.
    if !cards::reference_rules_enabled()
        && (position
            .locations
            .iter()
            .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
            .any(|card| card.id == "HydroMan" && card.revealed)
            || position
                .graveyard
                .iter()
                .flatten()
                .chain(position.banished.iter().flatten())
                .chain(position.opponent.graveyard.iter().flatten())
                .chain(position.opponent.banished.iter().flatten())
                .any(|card| card.id == "HydroMan")
            || position.opponent.seen.iter().any(|id| id == "HydroMan")
            || position
                .history
                .as_ref()
                .and_then(|history| history.actions.as_ref())
                .is_some_and(|actions| {
                    actions
                        .iter()
                        .any(|action| action.card_id == "HydroMan" && action.resolved)
                }))
    {
        extra_state.insert("Hydro-Man modified deck top identity and cost not exported".to_owned());
    }
    // MrNegativeBehavior::SwapPowerAndCost 0x7f04ce0 mutates hidden deck cards.
    if !cards::reference_rules_enabled()
        && (position
            .locations
            .iter()
            .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
            .any(|card| card.id == "MrNegative" && card.revealed)
            || position
                .graveyard
                .iter()
                .flatten()
                .chain(position.banished.iter().flatten())
                .chain(position.opponent.graveyard.iter().flatten())
                .chain(position.opponent.banished.iter().flatten())
                .any(|card| card.id == "MrNegative")
            || position.opponent.seen.iter().any(|id| id == "MrNegative")
            || position
                .history
                .as_ref()
                .and_then(|history| history.actions.as_ref())
                .is_some_and(|actions| {
                    actions
                        .iter()
                        .any(|action| action.card_id == "MrNegative" && action.resolved)
                }))
    {
        extra_state
            .insert("Mr Negative deck card cost and power modifications not exported".to_owned());
    }
    // MonstroOctopusBehavior::ReduceCost 0x7efec0c reads the ordered deck top.
    if !cards::reference_rules_enabled() && position.turn > 1 && card_ids.contains("MonstroOctopus")
    {
        extra_state.insert("Monstro Octopus deck top identity and cost not exported".to_owned());
    }
    // GhostSpiderBehavior::MoveCard 0x7eb473c uses the player's last play,
    // including one whose card has since left the board.
    if !cards::reference_rules_enabled() && position.turn > 1 {
        for (seat, relevant) in [
            (
                0,
                position.hand.iter().any(|card| card.id == "GhostSpider")
                    || position.deck.iter().any(|id| id == "GhostSpider"),
            ),
            (
                1,
                position
                    .opponent
                    .known
                    .iter()
                    .chain(&position.opponent.seen)
                    .chain(position.opponent.deck_list.iter().flatten())
                    .chain(position.opponent.pool.iter().flatten())
                    .any(|id| id == "GhostSpider"),
            ),
        ] {
            if relevant
                && (position.complete_past_actions(seat).is_none()
                    || position.current_action_present(seat) != Some(false))
            {
                extra_state.insert(format!(
                    "Ghost Spider seat {seat} last played card history unknown"
                ));
            }
        }
        // AeroBehavior 0x7e5fb48 reads the opposing player's last play.
        let own_aero = position.hand.iter().any(|card| card.id == "Aero")
            || position.deck.iter().any(|id| id == "Aero");
        let enemy_aero = position
            .opponent
            .known
            .iter()
            .chain(&position.opponent.seen)
            .chain(position.opponent.deck_list.iter().flatten())
            .chain(position.opponent.pool.iter().flatten())
            .any(|id| id == "Aero");
        for (target, relevant) in [(1, own_aero), (0, enemy_aero)] {
            if relevant
                && (position.complete_past_actions(target).is_none()
                    || position.current_action_present(target) != Some(false))
            {
                extra_state.insert(format!(
                    "Aero seat {target} last played card history unknown"
                ));
            }
        }
    }
    // ThenaBehavior::BuffSelf 0x7cdaa4c counts committed current-turn
    // staged plays; unresolved or absent history cannot be restored.
    if !cards::reference_rules_enabled() {
        for (seat, relevant) in [
            (
                0,
                position.hand.iter().any(|card| card.id == "Thena")
                    || position.deck.iter().any(|id| id == "Thena")
                    || position
                        .locations
                        .iter()
                        .any(|lane| lane.mine.iter().any(|card| card.id == "Thena")),
            ),
            (
                1,
                position
                    .opponent
                    .known
                    .iter()
                    .chain(&position.opponent.seen)
                    .chain(position.opponent.deck_list.iter().flatten())
                    .chain(position.opponent.pool.iter().flatten())
                    .any(|id| id == "Thena")
                    || position
                        .locations
                        .iter()
                        .any(|lane| lane.theirs.iter().any(|card| card.id == "Thena")),
            ),
        ] {
            if relevant && position.current_action_present(seat) != Some(false) {
                extra_state.insert(format!(
                    "Thena seat {seat} current-turn staged play history unknown"
                ));
            }
        }
    }
    // SubterraneaBehavior::AddRocks 0x7d84dbc shuffles generated cards into
    // both decks. The opponent's resulting hidden deck composition is not exported.
    if !cards::reference_rules_enabled()
        && position.turn > 1
        && (position.locations.iter().any(|lane| {
            lane.id.as_deref() == Some("Subterranea")
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
        }) || position
            .previous_turn_locations
            .as_ref()
            .is_some_and(|history| history.ids.iter().flatten().any(|id| id == "Subterranea")))
    {
        extra_state
            .insert("Subterranea shuffled opponent deck composition not exported".to_owned());
    }
    // LechuguillaBehavior::ShuffleRocks 0x7d618d0 can change either
    // hidden deck after a resolved play, including cards that left the board.
    if !cards::reference_rules_enabled()
        && position.turn > 1
        && position.locations.iter().any(|lane| {
            lane.id.as_deref() == Some("Lechuguilla")
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
        })
    {
        extra_state.insert("Lechuguilla shuffled deck composition not exported".to_owned());
    }
    if !cards::reference_rules_enabled()
        && position.turn > 1
        && position.locations.iter().any(|lane| {
            lane.id.as_deref() == Some("VibraniumMines")
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
        })
    {
        extra_state.insert("Vibranium Mines shuffled deck composition not exported".to_owned());
    }
    if !cards::reference_rules_enabled()
        && position.turn > 1
        && resolved_source_possible(position, "Djinn")
    {
        extra_state.insert("Djinn pending next-turn energy grant not exported".to_owned());
    }
    if !cards::reference_rules_enabled()
        && position.locations.iter().any(|lane| {
            lane.id.as_deref() == Some("Panoptichron")
                && lane
                    .mine
                    .iter()
                    .chain(&lane.theirs)
                    .any(|card| card.revealed && card.created.is_none())
        })
    {
        extra_state.insert("Panoptichron card creation provenance not exported".to_owned());
    }
    if !cards::reference_rules_enabled()
        && position.turn > 1
        && position.locations.iter().any(|lane| {
            lane.id.as_deref() == Some("BlackVortex")
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
        })
    {
        // BlackVortexBehavior 0x7d407d4 uses per-player consumed flags.
        extra_state.insert("Black Vortex per-player transformation flags not exported".to_owned());
    }
    if !cards::reference_rules_enabled()
        && position
            .locations
            .iter()
            .any(|lane| lane.id.as_deref() == Some("SacredTimeline"))
    {
        extra_state.insert("Sacred Timeline starting hand identity not exported".to_owned());
    }
    // Stones may originate from Thanos even when Thanos himself is not visible.
    // The opponent's post-shuffle deck composition is not exported.
    let stone = |id: &str| {
        matches!(
            id,
            "SpaceStone" | "TimeStone" | "RealityStone" | "PowerStone" | "MindStone" | "SoulStone"
        )
    };
    let own_stone = position.hand.iter().any(|card| stone(&card.id))
        || position.deck.iter().any(|id| stone(id))
        || position
            .graveyard
            .iter()
            .flatten()
            .any(|card| stone(&card.id))
        || position
            .banished
            .iter()
            .flatten()
            .any(|card| stone(&card.id))
        || position
            .locations
            .iter()
            .any(|lane| lane.mine.iter().any(|card| stone(&card.id)));
    let opposing_stone = position
        .opponent
        .seen
        .iter()
        .chain(&position.opponent.known)
        .chain(position.opponent.deck_list.iter().flatten())
        .chain(position.opponent.pool.iter().flatten())
        .any(|id| stone(id))
        || position
            .opponent
            .graveyard
            .iter()
            .flatten()
            .any(|card| stone(&card.id))
        || position
            .opponent
            .banished
            .iter()
            .flatten()
            .any(|card| stone(&card.id))
        || position
            .locations
            .iter()
            .any(|lane| lane.theirs.iter().any(|card| stone(&card.id)));
    if !cards::reference_rules_enabled()
        && ((own_stone && position.deck_count != Some(position.deck.len())) || opposing_stone)
    {
        extra_state
            .insert("Infinity Stone owner's post-shuffle deck composition unknown".to_owned());
    }
    // ThorBehavior::AddMjolnir (0x7cdcc10) changes deck
    // composition, which is not exported for the opponent.
    if !cards::reference_rules_enabled() {
        let own_thor = position.locations.iter().any(|lane| {
            lane.mine
                .iter()
                .any(|card| card.id == "Thor" && card.revealed)
        }) || position
            .graveyard
            .iter()
            .flatten()
            .any(|card| card.id == "Thor")
            || position
                .banished
                .iter()
                .flatten()
                .any(|card| card.id == "Thor")
            || position
                .history
                .as_ref()
                .and_then(|history| history.actions.as_ref())
                .is_some_and(|actions| {
                    actions.iter().any(|action| {
                        action.owner == "local"
                            && action.card_id == "Thor"
                            && action.kind == "play"
                            && action.resolved
                    })
                });
        let opposing_thor = position.locations.iter().any(|lane| {
            lane.theirs
                .iter()
                .any(|card| card.id == "Thor" && card.revealed)
        }) || position.opponent.seen.iter().any(|id| id == "Thor");
        if (own_thor && position.deck_count != Some(position.deck.len())) || opposing_thor {
            extra_state.insert("Thor Mjolnir post-shuffle deck composition unknown".to_owned());
        }
        let widow_seen = position.locations.iter().any(|lane| {
            lane.mine
                .iter()
                .any(|card| card.id == "BlackWidow" && card.revealed)
        }) || position
            .graveyard
            .iter()
            .flatten()
            .any(|card| card.id == "BlackWidow")
            || position
                .banished
                .iter()
                .flatten()
                .any(|card| card.id == "BlackWidow")
            || position
                .history
                .as_ref()
                .and_then(|history| history.actions.as_ref())
                .is_some_and(|actions| {
                    actions
                        .iter()
                        .any(|action| action.owner == "local" && action.card_id == "BlackWidow")
                });
        if position.opponent.hand > 0 && widow_seen {
            extra_state.insert("Black Widow opponent WidowsBite hand fate unknown".to_owned());
        }
    }
    // Darkhawk counts cards, not identities: a reported own deck count is
    // sufficient even when the deck composition is sampled.
    if !cards::reference_rules_enabled()
        && position.deck_count.is_none()
        && (position
            .opponent
            .seen
            .iter()
            .chain(&position.opponent.known)
            .chain(position.opponent.deck_list.iter().flatten())
            .chain(position.opponent.pool.iter().flatten())
            .any(|id| id == "Darkhawk")
            || position
                .locations
                .iter()
                .any(|lane| lane.theirs.iter().any(|card| card.id == "Darkhawk")))
    {
        extra_state.insert("Darkhawk opponent deck count unknown".to_owned());
    }
    if !cards::reference_rules_enabled() {
        let own_cobra_in_hand = position.hand.iter().any(|card| card.id == "Cobra");
        let unknown_target = |cards: &[position::CardRef]| {
            cards
                .iter()
                .any(|card| card.revealed && card.created.is_none())
        };
        if position.locations.iter().any(|lane| {
            ((own_cobra_in_hand
                || lane
                    .mine
                    .iter()
                    .any(|card| card.id == "Cobra" && card.revealed))
                && unknown_target(&lane.theirs))
                || (lane
                    .theirs
                    .iter()
                    .any(|card| card.id == "Cobra" && card.revealed)
                    && unknown_target(&lane.mine))
        }) {
            extra_state.insert("Cobra target created provenance unknown".to_owned());
        }
    }
    if !cards::reference_rules_enabled() {
        let own = position.hand.iter().any(|card| card.id == "Diamondback")
            || position.deck.iter().any(|id| id == "Diamondback");
        let opponent = position
            .opponent
            .known
            .iter()
            .chain(&position.opponent.seen)
            .chain(position.opponent.deck_list.iter().flatten())
            .chain(position.opponent.pool.iter().flatten())
            .any(|id| id == "Diamondback");
        let unknown =
            |card: &position::CardRef| card.revealed && !card.has_complete_power_modifier_history();
        if position.locations.iter().any(|lane| {
            (own && lane.theirs.iter().any(unknown)) || (opponent && lane.mine.iter().any(unknown))
        }) {
            extra_state.insert("Diamondback target power modifier history incomplete".to_owned());
        }
    }
    if !cards::reference_rules_enabled() {
        let own = position.hand.iter().any(|card| card.id == "Patriot")
            || position.deck.iter().any(|id| id == "Patriot")
            || position
                .locations
                .iter()
                .any(|lane| lane.mine.iter().any(|card| card.id == "Patriot"));
        if own && text_change_evidence(position) {
            extra_state.insert("card text overrides not exported".to_owned());
        }
        let opponent = position
            .opponent
            .known
            .iter()
            .chain(&position.opponent.seen)
            .chain(position.opponent.deck_list.iter().flatten())
            .chain(position.opponent.pool.iter().flatten())
            .any(|id| id == "Patriot")
            || position
                .locations
                .iter()
                .any(|lane| lane.theirs.iter().any(|card| card.id == "Patriot"));
        let unknown = |card: &position::CardRef| {
            card.revealed && cards::no_text::has_no_text(&card.id).is_none()
        };
        if position.locations.iter().any(|lane| {
            (own && lane.mine.iter().any(unknown)) || (opponent && lane.theirs.iter().any(unknown))
        }) {
            extra_state.insert("Patriot target client behavior definition unknown".to_owned());
        }
    }
    if !cards::reference_rules_enabled() {
        for id in ["Wave", "Stick", "Psylocke"] {
            if installed_effect_possible(position, id) {
                extra_state.insert(format!("{id} effect may be active"));
            }
        }
    }
    if !cards::reference_rules_enabled() && position.turn > 1 {
        let own_infinaut = position.hand.iter().any(|card| card.id == "Infinaut")
            || position.deck.iter().any(|id| id == "Infinaut");
        let opposing_infinaut = position
            .opponent
            .known
            .iter()
            .chain(&position.opponent.seen)
            .chain(position.opponent.deck_list.iter().flatten())
            .chain(position.opponent.pool.iter().flatten())
            .any(|id| id == "Infinaut");
        for (seat, relevant) in [(0, own_infinaut), (1, opposing_infinaut)] {
            let complete = position.history.as_ref().is_some_and(|history| {
                history.actions.is_some()
                    && history
                        .action_completeness
                        .as_ref()
                        .is_some_and(|completeness| {
                            let turns = if seat == 0 {
                                &completeness.local
                            } else {
                                &completeness.opponent
                            };
                            turns.get(&(position.turn - 1).to_string()) == Some(&true)
                        })
            });
            if relevant && !complete {
                extra_state.insert(format!(
                    "Infinaut seat {seat} previous-turn staged play history unknown"
                ));
            }
        }
        let miles = position.hand.iter().any(|card| card.id == "MilesMorales")
            || position.deck.iter().any(|id| id == "MilesMorales")
            || position
                .opponent
                .known
                .iter()
                .chain(&position.opponent.seen)
                .chain(position.opponent.deck_list.iter().flatten())
                .chain(position.opponent.pool.iter().flatten())
                .any(|id| id == "MilesMorales");
        if miles {
            extra_state.insert(
                "Miles Morales previous-turn forced movement history not exported".to_owned(),
            );
        }
    }
    // A ledger entry proves augments are in use, but it does not enumerate
    // attached abilities that have not produced a stat update yet.
    if !cards::reference_rules_enabled()
        && position
            .hand
            .iter()
            .chain(
                position
                    .locations
                    .iter()
                    .flat_map(|lane| lane.mine.iter().chain(&lane.theirs)),
            )
            .any(|card| match &card.augments {
                Some(augments) if augments.complete && augments.items.is_empty() => false,
                Some(augments) if !augments.items.is_empty() => true,
                _ => card.stat_updates.as_ref().is_some_and(|updates| {
                    updates
                        .iter()
                        .any(|update| update.updater_augment_id.is_some())
                }),
            })
        && (!position.hand.is_empty()
            || position.locations.iter().any(|lane| !lane.mine.is_empty()))
    {
        extra_state.insert("Card augment attachments are not exported".to_owned());
    }
    for id in &position.opponent.seen {
        if matches!(id.as_str(), "IronFist" | "Enchantress") {
            state.insert(id.as_str());
        }
    }
    let previous = position.turn - 1;
    let delayed = previous > 0
        && position
            .locations
            .iter()
            .any(|l| l.id.as_deref() == Some("Lemuria") && l.revealed_turn == Some(previous));
    let mut face_down = 0;
    for lane in &position.locations {
        if position.turn == 5
            && lane
                .mine
                .iter()
                .any(|card| card.revealed && card.id == "Daredevil")
        {
            extra_state.insert("Daredevil-visible opponent plays not exported".to_owned());
        }
        face_down += lane.theirs_face_down;
        let refs: Vec<_> = lane
            .mine
            .iter()
            .map(|c| (0, c))
            .chain(lane.theirs.iter().map(|c| (1, c)))
            .collect();
        for (owner, card) in &refs {
            let id = card.id.as_str();
            card_ids.insert(id);
            if card.revealed {
                if !cards::reference_rules_enabled() {
                    if id == "Merlin" && card.played_turn.is_none_or(|turn| turn < position.turn) {
                        // MerlinBehavior::AddSpell 0x7ef7c94 remembers its last random index.
                        extra_state
                            .insert("Merlin previous random spell index not exported".to_owned());
                    }
                    if id == "FrankieRaye"
                        && card.played_turn.is_none_or(|turn| turn < position.turn)
                    {
                        // FrankieRayeBehavior 0x7eaba94 consumes a one-time trigger.
                        extra_state.insert(
                            "Frankie Raye prior trigger consumption not exported".to_owned(),
                        );
                    }
                    if id == "HopeSummers" && position.current_action_present(*owner) != Some(false)
                    {
                        // HopeSummersBehavior 0x7ec5cb4 queues next-turn energy.
                        extra_state.insert(
                            "Hope Summers current-turn queued energy grants unknown".to_owned(),
                        );
                    }
                }
                // NegasonicTeenageWarheadBehavior::DestroyCard 0x7f0b1cc
                // consumes the once-only trigger before attempting destruction.
                if id == "NegasonicTeenageWarhead"
                    && card.played_turn.is_none_or(|turn| turn < position.turn)
                    && card.negasonic_consumed().is_none()
                    && !cards::reference_rules_enabled()
                {
                    extra_state.insert(
                        "Negasonic Teenage Warhead trigger consumption not exported".to_owned(),
                    );
                }
                if id == "BlackKnight"
                    && card.played_turn.is_none_or(|turn| turn < position.turn)
                    && card.black_knight_consumed().is_none()
                    && !cards::reference_rules_enabled()
                {
                    // BlackKnightBehavior::AddEbonyBlade 0x7e72ce0.
                    extra_state.insert("Black Knight trigger consumption not exported".to_owned());
                }
                if id == "HumanTorchFirstSteps"
                    && card.played_turn.is_none_or(|turn| turn < position.turn)
                {
                    extra_state.insert("Human Torch First Steps doubling use unknown".to_owned());
                }
                if id == "PowerStone"
                    && (cards::reference_rules_enabled()
                        || position.complete_past_actions(*owner).is_none())
                {
                    extra_state
                        .insert("Power Stone distinct played-stone history unknown".to_owned());
                }
                if card.played_turn.is_none_or(|turn| turn < position.turn) {
                    if id == "JuggernautHorseman"
                        && (cards::reference_rules_enabled() || card.objective().is_none())
                    {
                        extra_state
                            .insert("Juggernaut Horseman objective completion unknown".to_owned());
                    }
                    if id == "Nebula"
                        && (cards::reference_rules_enabled() || card.nebula_active().is_none())
                    {
                        extra_state.insert("Nebula pre-end trigger state unknown".to_owned());
                    } else if id == "TheThingFirstSteps"
                        && (cards::reference_rules_enabled() || card.thing_fill_used().is_none())
                    {
                        extra_state
                            .insert("The Thing First Steps fill-trigger use unknown".to_owned());
                    }
                }
                if matches!(id, "IronFist" | "Enchantress")
                    || (card.played_turn.is_none() && matches!(id, "Hawkeye" | "JessicaJones"))
                {
                    state.insert(id);
                }
            } else if card.played_turn != Some(position.turn)
                && !(delayed && card.played_turn == Some(previous))
            {
                state.insert(id);
            }
        }
        for (owner, target) in &refs {
            if !target.revealed
                || target.played_turn != Some(previous)
                || !matches!(target.id.as_str(), "Hawkeye" | "JessicaJones")
            {
                continue;
            }
            for (source_owner, source) in &refs {
                if std::ptr::eq(*target, *source) || !source.revealed {
                    continue;
                }
                let repeats =
                    source.id == "JoaquinTorres" && source_owner == owner && target.id == "Hawkeye";
                let amplified = repeats
                    && refs.iter().any(|(other_owner, other)| {
                        !std::ptr::eq(*other, *source)
                            && other.revealed
                            && other.id == "Onslaught"
                            && other_owner == source_owner
                            && (other.played_turn.is_none() || other.played_turn == Some(previous))
                    });
                if amplified
                    || ((source.played_turn.is_none() || source.played_turn == Some(previous))
                        && (source.id == "Cosmo" || repeats))
                {
                    state.insert(target.id.as_str());
                    break;
                }
            }
        }
        if let Some(id) = &lane.id {
            loc_ids.insert(id.as_str());
            if id == "Ego" && !cards::reference_rules_enabled() {
                extra_state.insert("Ego autonomous staging not modeled".to_owned());
            }
            if id == "MountVesuvius" && !cards::reference_rules_enabled() && position.turn >= 5 {
                extra_state.insert("Mount Vesuvius retreat prohibition not modeled".to_owned());
            }
            if id == "Flooding" && lane.revealed_turn.is_none() && !cards::reference_rules_enabled()
            {
                extra_state.insert("Flooding reveal turn unknown".to_owned());
            }
            if id == "HotelInferno"
                && lane.revealed_turn.is_none()
                && !cards::reference_rules_enabled()
            {
                extra_state.insert("Hotel Inferno reveal turn unknown".to_owned());
            }
            if id == "AstralPlane"
                && lane.revealed_turn.is_none()
                && !cards::reference_rules_enabled()
            {
                extra_state.insert("Astral Plane reveal turn unknown".to_owned());
            }
            if id == "CastleZemo" && !cards::reference_rules_enabled() {
                extra_state.insert("Castle Zemo per-player betrayed state not exported".to_owned());
            }
            if id == "Krakoa" && !cards::reference_rules_enabled() {
                extra_state.insert(
                    "Krakoa prior consumption and affected top-deck power unknown".to_owned(),
                );
            }
            if id == "AuntMays" && !cards::reference_rules_enabled() {
                extra_state.insert(
                    "Aunt May's per-player movement trigger consumption unknown".to_owned(),
                );
            }
            if id == "FracturedFrontier" && !cards::reference_rules_enabled() {
                if lane.revealed_turn.is_none() {
                    extra_state.insert("Fractured Frontier reveal turn unknown".to_owned());
                }
                if lane.revealed_turn.is_none_or(|turn| turn == position.turn) {
                    extra_state.insert(
                        "Fractured Frontier draw-completed turn and bottom hand order not exported"
                            .to_owned(),
                    );
                }
            }
            if !cards::reference_rules_enabled()
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
                && (id == "WhiteHotRoom" || id == "SakaarGrandPrix" && position.turn > 4)
            {
                extra_state.insert(format!(
                    "{id} prior permanent max energy grants not exported"
                ));
            }
            if id == "WhiteHotRoom"
                && !cards::reference_rules_enabled()
                && lane
                    .history_state
                    .as_ref()
                    .and_then(|state| state.is_given)
                    .is_none()
            {
                extra_state.insert("White Hot Room IsGiven trigger state unknown".to_owned());
            }
            if id == "ValleyOfTheHand" && position.turn <= 5 && !cards::reference_rules_enabled() {
                extra_state.insert(
                    "Valley of the Hand complete destroyed-card graveyard state not exported"
                        .to_owned(),
                );
            }
            if id == "Clubhouse"
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
                && !cards::reference_rules_enabled()
            {
                extra_state
                    .insert("Clubhouse per-player fill trigger state not exported".to_owned());
            }
            if id == "TimeTheater" && position.turn < position.total_turns {
                let spent = (!cards::reference_rules_enabled())
                    .then(|| {
                        lane.history_state
                            .as_ref()?
                            .time_theater_used
                            .as_ref()?
                            .complete()
                    })
                    .flatten();
                if !position.deck.is_empty()
                    && position.hand.len() < 7
                    && spent.is_none_or(|flags| !flags[0])
                {
                    extra_state.insert("Time Theater use unknown for seat 0".to_owned());
                }
                if position.opponent.deck > 0
                    && position.opponent.hand < 7
                    && spent.is_none_or(|flags| !flags[1])
                {
                    extra_state.insert("Time Theater use unknown for seat 1".to_owned());
                }
            }
            if id == "QuantumRealm" {
                for (_, card) in &refs {
                    if card.revealed
                        && card.played_turn.is_none_or(|turn| {
                            turn < position.turn
                                && lane.revealed_turn.is_none_or(|reveal| turn >= reveal)
                        })
                        && (cards::reference_rules_enabled()
                            || card.verified_base_power().is_none())
                    {
                        extra_state.insert(format!("Quantum Realm base unknown for {}", card.id));
                    }
                }
            }
            if matches!(id.as_str(), "TheAbbey" | "Raft")
                && lane.revealed_turn.is_none_or(|turn| turn < position.turn)
                && (cards::reference_rules_enabled()
                    || lane
                        .history_state
                        .as_ref()
                        .and_then(|h| h.is_given)
                        .is_none())
            {
                extra_state.insert(format!("{id} consumed trigger state unknown"));
            }
            if (lane.revealed_turn.is_none()
                && matches!(
                    id.as_str(),
                    "Lemuria" | "Sakaar" | "TinkerersWorkshop" | "ProjectPegasus"
                ))
                || (id == "OttosLab" && lane.ottos_lab_spent.is_none())
                || (id == "Vormir"
                    && (cards::reference_rules_enabled()
                        || !lane
                            .history_state
                            .as_ref()
                            .and_then(|h| h.vormir_consumed.as_ref())
                            .is_some_and(|v| v.local.is_some() && v.opponent.is_some())))
                || (id == "CollapsedMine"
                    && (cards::reference_rules_enabled()
                        || (0..2).any(|seat| position.current_action_present(seat).is_none())))
            {
                state.insert(id.as_str());
            }
        }
    }
    let unknown_cards: Vec<_> = card_ids
        .into_iter()
        .filter(|id| cards::get(id).is_none())
        .collect();
    let unknown_locations: Vec<_> = loc_ids
        .into_iter()
        .filter(|id| locations::get(id).is_none())
        .collect();
    let deck = position
        .deck_count
        .filter(|&n| n != position.deck.len())
        .map(|n| json!({"listed":position.deck.len(),"counted":n}));
    let hidden = position.opponent.hand + position.opponent.deck + face_down;
    let unseen = opponent_candidates(position).1.len();
    let opponent = (hidden > unseen).then(|| json!({"hidden":hidden,"unseen_in_pool":unseen}));
    let state: BTreeSet<String> = state
        .into_iter()
        .map(str::to_owned)
        .chain(extra_state)
        .collect();
    json!({"cards":unknown_cards,"locations":unknown_locations,"state":state,"deck":deck,"opponent":opponent,"face_down":if delayed || face_down == 0 {None} else {Some(face_down)}})
}

#[cfg(test)]
mod location_change_tests {
    use super::*;
    #[test]
    fn unrestored_ritual_and_arrow_effects_are_gated() {
        let mut position = Position {
            turn: 1,
            ..Position::default()
        };
        position.hand.push(crate::position::CardRef {
            id: "SummoningRitual01Dormammu".into(),
            ..Default::default()
        });
        assert!(
            coverage(&position)["state"]
                .to_string()
                .contains("Dormammu ritual source")
        );
        position.turn = 3;
        position.opponent.seen.push("KateBishop".into());
        assert!(
            coverage(&position)["state"]
                .to_string()
                .contains("hidden opponent arrows")
        );
    }

    #[test]
    fn prior_energy_updates_and_owner_restriction_fail_closed() {
        let mut position = Position {
            turn: 4,
            ..Position::default()
        };
        position.opponent.seen.push("Electro".into());
        position.hand.push(crate::position::CardRef {
            id: "Viper".into(),
            ..Default::default()
        });
        let state = coverage(&position)["state"].to_string();
        assert!(state.contains("Electro prior permanent energy update not exported"));
        assert!(state.contains("Viper CanChangeCardOwnerHere restriction state not exported"));
        position.opponent.seen.clear();
        position.locations[0].id = Some("WhiteHotRoom".into());
        position.locations[0].revealed_turn = Some(2);
        assert!(
            coverage(&position)["state"]
                .to_string()
                .contains("WhiteHotRoom prior permanent max energy grants not exported")
        );
    }

    #[test]
    fn previous_locations_gate_replaced_energy_grants() {
        let mut position = Position {
            turn: 3,
            ..Position::default()
        };
        let gated =
            |p: &Position| {
                coverage(p)["state"].as_array().unwrap().iter().any(|v| {
                    v == "Feast/AltarOfDeath prior-turn pending player energy grants unknown"
                })
            };
        assert!(gated(&position));
        position.turn = 1;
        assert!(!gated(&position));
        position.turn = 3;
        for replacement in ["Westview", "MirrorDimension"] {
            position.locations[0].id = Some(replacement.into());
            for old in ["Feast", "AltarOfDeath"] {
                position.previous_turn_locations = Some(position::PreviousTurnLocations {
                    complete: true,
                    ids: [vec![old.into()], vec![], vec![]],
                });
                assert!(gated(&position));
            }
        }
        position.previous_turn_locations = Some(position::PreviousTurnLocations {
            complete: true,
            ids: [vec!["Ruins".into()], vec![], vec![]],
        });
        position.locations[0].id = Some("Feast".into());
        assert!(!gated(&position));
        position.previous_turn_locations.as_mut().unwrap().complete = false;
        assert!(gated(&position));
    }

    #[test]
    fn public_morphing_card_on_either_side_marks_possible_grants() {
        for seat in 0..2 {
            let mut position = Position {
                turn: 3,
                ..Position::default()
            };
            let card = position::CardRef {
                id: "ScarletWitch".to_owned(),
                revealed: true,
                ..position::CardRef::default()
            };
            if seat == 0 {
                position.locations[0].mine.push(card);
            } else {
                position.locations[0].theirs.push(card);
            }
            assert!(location_change_evidence(&position));
            assert!(
                coverage(&position)["state"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| {
                        value
                            == "a location may have been replaced (pending location grants unknown)"
                    })
            );
        }
    }
    #[test]
    fn patriot_with_public_text_changer_declines_unexported_overrides() {
        let mut position = Position {
            turn: 3,
            ..Position::default()
        };
        position.hand.push(position::CardRef {
            id: "Patriot".into(),
            ..Default::default()
        });
        assert!(
            !coverage(&position)["state"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "card text overrides not exported")
        );
        position.locations[0].theirs.push(position::CardRef {
            id: "Enchantress".into(),
            revealed: true,
            ..Default::default()
        });
        assert!(
            coverage(&position)["state"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "card text overrides not exported")
        );
    }
}
