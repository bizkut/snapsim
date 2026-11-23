//! Reconstruct a planning state from the Python advisor's public position format.
use crate::{
    cards::{self, CardDef},
    data,
    game::{Card, CardStatUpdate, Game, Lane, Player, Stat, StatOperation},
    locations::{self, LocationDef},
    random::PythonRandom,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

fn yes() -> bool {
    true
}
fn six() -> u8 {
    6
}
/// An observed client ledger entry. Source is a seat, not an entity ID.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ObservedStatUpdate {
    pub stat: String,
    pub update: String,
    pub value: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updater_card_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updater_location_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updater_faction_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updater_augment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updater_league_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PowerModifier {
    pub operation: String,
    pub amount: i32,
    pub source: Option<()>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PowerModifiers {
    pub complete: bool,
    pub items: Vec<PowerModifier>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedAbilityContext {
    pub fill_trigger_used: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_end_active: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedAbility {
    pub ability_def_id: String,
    pub index: u32,
    pub kind: Option<String>,
    pub lifetime: Option<String>,
    pub has_triggered_once: Option<bool>,
    pub state: Option<ReviewedAbilityContext>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedObjective {
    pub state: String,
    pub progress: u32,
    pub goal: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbilityState {
    pub complete: bool,
    pub abilities: Vec<ReviewedAbility>,
    pub objective: Option<ObservedObjective>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedAugment {
    pub index: u32,
    pub id: String,
    pub kind: Option<String>,
    pub ability_indexes: Vec<u32>,
    pub overrides: Option<()>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Augments {
    pub complete: bool,
    pub items: Vec<ObservedAugment>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CardRef {
    pub id: String,
    #[serde(default)]
    pub cost: Option<i32>,
    /// Exact pre-ongoing cost when projected from an existing simulation.
    #[serde(skip)]
    pub cost_basis: Option<i32>,
    #[serde(default)]
    pub power: Option<i32>,
    #[serde(default = "yes")]
    pub revealed: bool,
    #[serde(default)]
    pub played_turn: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_revealed: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activated: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_moves_used: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_activate: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_targets: Option<Vec<usize>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_resolution_order: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns_drawn: Option<Vec<(u8, bool)>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moves_per_turn: Option<HashMap<String, u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_card_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_location_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_player: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merged_to_entity_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stat_updates: Option<Vec<ObservedStatUpdate>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_modifiers: Option<PowerModifiers>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_base_power: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ability_state: Option<AbilityState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub augments: Option<Augments>,
}

impl CardRef {
    fn one_time_consumed(&self, card_id: &str, ability_id: &str) -> Option<bool> {
        if self.id != card_id {
            return None;
        }
        let state = self.ability_state.as_ref()?;
        if !state.complete || state.objective.is_some() || state.abilities.len() != 1 {
            return None;
        }
        let ability = &state.abilities[0];
        if ability.ability_def_id != ability_id
            || ability.kind.as_deref() != Some("trigger")
            || ability.lifetime.as_deref() != Some("OneTime")
            || ability.state.is_some()
        {
            return None;
        }
        ability.has_triggered_once
    }

    pub(crate) fn negasonic_consumed(&self) -> Option<bool> {
        self.one_time_consumed(
            "NegasonicTeenageWarhead",
            "NegasonicTeenageWarheadPostCardResolvedTrigger",
        )
    }

    pub(crate) fn black_knight_consumed(&self) -> Option<bool> {
        self.one_time_consumed("BlackKnight", "BlackKnightPostCardDiscardedTrigger")
    }

    pub(crate) fn thing_fill_used(&self) -> Option<bool> {
        if self.id != "TheThingFirstSteps" {
            return None;
        }
        let state = self.ability_state.as_ref()?;
        if state.abilities.len() != 1 || state.objective.is_some() {
            return None;
        }
        let mut matching = state.abilities.iter().filter(|ability| {
            ability.ability_def_id == "TheThingFirstStepsEndOfTurnTrigger"
                && ability.kind.as_deref() == Some("trigger")
                && ability.lifetime.is_none()
                && ability.has_triggered_once.is_none()
        });
        let ability = matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        let context = ability.state.as_ref()?;
        if context.pre_end_active.is_some() {
            return None;
        }
        context.fill_trigger_used
    }

    pub(crate) fn nebula_active(&self) -> Option<bool> {
        if self.id != "Nebula" {
            return None;
        }
        let state = self.ability_state.as_ref()?;
        if state.abilities.len() != 1 || state.objective.is_some() {
            return None;
        }
        let ability = &state.abilities[0];
        if ability.ability_def_id != "NebulaPreEndOfTurnTrigger"
            || ability.kind.as_deref() != Some("trigger")
            || ability.lifetime.is_some()
            || ability.has_triggered_once.is_some()
        {
            return None;
        }
        let context = ability.state.as_ref()?;
        if context.fill_trigger_used.is_some() {
            return None;
        }
        (context.pre_end_active == Some(true)).then_some(true)
    }

    pub(crate) fn objective(&self) -> Option<(usize, bool)> {
        if self.id != "JuggernautHorseman" {
            return None;
        }
        let state = self.ability_state.as_ref()?;
        if !state.abilities.is_empty() {
            return None;
        }
        let objective = state.objective.as_ref()?;
        if objective.goal != 6 || objective.progress > objective.goal {
            return None;
        }
        match objective.state.as_str() {
            "InProgress" if objective.progress < objective.goal => {
                Some((objective.progress as usize, false))
            }
            "Completed" if objective.progress == objective.goal => {
                Some((objective.progress as usize, true))
            }
            _ => None,
        }
    }

    pub(crate) fn archangel_objective(&self) -> Option<(usize, bool)> {
        if self.id != "ArchangelHorseman" {
            return None;
        }
        let state = self.ability_state.as_ref()?;
        if !state.abilities.is_empty() {
            return None;
        }
        let objective = state.objective.as_ref()?;
        if objective.goal != 4 || objective.progress > objective.goal {
            return None;
        }
        match objective.state.as_str() {
            "InProgress" if objective.progress < 4 => Some((objective.progress as usize, false)),
            "Completed" if objective.progress == 4 => Some((4, true)),
            _ => None,
        }
    }

    pub(crate) fn selene_objective(&self) -> Option<(usize, bool)> {
        if self.id != "SeleneHorseman" {
            return None;
        }
        let state = self.ability_state.as_ref()?;
        if !state.abilities.is_empty() {
            return None;
        }
        let objective = state.objective.as_ref()?;
        if objective.goal != 3 || objective.progress > objective.goal {
            return None;
        }
        match objective.state.as_str() {
            "InProgress" if objective.progress < 3 => Some((objective.progress as usize, false)),
            "Completed" if objective.progress == 3 => Some((3, true)),
            _ => None,
        }
    }

    pub(crate) fn verified_base_power(&self) -> Option<i32> {
        let printed = CARD_DATA.get(&self.id)?.power;
        if self
            .effective_base_power
            .is_some_and(|base| base != printed)
        {
            return None;
        }
        let modifiers = self.power_modifiers.as_ref()?;
        let updates = self.stat_updates.as_ref()?;
        if !modifiers.complete || modifiers.items.len() != updates.len() {
            return None;
        }
        let mut power = printed;
        for (modifier, update) in modifiers.items.iter().zip(updates) {
            if update.stat != "Power"
                || update.update != modifier.operation
                || update.value != modifier.amount
            {
                return None;
            }
            power = match modifier.operation.as_str() {
                "Add" => power.checked_add(modifier.amount)?,
                "Multiply" => power.checked_mul(modifier.amount)?,
                // A Set may be a base override or another modifier; the two ledgers
                // do not identify which, so do not guess.
                _ => return None,
            };
        }
        (self.power == Some(power)).then_some(printed)
    }

    /// Both client power ledgers must be complete and agree entry for entry.
    pub(crate) fn has_complete_power_modifier_history(&self) -> bool {
        let Some(modifiers) = &self.power_modifiers else {
            return false;
        };
        let Some(updates) = &self.stat_updates else {
            return false;
        };
        modifiers.complete
            && modifiers.items.len() == updates.len()
            && modifiers
                .items
                .iter()
                .zip(updates)
                .all(|(modifier, update)| {
                    update.stat == "Power"
                        && update.update == modifier.operation
                        && update.value == modifier.amount
                        && matches!(update.update.as_str(), "Add" | "Multiply" | "Set")
                })
    }
}
/// Reviewed location context. Missing values remain unknown, not false.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocationHistoryState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vormir_consumed: Option<SeatFlags>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_theater_used: Option<SeatFlags>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_given: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SeatFlags {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opponent: Option<bool>,
}

impl SeatFlags {
    pub(crate) fn complete(&self) -> Option<[bool; 2]> {
        Some([self.local?, self.opponent?])
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryAction {
    pub owner: String,
    pub turn: u8,
    pub kind: String,
    pub location_slot: usize,
    pub card_entity_id: i64,
    pub card_id: String,
    pub resolved: bool,
    pub turn_resolved: Option<u8>,
    pub resolution_order: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCompleteness {
    #[serde(default)]
    pub local: HashMap<String, bool>,
    #[serde(default)]
    pub opponent: HashMap<String, bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnDrawHistory {
    pub card_ids: Option<Vec<String>>,
    pub complete: bool,
    pub last_drawn_entity_id: Option<i64>,
    pub last_drawn_copy_state: Option<()>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedHistory {
    pub actions: Option<Vec<HistoryAction>>,
    pub action_completeness: Option<ActionCompleteness>,
    pub own_draws: Option<OwnDrawHistory>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LocationRef {
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub destroyed: bool,
    pub mine: Vec<CardRef>,
    pub theirs: Vec<CardRef>,
    #[serde(default)]
    pub theirs_face_down: usize,
    #[serde(default)]
    pub revealed_turn: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ottos_lab_spent: Option<[bool; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_state: Option<LocationHistoryState>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Opponent {
    pub hand: usize,
    pub deck: usize,
    #[serde(default)]
    pub energy: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_energy: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_energy: Option<i32>,
    #[serde(default)]
    pub seen: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known: Vec<String>,
    #[serde(default)]
    pub deck_list: Option<Vec<String>>,
    #[serde(default)]
    pub pool: Option<Vec<String>>,
    #[serde(default)]
    pub pending_power: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graveyard: Option<Vec<CardRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banished: Option<Vec<CardRef>>,
}
/// All location identities held during the preceding turn, in lane order.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreviousTurnLocations {
    pub complete: bool,
    pub ids: [Vec<String>; 3],
}

/// Visible cube values; no per-player snap eligibility is inferred from them.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ObservedStakes {
    pub current: u32,
    pub next: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_multiplier: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_multiplier: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_game_multiplier: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_raise: Option<[Option<bool>; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_raiser: Option<usize>,
}

/// Privacy-filtered client provenance. Completeness is asserted independently
/// for each entity and list; absence of the field is not an empty list.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Provenance {
    pub schema_version: u32,
    pub entities: Vec<ProvenanceEntity>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub entities_complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledger: Option<ProvenanceLedger>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProvenanceLedger {
    pub complete: bool,
    #[serde(default)]
    pub complete_for_legality: bool,
    #[serde(default)]
    pub legality_reasons: Vec<String>,
    pub reasons: Vec<String>,
    pub match_start_seen: bool,
    pub items: Vec<ProvenanceLedgerItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProvenanceLedgerItem {
    pub effect_key: String,
    pub kind: String,
    pub target_entity_id: i64,
    pub creator_entity_id: Option<i64>,
    pub creator_def_id: String,
    pub created_turn: Option<u32>,
    pub expiry_turn: Option<u32>,
    pub attribution: String,
    pub reasons: Vec<String>,
    #[serde(default)]
    pub legality_relevant: Option<bool>,
    #[serde(default)]
    pub operation: Option<String>,
    #[serde(default)]
    pub amount: Option<i32>,
    #[serde(default)]
    pub destination_entity_id: Option<i64>,
    #[serde(default)]
    pub stacks: Option<u32>,
    #[serde(default)]
    pub consumed: Option<bool>,
    #[serde(default)]
    pub ability_def_id: Option<String>,
    #[serde(default)]
    pub ability_index: Option<u32>,
    #[serde(default)]
    pub remaining: Option<u32>,
    #[serde(default)]
    pub active: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProvenanceList<T> {
    pub complete: bool,
    pub reasons: Vec<String>,
    pub items: Vec<T>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProvenanceEntity {
    pub kind: String,
    pub entity_id: i64,
    pub card_id: Option<String>,
    pub lane_index: Option<usize>,
    pub abilities: ProvenanceList<ProvenanceAbility>,
    pub movable_components: ProvenanceList<ProvenanceMoveComponent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<ProvenanceList<ProvenanceModifier>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub play_requirements: Option<ProvenanceList<ProvenancePlayRequirement>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProvenanceModifier {
    #[serde(rename = "type")]
    pub kind: String,
    pub operation: String,
    pub amount: i32,
    pub creator_entity_id: Option<i64>,
    pub owner_def_type: String,
    pub owner_def_id: String,
    pub player_entity_id: i64,
    pub turn_created: Option<u32>,
    pub expiry_turn: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProvenancePlayRequirement {
    pub def_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProvenanceAbility {
    pub ability_def_id: String,
    pub owner_def_type: String,
    pub owner_def_id: String,
    pub index: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_created: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_entity_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_turn: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifetime: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProvenanceMoveComponent {
    pub source_entity_id: i64,
    pub owner_def_type: String,
    pub owner_def_id: String,
    pub ability_index: u32,
    pub created_ticks: String,
    pub destination_entity_id: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Position {
    pub turn: u8,
    #[serde(default = "six")]
    pub total_turns: u8,
    pub energy: i32,
    /// Persistent maximum energy before temporary one-turn grants.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_energy: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_energy: Option<i32>,
    pub hand: Vec<CardRef>,
    pub deck: Vec<String>,
    /// Whether each corresponding deck entry was created; None means unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck_created: Option<Vec<Option<bool>>>,
    pub locations: [LocationRef; 3],
    pub opponent: Opponent,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub pending_power: i32,
    #[serde(default)]
    pub deck_count: Option<usize>,
    /// Per-seat snap state for the current turn, when observed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapped_this_turn: Option<[bool; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stakes: Option<ObservedStakes>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graveyard: Option<Vec<CardRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banished: Option<Vec<CardRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<ObservedHistory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_turn_locations: Option<PreviousTurnLocations>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_entity_ids: Option<[i64; 2]>,
}

impl Position {
    /// BlackSwanBehavior::AddSetCostAbilityToPlayer 0x7e7386c and its
    /// static constructor 0x7e74384 create BlackSwanOngoing, lifetime
    /// NextTurn, with the source card and current turn. SetCardCost
    /// 0x7e739f4 sets printed one-cost cards to zero.
    pub fn black_swan_effect(&self, item: &ProvenanceLedgerItem) -> Option<(usize, u8)> {
        if item.kind != "ability"
            || item.attribution != "exact"
            || item.active != Some(true)
            || !item.reasons.is_empty()
            || item.creator_def_id != "BlackSwan"
            || item.ability_def_id.as_deref() != Some("BlackSwanOngoing")
        {
            return None;
        }
        let created = item.created_turn?;
        let expiry = created.checked_add(2)?;
        if created == 0
            || item.expiry_turn.is_some_and(|turn| turn != expiry)
            || u32::from(self.turn) < created
            || u32::from(self.turn) >= expiry
        {
            return None;
        }
        let creator = item.creator_entity_id?;
        let index = item.ability_index?;
        let seat = self
            .player_entity_ids?
            .iter()
            .position(|id| *id == item.target_entity_id)?;
        let provenance = self.provenance.as_ref()?;
        let mut matches = provenance
            .entities
            .iter()
            .filter(|entity| entity.kind == "player" && entity.entity_id == item.target_entity_id);
        let entity = matches.next()?;
        if matches.next().is_some() || !entity.abilities.complete {
            return None;
        }
        let mut abilities = entity.abilities.items.iter().filter(|ability| {
            ability.ability_def_id == "BlackSwanOngoing"
                && ability.index == index
                && ability.owner_def_id == "BlackSwan"
                && ability.origin_entity_id == Some(creator)
                && ability.created_turn == Some(created)
                && ability
                    .lifetime
                    .as_ref()
                    .and_then(serde_json::Value::as_u64)
                    == Some(2)
                && ability.active == Some(true)
        });
        abilities.next()?;
        if abilities.next().is_some() {
            return None;
        }
        Some((seat, u8::try_from(expiry - 1).ok()?))
    }

    fn black_swan_inventory_known(&self, legality: bool) -> bool {
        let Some(provenance) = &self.provenance else {
            return false;
        };
        let Some(ledger) = &provenance.ledger else {
            return false;
        };
        if provenance.schema_version != 2
            || !provenance.entities_complete
            || if legality {
                !ledger.complete_for_legality
            } else {
                !ledger.complete
            }
            || !ledger.match_start_seen
            || self.player_entity_ids.is_none()
        {
            return false;
        }
        let local_id = self.player_entity_ids.unwrap()[0];
        if provenance
            .entities
            .iter()
            .filter(|entity| entity.kind == "player" && entity.entity_id == local_id)
            .count()
            != 1
        {
            return false;
        }
        for entity in provenance
            .entities
            .iter()
            .filter(|entity| entity.kind == "player")
        {
            if !entity.abilities.complete {
                return false;
            }
            for ability in entity
                .abilities
                .items
                .iter()
                .filter(|a| a.ability_def_id == "BlackSwanOngoing")
            {
                if ability.active == Some(false) {
                    continue;
                }
                if ledger
                    .items
                    .iter()
                    .filter(|item| {
                        item.target_entity_id == entity.entity_id
                            && item.ability_index == Some(ability.index)
                            && self.black_swan_effect(item).is_some()
                    })
                    .count()
                    != 1
                {
                    return false;
                }
            }
        }
        ledger
            .items
            .iter()
            .filter(|item| item.creator_def_id == "BlackSwan" && item.active != Some(false))
            .all(|item| self.black_swan_effect(item).is_some())
    }

    pub(crate) fn black_swan_known(&self) -> bool {
        self.black_swan_inventory_known(false)
    }

    /// Whether no unmodeled player-cost effects remain after hydration.
    pub fn player_cost_provenance_known(&self) -> bool {
        if !self.black_swan_inventory_known(true) {
            return false;
        }
        let provenance = self.provenance.as_ref().unwrap();
        let ledger = provenance.ledger.as_ref().unwrap();
        provenance
            .entities
            .iter()
            .filter(|entity| entity.kind == "player")
            .all(|entity| {
                entity
                    .abilities
                    .items
                    .iter()
                    .all(|ability| ability.ability_def_id == "BlackSwanOngoing")
            })
            && ledger
                .items
                .iter()
                .filter(|item| {
                    item.active != Some(false) && matches!(item.kind.as_str(), "cost" | "ability")
                })
                .all(|item| self.black_swan_effect(item).is_some())
    }

    /// MovableOngoingAbilityDef::ExecuteOngoingAction 0x7e0e6f4 creates
    /// these self-owned components. Only complete staged-move history with
    /// no competing source proves which intrinsic stacks were consumed.
    pub fn intrinsic_move_state(
        &self,
        item: &ProvenanceLedgerItem,
    ) -> Option<(i64, u8, Option<u8>)> {
        let (limit, ability_id) = match item.creator_def_id.as_str() {
            "Nightcrawler" => (1, "NightcrawlerMovableOngoing"),
            "SpiderManBrandNewDay" => (2, "SpiderManBrandNewDayMovableOngoing"),
            "Vision" => (0, "VisionMovableOngoing"),
            _ => return None,
        };
        if item.kind != "move_grant"
            || item.attribution != "exact"
            || !item.reasons.is_empty()
            || item.creator_entity_id != Some(item.target_entity_id)
            || item.destination_entity_id.is_some()
            || item.expiry_turn.is_some()
            || item.ability_def_id.as_deref() != Some(ability_id)
        {
            return None;
        }
        let provenance = self.provenance.as_ref()?;
        let ledger = provenance.ledger.as_ref()?;
        if provenance.schema_version != 2
            || !provenance.entities_complete
            || !ledger.complete_for_legality
            || !ledger.match_start_seen
            || ledger
                .items
                .iter()
                .filter(|entry| {
                    entry.kind == "move_grant" && entry.target_entity_id == item.target_entity_id
                })
                .count()
                != 1
        {
            return None;
        }
        let mut cards = self
            .locations
            .iter()
            .flat_map(|lane| {
                lane.mine
                    .iter()
                    .map(|card| (0, card))
                    .chain(lane.theirs.iter().map(|card| (1, card)))
            })
            .filter(|(_, card)| card.entity_id == Some(item.target_entity_id) && card.revealed);
        let (seat, card) = cards.next()?;
        if cards.next().is_some() || card.id != item.creator_def_id {
            return None;
        }
        let mut entities = provenance.entities.iter().filter(|entity| {
            entity.kind == "card"
                && entity.entity_id == item.target_entity_id
                && entity.card_id.as_deref() == Some(card.id.as_str())
        });
        let entity = entities.next()?;
        if entities.next().is_some()
            || !entity.abilities.complete
            || !entity.movable_components.complete
        {
            return None;
        }
        let index = item.ability_index?;
        if entity
            .abilities
            .items
            .iter()
            .filter(|ability| {
                ability.index == index
                    && ability.ability_def_id == item.ability_def_id.as_deref().unwrap_or("")
                    && ability.owner_def_id == card.id
            })
            .count()
            != 1
        {
            return None;
        }
        let components = entity
            .movable_components
            .items
            .iter()
            .filter(|component| {
                component.source_entity_id == item.target_entity_id
                    && component.owner_def_id == card.id
                    && component.ability_index == index
            })
            .count();
        if entity.movable_components.items.len() != components
            || match item.active {
                Some(true) => components != 1,
                Some(false) => components != 0,
                None => true,
            }
        {
            return None;
        }
        let history = self.history.as_ref()?;
        let completeness = history.action_completeness.as_ref()?;
        let turns = if seat == 0 {
            &completeness.local
        } else {
            &completeness.opponent
        };
        if (1..=self.turn).any(|turn| turns.get(&turn.to_string()) != Some(&true)) {
            return None;
        }
        let created = u8::try_from(item.created_turn?).ok()?;
        if created == 0 || created > self.turn {
            return None;
        }
        let owner = if seat == 0 { "local" } else { "opponent" };
        let mut moves = 0u8;
        let mut current = None;
        let mut plays = 0;
        let mut orders = HashSet::new();
        for action in history
            .actions
            .as_ref()?
            .iter()
            .filter(|action| action.owner == owner)
        {
            if action.turn == 0
                || action.turn > self.turn
                || !action.resolved
                || action.turn_resolved != Some(action.turn)
                || action.location_slot >= 3
                || !orders.insert((action.turn, action.resolution_order?))
            {
                return None;
            }
            if action.card_entity_id != item.target_entity_id {
                continue;
            }
            if action.card_id != card.id {
                return None;
            }
            if action.kind == "play" && action.turn == created {
                plays += 1;
            }
            if action.kind == "move" && action.turn >= created {
                moves = moves.checked_add(1)?;
                if action.turn == self.turn {
                    if current.is_some() && limit == 0 {
                        return None;
                    }
                    current = Some(action.turn);
                }
            }
        }
        if plays != 1
            || (limit > 0 && moves > limit)
            || (item.active == Some(false)
                && if limit == 0 {
                    current.is_none()
                } else {
                    moves < limit
                })
        {
            return None;
        }
        Some((item.target_entity_id, moves, current))
    }

    pub fn move_source_provenance_known(&self) -> bool {
        let Some(provenance) = &self.provenance else {
            return false;
        };
        let Some(ledger) = &provenance.ledger else {
            return false;
        };
        if provenance.schema_version != 2
            || !provenance.entities_complete
            || !ledger.complete_for_legality
            || !ledger.match_start_seen
        {
            return false;
        }
        if !provenance
            .entities
            .iter()
            .all(|entity| entity.movable_components.complete)
        {
            return false;
        }
        let items = ledger.items.iter().filter(|item| item.kind == "move_grant");
        if !items
            .clone()
            .all(|item| self.intrinsic_move_state(item).is_some())
        {
            return false;
        }
        provenance.entities.iter().all(|entity| {
            entity.movable_components.items.iter().all(|component| {
                items.clone().any(|item| {
                    item.target_entity_id == entity.entity_id
                        && item.creator_entity_id == Some(component.source_entity_id)
                        && item.ability_index == Some(component.ability_index)
                        && item.active == Some(true)
                })
            })
        })
    }
}

impl Position {
    pub(crate) fn complete_past_actions(&self, seat: usize) -> Option<Vec<&HistoryAction>> {
        let history = self.history.as_ref()?;
        let complete = history.action_completeness.as_ref()?;
        let turns = match seat {
            0 => &complete.local,
            1 => &complete.opponent,
            _ => return None,
        };
        if (1..self.turn).any(|turn| turns.get(&turn.to_string()) != Some(&true)) {
            return None;
        }
        let owner = if seat == 0 { "local" } else { "opponent" };
        let mut actions = Vec::new();
        let mut orders = HashSet::new();
        for action in history
            .actions
            .as_ref()?
            .iter()
            .filter(|a| a.owner == owner && a.turn < self.turn)
        {
            if action.turn == 0
                || action.card_entity_id < 0
                || action.location_slot >= 3
                || self.locations[action.location_slot].id.is_none()
                || !matches!(action.kind.as_str(), "play" | "move" | "activate")
                || !action.resolved
                || action.turn_resolved != Some(action.turn)
            {
                return None;
            }
            let order = action.resolution_order?;
            if !orders.insert((action.turn, order)) {
                return None;
            }
            let refs = self
                .locations
                .iter()
                .flat_map(|lane| {
                    if seat == 0 {
                        lane.mine.iter()
                    } else {
                        lane.theirs.iter()
                    }
                })
                .chain(if seat == 0 {
                    self.hand.iter()
                } else {
                    [].iter()
                })
                .chain(if seat == 0 {
                    self.graveyard.iter().flatten()
                } else {
                    self.opponent.graveyard.iter().flatten()
                })
                .chain(if seat == 0 {
                    self.banished.iter().flatten()
                } else {
                    self.opponent.banished.iter().flatten()
                });
            if refs
                .filter(|card| {
                    card.entity_id == Some(action.card_entity_id) && card.id == action.card_id
                })
                .count()
                != 1
            {
                return None;
            }
            actions.push(action);
        }
        actions.sort_by_key(|action| (action.turn, action.resolution_order));
        Some(actions)
    }

    pub(crate) fn current_action_present(&self, seat: usize) -> Option<bool> {
        let history = self.history.as_ref()?;
        let complete = history.action_completeness.as_ref()?;
        let turns = match seat {
            0 => &complete.local,
            1 => &complete.opponent,
            _ => return None,
        };
        if turns.get(&self.turn.to_string()) != Some(&true) {
            return None;
        }
        let owner = if seat == 0 { "local" } else { "opponent" };
        let mut present = false;
        for action in history
            .actions
            .as_ref()?
            .iter()
            .filter(|a| a.owner == owner && a.turn == self.turn)
        {
            if action.card_entity_id < 0
                || action.location_slot >= 3
                || self.locations[action.location_slot].id.is_none()
                || !matches!(action.kind.as_str(), "play" | "move" | "activate")
                // Unresolved entries may be undone or predicted; they do not prove a committed action.
                || !action.resolved
            {
                return None;
            }
            let refs = self
                .locations
                .iter()
                .flat_map(|lane| {
                    if seat == 0 {
                        lane.mine.iter()
                    } else {
                        lane.theirs.iter()
                    }
                })
                .chain(if seat == 0 {
                    self.hand.iter()
                } else {
                    [].iter()
                })
                .chain(if seat == 0 {
                    self.graveyard.iter().flatten()
                } else {
                    self.opponent.graveyard.iter().flatten()
                })
                .chain(if seat == 0 {
                    self.banished.iter().flatten()
                } else {
                    self.opponent.banished.iter().flatten()
                });
            if refs
                .filter(|card| {
                    card.entity_id == Some(action.card_entity_id) && card.id == action.card_id
                })
                .count()
                != 1
            {
                return None;
            }
            present = true;
        }
        Some(present)
    }
}

static CARD_DATA: LazyLock<HashMap<String, data::CardData>> = LazyLock::new(|| {
    data::cards()
        .into_iter()
        .map(|x| (x.carddefid.clone(), x))
        .collect()
});
static LOCATION_DATA: LazyLock<HashMap<String, data::LocationData>> = LazyLock::new(|| {
    data::locations()
        .into_iter()
        .map(|x| (x.carddefid.clone(), x))
        .collect()
});
static CARD_FALLBACKS: LazyLock<HashMap<&'static str, CardDef>> = LazyLock::new(|| {
    CARD_DATA
        .keys()
        .filter(|id| cards::get(id).is_none())
        .map(|id| (id.as_str(), CardDef::vanilla(id.as_str())))
        .collect()
});
static LOCATION_FALLBACKS: LazyLock<HashMap<&'static str, LocationDef>> = LazyLock::new(|| {
    LOCATION_DATA
        .keys()
        .filter(|id| locations::get(id).is_none())
        .map(|id| {
            (
                id.as_str(),
                locations::location(id.as_str(), 0, 0, 1, false),
            )
        })
        .collect()
});
pub(crate) static UNKNOWN_LOCATION: LocationDef = locations::location("", 0, 0, 1, false);
pub(crate) fn location_def(id: &str) -> Option<&'static LocationDef> {
    locations::get(id).or_else(|| LOCATION_FALLBACKS.get(id))
}
pub(crate) fn card_def(id: &str) -> Result<&'static CardDef, String> {
    cards::get(id)
        .or_else(|| CARD_FALLBACKS.get(id))
        .ok_or_else(|| format!("unknown card ID {id:?}"))
}
pub(crate) fn recruit_source(s: &str) -> bool {
    matches!(
        s,
        "Starter Card" | "Recruit Season" | "Collection Level 1-14" | "Series 1"
    )
}
pub(crate) fn outside<'a>(ids: impl IntoIterator<Item = &'a String>) -> bool {
    ids.into_iter().any(|id| {
        CARD_DATA
            .get(id)
            .is_some_and(|d| d.status == "released" && !recruit_source(&d.source))
    })
}
pub fn recruit_cards() -> Vec<String> {
    cards::deck_eligible_ids()
        .iter()
        .filter(|id| {
            CARD_DATA
                .get(**id)
                .is_some_and(|d| d.status == "released" && recruit_source(&d.source))
        })
        .map(|id| (*id).to_owned())
        .collect()
}
pub(crate) fn all_cards() -> Vec<String> {
    cards::deck_eligible_ids()
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}
/// The frozen Python reference's locations. Parity gates pin location pools to it, so locations
/// added since (like cards beyond the reference pool) do not change the reference's random draws.
static REFERENCE_LOCATIONS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../parity/reference-locations.json"))
        .expect("invalid parity/reference-locations.json")
});
pub(crate) fn all_locations(recruit: bool) -> Vec<String> {
    let pinned = cards::reference_pool_enabled();
    let mut ids: Vec<_> = locations::REGISTRY
        .iter()
        .filter(|d| !pinned || REFERENCE_LOCATIONS.contains(&d.id))
        .filter(|d| !matches!(d.id, "SubterraneaTutorial" | "SakaarTutorial"))
        .filter(|d| {
            !recruit
                || LOCATION_DATA.get(d.id).is_some_and(|x| {
                    x.status == "released" && matches!(x.difficulty.as_str(), "Basic" | "Standard")
                })
        })
        .map(|d| d.id.to_owned())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}
fn sample<T: Clone>(rng: &mut PythonRandom, pool: &[T], count: usize) -> Result<Vec<T>, String> {
    if count > pool.len() {
        return Err(format!(
            "cannot draw {count} distinct items from a pool of {}",
            pool.len()
        ));
    }
    Ok(rng
        .sample_indices(pool.len(), count)
        .into_iter()
        .map(|i| pool[i].clone())
        .collect())
}
pub(crate) fn make_card(id: &str, owner: usize) -> Result<Card, String> {
    let def = card_def(id)?;
    let row = CARD_DATA
        .get(id)
        .ok_or_else(|| format!("unknown card ID {id:?}"))?;
    Ok(Card {
        def,
        override_ability: None,
        morphed_from: None,
        morphed_from_entity: None,
        replaced_card_def_id: None,
        owner,
        cost: row.cost,
        ongoing_cost: 0,
        dynamic_assigned_cost: None,
        cost_effects: Vec::new(),
        next_cost_effect_sequence: 0,
        created: false,
        created_in_hand: None,
        created_hand_seat: None,
        creation_pending: false,
        created_by_card_id: None,
        created_by_location_id: None,
        created_by_player: None,
        observed_entity_id: None,
        merged_to_entity_id: None,
        permanent: row.power,
        base_power_override: None,
        stat_updates: Vec::new(),
        power_modifiers: Vec::new(),
        power_modifiers_complete: false,
        ongoing: 0,
        location_power: 0,
        last_lane: None,
        lane: None,
        zone: None,
        previous_zone: None,
        revealed: false,
        abilities_removed: false,
        latest_resolution_order: 0,
        repeating: false,
        move_turns: Vec::new(),
        player_moves_used: 0,
        player_move_turn: None,
        activated: false,
        move_staged: false,
        activate_staged: false,
        observed_can_activate: None,
        observed_move_targets: None,
        draw_extends_quickdraw: false,
        merged_to: None,
        drawn_turn: None,
        turn_played: 0,
        turn_revealed: None,
        attached_abilities: crate::game::initial_attachments(def),
        composed_abilities: Vec::new(),
        copied_ongoing_guards: Vec::new(),
        virtual_ongoing_ids: Vec::new(),
        ability_frames: Vec::new(),
        default_ability_generation: 0,
        external_moves: Vec::new(),
        state: def.initial_state.map(|init| init()),
        borrowed_original_state: None,
    })
}

pub(crate) fn apply_card_reference(card: &mut Card, reference: &CardRef, turn: u8) {
    if !cards::reference_rules_enabled() {
        if let Some(modifiers) = &reference.power_modifiers {
            let parsed: Option<Vec<_>> = modifiers
                .items
                .iter()
                .map(|item| {
                    let operation = match item.operation.as_str() {
                        "Add" => StatOperation::Add,
                        "Multiply" => StatOperation::Multiply,
                        "Set" => StatOperation::Set,
                        _ => return None,
                    };
                    Some(crate::game::CardPowerModifier {
                        operation,
                        value: item.amount,
                        source: None,
                    })
                })
                .collect();
            if let Some(parsed) = parsed {
                card.power_modifiers = parsed;
                card.power_modifiers_complete = modifiers.complete;
            }
        }
        if let Some(base) = reference.verified_base_power() {
            card.base_power_override = Some(base);
        }
        if let Some(used) = reference.thing_fill_used()
            && let Some(state) = card
                .state
                .as_deref_mut()
                .and_then(|state| state.as_any_mut().downcast_mut::<bool>())
        {
            *state = used;
        }
        if let Some(used) = reference.negasonic_consumed() {
            cards::meta_l3::restore_negasonic(card, used);
        }
        if let Some(used) = reference.black_knight_consumed() {
            cards::meta_l3::restore_black_knight(card, used);
        }
        if reference.nebula_active() == Some(true)
            && let Some(state) = card
                .state
                .as_deref_mut()
                .and_then(|state| state.as_any_mut().downcast_mut::<Option<u8>>())
        {
            *state = Some(turn);
        }
        if let Some((progress, completed)) = reference.objective() {
            cards::restore_objective(card, progress, completed);
        }
        if let Some((progress, completed)) = reference.archangel_objective() {
            cards::meta_l3::restore_archangel(card, progress, completed);
        }
        if let Some((progress, completed)) = reference.selene_objective() {
            cards::meta_m3::restore_selene(card, progress, completed);
        }
    }
    card.activated = reference.activated.unwrap_or(false);
    card.turn_revealed = reference.turn_revealed;
    card.created = reference.created.unwrap_or(false);
    card.latest_resolution_order = reference.latest_resolution_order.unwrap_or(0);
    card.created_by_card_id
        .clone_from(&reference.created_by_card_id);
    card.created_by_location_id
        .clone_from(&reference.created_by_location_id);
    card.created_by_player = reference.created_by_player.filter(|&seat| seat < 2);
    card.observed_entity_id = reference.entity_id;
    card.merged_to_entity_id = reference.merged_to_entity_id;
    if let Some(draws) = &reference.turns_drawn {
        // Card.IsQuickdrawActive (0x7cb1b88), predicate 0x7cbe0d0.
        if let Some(&(drawn, extended)) = draws
            .iter()
            .rev()
            .find(|&&(drawn, extended)| {
                drawn == turn || (extended && drawn.checked_add(1) == Some(turn))
            })
            .or_else(|| draws.last())
        {
            card.drawn_turn = Some(drawn);
            card.draw_extends_quickdraw = extended;
        }
    }
    if let Some(moves) = &reference.moves_per_turn {
        let mut turns: Vec<_> = moves
            .iter()
            .filter_map(|(key, count)| key.parse::<u8>().ok().map(|turn| (turn, *count)))
            .collect();
        turns.sort_unstable_by_key(|&(turn, _)| turn);
        for (turn, count) in turns {
            card.move_turns
                .extend(std::iter::repeat_n(turn, count as usize));
            card.player_moves_used = card.player_moves_used.saturating_add(count);
        }
    } else {
        card.player_moves_used = reference.player_moves_used.unwrap_or(0);
    }
}

fn needs_observed_reference(reference: &CardRef) -> bool {
    reference.entity_id.is_some()
        || reference.merged_to_entity_id.is_some()
        || reference.latest_resolution_order.is_some()
        || reference.turns_drawn.is_some()
        || reference.stat_updates.is_some()
}

fn observed_update(update: &ObservedStatUpdate) -> Option<CardStatUpdate> {
    let stat = match update.stat.as_str() {
        "Power" => Stat::Power,
        "Cost" => Stat::Cost,
        _ => return None,
    };
    let operation = match update.update.as_str() {
        "Add" => StatOperation::Add,
        "Multiply" => StatOperation::Multiply,
        "Set" => StatOperation::Set,
        _ => return None,
    };
    Some(CardStatUpdate {
        stat,
        operation,
        value: update.value,
        // Unknown sources must not be mistaken for either player's effects.
        source: update.source.filter(|&seat| seat < 2).unwrap_or(usize::MAX),
        ongoing: false,
        ongoing_source: None,
    })
}
pub fn recruit_weights() -> HashMap<String, f64> {
    let pool = recruit_cards();
    let mut sizes: HashMap<&str, usize> = HashMap::new();
    for id in &pool {
        *sizes.entry(&CARD_DATA[id].source).or_default() += 1;
    }
    let reveals = |s: &str| match s {
        "Starter Card" => 15,
        "Collection Level 1-14" => 5,
        "Series 1" => 4,
        "Recruit Season" => 2,
        _ => 0,
    };
    let total = 12 + sizes.keys().map(|s| reveals(s)).sum::<usize>();
    pool.iter()
        .map(|id| {
            let source = CARD_DATA[id].source.as_str();
            (
                id.clone(),
                (reveals(source) as f64 / sizes[source] as f64 + 12.0 / pool.len() as f64)
                    / total as f64,
            )
        })
        .collect()
}

/// Recover Monstro's pre-ongoing cost only from an authoritative simulated
/// value or an available, unambiguously attributable client stat ledger.
pub(crate) fn monstro_cost_basis(position: &Position, card: &CardRef) -> Result<i32, String> {
    if let Some(basis) = card.cost_basis {
        return Ok(basis);
    }
    let updates = card
        .stat_updates
        .as_ref()
        .ok_or("MonstroOctopus stat_updates cost history not exported")?;
    let mut basis = CARD_DATA[&card.id].cost;
    for update in updates.iter().filter(|update| update.stat == "Cost") {
        let card_source = update.updater_card_id.as_deref();
        let location_source = update.updater_location_id.as_deref();
        let sources = usize::from(card_source.is_some())
            + usize::from(location_source.is_some())
            + usize::from(update.updater_faction_id.is_some())
            + usize::from(update.updater_augment_id.is_some())
            + usize::from(update.updater_league_id.is_some());
        if sources != 1
            || update.updater_faction_id.is_some()
            || update.updater_augment_id.is_some()
            || update.updater_league_id.is_some()
        {
            return Err("MonstroOctopus stat_updates cost updater attribution incomplete".into());
        }
        if card_source == Some("MonstroOctopus") {
            continue;
        }
        if let Some(id) = location_source {
            let def =
                locations::get(id).ok_or("MonstroOctopus stat_updates location updater unknown")?;
            if def.modify_cost.is_some() {
                let active = position
                    .locations
                    .iter()
                    .any(|lane| !lane.destroyed && lane.id.as_deref() == Some(id));
                if active
                    && def.on_reveal.is_none()
                    && def.on_turn_start.is_none()
                    && def.on_turn_end.is_none()
                    && def.on_event.is_none()
                {
                    continue;
                }
                return Err(
                    "MonstroOctopus stat_updates location cost attribution ambiguous".into(),
                );
            }
        }
        if let Some(id) = card_source {
            let def = cards::get(id).ok_or("MonstroOctopus stat_updates card updater unknown")?;
            if def.ongoing.is_some()
                || def.in_hand_ongoing.is_some()
                || def.late_ongoing.is_some()
                || def.late_in_hand_ongoing.is_some()
                || def.conditional_in_hand_ongoing.is_some()
            {
                return Err("MonstroOctopus stat_updates card cost attribution ambiguous".into());
            }
        }
        basis = match update.update.as_str() {
            "Add" => basis.checked_add(update.value),
            "Multiply" => basis.checked_mul(update.value),
            "Set" => Some(update.value),
            _ => None,
        }
        .ok_or("MonstroOctopus stat_updates cost operation invalid")?;
    }
    Ok(basis)
}

/// Randomly generated visible abilities are part of one observed position,
/// not a fresh hidden-information deal for each rollout.
#[derive(Default)]
pub(crate) struct FrozenGeneratedAbilities {
    cards: Vec<FrozenGeneratedCard>,
}

struct FrozenGeneratedCard {
    lane: usize,
    owner: usize,
    slot: usize,
    state: Option<Box<dyn crate::game::CardState>>,
    abilities: Vec<crate::engine_ext::abilities::AbilityInstance>,
    default_red: bool,
    override_ability: Option<&'static CardDef>,
    virtual_red: Vec<FrozenVirtualRed>,
}

struct FrozenVirtualRed {
    source: (usize, usize, usize),
    source_instance: Option<u64>,
    ability: crate::engine_ext::abilities::AbilityInstance,
    children: Vec<crate::engine_ext::abilities::AbilityInstance>,
}

impl FrozenGeneratedAbilities {
    pub(crate) fn capture(game: &Game<'_>, seat: usize) -> Self {
        let mut cards = Vec::new();
        let identity = |id: usize| {
            let card = &game.cards[id];
            let lane = card.lane?;
            let owner = usize::from(card.owner != seat);
            let slot = game.lanes[lane]
                .cards
                .iter()
                .copied()
                .filter(|&other| {
                    game.cards[other].owner == card.owner
                        && (owner == 0 || game.cards[other].revealed)
                })
                .position(|other| other == id)?;
            Some((lane, owner, slot))
        };
        for (lane, location) in game.lanes.iter().enumerate() {
            for relative_owner in 0..2 {
                let owner = if relative_owner == 0 { seat } else { 1 - seat };
                for (slot, &id) in location
                    .cards
                    .iter()
                    .filter(|&&id| {
                        game.cards[id].owner == owner
                            && (relative_owner == 0 || game.cards[id].revealed)
                    })
                    .enumerate()
                {
                    let card = &game.cards[id];
                    let default_red = card.revealed && card.default_ability().id == "RedOnslaught";
                    let virtual_red: Vec<_> = card
                        .virtual_ongoing_ids
                        .iter()
                        .filter_map(|entry| {
                            if entry.4.def.id != "RedOnslaught" {
                                return None;
                            }
                            Some(FrozenVirtualRed {
                                source: identity(entry.0)?,
                                source_instance: entry.1,
                                ability: entry.4.clone(),
                                children: card
                                    .composed_abilities
                                    .iter()
                                    .filter(|ability| {
                                        ability.parent.is_some_and(|parent| parent.0 == entry.4.id)
                                    })
                                    .cloned()
                                    .collect(),
                            })
                        })
                        .collect();
                    if default_red || !virtual_red.is_empty() {
                        cards.push(FrozenGeneratedCard {
                            lane,
                            owner: relative_owner,
                            slot,
                            state: card.state.clone(),
                            abilities: card
                                .composed_abilities
                                .iter()
                                .filter(|ability| {
                                    ability.parent == Some((0, card.default_ability_generation))
                                })
                                .cloned()
                                .collect(),
                            default_red,
                            override_ability: card.override_ability,
                            virtual_red,
                        });
                    }
                }
            }
        }
        Self { cards }
    }

    fn apply(&self, game: &mut Game<'_>) {
        fn board_card(game: &Game<'_>, identity: (usize, usize, usize)) -> Option<usize> {
            let (lane, owner, slot) = identity;
            game.lanes[lane]
                .cards
                .iter()
                .copied()
                .filter(|&id| {
                    game.cards[id].owner == owner && (owner == 0 || game.cards[id].revealed)
                })
                .nth(slot)
        }
        for source in &self.cards {
            let Some(target) = board_card(game, (source.lane, source.owner, source.slot)) else {
                continue;
            };
            let generation = game.cards[target].default_ability_generation;
            if source.default_red {
                game.cards[target].override_ability = source.override_ability;
                game.cards[target].state = source.state.clone();
                game.cards[target].composed_abilities = source
                    .abilities
                    .iter()
                    .cloned()
                    .map(|mut ability| {
                        ability.origin = Some(target);
                        ability.parent = Some((0, generation));
                        ability
                    })
                    .collect();
            }
            for virtual_red in &source.virtual_red {
                let Some(other) = board_card(game, virtual_red.source) else {
                    continue;
                };
                let source_generation = if virtual_red.source_instance.is_none() {
                    game.cards[other].default_ability_generation
                } else {
                    0
                };
                let mut ability = virtual_red.ability.clone();
                ability.origin = Some(other);
                ability.copied_from = Some((other, virtual_red.source_instance, source_generation));
                game.cards[target].virtual_ongoing_ids.push((
                    other,
                    virtual_red.source_instance,
                    source_generation,
                    generation,
                    ability,
                ));
                game.cards[target].composed_abilities.extend(
                    virtual_red.children.iter().cloned().map(|mut child| {
                        child.origin = Some(target);
                        child
                    }),
                );
            }
        }
    }
}

pub fn build_game<'a>(position: &Position, rng: &'a mut PythonRandom) -> Result<Game<'a>, String> {
    build_game_with_generated(position, rng, None)
}

pub(crate) fn build_game_with_generated<'a>(
    position: &Position,
    rng: &'a mut PythonRandom,
    frozen: Option<&FrozenGeneratedAbilities>,
) -> Result<Game<'a>, String> {
    if position.turn == 0 || position.turn > position.total_turns || position.hand.len() > 7 {
        return Err("invalid turn or hand size".into());
    }
    let mut deck_ids: Vec<_> = position
        .deck
        .iter()
        .enumerate()
        .map(|(index, id)| {
            (
                id.clone(),
                position
                    .deck_created
                    .as_ref()
                    .and_then(|v| v.get(index))
                    .copied()
                    .flatten(),
            )
        })
        .collect();
    let missing = position
        .deck_count
        .unwrap_or(deck_ids.len())
        .saturating_sub(deck_ids.len());
    if !cards::reference_rules_enabled()
        && missing > 0
        && position.hand.iter().any(|card| card.id == "MonstroOctopus")
    {
        return Err("MonstroOctopus own deck card identities incomplete".into());
    }
    if missing > 0 {
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
            .chain(position.graveyard.iter().flatten().map(|c| &c.id))
            .chain(position.banished.iter().flatten().map(|c| &c.id))
            .collect();
        let pool = if position.mode.as_deref() == Some("recruit") && !outside(known.iter().copied())
        {
            recruit_cards()
        } else {
            all_cards()
        };
        let candidates: Vec<_> = pool.into_iter().filter(|id| !known.contains(id)).collect();
        deck_ids.extend(
            sample(rng, &candidates, missing)?
                .into_iter()
                .map(|id| (id, None)),
        );
    }
    deck_ids.sort();
    rng.shuffle(&mut deck_ids);
    if let Some(count) = position.deck_count {
        deck_ids.truncate(count);
    }
    let opp = &position.opponent;
    let recruit = opp.deck_list.is_none()
        && opp.pool.is_none()
        && position.mode.as_deref() == Some("recruit")
        && !outside(opp.seen.iter().chain(&opp.known));
    let mut pool = if let Some(list) = &opp.deck_list {
        list.clone()
    } else if let Some(list) = &opp.pool {
        list.clone()
    } else if recruit {
        recruit_cards()
    } else {
        all_cards()
    };
    pool.sort();
    let mut known_set = HashSet::new();
    for id in &opp.known {
        if cards::get(id).is_none() || cards::is_token(id) {
            return Err("opponent.known contains a non-deck-eligible card ID".into());
        }
        known_set.insert(id);
    }
    if known_set.len() > 12 {
        return Err("opponent.known contains more than 12 unique card IDs".into());
    }
    let mut forced_set = HashSet::new();
    let mut forced: Vec<String> = opp
        .known
        .iter()
        .filter(|id| !opp.seen.contains(id) && forced_set.insert(*id))
        .cloned()
        .collect();
    let mut candidates: Vec<_> = pool
        .iter()
        .filter(|id| !opp.seen.contains(id) && !known_set.contains(id))
        .cloned()
        .collect();
    let count = opp.hand
        + opp.deck
        + position
            .locations
            .iter()
            .map(|l| l.theirs_face_down)
            .sum::<usize>();
    if forced.len() > count {
        return Err("opponent.known exceeds the hidden card count".into());
    }
    let count = count - forced.len();
    let mut hidden = if count == 0 {
        Vec::new()
    } else if candidates.len() >= count {
        if recruit && opp.prior.as_deref() != Some("uniform") {
            candidates.dedup();
            let weights = recruit_weights();
            let mut clocks: Vec<_> = candidates
                .iter()
                .map(|id| ((-rng.random()).ln_1p() / weights[id], id.clone()))
                .collect();
            clocks.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
            if count > clocks.len() {
                return Err("not enough distinct weighted cards".into());
            }
            clocks.into_iter().take(count).map(|(_, id)| id).collect()
        } else {
            sample(rng, &candidates, count)?
        }
    } else {
        let fallback = if opp.known.is_empty() {
            &pool
        } else {
            &candidates
        };
        if fallback.is_empty() {
            return Err(format!(
                "opponent needs {count} hidden cards, but the pool is empty"
            ));
        }
        let fallback = fallback.clone();
        while candidates.len() < count {
            let index = (rng.random() * fallback.len() as f64) as usize;
            candidates.push(fallback[index].clone());
        }
        rng.shuffle(&mut candidates);
        candidates
    };
    if !forced.is_empty() {
        hidden.append(&mut forced);
        rng.shuffle(&mut hidden);
    }
    let known_locations: Vec<_> = position
        .locations
        .iter()
        .filter_map(|l| l.id.as_ref())
        .collect();
    let recruit_locations = position.mode.as_deref() == Some("recruit")
        && known_locations.iter().all(|id| {
            LOCATION_DATA.get(id.as_str()).is_some_and(|d| {
                d.status == "released" && matches!(d.difficulty.as_str(), "Basic" | "Standard")
            })
        });
    let possible: Vec<_> = all_locations(recruit_locations)
        .into_iter()
        .filter(|id| {
            !known_locations.contains(&id)
                && (cards::reference_rules_enabled() || *id != "SacredTimeline")
        })
        .collect();
    let hidden_count = position
        .locations
        .iter()
        .filter(|lane| lane.id.is_none() && !lane.destroyed)
        .count();
    let hidden_locations = sample(rng, &possible, hidden_count)?;
    let mut selected = hidden_locations.into_iter();
    let lanes: [Lane; 3] = std::array::from_fn(|i| {
        let src = &position.locations[i];
        let id = src.id.clone().unwrap_or_else(|| {
            if src.destroyed {
                String::new()
            } else {
                selected.next().unwrap()
            }
        });
        let def = location_def(&id).unwrap_or(&UNKNOWN_LOCATION);
        Lane {
            def,
            observed_id: (!src.destroyed && def.id != id).then_some(id),
            revealed: src.id.is_some(),
            destroyed: src.destroyed,
            ability_flags: crate::game::LocationAbilityFlags::default(),
            copied_reveal_turn: None,
            revealed_turn: src.id.as_ref().map(|_| {
                src.revealed_turn
                    .unwrap_or(((i + 1) as u8).min(position.turn))
            }),
            cards: Vec::new(),
            bonus: [0; 2],
            contributions: Vec::new(),
            multiplier: [1; 2],
            state: src
                .ottos_lab_spent
                .map(|spent| Box::new(spent) as Box<dyn crate::game::CardState>)
                .or_else(|| {
                    if cards::reference_rules_enabled() {
                        return None;
                    }
                    let history = src.history_state.as_ref()?;
                    match src.id.as_deref()? {
                        "Vormir" => history
                            .vormir_consumed
                            .as_ref()?
                            .complete()
                            .map(|spent| Box::new(spent) as Box<dyn crate::game::CardState>),
                        "TimeTheater" => history
                            .time_theater_used
                            .as_ref()?
                            .complete()
                            .map(|spent| Box::new(spent) as Box<dyn crate::game::CardState>),
                        "TheAbbey" | "Raft" | "WhiteHotRoom" => history
                            .is_given
                            .map(|spent| Box::new(spent) as Box<dyn crate::game::CardState>),
                        _ => None,
                    }
                })
                .or_else(|| def.initial_state.map(|init| init())),
        }
    });
    let priority = match position.priority.as_deref() {
        None => None,
        Some("me") => Some(0),
        Some("opponent") => Some(1),
        _ => return Err("invalid priority".into()),
    };
    let mut game = Game {
        cards: Vec::new(),
        move_provenance_known: false,
        last_move_source: None,
        ability_call_depth: 0,
        replay_hooks: None,
        players: [Player::default(), Player::default()],
        human_seat: Some(0),
        lanes,
        location_pool: all_locations(position.mode.as_deref() == Some("recruit"))
            .iter()
            .map(|id| location_def(id).expect("registered location"))
            .collect(),
        turn: position.turn,
        stakes: None,
        next_resolution_order: 1,
        is_wait_for_staging: true,
        is_start_turn: false,
        reveal_priority: priority,
        total_turns: position.total_turns,
        played: Default::default(),
        pending_plays: Default::default(),
        played_on_reveal: [false; 2],
        delays_reveals: false,
        ending_reveals: false,
        delayed_reveals: Default::default(),
        deferred_added_reveals: Vec::new(),
        post_resolve_plays: Vec::new(),
        end_turn_lane_order: None,
        current_location: 0,
        current_ongoing_source: None,
        location_reveal_order: [0, 1, 2],
        destroyed: None,
        score_overrides: [None; 2],
        merge_batches: Vec::new(),
        staging_order: None,
        rng: RefCell::new(crate::game::GameRandom::Borrowed(rng)),
    };
    if let Some(list) = &position.opponent.deck_list {
        game.players[1].starting_deck = list
            .iter()
            .map(|id| card_def(id).ok().map(|def| def.id))
            .collect();
    }
    // Python's Game._from_position: revealed lanes count as already scheduled, and the
    // remaining reveals follow the hidden lanes in lane order.
    let mut order: Vec<usize> = (0..3).filter(|&lane| game.lanes[lane].revealed).collect();
    if let Some(snapped) = position.snapped_this_turn {
        for (seat, did_snap) in snapped.into_iter().enumerate() {
            if did_snap {
                game.players[seat].snapped_turns.push(position.turn);
            }
        }
    }
    game.current_location = order.len();
    order.extend((0..3).filter(|&lane| !game.lanes[lane].revealed));
    game.location_reveal_order = [order[0], order[1], order[2]];
    game.delays_reveals = (0..game.lanes.len()).any(|lane| {
        game.location_abilities_active(lane)
            && game.lanes[lane].def.delays_reveals
            && game.lanes[lane].revealed_turn == Some(position.turn)
    });
    if let Some(ledger) = position.provenance.as_ref().and_then(|p| p.ledger.as_ref()) {
        for item in &ledger.items {
            if let Some((seat, until)) = position.black_swan_effect(item) {
                game.players[seat].attach_cost_effect(
                    crate::game::PlayerCostEffectKind::BlackSwan {
                        until,
                        printed_cost: 1,
                        value: 0,
                    },
                );
            }
        }
    }
    game.players[0].energy = position.energy;
    for (seat, default, maximum) in [
        (0, position.default_energy, position.max_energy),
        (1, opp.default_energy, opp.max_energy),
    ] {
        let base = default.unwrap_or(i32::from(position.turn));
        game.players[seat].default_energy_offset = base - i32::from(position.turn);
        if let Some(maximum) = maximum {
            game.players[seat].permanent_energy = maximum - base;
        }
    }
    game.players[0].next_play_power = position.pending_power;
    game.players[1].next_play_power = opp.pending_power;
    let mut observed_refs: Vec<(usize, &CardRef)> = Vec::new();
    let mut powers = Vec::new();
    for r in &position.hand {
        let mut c = make_card(&r.id, 0)?;
        c.zone = Some(crate::zones::CardZone::Hand(0));
        apply_card_reference(&mut c, r, position.turn);
        if let Some(cost) = r.cost {
            c.cost = cost;
        }
        if let Some(power) = r.power {
            c.permanent = power;
            if r.entity_id.is_some() {
                powers.push((game.cards.len(), power));
            }
        }
        if needs_observed_reference(r) {
            observed_refs.push((game.cards.len(), r));
        }
        let id = game.cards.len();
        game.cards.push(c);
        game.push_hand(0, id);
    }
    for (id, created) in &deck_ids {
        let index = game.cards.len();
        let mut card = make_card(id, 0)?;
        card.created = created.unwrap_or(false);
        card.zone = Some(crate::zones::CardZone::Deck(0));
        game.cards.push(card);
        game.push_deck(0, index);
    }
    for (i, id) in hidden.iter().take(opp.hand + opp.deck).enumerate() {
        let index = game.cards.len();
        game.cards.push(make_card(id, 1)?);
        game.cards[index].zone = Some(if i < opp.hand {
            crate::zones::CardZone::Hand(1)
        } else {
            crate::zones::CardZone::Deck(1)
        });
        if i < opp.hand {
            game.push_hand(1, index);
        } else {
            game.push_deck(1, index);
        }
    }
    let delayed = position.turn.checked_sub(1).filter(|&t| {
        t > 0
            && position
                .locations
                .iter()
                .any(|l| l.id.as_deref() == Some("Lemuria") && l.revealed_turn == Some(t))
    });
    let mut hidden_board = hidden.iter().skip(opp.hand + opp.deck);
    let mut revealed = Vec::new();
    for (lane, src) in position.locations.iter().enumerate() {
        for (owner, refs) in [&src.mine, &src.theirs].into_iter().enumerate() {
            for r in refs {
                let mut c = make_card(&r.id, owner)?;
                c.cost = r.cost.unwrap_or(c.cost);
                c.lane = Some(lane);
                c.zone = Some(crate::zones::CardZone::Location(lane));
                c.turn_played = r.played_turn.unwrap_or(0);
                c.revealed = r.revealed;
                apply_card_reference(&mut c, r, position.turn);
                c.observed_can_activate = r.can_activate;
                c.observed_move_targets = r.move_targets.as_ref().map(|lanes| {
                    lanes
                        .iter()
                        .filter(|&&lane| lane < 3)
                        .fold(0u8, |mask, &lane| mask | (1 << lane))
                });
                let id = game.cards.len();
                if needs_observed_reference(r) {
                    observed_refs.push((id, r));
                }
                game.cards.push(c);
                game.push_location(lane, id);
                if r.revealed {
                    revealed.push(id);
                } else if r.played_turn == Some(position.turn) {
                    game.played[owner].push((id, lane));
                } else if delayed.is_some() && r.played_turn == delayed {
                    game.delayed_reveals[owner].push(crate::game::QueuedReveal {
                        card: id,
                        staging: None,
                        persistent: false,
                    });
                }
                if let Some(power) = r.power {
                    powers.push((id, power));
                }
            }
        }
        for _ in 0..src.theirs_face_down {
            let mut c = make_card(hidden_board.next().unwrap(), 1)?;
            c.lane = Some(lane);
            c.zone = Some(crate::zones::CardZone::Location(lane));
            let id = game.cards.len();
            if let Some(t) = delayed {
                c.turn_played = t;
                game.delayed_reveals[1].push(crate::game::QueuedReveal {
                    card: id,
                    staging: None,
                    persistent: false,
                });
            }
            game.cards.push(c);
            game.push_location(lane, id);
        }
    }
    // The reader exposes these public zones without the lane of destruction.
    // Keep the lane unknown rather than manufacturing a location.
    for (owner, graveyard, banished) in [
        (
            0,
            position.graveyard.as_deref(),
            position.banished.as_deref(),
        ),
        (1, opp.graveyard.as_deref(), opp.banished.as_deref()),
    ] {
        for (is_graveyard, refs) in [(true, graveyard), (false, banished)] {
            for r in refs.unwrap_or_default() {
                let mut card = make_card(&r.id, owner)?;
                card.zone = Some(if is_graveyard {
                    crate::zones::CardZone::Graveyard(owner)
                } else {
                    crate::zones::CardZone::Banished(owner)
                });
                apply_card_reference(&mut card, r, position.turn);
                card.cost = r.cost.unwrap_or(card.cost);
                card.permanent = r.power.unwrap_or(card.permanent);
                card.revealed = r.revealed;
                card.turn_played = r.played_turn.unwrap_or(0);
                let id = game.cards.len();
                if needs_observed_reference(r) {
                    observed_refs.push((id, r));
                }
                game.cards.push(card);
                if is_graveyard {
                    game.push_graveyard(owner, id, None);
                    game.players[owner].graveyard_order.push(id);
                } else {
                    game.push_banished(owner, id);
                }
            }
        }
    }
    if !cards::reference_rules_enabled() {
        game.refresh_location_ability_flags();
    }
    let entities: HashMap<i64, usize> = observed_refs
        .iter()
        .filter_map(|&(id, r)| r.entity_id.map(|entity| (entity, id)))
        .collect();
    if !cards::reference_rules_enabled()
        && let Some(ledger) = position.provenance.as_ref().and_then(|p| p.ledger.as_ref())
    {
        for item in &ledger.items {
            if let Some((entity, moves, current)) = position.intrinsic_move_state(item)
                && let Some(&card) = entities.get(&entity)
            {
                game.cards[card].player_moves_used = moves;
                game.cards[card].player_move_turn = current;
            }
        }
    }
    for &(id, r) in &observed_refs {
        game.cards[id].merged_to = r
            .merged_to_entity_id
            .filter(|&entity| entity != 0)
            .and_then(|entity| entities.get(&entity).copied());
    }
    if !cards::reference_rules_enabled() {
        for seat in 0..2 {
            if position.current_action_present(seat) == Some(true) {
                game.players[seat].non_play_action_turn = Some(position.turn);
            }
            // Action completeness is per turn, so a complete recent turn remains
            // useful even if an earlier turn was not observed. These record
            // staged player moves only, not forced moves.
            if let Some(history) = position.history.as_ref()
                && let (Some(actions), Some(completeness)) =
                    (&history.actions, &history.action_completeness)
            {
                let (owner, turns) = if seat == 0 {
                    ("local", &completeness.local)
                } else {
                    ("opponent", &completeness.opponent)
                };
                for action in actions.iter().filter(|action| {
                    action.owner == owner
                        && action.turn > 0
                        && action.turn < position.turn
                        && turns.get(&action.turn.to_string()) == Some(&true)
                        && action.resolved
                        && action.turn_resolved == Some(action.turn)
                }) {
                    let target = match action.kind.as_str() {
                        "play" => &mut game.players[seat].observed_play_turns,
                        "move" => &mut game.players[seat].observed_move_turns,
                        _ => continue,
                    };
                    if !target.contains(&action.turn) {
                        target.push(action.turn);
                    }
                }
            }
            if let Some(actions) = position.complete_past_actions(seat) {
                for action in actions.into_iter().filter(|action| action.kind == "play") {
                    game.players[seat]
                        .staged_lane_history
                        .push((action.turn, action.location_slot));
                    if let Some(&card) = entities.get(&action.card_entity_id) {
                        game.players[seat].play_history.push((action.turn, card));
                        game.players[seat].resolved_play_history.push(
                            crate::game::ResolvedPlayEvent {
                                turn: action.turn,
                                card,
                                def_id: game.cards[card].def.id,
                                printed_cost: game.printed_cost(card),
                                lane: Some(action.location_slot),
                                staged_lane: Some(action.location_slot),
                                power: None,
                            },
                        );
                    }
                }
            }
        }
    }
    // Use observed resolution order; use the historic deterministic approximation
    // only for cards whose order is unavailable.
    let mut resolution_order = revealed.clone();
    let first = priority.unwrap_or(0);
    resolution_order.sort_by_key(|&id| {
        let card = &game.cards[id];
        (
            card.turn_played,
            usize::from(card.owner != first),
            card.lane.unwrap_or(0),
            game.zone_position(id).unwrap_or(0),
        )
    });
    let explicit: HashMap<_, _> = observed_refs
        .iter()
        .filter_map(|(id, r)| r.latest_resolution_order.map(|order| (*id, order)))
        .collect();
    if explicit.is_empty() {
        for id in resolution_order {
            game.cards[id].latest_resolution_order = game.next_resolution_order;
            game.next_resolution_order += 1;
        }
    } else {
        let mut occupied: HashSet<u32> = explicit.values().copied().collect();
        for id in resolution_order {
            let order = if let Some(&order) = explicit.get(&id) {
                order
            } else {
                while occupied.contains(&game.next_resolution_order) {
                    game.next_resolution_order = game
                        .next_resolution_order
                        .checked_add(1)
                        .ok_or("resolution order overflow")?;
                }
                let order = game.next_resolution_order;
                occupied.insert(order);
                game.next_resolution_order =
                    order.checked_add(1).ok_or("resolution order overflow")?;
                order
            };
            game.cards[id].latest_resolution_order = order;
        }
        if let Some(&maximum) = occupied.iter().max() {
            game.next_resolution_order = game
                .next_resolution_order
                .max(maximum.checked_add(1).ok_or("resolution order overflow")?);
        }
    }
    for id in revealed {
        if matches!(game.cards[id].def.id, "Hawkeye" | "JessicaJones")
            && position.turn > 1
            && game.cards[id].turn_played == position.turn - 1
        {
            let lane = game.cards[id].lane.unwrap();
            let before = game.cards[id].turn_played;
            let active = game.location_abilities_active(lane)
                && game.lanes[lane].revealed_turn.unwrap_or(0) <= before;
            // Reconstruction heuristic, not a client predicate: excluding hidden
            // abilities is provisional; restored cards have no hidden instances.
            let sources: Vec<_> = game.lanes[lane]
                .cards
                .iter()
                .copied()
                .filter(|&s| {
                    game.cards[s].revealed
                        && game.cards[s].has_ongoing(false)
                        && game.cards[s].turn_played < before
                })
                .collect();
            if !game.lanes[lane].ability_flags.on_reveal
                || sources.iter().any(|&s| {
                    game.cards[s].ongoing_guard_abilities().any(|a| {
                        a.supports(crate::engine_ext::abilities::AbilityKind::Ongoing)
                            && a.def.blocks_reveal
                    })
                })
            {
                continue;
            }
            let base = if active {
                game.lanes[lane].def.ongoing_repeats as usize
            } else {
                1
            };
            let mut runs = if active {
                game.lanes[lane].def.reveal_repeats as usize
            } else {
                1
            };
            for &source in &sources {
                for repetitions in game.cards[source]
                    .ongoing_guard_abilities()
                    .filter(|a| a.supports(crate::engine_ext::abilities::AbilityKind::Ongoing))
                    .filter_map(|a| a.def.reveal_repetitions)
                {
                    let extra: usize = sources
                        .iter()
                        .filter(|&&other| {
                            other != source && game.cards[other].owner == game.cards[source].owner
                        })
                        .map(|&other| {
                            game.cards[other]
                                .ongoing_guard_abilities()
                                .filter(|a| {
                                    a.supports(crate::engine_ext::abilities::AbilityKind::Ongoing)
                                })
                                .map(|a| a.def.extra_ongoing_runs as usize)
                                .sum::<usize>()
                        })
                        .sum();
                    runs += repetitions(&game, source, id) as usize * base * (1 + extra);
                }
            }
            let hawkeye = game.cards[id].def.id == "Hawkeye";
            for _ in 0..runs {
                game.cards[id].attach_watch(
                    id,
                    position.turn,
                    lane,
                    hawkeye,
                    if hawkeye { 4 } else { 5 },
                );
            }
        }
    }
    // The client CardStatUpdate exposes stat, operation, value, player and
    // updater IDs, but no ongoing flag. get_Power/get_Cost already report
    // totals: seed ledger rows as history, never reapply them as stat deltas.
    for &(id, reference) in &observed_refs {
        if let Some(draws) = &reference.turns_drawn {
            let owner = game.cards[id].owner;
            game.players[owner]
                .draws
                .extend(draws.iter().map(|&(turn, _)| (turn, id)));
        }
        if let Some(updates) = &reference.stat_updates {
            game.cards[id]
                .stat_updates
                .extend(updates.iter().filter_map(observed_update));
        }
    }
    for player in &mut game.players {
        player.draws.sort_by_key(|&(turn, _)| turn);
    }
    for id in 0..game.cards.len() {
        if game.cards[id].def.id == "ThanosFracturedFrontier"
            && (game.cards[id].lane.is_some()
                || game.players[game.cards[id].owner].hand.contains(&id)
                || game.players[game.cards[id].owner].deck.contains(&id))
        {
            crate::engine_ext::quickdraw::install(&mut game, id);
        }
    }
    // Reconstruct the full ongoing ledger before entering the staging window.
    game.is_wait_for_staging = false;
    // MonstroOctopusBehavior::ReduceCost (0x7efec0c) reads the literal
    // deck top. A live cost observation conditions the hidden order: the
    // first qualifying entry in an already-uniform shuffle is uniform.
    let mut monstro = Vec::new();
    if !cards::reference_rules_enabled() {
        for (index, card) in position.hand.iter().enumerate() {
            if card.id == "MonstroOctopus" {
                let observed = card
                    .cost
                    .ok_or("MonstroOctopus current cost not exported")?;
                let basis = monstro_cost_basis(position, card)?;
                monstro.push((game.players[0].hand[index], observed, basis));
            }
        }
    }
    if !cards::reference_rules_enabled() && !monstro.is_empty() {
        let mut trial_rng = game.fork_rng();
        let mut trial = game.fork(trial_rng.as_mut());
        let deck_len = trial.players[0].deck.len();
        let mut chosen = None;
        for index in 0..deck_len.max(1) {
            if deck_len != 0 {
                trial.swap_deck(0, 0, index);
            }
            for &(card, _, basis) in &monstro {
                trial.cards[card].cost = basis;
            }
            trial.ongoing();
            let matches = monstro
                .iter()
                .all(|&(card, observed, _)| trial.card_cost(card) == observed);
            if deck_len != 0 {
                trial.swap_deck(0, 0, index);
            }
            if matches {
                chosen = Some(index);
                break;
            }
        }
        let Some(index) = chosen else {
            return Err(
                "MonstroOctopus observed cost is inconsistent with every possible deck top".into(),
            );
        };
        if deck_len != 0 {
            game.swap_deck(0, 0, index);
        }
        for &(card, _, basis) in &monstro {
            game.cards[card].cost = basis;
        }
    }
    if let Some(frozen) = frozen {
        frozen.apply(&mut game);
    }
    game.ongoing();
    // The reader does not label ongoing rows. An observed row is removed only
    // when its named active updater and an identical recomputed contribution
    // match; a multiset match preserves repeated independent contributions.
    // Faction, augment and league updaters are historical, never card/location ongoing.
    if observed_refs.iter().any(|(_, r)| {
        r.stat_updates
            .as_ref()
            .is_some_and(|updates| !updates.is_empty())
    }) {
        // This metadata heuristic has no direct client predicate. Excluding
        // hidden abilities is provisional; restored cards have no hidden instances.
        let mut active_cards: HashSet<_> = game
            .lanes
            .iter()
            .flat_map(|lane| &lane.cards)
            .filter_map(|&id| {
                let card = &game.cards[id];
                (card.revealed && card.has_ongoing(false)).then_some(card.def.id)
            })
            .collect();
        active_cards.extend(
            game.players
                .iter()
                .flat_map(|player| &player.hand)
                .filter_map(|&id| {
                    let card = &game.cards[id];
                    (!card.abilities_removed
                        && card.abilities().any(|a| {
                            a.supports(crate::engine_ext::abilities::AbilityKind::Ongoing)
                                && (a.def.in_hand_ongoing.is_some()
                                    || a.def.late_in_hand_ongoing.is_some()
                                    || a.def.conditional_in_hand_ongoing.is_some())
                        }))
                    .then_some(card.def.id)
                }),
        );
        let active_locations: HashSet<_> = game
            .lanes
            .iter()
            .filter(|lane| lane.revealed)
            .map(|lane| lane.def.id)
            .collect();
        let mut removed = false;
        for &(id, reference) in &observed_refs {
            let Some(observed) = &reference.stat_updates else {
                continue;
            };
            let mut used = HashSet::new();
            let mut remove = Vec::new();
            let mut historical = 0;
            for entry in observed {
                let Some(update) = observed_update(entry) else {
                    continue;
                };
                let is_external = entry.updater_faction_id.is_some()
                    || entry.updater_augment_id.is_some()
                    || entry.updater_league_id.is_some();
                let is_active_updater = !is_external
                    && (entry
                        .updater_card_id
                        .as_deref()
                        .is_some_and(|name| active_cards.contains(name))
                        || entry
                            .updater_location_id
                            .as_deref()
                            .is_some_and(|name| active_locations.contains(name)));
                if is_active_updater
                    && let Some((index, _)) =
                        game.cards[id]
                            .stat_updates
                            .iter()
                            .enumerate()
                            .find(|(index, current)| {
                                current.ongoing
                                    && !used.contains(index)
                                    && current.stat == update.stat
                                    && current.operation == update.operation
                                    && current.value == update.value
                                    && (update.source == usize::MAX
                                        || current.source == update.source)
                            })
                {
                    used.insert(index);
                    remove.push(historical);
                }
                historical += 1;
            }
            for index in remove.into_iter().rev() {
                game.cards[id].stat_updates.remove(index);
                removed = true;
            }
        }
        if removed {
            game.ongoing();
        }
    }
    for (id, power) in powers {
        game.cards[id].permanent = power - game.cards[id].ongoing - game.cards[id].location_power;
    }
    for (index, r) in position.hand.iter().enumerate() {
        if !cards::reference_rules_enabled() && r.id == "MonstroOctopus" && r.cost.is_some() {
            continue;
        }
        if let Some(observed) = r.cost {
            let id = game.players[0].hand[index];
            let printed = CARD_DATA[&r.id].cost;
            let mut costs: Vec<_> = (0..observed + 3).collect();
            costs.sort_by_key(|&cost| (cost != printed, (cost - observed).abs()));
            let mut found = false;
            for cost in costs {
                game.cards[id].cost = cost;
                if game.card_cost(id) == observed {
                    found = true;
                    break;
                }
            }
            if !found {
                game.cards[id].cost = observed;
            }
        }
    }
    game.is_wait_for_staging = true;
    game.players[1].energy = opp.energy.unwrap_or_else(|| {
        opp.max_energy
            .or(opp.default_energy)
            .unwrap_or(i32::from(position.turn))
            + game
                .lanes
                .iter()
                .enumerate()
                .filter(|(i, _)| game.location_abilities_active(*i))
                .map(|(i, l)| {
                    (if l.revealed_turn == Some(position.turn) {
                        l.def.reveal_energy
                    } else {
                        0
                    }) + l.def.energy_bonus.map_or(0, |f| f(&game, i, 1))
                })
                .sum::<i32>()
    });
    Ok(game)
}

/// Canonical, ownership-preserving materialized state for cross-language parity.
pub fn state_json(game: &Game<'_>) -> Value {
    let card = |id: usize| {
        let c = &game.cards[id];
        json!({"id": c.def.id, "owner": c.owner, "cost": game.card_cost(id), "power": c.power(), "revealed": c.revealed, "turn_played": c.turn_played, "watches": c.watch_contexts()})
    };
    json!({ "turn": game.turn, "total_turns": game.total_turns, "priority": game.reveal_priority,
        "players": game.players.iter().map(|p| json!({"energy": p.energy, "pending_power": p.next_play_power, "hand": p.hand.iter().map(|&id| card(id)).collect::<Vec<_>>(), "deck": p.deck.iter().map(|&id| game.cards[id].def.id).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "locations": game.lanes.iter().map(|l| json!({"id": l.observed_id.as_deref().unwrap_or(l.def.id), "revealed": l.revealed, "revealed_turn": l.revealed_turn, "spent": l.state.as_deref().and_then(|s| s.as_any().downcast_ref::<[bool;2]>()), "cards": l.cards.iter().map(|&id| card(id)).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "pending_reveals": game.delayed_reveals.iter().map(|p| p.iter().map(|entry| json!([game.cards[entry.card].def.id, game.cards[entry.card].lane])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "played": game.played.iter().map(|p| p.iter().map(|&(id, lane)| json!([game.cards[id].def.id, lane])).collect::<Vec<_>>()).collect::<Vec<_>>() })
}

#[cfg(test)]
mod observed_history_tests {
    use super::*;

    fn base() -> Position {
        serde_json::from_value(json!({
            "turn":3,"total_turns":6,"energy":3,"hand":[],"deck":[],
            "opponent":{"hand":0,"deck":0},
            "locations":[
                {"id":"Vormir","mine":[],"theirs":[]},
                {"id":"TheAbbey","mine":[],"theirs":[]},
                {"id":"CollapsedMine","mine":[],"theirs":[]}
            ]
        }))
        .unwrap()
    }
    fn intrinsic_fixture(id: &str, moves: &[u8], active: bool) -> Position {
        let mut position = base();
        position.locations[0].mine.push(CardRef {
            id: id.into(),
            entity_id: Some(42),
            revealed: true,
            ..Default::default()
        });
        let ability = format!("{id}MovableOngoing");
        let components = if active {
            json!([{"source_entity_id":42,"owner_def_type":"Card",
            "owner_def_id":id,"ability_index":4,"created_ticks":"123","destination_entity_id":0}])
        } else {
            json!([])
        };
        position.provenance = Some(serde_json::from_value(json!({
            "schema_version":2, "entities_complete":true,
            "entities":[{"kind":"card","entity_id":42,"card_id":id,"lane_index":0,
                "abilities":{"complete":true,"reasons":[],"items":[{
                    "ability_def_id":ability,"owner_def_type":"Card","owner_def_id":id,"index":4}]},
                "movable_components":{"complete":true,"reasons":[],"items":components}}],
            "ledger":{"complete":true,"complete_for_legality":true,"match_start_seen":true,
                "reasons":[],"items":[{"effect_key":"move","kind":"move_grant",
                    "target_entity_id":42,"creator_entity_id":42,"creator_def_id":id,
                    "created_turn":1,"expiry_turn":null,"attribution":"exact","reasons":[],
                    "active":active,"ability_def_id":ability,"ability_index":4}]}
        })).unwrap());
        let mut actions = vec![
            json!({"owner":"local","turn":1,"kind":"play","location_slot":0,
            "card_entity_id":42,"card_id":id,"resolved":true,"turn_resolved":1,"resolution_order":1}),
        ];
        for (index, turn) in moves.iter().enumerate() {
            actions.push(
                json!({"owner":"local","turn":turn,"kind":"move","location_slot":1,
                "card_entity_id":42,"card_id":id,"resolved":true,"turn_resolved":turn,
                "resolution_order":index + 2}),
            );
        }
        position.history = Some(
            serde_json::from_value(json!({"actions":actions,
            "action_completeness":{"local":{"1":true,"2":true,"3":true},"opponent":{}},
            "own_draws":null}))
            .unwrap(),
        );
        position
    }

    #[test]
    fn intrinsic_move_sources_require_proven_consumption() {
        for (id, moves, active, expected) in [
            ("Nightcrawler", vec![], true, (0, None)),
            ("Nightcrawler", vec![2], false, (1, None)),
            ("SpiderManBrandNewDay", vec![2], true, (1, None)),
            ("SpiderManBrandNewDay", vec![2, 3], false, (2, Some(3))),
            ("Vision", vec![], true, (0, None)),
            ("Vision", vec![3], false, (1, Some(3))),
        ] {
            let mut position = intrinsic_fixture(id, &moves, active);
            let item = &position
                .provenance
                .as_ref()
                .unwrap()
                .ledger
                .as_ref()
                .unwrap()
                .items[0];
            assert_eq!(
                position.intrinsic_move_state(item),
                Some((42, expected.0, expected.1))
            );
            assert!(position.move_source_provenance_known());
            let mut rng = PythonRandom::seed(1);
            let game = build_game(&position, &mut rng).unwrap();
            let card = game.lanes[0].cards[0];
            assert_eq!(
                (
                    game.cards[card].player_moves_used,
                    game.cards[card].player_move_turn
                ),
                expected
            );
            position
                .history
                .as_mut()
                .unwrap()
                .action_completeness
                .as_mut()
                .unwrap()
                .local
                .remove("2");
            let item = &position
                .provenance
                .as_ref()
                .unwrap()
                .ledger
                .as_ref()
                .unwrap()
                .items[0];
            assert_eq!(position.intrinsic_move_state(item), None);
            assert!(!position.move_source_provenance_known());
        }
    }
    #[test]
    fn black_swan_hydrates_only_an_exact_active_player_grant() {
        let mut position = base();
        position.player_entity_ids = Some([10, 20]);
        position.hand.push(CardRef {
            id: "AntMan".into(),
            cost: Some(0),
            ..Default::default()
        });
        position.provenance = Some(serde_json::from_value(json!({
            "schema_version": 2, "entities_complete": true,
            "entities": [{"kind": "player", "entity_id": 10, "card_id": null, "lane_index": null,
                "abilities": {"complete": true, "reasons": [], "items": [{
                    "ability_def_id": "BlackSwanOngoing", "owner_def_type": "Card",
                    "owner_def_id": "BlackSwan", "index": 4, "origin_entity_id": 42,
                    "created_turn": 2, "lifetime": 2, "active": true}]},
                "movable_components": {"complete": true, "reasons": [], "items": []}}],
            "ledger": {"complete": true, "match_start_seen": true, "reasons": [], "items": [{
                "effect_key": "swan", "kind": "ability", "target_entity_id": 10,
                "creator_entity_id": 42, "creator_def_id": "BlackSwan", "created_turn": 2,
                "expiry_turn": null, "attribution": "exact", "reasons": [], "active": true,
                "ability_def_id": "BlackSwanOngoing", "ability_index": 4}]}
        })).unwrap());
        assert!(position.black_swan_known());
        let mut rng = PythonRandom::seed(1);
        let game = build_game(&position, &mut rng).unwrap();
        assert_eq!(game.card_cost(game.players[0].hand[0]), 0);
        assert_eq!(game.players[0].cost_effects.len(), 1);
        position
            .provenance
            .as_mut()
            .unwrap()
            .ledger
            .as_mut()
            .unwrap()
            .items[0]
            .attribution = "ambiguous".into();
        assert!(!position.black_swan_known());
        let mut rng = PythonRandom::seed(1);
        let game = build_game(&position, &mut rng).unwrap();
        assert!(game.players[0].cost_effects.is_empty());
    }
    #[test]
    fn monstro_samples_only_deck_tops_matching_observed_cost() {
        let mut position = base();
        position.locations[0].id = Some("Ruins".into());
        position.locations[1].id = Some("Xandar".into());
        position.locations[2].id = Some("KamarTaj".into());
        position.hand.push(CardRef {
            id: "MonstroOctopus".into(),
            cost: Some(2),
            stat_updates: Some(vec![]),
            ..Default::default()
        });
        position.deck = vec!["Shocker".into(), "Hulk".into(), "Wasp".into()];
        for seed in 0..20 {
            let mut rng = PythonRandom::seed(seed);
            let mut game = build_game(&position, &mut rng).unwrap();
            assert_eq!(game.cards[game.top_card(0).unwrap()].def.id, "Hulk");
            assert_eq!(game.card_cost(game.players[0].hand[0]), 2);
            let card = game.players[0].hand[0];
            game.stage_action(0, crate::game::Action::Play { card, lane: 0 })
                .unwrap();
        }
    }

    #[test]
    fn monstro_inconsistent_observed_cost_fails_closed() {
        let mut position = base();
        position.hand.push(CardRef {
            id: "MonstroOctopus".into(),
            cost: Some(2),
            stat_updates: Some(vec![]),
            ..Default::default()
        });
        position.deck = vec!["Shocker".into(), "Wasp".into()];
        let mut rng = PythonRandom::seed(1);
        let error = match build_game(&position, &mut rng) {
            Ok(_) => panic!("no deck top can explain Monstro's observed cost"),
            Err(error) => error,
        };
        assert!(error.contains("MonstroOctopus observed cost"));
    }

    #[test]
    fn monstro_projection_preserves_known_cost_basis() {
        let mut position = base();
        position.hand.push(CardRef {
            id: "MonstroOctopus".into(),
            cost: Some(4),
            cost_basis: Some(10),
            ..Default::default()
        });
        position.deck = vec!["Hulk".into(), "Shocker".into()];
        let mut rng = PythonRandom::seed(3);
        let game = build_game(&position, &mut rng).unwrap();
        assert_eq!(game.cards[game.top_card(0).unwrap()].def.id, "Hulk");
        assert_eq!(game.card_cost(game.players[0].hand[0]), 4);
    }

    #[test]
    fn monstro_uses_complete_permanent_cost_history() {
        let mut position = base();
        position.hand.push(serde_json::from_value(json!({
            "id":"MonstroOctopus", "cost":1,
            "stat_updates":[{"stat":"Cost", "update":"Add", "value":-1, "updater_card_id":"Beast"}]
        })).unwrap());
        position.deck = vec!["Hulk".into(), "Shocker".into()];
        let mut rng = PythonRandom::seed(3);
        let game = build_game(&position, &mut rng).unwrap();
        assert_eq!(game.cards[game.top_card(0).unwrap()].def.id, "Hulk");
        assert_eq!(game.card_cost(game.players[0].hand[0]), 1);
    }

    #[test]
    fn monstro_missing_history_or_deck_identity_is_named() {
        let mut position = base();
        position.hand.push(CardRef {
            id: "MonstroOctopus".into(),
            cost: Some(2),
            ..Default::default()
        });
        position.deck = vec!["Hulk".into()];
        let mut rng = PythonRandom::seed(1);
        assert!(
            matches!(build_game(&position, &mut rng), Err(error) if error.contains("stat_updates"))
        );
        position.hand[0].stat_updates = Some(vec![]);
        position.deck_count = Some(2);
        let mut rng = PythonRandom::seed(1);
        assert!(
            matches!(build_game(&position, &mut rng), Err(error) if error.contains("deck card identities incomplete"))
        );
    }

    #[test]
    fn hidden_location_sampling_excludes_unrestorable_sacred_timeline() {
        let mut position = base();
        position.locations[0].id = None;
        for seed in 0..300 {
            let mut rng = PythonRandom::seed(seed);
            let game = build_game(&position, &mut rng).unwrap();
            assert_ne!(game.lanes[0].def.id, "SacredTimeline");
        }
    }

    #[test]
    fn observed_energy_baselines_are_hydrated_independently() {
        let mut position = base();
        position.default_energy = Some(4);
        position.max_energy = Some(6);
        position.opponent.default_energy = Some(2);
        position.opponent.max_energy = Some(5);
        let mut rng = PythonRandom::seed(1);
        let game = build_game(&position, &mut rng).unwrap();
        assert_eq!(game.players[0].default_energy_offset, 1);
        assert_eq!(game.players[0].permanent_energy, 2);
        assert_eq!(game.players[0].energy, 3);
        assert_eq!(game.players[1].default_energy_offset, -1);
        assert_eq!(game.players[1].permanent_energy, 3);
        assert_eq!(game.players[1].energy, 5);
    }

    #[test]
    fn complete_location_state_and_empty_actions_are_restored() {
        let mut position = base();
        let before = crate::coverage::coverage(&position)["state"].to_string();
        assert!(before.contains("Vormir"));
        assert!(before.contains("TheAbbey"));
        assert!(before.contains("CollapsedMine"));
        position.locations[0].history_state = serde_json::from_value(json!({
            "vormir_consumed":{"local":true,"opponent":false}
        }))
        .unwrap();
        position.locations[1].history_state =
            serde_json::from_value(json!({"is_given":true})).unwrap();
        position.history = serde_json::from_value(json!({
            "actions":[], "action_completeness":{"local":{"3":true},"opponent":{"3":true}}
        }))
        .unwrap();
        let after = crate::coverage::coverage(&position)["state"].to_string();
        assert!(!after.contains("Vormir"));
        assert!(!after.contains("TheAbbey"));
        assert!(!after.contains("CollapsedMine"));
        let mut rng = PythonRandom::seed(1);
        let game = build_game(&position, &mut rng).unwrap();
        assert_eq!(
            game.lanes[0]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<[bool; 2]>(),
            Some(&[true, false])
        );
        assert_eq!(
            game.lanes[1]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<bool>(),
            Some(&true)
        );
        position.locations[0]
            .history_state
            .as_mut()
            .unwrap()
            .vormir_consumed
            .as_mut()
            .unwrap()
            .opponent = None;
        position
            .history
            .as_mut()
            .unwrap()
            .action_completeness
            .as_mut()
            .unwrap()
            .opponent
            .insert("3".into(), false);
        let incomplete = crate::coverage::coverage(&position)["state"].to_string();
        assert!(incomplete.contains("Vormir"));
        assert!(incomplete.contains("CollapsedMine"));
    }

    #[test]
    fn reviewed_thing_state_requires_the_exact_trigger() {
        let mut card: CardRef = serde_json::from_value(json!({
            "id":"TheThingFirstSteps","ability_state":{"complete":false,"abilities":[{
                "ability_def_id":"TheThingFirstStepsEndOfTurnTrigger","index":0,"kind":"trigger",
                "lifetime":null,"has_triggered_once":null,"state":{"fill_trigger_used":true}
            }],"objective":null}
        }))
        .unwrap();
        assert_eq!(card.thing_fill_used(), Some(true));
        let mut runtime = make_card(&card.id, 0).unwrap();
        apply_card_reference(&mut runtime, &card, 3);
        assert_eq!(
            runtime
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<bool>(),
            Some(&true)
        );
        card.ability_state.as_mut().unwrap().abilities[0].ability_def_id = "other".into();
        assert_eq!(card.thing_fill_used(), None);
    }

    #[test]
    fn negasonic_restores_only_complete_reviewed_trigger() {
        let mut reference: CardRef = serde_json::from_value(json!({
            "id":"NegasonicTeenageWarhead","ability_state":{"complete":true,"abilities":[{
                "ability_def_id":"NegasonicTeenageWarheadPostCardResolvedTrigger",
                "index":0,"kind":"trigger","lifetime":"OneTime",
                "has_triggered_once":true,"state":null
            }],"objective":null}
        }))
        .unwrap();
        let mut runtime = make_card(&reference.id, 0).unwrap();
        apply_card_reference(&mut runtime, &reference, 3);
        assert_eq!(
            runtime
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<bool>(),
            Some(&true)
        );
        reference.ability_state.as_mut().unwrap().complete = false;
        assert_eq!(reference.negasonic_consumed(), None);
    }

    #[test]
    fn nebula_presence_and_objective_states_are_not_guessed() {
        let mut nebula: CardRef = serde_json::from_value(json!({
            "id":"Nebula","ability_state":{"complete":false,"abilities":[{
                "ability_def_id":"NebulaPreEndOfTurnTrigger","index":0,"kind":"trigger",
                "lifetime":null,"has_triggered_once":null,"state":{"pre_end_active":true}
            }],"objective":null}
        }))
        .unwrap();
        assert_eq!(nebula.nebula_active(), Some(true));
        let mut runtime = make_card("Nebula", 0).unwrap();
        apply_card_reference(&mut runtime, &nebula, 3);
        assert_eq!(
            runtime
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<Option<u8>>(),
            Some(&Some(3))
        );
        nebula.ability_state.as_mut().unwrap().abilities.clear();
        assert_eq!(nebula.nebula_active(), None);
        let mut card: CardRef = serde_json::from_value(json!({
            "id":"JuggernautHorseman","ability_state":{"complete":false,"abilities":[],
                "objective":{"state":"Completed","progress":6,"goal":6}}
        }))
        .unwrap();
        assert_eq!(card.objective(), Some((6, true)));
        card.ability_state
            .as_mut()
            .unwrap()
            .objective
            .as_mut()
            .unwrap()
            .state = "GoalReached".into();
        assert_eq!(card.objective(), None);
    }

    #[test]
    fn selene_objective_requires_consistent_observed_state() {
        let mut card: CardRef = serde_json::from_value(json!({
            "id":"SeleneHorseman","ability_state":{"complete":false,"abilities":[],
                "objective":{"state":"InProgress","progress":2,"goal":3}}
        }))
        .unwrap();
        assert_eq!(card.selene_objective(), Some((2, false)));
        let objective = card
            .ability_state
            .as_mut()
            .unwrap()
            .objective
            .as_mut()
            .unwrap();
        objective.state = "Completed".into();
        assert_eq!(card.selene_objective(), None);
    }

    #[test]
    fn historical_actions_require_complete_turns_and_matching_entities() {
        let mut position = base();
        position.locations[0].mine.push(CardRef {
            id: "PowerStone".into(),
            entity_id: Some(42),
            revealed: true,
            played_turn: Some(1),
            ..Default::default()
        });
        position.history = serde_json::from_value(json!({
            "actions":[{"owner":"local","turn":1,"kind":"play","location_slot":0,
                "card_entity_id":42,"card_id":"PowerStone","resolved":true,
                "turn_resolved":1,"resolution_order":2}],
            "action_completeness":{"local":{"1":true,"2":true},"opponent":{}}
        }))
        .unwrap();
        assert_eq!(position.complete_past_actions(0).unwrap().len(), 1);
        let mut rng = PythonRandom::seed(1);
        let game = build_game(&position, &mut rng).unwrap();
        assert_eq!(game.players[0].play_history.len(), 1);
        assert!(game.has_played_on_turn(0, 1));
        assert!(!game.has_moved_on_turn(0, 1));
        position
            .history
            .as_mut()
            .unwrap()
            .action_completeness
            .as_mut()
            .unwrap()
            .local
            .insert("2".into(), false);
        assert!(position.complete_past_actions(0).is_none());
        let game = build_game(&position, &mut rng).unwrap();
        assert!(game.has_played_on_turn(0, 1));
        position
            .history
            .as_mut()
            .unwrap()
            .action_completeness
            .as_mut()
            .unwrap()
            .local
            .insert("2".into(), true);
        position.history.as_mut().unwrap().actions.as_mut().unwrap()[0].card_id = "Hulk".into();
        assert!(position.complete_past_actions(0).is_none());
    }

    #[test]
    fn unresolved_actions_cannot_prove_a_non_skip() {
        let mut position = base();
        position.locations[0].mine.push(CardRef {
            id: "Hawkeye".into(),
            entity_id: Some(42),
            ..Default::default()
        });
        position.history = serde_json::from_value(json!({
            "actions":[{"owner":"local","turn":3,"kind":"move","location_slot":0,
                "card_entity_id":42,"card_id":"Hawkeye","resolved":false,
                "turn_resolved":null,"resolution_order":null}],
            "action_completeness":{"local":{"3":true},"opponent":{"3":true}}
        }))
        .unwrap();
        assert_eq!(position.current_action_present(0), None);
        assert!(
            crate::coverage::coverage(&position)["state"]
                .to_string()
                .contains("CollapsedMine")
        );
    }

    #[test]
    fn quantum_base_requires_matching_complete_modifier_ledger() {
        let printed = CARD_DATA["Hawkeye"].power;
        let mut card: CardRef = serde_json::from_value(json!({
            "id":"Hawkeye","power":printed + 2,
            "power_modifiers":{"complete":true,"items":[{"operation":"Add","amount":2,"source":null}]},
            "stat_updates":[{"stat":"Power","update":"Add","value":2}]
        })).unwrap();
        assert_eq!(card.verified_base_power(), Some(printed));
        assert!(card.has_complete_power_modifier_history());
        card.power = Some(printed + 1);
        assert_eq!(card.verified_base_power(), None);
        card.power = Some(printed + 2);
        card.power_modifiers.as_mut().unwrap().complete = false;
        assert_eq!(card.verified_base_power(), None);
        assert!(!card.has_complete_power_modifier_history());
        card.power_modifiers.as_mut().unwrap().complete = true;
        card.power_modifiers.as_mut().unwrap().items[0].operation = "Set".into();
        assert_eq!(card.verified_base_power(), None);
        assert!(!card.has_complete_power_modifier_history());
    }

    #[test]
    fn complete_empty_augments_clear_only_the_attachment_reason() {
        let mut position = base();
        let mut card: CardRef = serde_json::from_value(json!({
            "id":"Hawkeye","stat_updates":[{"stat":"Power","update":"Add","value":2,
                "updater_augment_id":"FullLocationPowerAugment"}]
        }))
        .unwrap();
        position.hand.push(card.clone());
        assert!(
            crate::coverage::coverage(&position)["state"]
                .to_string()
                .contains("Card augment")
        );
        card.augments = serde_json::from_value(json!({"complete":true,"items":[]})).unwrap();
        position.hand[0] = card;
        assert!(
            !crate::coverage::coverage(&position)["state"]
                .to_string()
                .contains("Card augment")
        );
    }
}
