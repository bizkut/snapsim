//! Game state and turn lifecycle. Board and hand entries are stable arena indices.
pub use crate::engine_ext::abilities::{AbilityInstance, AbilityKind, AbilityRef};
use crate::{
    cards::{self, CardDef},
    data,
    locations::{self, LocationDef},
    random::GameRng,
};
use serde::Serialize;
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        LazyLock,
        atomic::{AtomicU64, Ordering},
    },
};
pub trait CardState: std::any::Any + Send {
    fn clone_box(&self) -> Box<dyn CardState>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: std::any::Any + Clone + Send> CardState for T {
    fn clone_box(&self) -> Box<dyn CardState> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
impl Clone for Box<dyn CardState> {
    fn clone(&self) -> Self {
        self.as_ref().clone_box()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Power,
    Cost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatOperation {
    Add,
    Multiply,
    Set,
}

/// Stable identity of an ongoing ability during a pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OngoingSource {
    Card {
        card: usize,
        instance: Option<u64>,
        hand: bool,
    },
    Location {
        lane: usize,
    },
    Player {
        seat: usize,
        sequence: u64,
    },
    CardCost {
        card: usize,
        sequence: u64,
    },
}

#[derive(Clone, Copy)]
enum ScheduledOngoingHook {
    Ordinary(cards::CardHook),
    Conditional(cards::ConditionalCardHook),
}

#[derive(Clone, Copy)]
struct ScheduledOngoing {
    card: usize,
    instance: Option<u64>,
    hook: ScheduledOngoingHook,
    def_id: &'static str,
    kind: &'static str,
    runs: usize,
    hand: bool,
}

#[derive(Clone, Copy)]
enum PendingConditional {
    Card(ScheduledOngoing),
    Location {
        lane: usize,
        hook: fn(&mut Game<'_>, usize) -> bool,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct CardStatUpdate {
    pub stat: Stat,
    pub operation: StatOperation,
    pub value: i32,
    pub source: usize,
    pub ongoing: bool,
    pub(crate) ongoing_source: Option<OngoingSource>,
}

/// A client CardPowerModifier, distinct from its power stat-update history.
#[derive(Clone, Copy, Debug)]
pub struct CardPowerModifier {
    pub operation: StatOperation,
    pub value: i32,
    pub source: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
pub enum CardCostEffectKind {
    NextTurnDelta { turn: u8, delta: i32 },
    TemporarySet { turn: u8, value: i32 },
    WhiteQueenWinning { lane: usize, seat: usize },
}

#[derive(Clone, Copy, Debug)]
pub struct CardCostEffect {
    pub sequence: u64,
    pub kind: CardCostEffectKind,
}

static NEXT_ATTACHED_ABILITY_ID: AtomicU64 = AtomicU64::new(1);

/// Card-owned ability instances retain context independently of the card's text.
#[derive(Clone)]
pub struct AttachedAbility {
    pub id: &'static str,
    pub owner_def: &'static str,
    pub origin: Option<usize>,
    pub index: u64,
    pub kind: AttachedAbilityKind,
}

#[derive(Clone, Copy)]
pub enum AttachedAbilityKind {
    Watch {
        turn: u8,
        lane: usize,
        expect: bool,
        power: i32,
        staged_only: bool,
    },
    MovePower {
        source: usize,
        turn: u8,
        hook: crate::cards::PlayHook,
    },
    NicoPrepare {
        used: u8,
    },
    RevealAtGameEnd,
}

pub fn initial_attachments(def: &'static CardDef) -> Vec<AttachedAbility> {
    if def.id == "NicoMinoru" {
        vec![AttachedAbility {
            id: "NicoMinoruStartOfTurnTrigger",
            owner_def: "NicoMinoru",
            origin: None,
            index: NEXT_ATTACHED_ABILITY_ID.fetch_add(1, Ordering::Relaxed),
            kind: AttachedAbilityKind::NicoPrepare { used: 0 },
        }]
    } else {
        Vec::new()
    }
}

impl Card {
    pub fn attach_ability(&mut self, mut ability: AttachedAbility) {
        ability.index = NEXT_ATTACHED_ABILITY_ID.fetch_add(1, Ordering::Relaxed);
        self.attached_abilities.push(ability);
    }

    pub fn attach_cost_effect(&mut self, kind: CardCostEffectKind) {
        let sequence = self.next_cost_effect_sequence;
        self.next_cost_effect_sequence += 1;
        self.cost_effects.push(CardCostEffect { sequence, kind });
    }

    pub fn attach_watch(&mut self, origin: usize, turn: u8, lane: usize, expect: bool, power: i32) {
        let owner_def = if expect { "Hawkeye" } else { "JessicaJones" };
        self.attach_ability(AttachedAbility {
            id: if expect {
                "HawkeyePreEndOfTurnTrigger"
            } else {
                "JessicaJonesPreEndOfTurnTrigger"
            },
            owner_def,
            origin: Some(origin),
            index: 0,
            kind: AttachedAbilityKind::Watch {
                turn,
                lane,
                expect,
                power,
                staged_only: false,
            },
        });
    }

    pub fn watch_contexts(&self) -> Vec<(u8, usize)> {
        self.attached_abilities
            .iter()
            .filter_map(|ability| match ability.kind {
                AttachedAbilityKind::Watch { turn, lane, .. } => Some((turn, lane)),
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct Card {
    pub def: &'static CardDef,
    /// Effective text; card identity, printed stats and ownership remain unchanged.
    pub override_ability: Option<&'static CardDef>,
    pub morphed_from: Option<&'static str>,
    pub morphed_from_entity: Option<usize>,
    pub replaced_card_def_id: Option<&'static str>,
    pub owner: usize,
    pub cost: i32,
    pub ongoing_cost: i32,
    /// Card dynamic text AssignedCost, set by Askanison IncrementCost 0x7e6be3c.
    pub dynamic_assigned_cost: Option<i32>,
    pub cost_effects: Vec<CardCostEffect>,
    pub next_cost_effect_sequence: u64,
    /// Lasting cost before location modifiers.
    pub created: bool,
    /// Whether creation occurred in hand; None means the origin is unobserved.
    pub created_in_hand: Option<bool>,
    /// Seat whose hand was the creation zone, when known.
    pub created_hand_seat: Option<usize>,
    pub(crate) creation_pending: bool,
    /// Visible client provenance; absent when not reported.
    pub created_by_card_id: Option<String>,
    pub created_by_location_id: Option<String>,
    pub created_by_player: Option<usize>,
    pub observed_entity_id: Option<i64>,
    pub merged_to_entity_id: Option<i64>,
    pub permanent: i32,
    pub base_power_override: Option<i32>,
    pub stat_updates: Vec<CardStatUpdate>,
    pub power_modifiers: Vec<CardPowerModifier>,
    pub power_modifiers_complete: bool,
    pub ongoing: i32,
    pub location_power: i32,
    pub lane: Option<usize>,
    /// Last location this card left, updated before movement or destruction hooks.
    pub last_lane: Option<usize>,
    /// The last two zone identities; restored games may not expose them.
    pub zone: Option<crate::zones::CardZone>,
    pub previous_zone: Option<crate::zones::CardZone>,
    pub revealed: bool,
    pub abilities_removed: bool,
    pub repeating: bool,
    pub latest_resolution_order: u32,
    pub turn_played: u8,
    /// Last observed or simulated reveal turn; absent if not exported.
    pub turn_revealed: Option<u8>,
    pub drawn_turn: Option<u8>,
    pub draw_extends_quickdraw: bool,
    pub move_turns: Vec<u8>,
    /// Voluntary movement and activation state (forced movement does not consume stacks).
    pub player_moves_used: u8,
    pub player_move_turn: Option<u8>,
    pub(crate) external_moves: Vec<crate::engine_ext::merge::CardMoveGrant>,
    pub activated: bool,
    pub move_staged: bool,
    pub activate_staged: bool,
    /// Optional current-turn legality reported by the live client.
    pub observed_can_activate: Option<bool>,
    pub observed_move_targets: Option<u8>,
    pub merged_to: Option<usize>,
    pub attached_abilities: Vec<AttachedAbility>,
    pub composed_abilities: Vec<AbilityInstance>,
    pub(crate) copied_ongoing_guards: Vec<&'static CardDef>,
    pub(crate) virtual_ongoing_ids: Vec<(usize, Option<u64>, u64, u64, AbilityInstance)>,
    pub(crate) ability_frames: Vec<crate::engine_ext::abilities::AbilityFrame>,
    pub(crate) default_ability_generation: u64,
    pub state: Option<Box<dyn CardState>>,
    /// Persistent state hidden temporarily while another card's reveal runs.
    pub(crate) borrowed_original_state: Option<Option<Box<dyn CardState>>>,
}
impl Card {
    pub fn ability_def(&self) -> &'static CardDef {
        self.override_ability.unwrap_or(self.def)
    }
    pub(crate) fn persistent_state_mut(&mut self) -> Option<&mut (dyn CardState + 'static)> {
        self.borrowed_original_state
            .as_mut()
            .unwrap_or(&mut self.state)
            .as_deref_mut()
    }
    pub fn power(&self) -> i32 {
        self.permanent + self.ongoing + self.location_power
    }
    pub fn has_ongoing(&self, include_hidden: bool) -> bool {
        (include_hidden && !self.external_moves.is_empty())
            || self.abilities().any(|ability| {
                (ability.kind != AbilityKind::Default || !self.abilities_removed)
                    && (include_hidden || !ability.hidden)
                    && ability.supports(AbilityKind::Ongoing)
                    && (ability.def.ongoing.is_some()
                        || ability.def.late_ongoing.is_some()
                        || ability.def.ongoing_ability
                        || ability.def.cost_guard.is_some()
                        || ability.def.destroy_guard.is_some()
                        || ability.def.blocks_expensive_plays
                        || ability.def.blocks_spawns
                        || ability.def.blocks_reveal
                        || ability.def.prevents_destroy
                        || ability.def.locks_lane)
            })
    }
    /// GorrBehavior::GetPowerChange (0x7eb883c), predicate 0x7eb8d4c.
    /// Additional resolve instances count independently of the default text.
    pub fn has_resolve_trigger(&self) -> bool {
        self.abilities().any(|ability| {
            (ability.id.is_some() || !self.abilities_removed)
                && ability.supports(AbilityKind::Reveal)
                && ability.def.reveal.is_some()
        })
    }

    /// FantasticarBehavior::GetTargetCard (0x7ea7514), predicate 0x7ea7c8c.
    pub fn has_end_turn_trigger(&self) -> bool {
        self.abilities().any(|ability| {
            let present = if cards::reference_rules_enabled() {
                !self.abilities_removed
            } else {
                (ability.id.is_some() || !self.abilities_removed)
                    && !ability.hidden
                    && !ability.def.end_turn_trigger_hidden
            };
            present
                && ability.supports(AbilityKind::Trigger)
                && (ability.def.end_turn.is_some() || ability.def.end_turn_trigger)
        })
    }

    /// Card::HasNoText (0x7cb3768) uses the effective client behavior
    /// definition, not its printed description or currently suppressed abilities.
    pub fn has_no_text(&self) -> Option<bool> {
        if self.abilities_removed {
            Some(true)
        } else {
            cards::no_text::has_no_text(self.default_ability().id)
        }
    }

    pub fn can_reduce_power(&self) -> bool {
        !self.revealed
            || !self.ongoing_guard_abilities().any(|ability| {
                ability.supports(AbilityKind::Ongoing) && ability.def.immune_destroy_move
            })
    }

    pub fn reduce_power(&mut self, amount: i32) {
        if self.can_reduce_power() {
            self.permanent -= amount;
        }
    }
    pub fn has_no_ability(&self) -> bool {
        self.abilities_removed
            || CATALOG
                .get(self.default_ability().id)
                .is_some_and(|data| data.ability.is_empty())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerTrigger {
    StartTurn,
    BeforeCardRevealed,
    Quickdraw,
    CardPlayed,
    CardResolved,
    CardBanished,
    EndTurn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectLifetime {
    NextCard,
    /// Next played card other than the source, including on later turns.
    NextOtherCard,
    ThisTurn,
    UntilTriggered,
}

#[derive(Clone)]
/// Player-owned ability: captured state survives removal of the originating card.
pub struct PendingEffect {
    pub source: usize,
    pub lane: Option<usize>,
    pub created_turn: u8,
    pub trigger: PlayerTrigger,
    pub lifetime: EffectLifetime,
    pub state: Box<dyn CardState>,
    /// Returns true when the effect has completed.
    pub apply: fn(&mut Game<'_>, &mut PendingEffect, Option<usize>) -> bool,
}

#[derive(Clone, Copy, Debug)]
pub struct StagedPlay {
    pub turn: u8,
    pub card: usize,
    pub def_id: &'static str,
    pub printed_cost: i32,
    pub created: bool,
    pub resolved: bool,
}

/// Identity of the action that staged a card, independent of later movement or ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StagingContext {
    pub seat: usize,
    pub lane: usize,
}

/// A pending reveal; restored positions may not expose its staging context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueuedReveal {
    pub card: usize,
    pub staging: Option<StagingContext>,
    pub persistent: bool,
}

/// A player-owned movement permission; one grant may be shared by a whole lane.
#[derive(Clone)]
pub struct MoveGrant {
    pub seat: usize,
    pub source: usize,
    /// Definition that owns the granted ability, including copied abilities.
    pub owner_def_id: Option<&'static str>,
    pub card: Option<usize>,
    pub lane: Option<usize>,
    pub destination: Option<usize>,
    pub start_turn: u8,
    pub end_turn: u8,
    pub remaining: u8,
    pub unlimited: bool,
    /// Optional condition evaluated with (source, target) when checking the grant.
    pub condition: Option<fn(&Game<'_>, usize, usize) -> bool>,
    /// Invoked with (source card, moved card) after the granted move resolves.
    pub on_move: Option<crate::cards::PlayHook>,
}

/// One player action in the order it was staged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    Play { card: usize, lane: usize },
    Move { card: usize, lane: usize },
    Activate { card: usize },
    Return { card: usize },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MoveSource {
    Intrinsic,
    Attached(usize),
    External(usize),
    Player(usize),
    Passive(&'static str),
}

#[derive(Clone, Copy)]
struct MoveCandidate {
    source: MoveSource,
    priority: i32,
    // Entries in the same ordered collection retain their creation order.
    order_group: u8,
    impact: (u8, u8, u8, Option<usize>),
}

pub(crate) fn move_source_effects_conflict(
    left: (u8, u8, u8, Option<usize>),
    right: (u8, u8, u8, Option<usize>),
) -> bool {
    left != right
}

#[derive(Clone)]
struct StagedAction {
    action: Action,
    cost: i32,
    hand_index: Option<usize>,
}

/// Immutable event snapshot, retained when the card returns or is resurrected.
#[derive(Clone, Copy, Debug)]
pub struct CardZoneEvent {
    pub turn: u8,
    pub card: usize,
    pub power: i32,
    pub lane: Option<usize>,
}

/// Identity and lane are snapshots, not mutable properties of the entity.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedPlayEvent {
    pub turn: u8,
    pub card: usize,
    pub def_id: &'static str,
    pub printed_cost: i32,
    pub staged_lane: Option<usize>,
    pub lane: Option<usize>,
    /// Historical action observations do not export resolution-time power.
    pub power: Option<i32>,
}

/// Energy at the end of resolution, before end-of-turn triggers.
#[derive(Clone, Copy, Debug)]
pub struct TurnEnergyEvent {
    pub turn: u8,
    pub spent: i32,
    pub remaining: i32,
    pub bonus: i32,
}

/// Player-owned cost abilities retain their attachment order within a priority.
#[derive(Clone, Copy, Debug)]
pub enum PlayerCostEffectKind {
    Sandman {
        turn: u8,
    },
    BaronMordo {
        card: usize,
        until: u8,
        value: i32,
    },
    BlackSwan {
        until: u8,
        printed_cost: i32,
        value: i32,
    },
    Wave {
        until: u8,
    },
    ChampionDoom {
        turn: u8,
    },
}

impl PlayerCostEffectKind {
    /// Priority and staging flags of the client player-owned stat ability.
    /// Constructors: Sandman 0x7f2e540, Mordo 0x7e6ee98,
    /// Black Swan 0x7e73fa0, Wave 0x7cef518.
    fn ongoing_schedule(self) -> (u8, bool, bool, bool) {
        let priority = match self {
            Self::Sandman { .. } => 0,
            Self::BaronMordo { .. } | Self::BlackSwan { .. } => 2,
            Self::Wave { .. } => 4,
            Self::ChampionDoom { .. } => 4,
        };
        (priority, false, false, false)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PlayerCostEffect {
    pub sequence: u64,
    pub kind: PlayerCostEffectKind,
}

#[derive(Clone, Default)]
pub struct Player {
    pub hand: Vec<usize>,
    pub deck: Vec<usize>,
    /// Original deck identities; None means a restored position did not report them.
    pub starting_deck: Option<Vec<&'static str>>,
    /// Original deck entity indices, retained even when cards change zones or identity.
    /// Unknown when reconstructing a position without complete instance provenance.
    pub starting_deck_instances: Option<Vec<usize>>,
    /// Original opening-hand entity indices; absent in unobserved restored positions.
    pub starting_hand: Option<Vec<usize>>,
    pub discard: Vec<usize>,
    pub banished: Vec<usize>,
    pub set_aside: Vec<usize>,
    /// Cards in destruction order and their death lane, if it was observed.
    pub graveyard: Vec<(usize, Option<usize>)>,
    /// Current graveyard cards in combined discard/destruction insertion order.
    /// Graveyard::GetDestroyedAndDiscardedCards 0x7dd80e8 filters this order.
    pub graveyard_order: Vec<usize>,
    /// Successful discard and destruction events in occurrence order.
    pub discard_history: Vec<CardZoneEvent>,
    pub destruction_history: Vec<CardZoneEvent>,
    /// Distinct recipient entities in Player.StealsPerCardEntityId.
    pub stolen_power_recipients: Vec<usize>,
    /// Colonel America's shared set of turns folded from its discard context.
    pub colonel_discard_turns: Vec<u8>,
    pub pending_effects: Vec<PendingEffect>,
    pub(crate) sinister_six: Vec<crate::engine_ext::sinister::SinisterState>,
    pub cost_effects: Vec<PlayerCostEffect>,
    /// Player-owned next-turn restrictions, retaining the targeted entity.
    pub play_locks: Vec<(usize, u8)>,
    pub next_cost_effect_sequence: u64,
    /// Snowguard Hawk's player-owned location silence, through this turn.
    pub location_silence_until: Option<u8>,
    /// (staging turn, originating card) for Supergiant concealment.
    pub supergiant_hidden_turns: Vec<(u8, usize)>,
    /// Invisible Woman First Steps' player-owned cumulative multiplier and active turn.
    pub end_turn_multiplier: u32,
    pub end_turn_multiplier_turn: Option<u8>,
    pub move_grants: Vec<MoveGrant>,
    staged_actions: Vec<StagedAction>,
    pub scheduled_energy: Vec<(u8, i32)>,
    /// Persistent increase to energy refreshed on each subsequent turn.
    pub permanent_energy: i32,
    /// Offset of the client's default energy from the normal turn progression.
    pub default_energy_offset: i32,
    pub movable_cards_played: Vec<usize>,
    /// PlayerMoveStatsComponent.NumberOfPlayerCardsMovedThisGame.
    pub cards_moved_this_game: u32,
    pub last_moved_card: Option<usize>,
    /// Drawn cards, preserving the turn and draw order.
    pub draws: Vec<(u8, usize)>,
    /// Resolved played cards, preserving turn and resolution order.
    pub play_history: Vec<(u8, usize)>,
    pub resolved_play_history: Vec<ResolvedPlayEvent>,
    /// Number of resolved plays that had On Reveal text at resolution time.
    pub on_reveal_cards_played: usize,
    /// Snapshot of all staged plays, including those not yet resolved.
    pub staged_history: Vec<StagedPlay>,
    /// Original turn and lane for every staged play, retained after zone changes.
    pub staged_lane_history: Vec<(u8, usize)>,
    /// Observed turns containing plays or moves, including cards no longer visible.
    pub observed_play_turns: Vec<u8>,
    pub observed_move_turns: Vec<u8>,
    /// Turn of a player-staged move or activation, even if resolution fails.
    pub non_play_action_turn: Option<u8>,
    /// Last turn on which a voluntary move was staged from each lane.
    pub move_from_turn: [Option<u8>; 3],
    /// Turns on which this player snapped.
    pub snapped_turns: Vec<u8>,
    /// (turn, energy spent, all available energy spent).
    pub energy_history: Vec<(u8, i32, bool)>,
    pub turn_energy_history: Vec<TurnEnergyEvent>,
    pub energy_spent_this_turn: i32,
    /// PlayerEnergyComponent.HighestEnergyHadByPlayerThisGame.
    pub highest_energy_this_game: i32,
    pub energy: i32,
    pub next_play_power: i32,
    pub pending_left_moves: u32,
}

impl Player {
    pub fn attach_cost_effect(&mut self, kind: PlayerCostEffectKind) {
        let sequence = self.next_cost_effect_sequence;
        self.next_cost_effect_sequence += 1;
        self.cost_effects.push(PlayerCostEffect { sequence, kind });
    }
}

/// Location ongoing-stat flags; true means the corresponding ability is allowed.
#[derive(Clone, Copy)]
pub(crate) struct LocationAbilityFlags {
    pub on_reveal: bool,
    pub ongoing: bool,
    pub hidden_ongoing: bool,
    pub trigger: bool,
    pub activate: bool,
}

impl Default for LocationAbilityFlags {
    fn default() -> Self {
        Self {
            on_reveal: true,
            ongoing: true,
            hidden_ongoing: true,
            trigger: true,
            activate: true,
        }
    }
}

#[derive(Clone)]
pub struct Lane {
    pub def: &'static LocationDef,
    /// Original ID when an unsupported location is represented by an inert rule.
    pub observed_id: Option<String>,
    pub revealed: bool,
    pub destroyed: bool,
    pub(crate) ability_flags: LocationAbilityFlags,
    /// Future reveal turn for a copied hidden location.
    pub copied_reveal_turn: Option<u8>,
    pub revealed_turn: Option<u8>,
    pub cards: Vec<usize>,
    pub bonus: [i32; 2],
    pub multiplier: [i32; 2],
    pub(crate) contributions: Vec<crate::lane_contributions::LaneContribution>,
    pub state: Option<Box<dyn CardState>>,
}
#[derive(Serialize)]
pub struct Played {
    pub card: &'static str,
    pub lane: usize,
}
#[derive(Serialize)]
pub struct TurnTrace {
    pub turn: u8,
    pub priority: usize,
    pub plays: [Vec<Played>; 2],
    pub powers: [[i32; 2]; 3],
}
/// Events delivered to registered observers in board/lane order.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    GameStart,
    GameEnd,
    TurnStart,
    TurnEnd,
    Ongoing,
    CardDrawn {
        card: usize,
        seat: usize,
    },
    CardReturnedToHand {
        card: usize,
    },
    CardStaged {
        card: usize,
        seat: usize,
    },
    BeforeCardRevealed {
        card: usize,
    },
    /// A staged action about to resolve with its original seat and lane.
    StagedCardResolving {
        card: usize,
        seat: usize,
        lane: usize,
    },
    AfterTurnEnd,
    CardRevealed {
        card: usize,
    },
    CardPlayed {
        card: usize,
    },
    /// PostCardResolvedTrigger, after the resolving card's skill banishment.
    CardResolved {
        card: usize,
    },
    /// A resolved staged action whose original seat and lane are known.
    StagedCardResolved {
        card: usize,
        seat: usize,
        lane: usize,
    },
    CardActivated {
        card: usize,
    },
    CardAdded {
        card: usize,
        lane: usize,
    },
    BeforeCardMoved {
        card: usize,
        from: usize,
        to: usize,
    },
    CardMoved {
        card: usize,
        from: usize,
        to: usize,
    },
    CardOwnerChanged {
        card: usize,
        old: usize,
        new: usize,
    },
    CardMerged {
        card: usize,
        absorbed: usize,
    },
    /// One client PostCardMergedTrigger for a batch; index into merge_batches.
    CardMergedBatch {
        card: usize,
        batch: usize,
    },
    CardDestroyed {
        card: usize,
        lane: usize,
    },
    CardResurrected {
        card: usize,
        lane: usize,
    },
    CardDiscarded {
        card: usize,
        seat: usize,
    },
    LocationRevealed {
        lane: usize,
    },
    /// PostLocationMorphedTrigger, after a location replacement resolves.
    LocationMorphed {
        lane: usize,
    },
    EnergyChanged {
        seat: usize,
        old: i32,
        new: i32,
    },
    TurnCountChanged {
        old: u8,
        new: u8,
    },
    Empower {
        card: usize,
        amount: i32,
        operation: StatOperation,
        is_steal: bool,
    },
}

#[cfg(test)]
mod stakes_tests {
    use super::{RetreatCost, Stakes};

    #[test]
    fn observed_current_and_projected_sequence() {
        let mut stakes = Stakes {
            current: 1,
            next: 1,
            maximum: 8,
            can_raise: true,
            player_multiplier: [1; 2],
            end_game_multiplier: 1,
            game_multiplier: 1,
            snapped: [false; 2],
            last_raiser: None,
            pending_retreat: [false; 2],
        };
        stakes.snap(0).unwrap();
        assert_eq!((stakes.current, stakes.next), (1, 2));
        stakes.advance_turn();
        assert_eq!((stakes.current, stakes.next), (2, 2));
        // Observed final-turn projection: 2 -> 4.
        stakes.set_end_game_multiplier(2);
        assert_eq!((stakes.current, stakes.next), (2, 4));
        stakes.finish_game();
        assert_eq!((stakes.current, stakes.next), (4, 4));
    }
    #[test]
    fn fixed_stake_and_provisional_retreat() {
        let mut stakes = Stakes {
            current: 2,
            next: 1,
            maximum: 2,
            can_raise: false,
            player_multiplier: [1; 2],
            end_game_multiplier: 1,
            game_multiplier: 1,
            snapped: [false; 2],
            last_raiser: None,
            pending_retreat: [false; 2],
        };
        assert!(stakes.snap(0).is_err());
        stakes.advance_turn();
        assert_eq!(stakes.current, 2);
        assert_eq!(
            stakes.retreat_now(0, RetreatCost::DisplayedCurrent),
            Ok([-2, 2])
        );
        assert_eq!(stakes.deferred_retreat([false, false]), None);
        assert_eq!(stakes.deferred_retreat([true, false]), Some([-2, 2]));
        assert_eq!(stakes.deferred_retreat([false, true]), Some([2, -2]));
        assert_eq!(stakes.deferred_retreat([true, true]), Some([0, 0]));
        stakes.set_end_game_multiplier(2);
        stakes.finish_game();
        assert_eq!(stakes.showdown(Some(1)), [-2, 2]);
        assert_eq!(stakes.showdown(None), [0, 0]);
    }

    #[test]
    fn projected_value_includes_all_multipliers_and_cap() {
        let mut stakes = Stakes {
            current: 2,
            next: 2,
            maximum: 16,
            can_raise: true,
            player_multiplier: [2, 1],
            end_game_multiplier: 2,
            game_multiplier: 2,
            snapped: [true, false],
            last_raiser: None,
            pending_retreat: [false; 2],
        };
        stakes.update_next();
        assert_eq!(stakes.next, 16);
        stakes.maximum = 8;
        stakes.update_next();
        assert_eq!(stakes.next, 8);
        stakes.advance_turn();
        assert_eq!((stakes.current, stakes.next), (8, 8));
    }
}

/// Cube amount captured by the client when a concession ends the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetreatCost {
    DisplayedCurrent,
}

/// Explicit stake state. Modes must supply their own initial and maximum values;
/// an absent state means that cube accounting is not configured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stakes {
    pub current: u32,
    pub next: u32,
    pub maximum: u32,
    pub can_raise: bool,
    pub player_multiplier: [u32; 2],
    pub end_game_multiplier: u32,
    pub game_multiplier: u32,
    pub snapped: [bool; 2],
    pub last_raiser: Option<usize>,
    pub pending_retreat: [bool; 2],
}

impl Stakes {
    /// Record a player's snap. Pending increases are promoted at turn end.
    pub fn snap(&mut self, seat: usize) -> Result<(), &'static str> {
        if seat >= 2 || !self.can_raise || self.snapped[seat] || self.next >= self.maximum {
            return Err("snap unavailable");
        }
        self.player_multiplier[seat] = self.player_multiplier[seat].saturating_mul(2);
        self.update_next();
        self.snapped[seat] = true;
        self.last_raiser = Some(seat);
        Ok(())
    }

    /// Projection in Game::UpdateCubeValueNextTurn @ 0x7db0df8–0x7db0e2c.
    pub fn update_next(&mut self) {
        self.next = self
            .current
            .saturating_mul(self.player_multiplier[0])
            .saturating_mul(self.player_multiplier[1])
            .saturating_mul(self.end_game_multiplier)
            .saturating_mul(self.game_multiplier)
            .min(self.maximum);
    }

    /// Game::CheckIncreaseStakesForNextTurn @ 0x7db0fc4 promotes player and
    /// game multipliers, then resets them; the end-game multiplier is separate.
    pub fn advance_turn(&mut self) {
        self.current = self
            .current
            .saturating_mul(self.player_multiplier[0])
            .saturating_mul(self.player_multiplier[1])
            .saturating_mul(self.game_multiplier)
            .min(self.maximum);
        self.player_multiplier = [1; 2];
        self.game_multiplier = 1;
        self.update_next();
    }

    pub fn set_end_game_multiplier(&mut self, multiplier: u32) {
        self.end_game_multiplier = multiplier;
        self.update_next();
    }

    /// CheckEndOfGameIncreaseStakesGameBehavior @ 0x7dccd14 applies the
    /// configured final multiplier, capped at MaxCubeValue.
    pub fn finish_game(&mut self) {
        self.current = self
            .current
            .saturating_mul(self.end_game_multiplier)
            .min(self.maximum);
        self.end_game_multiplier = 1;
        self.update_next();
    }

    pub fn showdown(&self, winner: Option<usize>) -> [i64; 2] {
        let cubes = i64::from(self.current);
        match winner {
            Some(0) => [cubes, -cubes],
            Some(1) => [-cubes, cubes],
            _ => [0, 0],
        }
    }

    /// WaitForStaging::HandleConcedeImmediatelyRequest 0x7e50204 validates
    /// the request against current cubes; result construction 0x7dd4324
    /// copies Game.CubeValue into the result.
    pub fn retreat_now(&self, seat: usize, rule: RetreatCost) -> Result<[i64; 2], &'static str> {
        let cubes = match rule {
            RetreatCost::DisplayedCurrent => i64::from(self.current),
        };
        match seat {
            0 => Ok([-cubes, cubes]),
            1 => Ok([cubes, -cubes]),
            _ => Err("invalid seat"),
        }
    }

    /// EndTurn::CheckForConcedes 0x7e09764 runs before reveal or stake
    /// promotion. GameMode::DetermineGameResult 0x7dd3a6c treats all
    /// players conceding as a tie.
    pub fn deferred_retreat(&self, conceded: [bool; 2]) -> Option<[i64; 2]> {
        match conceded {
            [false, false] => None,
            [true, true] => Some([0, 0]),
            [true, false] => Some([-i64::from(self.current), i64::from(self.current)]),
            [false, true] => Some([i64::from(self.current), -i64::from(self.current)]),
        }
    }
}

/// A game may borrow its caller's stream or own an independent stream.
pub enum GameRandom<'a> {
    Borrowed(&'a mut dyn GameRng),
    Owned(Box<dyn GameRng>),
}

impl<'a> std::ops::Deref for GameRandom<'a> {
    type Target = dyn GameRng + 'a;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(rng) => &**rng,
            Self::Owned(rng) => &**rng,
        }
    }
}

impl<'a> std::ops::DerefMut for GameRandom<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Borrowed(rng) => &mut **rng,
            Self::Owned(rng) => &mut **rng,
        }
    }
}

/// Hooks invoked by a replay trial, collected only when explicitly enabled.
/// The random draws are always recorded in replay trials: their roll-to-outcome
/// mapping is not decoded (`WeightedTable.GetRandom` 0x60708f0), so a trial that
/// drew an outcome different from the observed one proves nothing.
#[derive(Clone, Debug, Default)]
pub(crate) struct ReplayHooks {
    pub cards: std::collections::BTreeSet<(&'static str, &'static str)>,
    pub locations: std::collections::BTreeSet<(&'static str, &'static str)>,
    /// `Game::GetRandomLocationDefId` draws: the excluded definition and the drawn one.
    pub random_locations: Vec<(Option<String>, &'static str)>,
    /// Black Vortex's random 6-cost card: the transformed card and its new definition.
    pub random_cards: Vec<(usize, &'static str)>,
}

pub struct Game<'a> {
    pub cards: Vec<Card>,
    pub players: [Player; 2],
    pub human_seat: Option<usize>,
    pub lanes: [Lane; 3],
    pub location_pool: Vec<&'static LocationDef>,
    /// Definitions of the locations the game started with. `Game::GetRandomLocationDefId`
    /// (0x7db248c) never draws these (final `Except`, 0x7db2484).
    pub starting_location_ids: Vec<&'static str>,
    pub next_resolution_order: u32,
    pub turn: u8,
    pub stakes: Option<Stakes>,
    pub is_wait_for_staging: bool,
    pub is_start_turn: bool,
    pub reveal_priority: Option<usize>,
    pub total_turns: u8,
    pub played: [Vec<(usize, usize)>; 2],
    pub pending_plays: [Vec<(usize, usize)>; 2],
    pub played_on_reveal: [bool; 2],
    pub delays_reveals: bool,
    pub(crate) ending_reveals: bool,
    pub delayed_reveals: [Vec<QueuedReveal>; 2],
    pub(crate) deferred_added_reveals: Vec<usize>,
    pub(crate) post_resolve_plays: Vec<QueuedReveal>,
    pub(crate) end_turn_lane_order: Option<[usize; 3]>,
    /// Python's `current_location`: scheduled reveals consumed so far. A lane that a
    /// card already revealed (Scarlet Witch, Magik) still uses up its scheduled turn.
    pub current_location: usize,
    /// Python's `_location_reveal_order`: lanes in the order the schedule reveals them.
    pub location_reveal_order: [usize; 3],
    /// Optional replay observer, populated only for explicitly observed games.
    pub destroyed: Option<Vec<(usize, usize)>>,
    /// Location power overrides produced during the current ongoing pass.
    pub(crate) score_overrides: [Option<i32>; 2],
    pub(crate) current_ongoing_source: Option<OngoingSource>,
    pub(crate) ability_call_depth: u8,
    pub(crate) replay_hooks: Option<RefCell<ReplayHooks>>,
    pub(crate) merge_batches: Vec<Vec<usize>>,
    pub(crate) staging_order: Option<Vec<usize>>,
    pub(crate) move_provenance_known: bool,
    pub last_move_source: Option<&'static str>,
    pub rng: RefCell<GameRandom<'a>>,
}

static CATALOG: LazyLock<HashMap<String, data::CardData>> = LazyLock::new(|| {
    data::cards()
        .into_iter()
        .map(|row| (row.carddefid.clone(), row))
        .collect()
});
static HAS_EVENT_HOOKS: LazyLock<bool> = LazyLock::new(|| {
    cards::registry().any(|entry| entry.on_event.is_some())
        || locations::REGISTRY
            .iter()
            .any(|entry| entry.on_event.is_some())
});
static HAS_COST_GUARDS: LazyLock<bool> =
    LazyLock::new(|| cards::registry().any(|entry| entry.cost_guard.is_some()));
static HAS_SPAWN_BLOCKERS: LazyLock<bool> =
    LazyLock::new(|| cards::registry().any(|entry| entry.blocks_spawns));

impl<'a> Game<'a> {
    pub(crate) fn record_card_hook(&self, id: &'static str, kind: &'static str) {
        if let Some(hooks) = &self.replay_hooks {
            hooks.borrow_mut().cards.insert((id, kind));
        }
    }

    pub(crate) fn record_location_hook(&self, lane: usize, kind: &'static str) {
        if let Some(hooks) = &self.replay_hooks {
            hooks
                .borrow_mut()
                .locations
                .insert((self.lanes[lane].def.id, kind));
        }
    }

    pub(crate) fn record_random_card(&self, card: usize, id: &'static str) {
        if let Some(hooks) = &self.replay_hooks {
            hooks.borrow_mut().random_cards.push((card, id));
        }
    }

    /// Whether `Game::GetRandomLocationDefId` could have drawn `id` when `excluded`
    /// was the morphing location's definition.
    pub(crate) fn location_in_random_pool(&self, id: &str, excluded: Option<&str>) -> bool {
        self.location_def_drawable(id, excluded)
            && self.location_pool.iter().any(|def| def.id == id)
    }

    /// DefaultGameMode::HandleConcedeRequestInternal 0x7dd1bf4 rejects
    /// conceding while this player is the latest raiser on their snap turn.
    pub fn can_retreat_seat(&self, seat: usize, end_turn: bool) -> bool {
        seat < 2
            && self.can_retreat(end_turn)
            && !(self.stakes.as_ref().and_then(|stakes| stakes.last_raiser) == Some(seat)
                && self.players[seat].snapped_turns.contains(&self.turn))
    }

    /// Clone all simulation state against a separately cloned RNG stream.
    pub fn fork<'b>(&self, rng: &'b mut dyn GameRng) -> Game<'b> {
        self.fork_with_rng(GameRandom::Borrowed(rng))
    }

    fn fork_with_rng<'b>(&self, rng: GameRandom<'b>) -> Game<'b> {
        Game {
            cards: self.cards.clone(),
            ability_call_depth: self.ability_call_depth,
            replay_hooks: self.replay_hooks.clone(),
            players: self.players.clone(),
            human_seat: self.human_seat,
            lanes: self.lanes.clone(),
            location_pool: self.location_pool.clone(),
            starting_location_ids: self.starting_location_ids.clone(),
            next_resolution_order: self.next_resolution_order,
            turn: self.turn,
            stakes: self.stakes.clone(),
            is_wait_for_staging: self.is_wait_for_staging,
            is_start_turn: self.is_start_turn,
            reveal_priority: self.reveal_priority,
            total_turns: self.total_turns,
            played: self.played.clone(),
            pending_plays: self.pending_plays.clone(),
            played_on_reveal: self.played_on_reveal,
            delays_reveals: self.delays_reveals,
            ending_reveals: self.ending_reveals,
            delayed_reveals: self.delayed_reveals.clone(),
            deferred_added_reveals: self.deferred_added_reveals.clone(),
            post_resolve_plays: self.post_resolve_plays.clone(),
            end_turn_lane_order: self.end_turn_lane_order,
            current_location: self.current_location,
            location_reveal_order: self.location_reveal_order,
            destroyed: self.destroyed.clone(),
            score_overrides: self.score_overrides,
            current_ongoing_source: self.current_ongoing_source,
            merge_batches: self.merge_batches.clone(),
            staging_order: self.staging_order.clone(),
            move_provenance_known: self.move_provenance_known,
            last_move_source: self.last_move_source,
            rng: RefCell::new(rng),
        }
    }

    /// Clone a game and its random stream into an independently owned game.
    pub fn fork_owned(&self) -> Game<'static> {
        self.fork_with_rng(GameRandom::Owned(self.fork_rng()))
    }
    /// Detach a borrowed stream without cloning the rest of the game state.
    pub fn into_owned(self) -> Game<'static> {
        let rng = match self.rng.into_inner() {
            GameRandom::Borrowed(rng) => rng.clone_rng(),
            GameRandom::Owned(rng) => rng,
        };
        Game {
            rng: RefCell::new(GameRandom::Owned(rng)),
            ..self
        }
    }

    pub fn fork_rng(&self) -> Box<dyn GameRng> {
        self.rng.borrow().clone_rng()
    }
    fn lane_ongoing_allowed(&self, lane: usize) -> bool {
        if cards::reference_rules_enabled() {
            !self.lanes[lane].revealed || !self.lanes[lane].def.disables_ongoing
        } else {
            self.lanes[lane].ability_flags.ongoing
        }
    }

    // DeepSpaceBehavior::PreventCardAbilities 0x7d4e4dc disables card
    // trigger and activate abilities here, but not player-owned effects.
    pub(crate) fn card_triggers_allowed(&self, card: usize) -> bool {
        self.cards[card].lane.is_none_or(|lane| {
            if cards::reference_rules_enabled() {
                !self.lanes[lane].revealed || self.lanes[lane].def.id != "DeepSpace"
            } else {
                self.lanes[lane].ability_flags.trigger
            }
        })
    }

    pub(crate) fn emit(&mut self, event: Event) {
        if matches!(
            event,
            Event::CardDrawn { .. } | Event::StagedCardResolved { .. }
        ) {
            let _ = self
                .counted_trigger(|game| crate::engine_ext::sinister::sinister_event(game, event));
        }
        if !*HAS_EVENT_HOOKS {
            return;
        }
        let mut observers: Vec<_> = self
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| {
                (self.cards[id].revealed || matches!(event, Event::Empower { .. }))
                    && self.cards[id].has_event_ability(true, !self.cards[id].revealed)
            })
            .collect();
        if !cards::reference_rules_enabled() {
            // GetAllCardsForActivatingTriggerAbility (0x7dacfcc) starts
            // with revealed cards in latest-resolution order, then face-down cards.
            observers.sort_by_key(|&id| {
                (
                    !self.cards[id].revealed,
                    self.cards[id]
                        .revealed
                        .then_some(self.cards[id].latest_resolution_order),
                )
            });
        }
        // Game::ActivateTriggerAbility @ 0x7da93a0 dispatches locations
        // before card abilities for PostCardResolvedTrigger.
        if matches!(event, Event::CardResolved { .. }) {
            for lane in 0..3 {
                if self.location_abilities_active(lane)
                    && let Some(hook) = self.lanes[lane].def.on_event
                {
                    let _ = self.counted_trigger(|game| {
                        game.record_location_hook(lane, "on_event");
                        hook(game, lane, event)
                    });
                }
            }
        }
        for card in observers {
            if self.cards[card].lane.is_some() && self.card_triggers_allowed(card) {
                let face_down = !self.cards[card].revealed;
                self.dispatch_card_event(card, event, true, face_down);
            }
        }
        if matches!(
            event,
            Event::GameStart
                | Event::TurnStart
                | Event::TurnEnd
                | Event::AfterTurnEnd
                | Event::GameEnd
                | Event::Empower { .. }
                | Event::CardDrawn { .. }
                | Event::CardReturnedToHand { .. }
                | Event::CardDiscarded { .. }
        ) {
            let observers: Vec<_> = self
                .players
                .iter()
                .flat_map(|player| player.hand.iter().chain(&player.deck).copied())
                .filter(|&id| self.cards[id].has_event_ability(true, false))
                .collect();
            for card in observers {
                self.dispatch_card_event(card, event, true, false);
            }
        }
        if matches!(
            event,
            Event::BeforeCardRevealed { .. }
                | Event::CardMerged { .. }
                | Event::CardMergedBatch { .. }
                | Event::CardStaged { .. }
        ) {
            let observers: Vec<_> = self
                .players
                .iter()
                .flat_map(|player| player.hand.iter().copied())
                .filter(|&id| self.cards[id].has_event_ability(false, false))
                .collect();
            for card in observers {
                if self
                    .players
                    .iter()
                    .any(|player| player.hand.contains(&card))
                {
                    self.dispatch_card_event(card, event, false, false);
                }
            }
        }
        if matches!(event, Event::TurnStart) {
            let cards: Vec<_> = self
                .players
                .iter()
                .flat_map(|p| p.hand.iter().chain(&p.deck).copied())
                .chain(
                    self.lanes
                        .iter()
                        .flat_map(|lane| lane.cards.iter().copied()),
                )
                .filter(|&id| !self.cards[id].abilities_removed)
                .collect();
            for card in cards {
                let indices: Vec<_> = self.cards[card]
                    .attached_abilities
                    .iter()
                    .enumerate()
                    .filter_map(|(i, ability)| {
                        matches!(ability.kind, AttachedAbilityKind::NicoPrepare { .. }).then_some(i)
                    })
                    .collect();
                for index in indices {
                    self.prepare_nico(card, index);
                }
            }
        }
        if !matches!(event, Event::CardResolved { .. }) {
            for lane in 0..3 {
                if self.location_abilities_active(lane)
                    && let Some(hook) = self.lanes[lane].def.on_event
                {
                    let _ = self.counted_trigger(|game| {
                        game.record_location_hook(lane, "on_event");
                        hook(game, lane, event)
                    });
                }
            }
        }
    }
    pub(crate) fn emit_merged(&mut self, card: usize, absorbed: usize) {
        let event = Event::CardMerged { card, absorbed };
        self.emit(event);
        // The absorbed entity is already set aside, so it is absent from the
        // ordinary board and hand observer scans.
        self.dispatch_absorbed_merge(absorbed, event);
        self.ongoing();
    }
    /// Install an independent instance; repeated reveals can install repeated effects.
    pub fn add_player_effect(&mut self, seat: usize, effect: PendingEffect) {
        self.players[seat].pending_effects.push(effect);
    }

    fn trigger_player_effects(&mut self, seat: usize, trigger: PlayerTrigger, card: Option<usize>) {
        let effects = std::mem::take(&mut self.players[seat].pending_effects);
        let mut survivors = Vec::new();
        for mut effect in effects {
            let fired = self.ability_call_depth < 32
                && effect.trigger == trigger
                && !(trigger != PlayerTrigger::CardBanished
                    && card == Some(effect.source)
                    && (effect.created_turn == self.turn
                        || effect.lifetime == EffectLifetime::NextOtherCard));
            let completed = fired
                && self
                    .counted_trigger(|game| {
                        game.record_card_hook(game.cards[effect.source].def.id, "player_effect");
                        (effect.apply)(game, &mut effect, card)
                    })
                    .unwrap_or(false);
            let expired = fired
                && matches!(
                    effect.lifetime,
                    EffectLifetime::NextCard | EffectLifetime::NextOtherCard
                )
                || trigger == PlayerTrigger::EndTurn && effect.lifetime == EffectLifetime::ThisTurn;
            if !completed && !expired {
                survivors.push(effect);
            }
        }
        survivors.append(&mut self.players[seat].pending_effects);
        self.players[seat].pending_effects = survivors;
    }

    /// Player::HasCardStagedForPlay (0x7de9de0), including observed history.
    pub fn has_played_on_turn(&self, seat: usize, turn: u8) -> bool {
        self.players[seat].observed_play_turns.contains(&turn)
            || self.players[seat]
                .staged_history
                .iter()
                .any(|play| play.turn == turn)
    }

    /// MilesMoralesBehavior::ReduceCost (0x7ef8abc), including forced moves.
    pub fn has_moved_on_turn(&self, seat: usize, turn: u8) -> bool {
        self.players[seat].observed_move_turns.contains(&turn)
            || self
                .cards
                .iter()
                .any(|card| card.owner == seat && card.move_turns.contains(&turn))
    }

    pub fn set_total_turns(&mut self, turns: u8) {
        // Game::ModifyTotalNumTurns @ 0x7db19c0 clamps the new total to
        // the current turn, so removing Limbo cannot cancel an ongoing turn 7.
        let turns = if cards::reference_rules_enabled() {
            turns
        } else {
            turns.max(self.turn)
        };
        let old = self.total_turns;
        self.total_turns = turns;
        if old != turns {
            self.emit(Event::TurnCountChanged { old, new: turns });
        }
    }
    pub fn set_energy(&mut self, seat: usize, energy: i32) {
        let old = self.players[seat].energy;
        self.players[seat].energy = energy;
        // PlayerEnergySystem::ApplyEnergyUpdate 0x7ca0728 records the peak
        // before dispatching energy-change notifications.
        self.players[seat].highest_energy_this_game =
            self.players[seat].highest_energy_this_game.max(energy);
        if old != energy {
            self.emit(Event::EnergyChanged {
                seat,
                old,
                new: energy,
            });
        }
    }
    /// Player::AddPermanentEnergy (0x7ca0900) schedules a persistent maximum
    /// increase from the next turn for post-resolve and end-turn effects.
    pub fn add_permanent_energy(&mut self, seat: usize, amount: i32) {
        self.players[seat].permanent_energy += amount;
        if self.is_start_turn {
            self.set_energy(seat, (self.players[seat].energy + amount).max(0));
        }
    }

    pub fn schedule_energy(&mut self, seat: usize, turn: u8, amount: i32) {
        self.players[seat].scheduled_energy.push((turn, amount));
    }
    pub fn set_temporary_hand_cost(&mut self, card: usize, turn: u8, value: i32) {
        self.cards[card].attach_cost_effect(CardCostEffectKind::TemporarySet { turn, value });
        // Ability::AddAbility refreshes ongoing stats (0x7c8b180).
        if !cards::reference_rules_enabled() {
            self.ongoing();
        }
    }
    pub fn movable_cards_played_count(&self, seat: usize) -> usize {
        self.players[seat].movable_cards_played.len()
    }
    /// Game::GetAllRevealedCards (0x7dac98c) orders by latest resolution.
    pub fn revealed_cards_in_resolution_order(&self) -> Vec<usize> {
        let mut cards: Vec<_> = self
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&card| self.cards[card].revealed)
            .collect();
        cards.sort_by_key(|&card| self.cards[card].latest_resolution_order);
        cards
    }

    fn change_stat(
        &mut self,
        card: usize,
        stat: Stat,
        operation: StatOperation,
        value: i32,
        source: usize,
        is_steal: bool,
    ) {
        if stat == Stat::Power && !self.is_character(card) {
            return;
        }
        let target = &self.cards[card];
        let old = match stat {
            Stat::Power => target.permanent,
            Stat::Cost => target.cost,
        };
        let new = match operation {
            StatOperation::Add => old + value,
            StatOperation::Multiply => old * value,
            StatOperation::Set => value,
        };
        if stat == Stat::Power && new < old && !target.can_reduce_power() {
            return;
        }
        if stat == Stat::Cost && !self.cost_change_allowed(card, old, new) {
            return;
        }
        let correction_before = if stat == Stat::Power {
            self.luke_power_correction(card)
        } else {
            0
        };
        let target = &mut self.cards[card];
        target.stat_updates.push(CardStatUpdate {
            stat,
            operation,
            value,
            source,
            ongoing: false,
            ongoing_source: None,
        });
        if stat == Stat::Power {
            target.power_modifiers.push(CardPowerModifier {
                operation,
                value,
                source: Some(source),
            });
        }
        match stat {
            Stat::Power => target.permanent = new,
            Stat::Cost => target.cost = new,
        }
        if stat == Stat::Power {
            self.cards[card].ongoing += self.luke_power_correction(card) - correction_before;
        }
        if stat == Stat::Power && new > old {
            self.emit(Event::Empower {
                card,
                amount: new - old,
                operation,
                is_steal,
            });
        }
    }

    pub fn add_power(&mut self, card: usize, amount: i32, source: usize) {
        self.change_stat(card, Stat::Power, StatOperation::Add, amount, source, false);
    }
    /// AddPowerPermanently with its post-increase trigger disabled.
    pub fn add_power_silent(&mut self, card: usize, amount: i32, source: usize) {
        if !self.is_character(card) {
            return;
        }
        if amount < 0 && !self.cards[card].can_reduce_power() {
            return;
        }
        let correction_before = self.luke_power_correction(card);
        let target = &mut self.cards[card];
        target.permanent += amount;
        target.stat_updates.push(CardStatUpdate {
            stat: Stat::Power,
            operation: StatOperation::Add,
            value: amount,
            source,
            ongoing: false,
            ongoing_source: None,
        });
        target.power_modifiers.push(CardPowerModifier {
            operation: StatOperation::Add,
            value: amount,
            source: Some(source),
        });
        self.cards[card].ongoing += self.luke_power_correction(card) - correction_before;
    }
    pub fn set_power(&mut self, card: usize, value: i32, source: usize) {
        self.change_stat(card, Stat::Power, StatOperation::Set, value, source, false);
    }
    /// QuantumRealmBehavior::SetCardPower (0x7d765ac) sets the base,
    /// preserving the ordered permanent modifiers on top of it.
    pub fn set_base_power(&mut self, card: usize, value: i32) {
        if !self.is_character(card) {
            return;
        }
        let target = &self.cards[card];
        let old_base = target
            .base_power_override
            .unwrap_or(CATALOG[target.def.id].power);
        let mut old = old_base;
        let mut new = value;
        for update in target
            .stat_updates
            .iter()
            .filter(|u| !u.ongoing && u.stat == Stat::Power)
        {
            match update.operation {
                StatOperation::Add => {
                    old += update.value;
                    new += update.value;
                }
                StatOperation::Multiply => {
                    old *= update.value;
                    new *= update.value;
                }
                StatOperation::Set => {
                    old = update.value;
                    new = update.value;
                }
            }
        }
        let delta = new - old;
        if delta < 0 && !self.can_reduce_card_power(card) {
            return;
        }
        self.cards[card].base_power_override = Some(value);
        self.cards[card].permanent += delta;
        if delta > 0 {
            self.emit(Event::Empower {
                card,
                amount: delta,
                operation: StatOperation::Set,
                is_steal: false,
            });
        }
    }
    pub fn multiply_power(&mut self, card: usize, factor: i32, source: usize) {
        self.change_stat(
            card,
            Stat::Power,
            StatOperation::Multiply,
            factor,
            source,
            false,
        );
    }
    pub fn steal_power(&mut self, from: usize, to: usize, amount: i32, source: usize) {
        self.steal_power_many(&[from], to, amount, source);
    }
    /// Card::StealPowerPermanently (0x7cb733c) sends one aggregate increase trigger.
    pub fn steal_power_many(
        &mut self,
        from: &[usize],
        to: usize,
        amount: i32,
        source: usize,
    ) -> bool {
        let mut stolen = 0;
        for &card in from {
            // StealPowerPermanently (0x7cb733c) checks immunity before adding a modifier.
            if !self.can_reduce_card_power(card) {
                continue;
            }
            let before = self.cards[card].permanent;
            self.add_power(card, -amount, source);
            stolen += before - self.cards[card].permanent;
        }
        if stolen > 0 && self.is_character(to) {
            let owner = self.cards[to].owner;
            if !self.players[owner].stolen_power_recipients.contains(&to) {
                self.players[owner].stolen_power_recipients.push(to);
            }
            self.change_stat(to, Stat::Power, StatOperation::Add, stolen, source, true);
        }
        stolen > 0
    }
    /// Steal distinct amounts per target, emitting one aggregate Empower.
    pub fn steal_power_weighted(&mut self, from: &[(usize, i32)], to: usize, source: usize) {
        let mut stolen = 0;
        for &(card, amount) in from {
            if !self.can_reduce_card_power(card) {
                continue;
            }
            let before = self.cards[card].permanent;
            self.add_power(card, -amount, source);
            stolen += before - self.cards[card].permanent;
        }
        if stolen > 0 && self.is_character(to) {
            let owner = self.cards[to].owner;
            if !self.players[owner].stolen_power_recipients.contains(&to) {
                self.players[owner].stolen_power_recipients.push(to);
            }
            self.change_stat(to, Stat::Power, StatOperation::Add, stolen, source, true);
        }
    }
    pub fn add_cost(&mut self, card: usize, amount: i32, source: usize) {
        self.change_stat(card, Stat::Cost, StatOperation::Add, amount, source, false);
    }
    pub fn set_cost(&mut self, card: usize, value: i32, source: usize) {
        self.change_stat(card, Stat::Cost, StatOperation::Set, value, source, false);
    }
    pub fn add_ongoing_power(&mut self, card: usize, amount: i32, source: usize) {
        if self.is_character(card) && (amount >= 0 || self.can_reduce_card_power(card)) {
            self.cards[card].ongoing += amount;
            self.cards[card].stat_updates.push(CardStatUpdate {
                stat: Stat::Power,
                operation: StatOperation::Add,
                value: amount,
                source,
                ongoing: true,
                ongoing_source: self.current_ongoing_source,
            });
        }
    }
    /// Apply a location's power contribution while retaining it in the stat ledger.
    pub fn add_location_power(&mut self, card: usize, amount: i32) {
        if !self.is_character(card) {
            return;
        }
        self.cards[card].location_power += amount;
        let source = self.cards[card].owner;
        self.cards[card].stat_updates.push(CardStatUpdate {
            stat: Stat::Power,
            operation: StatOperation::Add,
            value: amount,
            source,
            ongoing: true,
            ongoing_source: self.current_ongoing_source,
        });
    }
    pub fn add_ongoing_cost(&mut self, card: usize, amount: i32, source: usize) {
        let old = self.cards[card].cost + self.cards[card].ongoing_cost;
        // Card::AddCost (0x7cb69e4) uses a zero minimum, and the general
        // overload (0x7cb6a2c) records a Set if that minimum clips the delta.
        let raw = old.saturating_add(amount);
        let clipped = !cards::reference_rules_enabled() && raw < 0;
        let new = if clipped { 0 } else { raw };
        if self.cost_change_allowed(card, old, new) {
            self.cards[card].ongoing_cost = new - self.cards[card].cost;
            self.cards[card].stat_updates.push(CardStatUpdate {
                stat: Stat::Cost,
                operation: if clipped {
                    StatOperation::Set
                } else {
                    StatOperation::Add
                },
                value: if clipped { new } else { amount },
                source,
                ongoing: true,
                ongoing_source: self.current_ongoing_source,
            });
        }
    }
    pub fn set_ongoing_cost(&mut self, card: usize, value: i32, source: usize) {
        let old = self.cards[card].cost + self.cards[card].ongoing_cost;
        if self.cost_change_allowed(card, old, value) {
            self.cards[card].ongoing_cost = value - self.cards[card].cost;
            self.cards[card].stat_updates.push(CardStatUpdate {
                stat: Stat::Cost,
                operation: StatOperation::Set,
                value,
                source,
                ongoing: true,
                ongoing_source: self.current_ongoing_source,
            });
        }
    }

    /// IsleOfSilenceBehavior::DisableOngoing (0x7d5a9ec).
    pub fn ongoing_active(&self, card: usize) -> bool {
        self.cards[card].has_ongoing(true)
            && self.cards[card]
                .lane
                .is_none_or(|lane| self.lane_ongoing_allowed(lane))
    }

    /// JeanGrey's staging-time ongoing examines its previous location while
    /// staged to move (ApplyMustPlayHereFirst 0x7ed2460).
    pub(crate) fn jean_ongoing_active(&self, card: usize) -> bool {
        let target = &self.cards[card];
        let lane = if self.is_wait_for_staging && target.move_staged {
            target.last_lane
        } else {
            target.lane
        };
        target.has_ongoing(true) && lane.is_none_or(|lane| self.lane_ongoing_allowed(lane))
    }

    pub fn cost_change_allowed(&self, card: usize, old: i32, new: i32) -> bool {
        if !*HAS_COST_GUARDS {
            return true;
        }
        self.lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .all(|id| {
                !self.cards[id].revealed
                    || !self.ongoing_active(id)
                    || self.cards[id]
                        .ongoing_guard_abilities()
                        .filter(|a| {
                            a.supports(AbilityKind::Ongoing)
                                && (a.id.is_some() || !self.cards[id].abilities_removed)
                        })
                        .all(|a| {
                            a.def.cost_guard.is_none_or(|guard| {
                                self.record_card_hook(a.def.id, "cost_guard");
                                guard(self, id, card, old, new)
                            })
                        })
            })
    }
    /// ScorpionBrandNewDayBehavior::DoubleAfflictions (0x7f3240c).
    pub fn double_afflictions(&mut self, source: usize) {
        let enemy = 1 - self.cards[source].owner;
        for lane in 0..3 {
            for index in 0..self.lanes[lane].cards.len() {
                let card = self.lanes[lane].cards[index];
                if !self.cards[card].revealed || self.cards[card].owner != enemy {
                    continue;
                }
                let amount: i32 = self.cards[card]
                    .stat_updates
                    .iter()
                    .filter(|u| {
                        u.stat == Stat::Power && u.operation == StatOperation::Add && u.value < 0
                    })
                    .map(|u| u.value)
                    .sum();
                if amount < 0 && self.cards[card].can_reduce_power() {
                    self.add_ongoing_power(card, amount, self.cards[source].owner);
                }
            }
        }
    }

    /// SuperiorIronManBehavior::DoublePowerIncrease (0x7ccd148).
    pub fn double_power_increases(&mut self, card: usize) {
        let mut current = CATALOG[self.cards[card].def.id].power;
        let mut bonus = 0;
        for update in self.cards[card]
            .stat_updates
            .iter()
            .filter(|u| u.stat == Stat::Power)
        {
            let next = match update.operation {
                StatOperation::Add => current + update.value,
                StatOperation::Multiply => current * update.value,
                StatOperation::Set => update.value,
            };
            bonus += (next - current).max(0);
            current = next;
        }
        self.cards[card].ongoing += bonus;
    }

    /// SuperiorIronManBehavior::DoubleCostReductions (0x7ccc9d8).
    pub fn double_cost_reductions(&self, card: usize, cost: i32) -> i32 {
        let mut current = CATALOG[self.cards[card].def.id].cost;
        let mut reductions = 0;
        for update in self.cards[card]
            .stat_updates
            .iter()
            .filter(|u| u.stat == Stat::Cost)
        {
            let next = match update.operation {
                StatOperation::Add => current + update.value,
                StatOperation::Multiply => current * update.value,
                StatOperation::Set => update.value,
            };
            reductions += (current - next).max(0);
            current = next;
        }
        for (lane, location) in self.lanes.iter().enumerate() {
            if cards::reference_rules_enabled()
                && location.revealed
                && let Some(hook) = location.def.modify_cost
            {
                self.record_location_hook(lane, "modify_cost");
                let next =
                    hook(self, lane, card, current).map_or(current, |update| update.apply(current));
                if self.cost_change_allowed(card, current, next) {
                    reductions += (current - next).max(0);
                    current = next;
                }
            }
        }
        cost - reductions
    }

    /// CardSlotsComponent::RemoveCard (0x7c8e710) normally shifts later slots left.
    pub fn zone_position(&self, card: usize) -> Option<usize> {
        let target = &self.cards[card];
        let lane = target.lane?;
        self.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&id| self.cards[id].owner == target.owner)
            .position(|id| id == card)
    }
    /// Location::IsCardInFrontRow (0x7ddf5e8).
    pub fn is_front_row(&self, card: usize) -> bool {
        self.zone_position(card).is_some_and(|slot| slot < 2)
    }
    /// LukeCageBehavior::BuffCards (0x7eeb1d4), across all locations.
    pub(crate) fn can_reduce_card_power(&self, card: usize) -> bool {
        self.cards[card].can_reduce_power()
            && (cards::reference_rules_enabled()
                || !self.cards[card].revealed
                || !self.is_front_row(card)
                || !self.lanes.iter().any(|lane| {
                    lane.cards.iter().any(|&id| {
                        self.cards[id].owner == self.cards[card].owner
                            && self.cards[id].revealed
                            && self.cards[id].default_ability().id == "LukeCage"
                            && self.ongoing_active(id)
                    })
                }))
    }

    // Card::AddPowerPermanently (0x7cb7210) always adds its modifier; Luke's
    // SetCanHavePowerReduced (0x7cb8478) suppresses it until protection ends.
    fn luke_power_correction(&self, card: usize) -> i32 {
        if self.can_reduce_card_power(card) || !self.cards[card].can_reduce_power() {
            return 0;
        }
        let target = &self.cards[card];
        let base = target
            .base_power_override
            .unwrap_or(CATALOG[target.def.id].power);
        let mut normal = base;
        let mut protected = base;
        for update in target
            .stat_updates
            .iter()
            .filter(|u| u.stat == Stat::Power && !u.ongoing)
        {
            let apply = |old: i32| match update.operation {
                StatOperation::Add => old + update.value,
                StatOperation::Multiply => old * update.value,
                StatOperation::Set => update.value,
            };
            normal = apply(normal);
            protected = protected.max(apply(protected));
        }
        protected - normal
    }
    pub fn is_back_row(&self, card: usize) -> bool {
        self.zone_position(card).is_some_and(|slot| slot > 1)
    }
    /// WithIncreasedPower, RVA 0x7d29874, compares current against printed power.
    pub fn has_increased_power(&self, card: usize) -> bool {
        self.is_character(card) && self.cards[card].power() > CATALOG[self.cards[card].def.id].power
    }
    /// ScorpionBrandNewDayBehavior::<IsAfflicted>b__3_0, RVA 0x7f32ef8.
    pub fn is_afflicted(&self, card: usize) -> bool {
        self.cards[card].stat_updates.iter().any(|update| {
            update.stat == Stat::Power && update.operation == StatOperation::Add && update.value < 0
        })
    }
    pub fn reset_power(&mut self, card: usize) {
        let target = &mut self.cards[card];
        target.base_power_override = None;
        target.permanent = CATALOG[target.def.id].power;
        target
            .stat_updates
            .retain(|update| update.stat != Stat::Power || update.ongoing);
    }
    pub fn is_character(&self, card: usize) -> bool {
        CATALOG[self.cards[card].def.id].card_type == "Character"
    }
    /// Faction affiliation from the client card definition, including its base card.
    pub fn in_faction(&self, card: usize, faction: &str) -> bool {
        CATALOG[self.cards[card].def.id]
            .factions
            .iter()
            .any(|value| value == faction)
    }
    /// Client CardDef::GetFactionCardDefId (0x7c1c33c).
    pub fn faction_card_def(id: &str, faction: &str) -> Option<&'static str> {
        let base = CATALOG.get(id)?;
        if !base.factions.iter().any(|value| value == faction) {
            return None;
        }
        base.faction_cards.first().map(String::as_str)
    }
    pub fn is_skill(&self, card: usize) -> bool {
        CATALOG[self.cards[card].def.id].card_type == "Skill"
    }
    /// Choose uniformly from registered, obtainable deck cards satisfying `filter`.
    pub fn random_card(&self, filter: impl Fn(&data::CardData) -> bool) -> Option<&'static str> {
        let candidates: Vec<_> = cards::deck_eligible_ids()
            .iter()
            .copied()
            .filter(|id| {
                let row = &CATALOG[*id];
                !row.random_pool_excluded && filter(row)
            })
            .collect();
        if candidates.is_empty() {
            None
        } else {
            Some(candidates[self.rng.borrow_mut().randbelow(candidates.len())])
        }
    }
    /// Card::IsQuickdrawActive (0x7cb1b88); turn zero does not qualify.
    pub fn is_quickdraw_active(&self, card: usize) -> bool {
        self.cards[card].drawn_turn.is_some_and(|turn| {
            turn != 0
                && (turn == self.turn
                    || self.cards[card].draw_extends_quickdraw
                        && turn.checked_add(1) == Some(self.turn))
        })
    }
    /// HydraBobBehavior::MoveToLocation (0x7ec8b00) checks both players' snap turns.
    pub fn snapped_this_turn(&self) -> bool {
        self.players
            .iter()
            .any(|player| player.snapped_turns.contains(&self.turn))
    }

    pub fn new<R: GameRng + 'a>(
        decks: [&[String]; 2],
        locations: &[String],
        rng: &'a mut R,
    ) -> Result<Self, String> {
        if locations.len() < 3 {
            return Err("at least three locations are required".into());
        }
        let mut available = Vec::with_capacity(locations.len());
        for id in locations {
            if available.iter().any(|d: &&LocationDef| d.id == id) {
                continue;
            }
            available.push(locations::get(id).ok_or_else(|| format!("unsupported location {id}"))?);
        }
        if available.len() < 3 {
            return Err("at least three distinct locations are required".into());
        }
        let pre_start = decks
            .iter()
            .flat_map(|deck| deck.iter())
            .any(|id| cards::get(id).is_some_and(|def| def.pre_game_start.is_some()));
        // Both shuffled decks and opening hands precede random location sampling.
        let mut cards = Vec::new();
        let mut players: [Player; 2] = Default::default();
        for seat in 0..2 {
            if decks[seat].len() != 12 {
                return Err("decks must have 12 cards".into());
            }
            for id in decks[seat] {
                if decks[seat].iter().filter(|other| *other == id).count() != 1 {
                    return Err(format!("duplicate card {id}"));
                }
                if cards::is_token(id) {
                    return Err(format!("non-ownable card {id}"));
                }
                let def = cards::get(id).ok_or_else(|| format!("unsupported card {id}"))?;
                let data = CATALOG
                    .get(id)
                    .ok_or_else(|| format!("missing catalog card {id}"))?;
                let index = cards.len();
                cards.push(Card {
                    def,
                    override_ability: None,
                    morphed_from: None,
                    morphed_from_entity: None,
                    replaced_card_def_id: None,
                    owner: seat,
                    cost: data.cost,
                    ongoing_cost: 0,
                    dynamic_assigned_cost: None,
                    cost_effects: Vec::new(),
                    next_cost_effect_sequence: 0,
                    created: false,
                    created_in_hand: Some(false),
                    created_hand_seat: None,
                    creation_pending: false,
                    created_by_card_id: None,
                    created_by_location_id: None,
                    created_by_player: None,
                    observed_entity_id: None,
                    merged_to_entity_id: None,
                    permanent: data.power,
                    base_power_override: None,
                    stat_updates: Vec::new(),
                    power_modifiers: Vec::new(),
                    power_modifiers_complete: true,
                    ongoing: 0,
                    location_power: 0,
                    lane: None,
                    last_lane: None,
                    zone: Some(crate::zones::CardZone::Deck(seat)),
                    previous_zone: None,
                    revealed: false,
                    abilities_removed: false,
                    repeating: false,
                    latest_resolution_order: 0,
                    turn_played: 0,
                    turn_revealed: None,
                    drawn_turn: None,
                    draw_extends_quickdraw: false,
                    move_turns: Vec::new(),
                    player_moves_used: 0,
                    player_move_turn: None,
                    external_moves: Vec::new(),
                    activated: false,
                    move_staged: false,
                    activate_staged: false,
                    observed_can_activate: None,
                    observed_move_targets: None,
                    merged_to: None,
                    attached_abilities: initial_attachments(def),
                    composed_abilities: Vec::new(),
                    copied_ongoing_guards: Vec::new(),
                    virtual_ongoing_ids: Vec::new(),
                    ability_frames: Vec::new(),
                    default_ability_generation: 0,
                    state: def.initial_state.map(|init| init()),
                    borrowed_original_state: None,
                });
                crate::zones::initialize_deck_card(&mut players[seat], index);
            }
            players[seat].starting_deck = Some(
                players[seat]
                    .deck
                    .iter()
                    .map(|&id| cards[id].def.id)
                    .collect(),
            );
            players[seat].starting_deck_instances = Some(players[seat].deck.clone());
            if !pre_start {
                rng.shuffle(&mut players[seat].deck);
            }
        }
        let chosen = if pre_start {
            [0, 1, 2]
        } else {
            let indices = rng.sample_indices(available.len(), 3);
            [indices[0], indices[1], indices[2]]
        };
        let lanes = std::array::from_fn(|i| Lane {
            def: available[chosen[i]],
            observed_id: None,
            revealed: false,
            destroyed: false,
            ability_flags: LocationAbilityFlags::default(),
            copied_reveal_turn: None,
            revealed_turn: None,
            cards: Vec::new(),
            bonus: [0; 2],
            multiplier: [1; 2],
            contributions: Vec::new(),
            state: if pre_start {
                None
            } else {
                available[chosen[i]].initial_state.map(|init| init())
            },
        });
        let mut game = Self {
            cards,
            ability_call_depth: 0,
            replay_hooks: None,
            players,
            human_seat: None,
            lanes,
            location_pool: available,
            starting_location_ids: Vec::new(),
            turn: 0,
            stakes: None,
            next_resolution_order: 1,
            is_wait_for_staging: false,
            is_start_turn: false,
            total_turns: 6,
            reveal_priority: None,
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
            location_reveal_order: [0, 1, 2],
            destroyed: None,
            score_overrides: [None; 2],
            current_ongoing_source: None,
            merge_batches: Vec::new(),
            staging_order: None,
            move_provenance_known: true,
            last_move_source: None,
            rng: RefCell::new(GameRandom::Borrowed(rng)),
        };
        if pre_start {
            // Game::GetAllCardsForActivatingTriggerAbility (0x7dacfcc)
            // materializes the entity list before any GameStart callback.
            // Newly generated deck cards therefore cannot join this dispatch.
            let original_count = game.cards.len();
            for card in 0..original_count {
                if game.cards[card].abilities_removed {
                    continue;
                }
                let original = game.cards[card].default_ability();
                let generation = game.cards[card].default_ability_generation;
                if let Some(hook) = original.pre_game_start {
                    let _ = game.counted_trigger(|game| {
                        game.record_card_hook(original.id, "pre_game_start");
                        hook(game, card)
                    });
                }
                if !game.cards[card].abilities_removed
                    && game.cards[card].default_ability_generation == generation
                    && original.id != "NicoMinoru"
                    && let Some(hook) = original.on_event
                {
                    let _ = game.counted_trigger(|game| {
                        game.record_card_hook(original.id, "on_event");
                        hook(game, card, Event::GameStart)
                    });
                }
            }
            let chosen;
            {
                let mut random = game.rng.borrow_mut();
                for player in &mut game.players {
                    for index in (1..player.deck.len()).rev() {
                        let other = random.randbelow(index + 1);
                        crate::zones::swap_initial_deck(player, index, other);
                    }
                }
                chosen = random.sample_indices(game.location_pool.len(), 3);
            }
            for (lane, &index) in chosen.iter().enumerate() {
                let def = game.location_pool[index];
                game.lanes[lane].def = def;
                game.lanes[lane].state = def.initial_state.map(|init| init());
            }
        } else {
            game.emit(Event::GameStart);
        }
        game.starting_location_ids = game.lanes.iter().map(|lane| lane.def.id).collect();
        // StartGame::OnEnter (0x7e47c68) triggers game-start before the opening draw.
        for seat in 0..2 {
            game.draw_initial_hand(seat);
        }
        for player in &mut game.players {
            player.starting_hand = Some(player.hand.clone());
        }
        game.turn = 1;
        Ok(game)
    }

    fn draw_initial_hand(&mut self, seat: usize) {
        // DrawSelfDef::DrawSelf (0x7cc1158) intercepts initial-hand draws only.
        // Keep the existing Quicksilver behavior when no other such card exists.
        for _ in 0..3 {
            if self.players[seat].hand.len() >= 7 {
                break;
            }
            let slot = self.players[seat]
                .deck
                .iter()
                .position(|&id| {
                    !self.cards[id].abilities_removed
                        && (self.cards[id].default_ability().start_in_hand
                            || self.cards[id].def.id == "Quicksilver"
                            || cards::meta_variants_c::skill_for_champion(self.cards[id].def.id)
                                .is_some())
                })
                .or_else(|| {
                    self.players[seat]
                        .deck
                        .iter()
                        .position(|&id| self.cards[id].def.draw_turn.is_none())
                });
            let Some(slot) = slot else {
                break;
            };
            let card = self.take_deck(seat, slot);
            let skill = if self.cards[card].abilities_removed {
                None
            } else {
                cards::meta_variants_c::skill_for_champion(self.cards[card].def.id)
            };
            if let Some(skill) = skill {
                let handled = self.counted_trigger(|game| {
                    if game.add_hand(seat, card, true) && game.players[seat].hand.len() < 7 {
                        let skill = game.create_card(skill, seat);
                        game.add_hand(seat, skill, false);
                    }
                });
                if handled.is_some() {
                    continue;
                }
            }
            self.add_hand(seat, card, true);
        }
    }

    pub fn played_cards_at(&self, seat: usize, lane: usize) -> impl Iterator<Item = usize> + '_ {
        self.pending_plays[seat]
            .iter()
            .chain(&self.played[seat])
            .filter_map(move |&(card, played_lane)| {
                (played_lane == lane && self.cards[card].turn_played > 0).then_some(card)
            })
    }

    pub fn played_at(&self, seat: usize, lane: usize) -> bool {
        self.played_cards_at(seat, lane).next().is_some()
    }

    /// SpecialDeckSystem::GetPlayerChampionCard (0x7ca2b30) selects the
    /// owner's original-deck Champion, not a subsequently created copy.
    pub fn player_champion(&self, seat: usize) -> Option<usize> {
        let original = self.players[seat].starting_deck_instances.as_ref();
        self.cards.iter().enumerate().find_map(|(id, card)| {
            (card.owner == seat
                && card.def.id.contains("Champion")
                && self.is_character(id)
                && original.map_or(!card.created, |deck| deck.contains(&id)))
            .then_some(id)
        })
    }

    pub fn create_card(&mut self, id: &str, owner: usize) -> usize {
        debug_assert!(cards::get(id).is_some(), "unregistered created card {id}");
        let def = crate::position::card_def(id).expect("unregistered created card");
        let row = CATALOG.get(id).expect("missing created card data");
        let index = self.cards.len();
        self.cards.push(Card {
            def,
            override_ability: None,
            morphed_from: None,
            morphed_from_entity: None,
            replaced_card_def_id: None,
            owner,
            ongoing_cost: 0,
            dynamic_assigned_cost: None,
            cost_effects: Vec::new(),
            next_cost_effect_sequence: 0,
            cost: row.cost,
            permanent: row.power,
            base_power_override: None,
            stat_updates: Vec::new(),
            power_modifiers: Vec::new(),
            power_modifiers_complete: true,
            created: true,
            created_in_hand: None,
            created_hand_seat: None,
            creation_pending: true,
            created_by_card_id: None,
            created_by_location_id: None,
            created_by_player: None,
            observed_entity_id: None,
            merged_to_entity_id: None,
            ongoing: 0,
            location_power: 0,
            lane: None,
            last_lane: None,
            zone: None,
            previous_zone: None,
            revealed: false,
            abilities_removed: false,
            repeating: false,
            latest_resolution_order: 0,
            turn_played: 0,
            turn_revealed: None,
            drawn_turn: None,
            draw_extends_quickdraw: false,
            move_turns: Vec::new(),
            player_moves_used: 0,
            player_move_turn: None,
            external_moves: Vec::new(),
            activated: false,
            move_staged: false,
            activate_staged: false,
            observed_can_activate: None,
            observed_move_targets: None,
            merged_to: None,
            attached_abilities: initial_attachments(def),
            composed_abilities: Vec::new(),
            copied_ongoing_guards: Vec::new(),
            virtual_ongoing_ids: Vec::new(),
            ability_frames: Vec::new(),
            default_ability_generation: 0,
            state: def.initial_state.map(|init| init()),
            borrowed_original_state: None,
        });
        index
    }

    /// Copy lasting card attributes, clearing board-only reveal state.
    pub fn copy_card(&mut self, source: usize, owner: usize) -> usize {
        let card = self.create_card(self.cards[source].def.id, owner);
        self.cards[card].cost = self.cards[source].cost;
        self.cards[card].permanent = self.cards[source].permanent;
        self.cards[card].base_power_override = self.cards[source].base_power_override;
        self.cards[card].stat_updates = self.cards[source]
            .stat_updates
            .iter()
            .copied()
            .filter(|update| !update.ongoing)
            .collect();
        self.cards[card].power_modifiers = self.cards[source].power_modifiers.clone();
        self.cards[card].power_modifiers_complete = self.cards[source].power_modifiers_complete;
        self.cards[card].abilities_removed = self.cards[source].abilities_removed;
        self.cards[card].override_ability = self.cards[source].override_ability;
        self.cards[card].dynamic_assigned_cost = self.cards[source].dynamic_assigned_cost;
        // Card::CopyEffects (0x7caf324) copies attached ability instances.
        self.cards[card].external_moves = self.cards[source].external_moves.clone();
        self.cards[card].composed_abilities = self.cards[source].copy_composed_abilities();
        self.copy_virtual_ongoing_contexts(source, card);
        if !cards::reference_rules_enabled() {
            let attached = self.cards[source].attached_abilities.clone();
            self.cards[card].attached_abilities.clear();
            for ability in attached {
                self.cards[card].attach_ability(ability);
            }
        }
        // White Queen's generated ongoing is a card-owned ability and its
        // captured location/player context is retained by Card::CopyEffects.
        for index in 0..self.cards[source].cost_effects.len() {
            let kind = self.cards[source].cost_effects[index].kind;
            if matches!(kind, CardCostEffectKind::WhiteQueenWinning { .. }) {
                self.cards[card].attach_cost_effect(kind);
            }
        }
        self.cards[card].morphed_from = self.cards[source].morphed_from;
        self.cards[card].morphed_from_entity = self.cards[source].morphed_from_entity;
        // Card::Copy (0x7caed0c) carries replacement provenance.
        self.cards[card].replaced_card_def_id = self.cards[source].replaced_card_def_id;
        self.cards[card].state = self.cards[source]
            .default_ability_state()
            .map(CardState::clone_box);
        if let Some(clear) = self.cards[card].default_ability().clear_reveal_state {
            clear(&mut self.cards[card]);
        }
        card
    }
    /// GetDefaultAbilities predicate (0x7cbe110): static IDs and abilities
    /// originating from a card with the receiver's original CardDefId.
    pub(crate) fn is_default_attached(&self, card: usize, ability: &AttachedAbility) -> bool {
        let receiver = &self.cards[card];
        (ability.id == "NicoMinoruStartOfTurnTrigger"
            && (receiver.def.id == "NicoMinoru" || receiver.default_ability().id == "NicoMinoru"))
            || ability
                .origin
                .and_then(|origin| self.cards.get(origin))
                .is_some_and(|origin| origin.def.id == receiver.def.id)
    }

    /// Card::RemoveText (0x7cb2ebc) removes default abilities, while
    /// preserving attached abilities originating from other cards.
    pub fn remove_text(&mut self, card: usize) {
        self.cards[card].default_ability_generation += 1;
        self.remove_default_composed(card);
        let mut abilities = std::mem::take(&mut self.cards[card].attached_abilities);
        abilities.retain(|ability| !self.is_default_attached(card, ability));
        self.cards[card].attached_abilities = abilities;
        let mut moves = std::mem::take(&mut self.cards[card].external_moves);
        let receiver = self.cards[card].def.id;
        moves.retain(|ability| {
            self.cards
                .get(ability.origin)
                .is_none_or(|origin| origin.def.id != receiver)
        });
        self.cards[card].external_moves = moves;
        self.cards[card].abilities_removed = true;
        self.cards[card].borrowed_original_state = None;
        self.cards[card].state = None;
        self.ongoing();
    }

    /// Copy the source's current text without changing this card's identity or stats.
    /// CopyTextAndActivateResolveAbility (0x7cbb3f4) refreshes ongoing before resolve.
    pub fn copy_text(&mut self, card: usize, source: usize, activate_reveal: bool) -> bool {
        let def = self.cards[source].default_ability();
        // Card::CopyText (0x7cbadb4) rejects an already installed override.
        if self.cards[card]
            .override_ability
            .is_some_and(|old| old.id == def.id)
        {
            return false;
        }
        self.cards[card].default_ability_generation += 1;
        self.remove_default_composed(card);
        let mut abilities = std::mem::take(&mut self.cards[card].attached_abilities);
        abilities.retain(|ability| !self.is_default_attached(card, ability));
        self.cards[card].attached_abilities = abilities;
        let removed = self.cards[source].abilities_removed;
        self.cards[card].override_ability = Some(def);
        self.cards[card].abilities_removed = removed;
        self.cards[card].borrowed_original_state = None;
        self.cards[card].state = def.initial_state.map(|init| init());
        if removed {
            self.cards[card].composed_abilities.clear();
            self.cards[card].external_moves.clear();
            self.cards[card].attached_abilities.clear();
        } else if def.id == "NicoMinoru" {
            for mut ability in initial_attachments(def) {
                ability.origin = Some(card);
                self.cards[card].attach_ability(ability);
            }
        }
        self.ongoing();
        if activate_reveal && !removed {
            // ActivateTextResolveAbility (0x7cbb424) uses the resolve count
            // from GetTriggerAbilityActivateCount (0x7cb2d1c).
            if cards::reference_rules_enabled() {
                self.trigger_on_reveal(card);
            } else {
                self.trigger_text_reveal(card);
            }
        }
        true
    }

    /// MorphIntoNewCard removes Set modifiers but retains Add and Multiply.
    pub fn transform_card(&mut self, card: usize, target_id: &str) -> bool {
        debug_assert!(
            cards::get(target_id).is_some() && CATALOG.contains_key(target_id),
            "unregistered transformation target {target_id}"
        );
        let Some(def) = cards::get(target_id) else {
            return false;
        };
        let Some(row) = CATALOG.get(target_id) else {
            return false;
        };
        self.cards[card].default_ability_generation += 1;
        let old = self.cards[card].def.id;
        // Card::<MorphIntoNewCard>b__477_0 (0x7cbf0cc) matches OperationType::Set.
        self.cards[card]
            .stat_updates
            .retain(|u| u.ongoing || u.operation != StatOperation::Set);
        self.cards[card]
            .power_modifiers
            .retain(|modifier| modifier.operation != StatOperation::Set);
        let mut power = row.power;
        let mut cost = row.cost;
        for update in self.cards[card].stat_updates.iter().filter(|u| !u.ongoing) {
            let value = match update.stat {
                Stat::Power => &mut power,
                Stat::Cost => &mut cost,
            };
            *value = match update.operation {
                StatOperation::Add => *value + update.value,
                StatOperation::Multiply => *value * update.value,
                StatOperation::Set => update.value,
            };
        }
        let target = &mut self.cards[card];
        target.def = def;
        target.morphed_from = Some(old);
        target.morphed_from_entity = None;
        // ClearAbilityOverrides (0x7cbba80) clears replacement provenance.
        target.replaced_card_def_id = None;
        target.override_ability = None;
        target.abilities_removed = false;
        target.borrowed_original_state = None;
        target.state = def.initial_state.map(|init| init());
        target.permanent = power;
        target.base_power_override = None;
        target.cost = cost;
        target
            .attached_abilities
            .retain(|a| !matches!(a.kind, AttachedAbilityKind::Watch { .. }));
        self.ongoing();
        // CompleteMorph (0x7cbb714) activates the new resolve text when eligible.
        if self.cards[card].lane.is_some() && self.cards[card].revealed {
            if cards::reference_rules_enabled() {
                self.trigger_on_reveal(card);
            } else {
                self.trigger_text_reveal(card);
            }
        }
        true
    }
    /// MorphIntoNewCardWithoutModifiers (0x7cbbe80).
    pub fn transform_card_without_modifiers(&mut self, card: usize, target_id: &str) -> bool {
        debug_assert!(
            cards::get(target_id).is_some() && CATALOG.contains_key(target_id),
            "unregistered transformation target {target_id}"
        );
        if cards::get(target_id).is_none() || !CATALOG.contains_key(target_id) {
            return false;
        }
        self.cards[card]
            .stat_updates
            .retain(|update| update.ongoing);
        self.cards[card].power_modifiers.clear();
        self.cards[card].power_modifiers_complete = true;
        self.transform_card(card, target_id)
    }

    /// MorphIntoCopy (0x7cbb5cc) copies the target's persistent card state.
    pub fn transform_into_copy(&mut self, card: usize, source: usize) -> bool {
        if card == source || self.cards[card].morphed_from_entity == Some(source) {
            return false;
        }
        let old = self.cards[card].def.id;
        let def = self.cards[source].def;
        let override_ability = self.cards[source].override_ability;
        let state = self.cards[source]
            .default_ability_state()
            .map(CardState::clone_box);
        let updates = self.cards[source]
            .stat_updates
            .iter()
            .copied()
            .filter(|update| !update.ongoing)
            .collect();
        let cost = self.cards[source].cost;
        let power = self.cards[source].permanent;
        let base_power_override = self.cards[source].base_power_override;
        let dynamic_assigned_cost = self.cards[source].dynamic_assigned_cost;
        let replaced_card_def_id = self.cards[source].replaced_card_def_id;
        let removed = self.cards[source].abilities_removed;
        // MorphIntoCopy (0x7cbb5cc) invokes Card::CopyEffects.
        let external_moves = self.cards[source].external_moves.clone();
        let composed_abilities = self.cards[source].copy_composed_abilities();
        let attached_abilities = (!cards::reference_rules_enabled())
            .then(|| self.cards[source].attached_abilities.clone());
        let power_modifiers = self.cards[source].power_modifiers.clone();
        let power_modifiers_complete = self.cards[source].power_modifiers_complete;
        let target = &mut self.cards[card];
        target.default_ability_generation += 1;
        target.def = def;
        target.morphed_from = Some(old);
        target.morphed_from_entity = Some(source);
        target.replaced_card_def_id = replaced_card_def_id;
        target.override_ability = override_ability;
        target.borrowed_original_state = None;
        target.state = state;
        target.external_moves = external_moves;
        target.composed_abilities = composed_abilities;
        for ability in &mut target.composed_abilities {
            if let Some((0, generation)) = &mut ability.parent {
                *generation = target.default_ability_generation;
            }
        }
        target.dynamic_assigned_cost = dynamic_assigned_cost;
        target.stat_updates = updates;
        target.power_modifiers = power_modifiers;
        target.power_modifiers_complete = power_modifiers_complete;
        target.cost = cost;
        target.permanent = power;
        target.base_power_override = base_power_override;
        target.abilities_removed = removed;
        if let Some(attached) = attached_abilities {
            target.attached_abilities.clear();
            for ability in attached {
                target.attach_ability(ability);
            }
        } else {
            target
                .attached_abilities
                .retain(|ability| !matches!(ability.kind, AttachedAbilityKind::Watch { .. }));
        }
        self.copy_virtual_ongoing_contexts(source, card);
        self.ongoing();
        if self.cards[card].lane.is_some() && self.cards[card].revealed {
            if cards::reference_rules_enabled() {
                self.trigger_on_reveal(card);
            } else {
                self.trigger_text_reveal(card);
            }
        }
        true
    }

    pub fn lane_locked(&self, lane: usize) -> bool {
        self.lanes[lane].cards.iter().any(|&card| {
            self.cards[card].revealed
                && self.ongoing_active(card)
                && self.cards[card]
                    .ongoing_guard_abilities()
                    .any(|ability| ability.supports(AbilityKind::Ongoing) && ability.def.locks_lane)
        })
    }

    /// Effective cost, Python's `energy_cost`: revealed location modifiers apply in lane
    /// order to cards in a hand only. Python resets that adjustment when a card is
    /// played, added to the board or discarded, so cards elsewhere use their lasting cost.
    pub fn card_cost(&self, card: usize) -> i32 {
        let mut cost = self.cards[card].cost + self.cards[card].ongoing_cost;
        if cards::reference_rules_enabled() {
            for effect in &self.players[self.cards[card].owner].cost_effects {
                if let PlayerCostEffectKind::BaronMordo {
                    card: target,
                    until,
                    value,
                } = effect.kind
                    && target == card
                    && self.turn <= until
                    && self.cost_change_allowed(card, cost, value)
                {
                    cost = value;
                }
            }
        }
        if self
            .players
            .iter()
            .any(|player| player.hand.contains(&card))
        {
            for (lane, location) in self.lanes.iter().enumerate() {
                if cards::reference_rules_enabled()
                    && location.revealed
                    && let Some(hook) = location.def.modify_cost
                {
                    self.record_location_hook(lane, "modify_cost");
                    let next =
                        hook(self, lane, card, cost).map_or(cost, |update| update.apply(cost));
                    if self.cost_change_allowed(card, cost, next) {
                        cost = next;
                    }
                }
            }
            if cards::reference_rules_enabled() {
                for effect in &self.cards[card].cost_effects {
                    if let CardCostEffectKind::NextTurnDelta { turn, delta } = effect.kind
                        && turn == self.turn
                        && self.cost_change_allowed(card, cost, cost + delta)
                    {
                        cost += delta;
                    }
                }
            }
            if !self.cards[card].abilities_removed
                && (cards::reference_rules_enabled()
                    || self.cards[card].default_ability().id != "SuperiorIronMan")
                && let Some(hook) = self.cards[card].default_ability().modify_final_cost
            {
                self.record_card_hook(self.cards[card].default_ability().id, "modify_final_cost");
                let next = hook(self, card, cost);
                if self.cost_change_allowed(card, cost, next) {
                    cost = next;
                }
            }
            if cards::reference_rules_enabled() {
                for effect in &self.cards[card].cost_effects {
                    if let CardCostEffectKind::TemporarySet { turn, value } = effect.kind
                        && turn == self.turn
                        && self.cost_change_allowed(card, cost, value)
                    {
                        cost = value;
                    }
                }
            }
            if cards::reference_rules_enabled() {
                for effect in &self.players[self.cards[card].owner].cost_effects {
                    if let PlayerCostEffectKind::BlackSwan {
                        until,
                        printed_cost,
                        value,
                    } = effect.kind
                        && self.turn <= until
                        && self.printed_cost(card) == printed_cost
                        && self.cost_change_allowed(card, cost, value)
                    {
                        cost = value;
                    }
                }
                if self.players[self.cards[card].owner].cost_effects.iter().any(|effect| {
                    matches!(effect.kind, PlayerCostEffectKind::Wave { until } if self.turn <= until)
                }) {
                    let capped = cost.min(4);
                    if self.cost_change_allowed(card, cost, capped) {
                        cost = capped;
                    }
                }
            }
        }
        cost.max(0)
    }

    /// QuakeBehavior::ShuffleLocations (0x7f1da74): move the other two
    /// location objects and their occupants, without rewriting slot history.
    pub fn swap_other_locations(&mut self, source: usize) {
        let a = (source + 1) % 3;
        let b = (source + 2) % 3;
        self.swap_location_objects(a, b);
        self.ongoing();
    }

    /// Replace and immediately reveal a location without moving its cards.
    pub fn replace_location(&mut self, lane: usize, id: &str) {
        let def = locations::get(id).expect("unregistered replacement location");
        self.replace_location_def(lane, def, None);
    }

    /// Replace with a given definition, which may be the approximate stand-in for a location
    /// without rules (then `observed_id` names the real one).
    pub fn replace_location_def(
        &mut self,
        lane: usize,
        def: &'static LocationDef,
        observed_id: Option<String>,
    ) {
        self.lanes[lane].def = def;
        self.lanes[lane].observed_id = observed_id;
        self.lanes[lane].state = def.initial_state.map(|init| init());
        self.lanes[lane].revealed = true;
        self.lanes[lane].copied_reveal_turn = None;
        self.lanes[lane].revealed_turn = Some(self.turn);
        self.delays_reveals |= self.location_abilities_active(lane) && def.delays_reveals;
        self.emit(Event::LocationRevealed { lane });
        if self.location_abilities_active(lane)
            && let Some(hook) = def.on_reveal
        {
            let _ = self.counted_trigger(|game| {
                game.record_location_hook(lane, "on_reveal");
                hook(game, lane)
            });
        }
        let minimum = self
            .lanes
            .iter()
            .enumerate()
            .filter(|(lane, _)| self.location_abilities_active(*lane))
            .map(|(_, location)| location.def.minimum_turns)
            .max()
            .unwrap_or(0);
        self.set_total_turns(6.max(minimum));
        if self.location_abilities_active(lane) {
            for seat in 0..2 {
                let energy = self.players[seat].energy + def.reveal_energy;
                self.set_energy(seat, energy);
            }
        }
        self.ongoing();
        self.emit(Event::LocationMorphed { lane });
    }

    pub fn has_space(&self, lane: usize, seat: usize, queued: usize) -> bool {
        self.lanes[lane]
            .cards
            .iter()
            .filter(|&&card| self.cards[card].owner == seat)
            .count()
            + queued
            < self.lane_slot_capacity(lane)
    }
    // SoulStoneBehavior::ProtectThanos (0x7f42b00) and
    // SpaceStoneBehavior::MakeThanosUnrestrictedPlay (0x7f442cc).
    fn thanos_stone_active(&self, card: usize, stone: &str) -> bool {
        let target = &self.cards[card];
        matches!(target.def.id, "Thanos" | "ThanosChampion")
            && self.lanes.iter().flat_map(|lane| &lane.cards).any(|&id| {
                let source = &self.cards[id];
                source.owner == target.owner
                    && source.revealed
                    && source.has_ongoing(false)
                    && source.default_ability().id == stone
            })
    }

    pub fn can_play(
        &self,
        card: usize,
        lane: usize,
        seat: usize,
        energy: i32,
        queued: usize,
    ) -> bool {
        self.can_play_with_count(card, lane, seat, energy, queued, queued)
    }

    pub fn can_play_with_count(
        &self,
        card: usize,
        lane: usize,
        seat: usize,
        energy: i32,
        queued: usize,
        queued_total: usize,
    ) -> bool {
        self.can_play_internal(card, lane, seat, energy, (queued, queued_total), false)
    }

    /// Jean Grey's candidate predicate uses Card::IsPlayableByCard (0x7cb0a28):
    /// card and location requirements, but not the player's available energy.
    pub(crate) fn can_play_for_jean(&self, card: usize, lane: usize, seat: usize) -> bool {
        self.can_play_internal(card, lane, seat, i32::MAX, (0, 0), true)
    }

    fn can_play_internal(
        &self,
        card: usize,
        lane: usize,
        seat: usize,
        energy: i32,
        counts: (usize, usize),
        skip_jean: bool,
    ) -> bool {
        let (queued, queued_total) = counts;
        if self.card_cost(card) > energy {
            return false;
        }
        // Card::IsPlayableByCard (0x7cb0a28) passes the ignore-restrictions
        // flag to Location::HasEmptyCardSlot (0x7ddb4c0); the latter compares
        // against the physical maximum of four rather than the location limit.
        let unrestricted = self.thanos_stone_active(card, "SpaceStone")
            || self.jeff_unrestricted(card)
            // WarMachineBehavior::UnrestrictPlay (0x7cee0f0) grants the owner's
            // hand cards IgnorePlayRestrictions while its ongoing is active.
            || (self.players[seat].hand.contains(&card)
                && self
                    .lanes
                    .iter()
                    .flat_map(|location| &location.cards)
                    .any(|&source| {
                        self.cards[source].owner == seat
                            && self.cards[source].revealed
                            && self.ongoing_active(source)
                            && self.cards[source].ongoing_guard_abilities().any(|ability| {
                                ability.supports(AbilityKind::Ongoing)
                                    && ability.def.grants_unrestricted_hand_plays
                            })
                    }));
        let has_space = if unrestricted && !cards::reference_rules_enabled() {
            !self.lanes[lane].destroyed
                && self.lanes[lane]
                    .cards
                    .iter()
                    .filter(|&&id| self.cards[id].owner == seat)
                    .count()
                    + queued
                    < 4
        } else {
            self.has_space(lane, seat, queued)
        };
        if !has_space {
            return false;
        }
        if unrestricted {
            return true;
        }
        if self.players[seat].hand.contains(&card)
            && self.players[seat].play_locks.contains(&(card, self.turn))
        {
            return false;
        }
        if self.lane_locked(lane)
            || (!self.cards[card].abilities_removed
                && self.cards[card]
                    .default_ability()
                    .can_play
                    .is_some_and(|test| {
                        self.record_card_hook(self.cards[card].default_ability().id, "can_play");
                        !test(self, card, lane, seat)
                    }))
        {
            return false;
        }
        if self
            .lanes
            .iter()
            .flat_map(|location| &location.cards)
            .any(|&source| {
                self.cards[source].revealed
                    && self.cards[source].ongoing_guard_abilities().any(|ability| {
                        ability.supports(AbilityKind::Ongoing)
                            && if ability.def.id == "JeanGrey" {
                                self.jean_ongoing_active(source)
                            } else {
                                self.ongoing_active(source)
                            }
                            && !(skip_jean && ability.def.id == "JeanGrey")
                            && ability.def.global_play_guard.is_some_and(|guard| {
                                self.record_card_hook(ability.def.id, "global_play_guard");
                                !guard(self, source, card, lane, seat, queued_total)
                            })
                    })
            })
        {
            return false;
        }
        if self.lanes[lane].cards.iter().any(|&source| {
            self.cards[source].revealed
                && self.ongoing_active(source)
                && self.cards[source].ongoing_guard_abilities().any(|ability| {
                    ability.supports(AbilityKind::Ongoing)
                        && ability.def.ongoing_play_guard.is_some_and(|guard| {
                            self.record_card_hook(ability.def.id, "ongoing_play_guard");
                            !guard(self, source, card, lane, seat)
                        })
                })
        }) {
            return false;
        }
        // GooseBehavior::PreventExpensiveCards (0x7eb7988).
        if self.card_cost(card) >= 4
            && self.lanes[lane].cards.iter().any(|&source| {
                self.cards[source].revealed
                    && self.ongoing_active(source)
                    && self.cards[source].ongoing_guard_abilities().any(|ability| {
                        ability.supports(AbilityKind::Ongoing) && ability.def.blocks_expensive_plays
                    })
            })
        {
            return false;
        }
        for (source, location) in self.lanes.iter().enumerate() {
            if !self.location_abilities_active(source) {
                continue;
            }
            // ParkerIndustriesBehavior (0x7d7177c) limits staged plays
            // globally, not just at the source location.
            if location
                .def
                .allows_play_globally_with_count
                .is_some_and(|test| {
                    self.record_location_hook(source, "allows_play_globally_with_count");
                    !test(self, source, seat, queued_total)
                })
            {
                return false;
            }
            let predicate = if source == lane {
                location.def.allows_play
            } else {
                location.def.allows_play_elsewhere
            };
            if let Some(test) = predicate
                && {
                    self.record_location_hook(
                        source,
                        if source == lane {
                            "allows_play"
                        } else {
                            "allows_play_elsewhere"
                        },
                    );
                    !test(self, source, card, lane, seat)
                }
            {
                return false;
            }
            if source == lane
                && let Some(test) = location.def.allows_play_with_count
                && {
                    self.record_location_hook(source, "allows_play_with_count");
                    !test(self, seat, queued_total)
                }
            {
                return false;
            }
            if source == lane
                && let Some(test) = location.def.allows_addition
                && {
                    self.record_location_hook(source, "allows_addition");
                    !test(self, source, card, lane, seat)
                }
            {
                return false;
            }
        }
        true
    }
    // Jeff's Init-priority ignore-restrictions ability is hidden and is
    // inactive while staged for play or movement (client defs.json).
    fn jeff_unrestricted(&self, card: usize) -> bool {
        let target = &self.cards[card];
        !target.abilities_removed
            && target.default_ability().id == "JeffTheBabyLandShark"
            && (cards::reference_rules_enabled()
                || target.lane.is_none_or(|lane| {
                    target.revealed
                        && !target.move_staged
                        && self.lanes[lane].ability_flags.hidden_ongoing
                }))
    }

    // Location::CanMoveCard passes the ignore-move stat to HasEmptyCardSlot
    // (0x7ddbb34); true compares against four physical slots.
    fn has_move_space(&self, card: usize, lane: usize) -> bool {
        let seat = self.cards[card].owner;
        if !cards::reference_rules_enabled() && self.jeff_unrestricted(card) {
            !self.lanes[lane].destroyed
                && self.lanes[lane]
                    .cards
                    .iter()
                    .filter(|&&id| self.cards[id].owner == seat)
                    .count()
                    < 4
        } else {
            self.has_space(lane, seat, 0)
        }
    }
    // ColossusBehavior::SetCantBeDestroyedOrMovedOrWeakened (0x7e834e8)
    // installs an ongoing movement restriction, which disappears when suppressed.
    fn immune_to_movement(&self, card: usize) -> bool {
        let target = &self.cards[card];
        (cards::reference_rules_enabled()
            || target
                .lane
                .is_none_or(|lane| self.lane_ongoing_allowed(lane)))
            && target.ongoing_guard_abilities().any(|ability| {
                ability.supports(AbilityKind::Ongoing)
                    && ability.def.immune_destroy_move
                    && (cards::reference_rules_enabled()
                        || ability.id.is_some()
                        || !target.abilities_removed)
            })
    }

    fn ongoing_blocks_move(&self, card: usize, source: usize) -> bool {
        !self.jeff_unrestricted(card)
            && self.lanes[source].cards.iter().copied().any(|provider| {
                self.cards[provider].revealed
                    && self.ongoing_active(provider)
                    && self.cards[provider]
                        .ongoing_guard_abilities()
                        .filter(|a| {
                            a.supports(AbilityKind::Ongoing)
                                && (a.kind != AbilityKind::Default
                                    || !self.cards[provider].abilities_removed)
                        })
                        .any(|a| {
                            a.def.blocks_player_move.is_some_and(|test| {
                                self.record_card_hook(a.def.id, "blocks_player_move");
                                test(self, provider, card)
                            })
                        })
            })
    }

    /// Install a movement permission without moving or revealing a card.
    pub fn grant_move(&mut self, mut grant: MoveGrant) {
        let source = &self.cards[grant.source];
        grant.owner_def_id = Some(
            source
                .ability_frames
                .last()
                .and_then(|frame| {
                    source
                        .composed_abilities
                        .iter()
                        .find(|ability| ability.id == frame.instance)
                })
                .map_or_else(|| source.default_ability().id, |ability| ability.def.id),
        );
        self.players[grant.seat].move_grants.push(grant);
    }

    fn matching_move_grant(
        &self,
        card: usize,
        seat: usize,
        destination: Option<usize>,
    ) -> Option<usize> {
        let lane = self.cards[card].lane?;
        self.players[seat].move_grants.iter().position(|grant| {
            (grant.unlimited || grant.remaining != 0)
                && grant.start_turn <= self.turn
                && self.turn <= grant.end_turn
                && grant.card.is_none_or(|target| target == card)
                && grant.lane.is_none_or(|target| target == lane)
                && grant.condition.is_none_or(|condition| {
                    self.record_card_hook(self.cards[grant.source].def.id, "move_grant_condition");
                    condition(self, grant.source, card)
                })
                && destination
                    .is_none_or(|lane| grant.destination.is_none_or(|target| target == lane))
        })
    }

    // FindMovableByPlayerSource (0x7c9d9ec): select the lowest priority,
    // then the earliest component. Only within-vector creation order is retained.
    fn select_move_source(
        &self,
        card: usize,
        destination: usize,
        seat: usize,
    ) -> (Option<MoveSource>, bool) {
        use crate::move_priority::{card_priority, location_priority};
        let target = &self.cards[card];
        let Some(source) = target.lane else {
            return (None, false);
        };
        let mut best: Option<MoveCandidate> = None;
        let mut ambiguous = false;
        let mut groups = 0u8;
        let mut consider = |candidate: MoveCandidate| {
            let group = 1u8 << candidate.order_group;
            match best {
                None => {
                    best = Some(candidate);
                    groups = group;
                }
                Some(previous) if candidate.priority < previous.priority => {
                    best = Some(candidate);
                    groups = group;
                    ambiguous = false;
                }
                Some(previous) if candidate.priority == previous.priority => {
                    if move_source_effects_conflict(candidate.impact, previous.impact)
                        && (!self.move_provenance_known
                            || candidate.order_group == 0
                            || groups != group)
                    {
                        ambiguous = true;
                    }
                    groups |= group;
                }
                _ => {}
            }
        };
        if !target.abilities_removed
            && self.lanes[source].ability_flags.hidden_ongoing
            && (target.player_moves_used < target.default_ability().move_limit
                || target.default_ability().move_each_turn
                    && target.player_move_turn != Some(self.turn))
        {
            consider(MoveCandidate {
                source: MoveSource::Intrinsic,
                priority: card_priority(target.default_ability().id),
                order_group: 0,
                impact: (1, u8::MAX, target.default_ability().move_limit, None),
            });
        }
        {
            for (index, ability) in target.composed_abilities.iter().enumerate() {
                if ability.kind == AbilityKind::Ongoing
                    && (if ability.hidden {
                        self.lanes[source].ability_flags.hidden_ongoing
                    } else {
                        self.lanes[source].ability_flags.ongoing
                    })
                    && (ability.player_moves_used < ability.def.move_limit
                        || ability.def.move_each_turn
                            && ability.player_move_turn != Some(self.turn))
                {
                    consider(MoveCandidate {
                        source: MoveSource::Attached(index),
                        priority: card_priority(ability.def.id),
                        order_group: 3,
                        impact: (4, u8::MAX, ability.def.move_limit, Some(index)),
                    });
                }
            }
        }
        if self.lanes[source].ability_flags.hidden_ongoing {
            for (index, grant) in target.external_moves.iter().enumerate() {
                if grant.start <= self.turn
                    && self.turn <= grant.end
                    && grant.remaining.is_none_or(|remaining| remaining > 0)
                {
                    consider(MoveCandidate {
                        source: MoveSource::External(index),
                        priority: card_priority(grant.owner_def_id),
                        order_group: 1,
                        impact: (
                            if grant.remaining.is_some() { 2 } else { 0 },
                            grant.end,
                            grant.remaining.unwrap_or(0),
                            grant.remaining.map(|_| index),
                        ),
                    });
                }
            }
        }
        for (index, grant) in self.players[seat].move_grants.iter().enumerate() {
            if (grant.unlimited || grant.remaining != 0)
                && grant.start_turn <= self.turn
                && self.turn <= grant.end_turn
                && grant.card.is_none_or(|target| target == card)
                && grant.lane.is_none_or(|lane| lane == source)
                && grant.destination.is_none_or(|lane| lane == destination)
                && grant
                    .condition
                    .is_none_or(|test| test(self, grant.source, card))
            {
                consider(MoveCandidate {
                    source: MoveSource::Player(index),
                    priority: card_priority(
                        grant
                            .owner_def_id
                            .unwrap_or(self.cards[grant.source].default_ability().id),
                    ),
                    order_group: 2,
                    impact: (
                        if grant.unlimited { 0 } else { 3 },
                        grant.end_turn,
                        grant.remaining,
                        if !grant.unlimited {
                            Some(index)
                        } else {
                            grant.on_move.map(|_| grant.source)
                        },
                    ),
                });
            }
        }
        for (lane, location) in self.lanes.iter().enumerate() {
            for &provider in &location.cards {
                if !self.cards[provider].revealed || !self.ongoing_active(provider) {
                    continue;
                }
                for ability in self.cards[provider]
                    .ongoing_guard_abilities()
                    .filter(|ability| {
                        ability.supports(AbilityKind::Ongoing)
                            && (ability.kind != AbilityKind::Default
                                || !self.cards[provider].abilities_removed)
                    })
                {
                    let applies = ability
                        .def
                        .grants_move_globally
                        .is_some_and(|test| test(self, provider, card))
                        || lane == source
                            && ability
                                .def
                                .grants_move_from
                                .is_some_and(|test| test(self, provider, card));
                    if applies {
                        consider(MoveCandidate {
                            source: MoveSource::Passive(ability.def.id),
                            priority: card_priority(ability.def.id),
                            order_group: 0,
                            impact: (0, u8::MAX, 0, None),
                        });
                    }
                }
            }
        }
        if self.location_abilities_active(destination)
            && self.lanes[destination]
                .def
                .grants_move_to
                .is_some_and(|test| test(self, destination, card, source, seat))
        {
            consider(MoveCandidate {
                source: MoveSource::Passive(self.lanes[destination].def.id),
                priority: location_priority(self.lanes[destination].def.id),
                order_group: 0,
                impact: (0, u8::MAX, 0, None),
            });
        }
        if self.location_abilities_active(source)
            && self.lanes[source]
                .def
                .grants_move_from
                .is_some_and(|test| test(self, source, card, destination, seat))
        {
            consider(MoveCandidate {
                source: MoveSource::Passive(self.lanes[source].def.id),
                priority: location_priority(self.lanes[source].def.id),
                order_group: 0,
                impact: (0, u8::MAX, 0, None),
            });
        }
        (best.map(|candidate| candidate.source), ambiguous)
    }

    pub(crate) fn move_source_tie_unknown(
        &self,
        card: usize,
        destination: usize,
        seat: usize,
    ) -> bool {
        self.select_move_source(card, destination, seat).1
    }

    /// Whether a player can voluntarily move this card to the destination now.
    pub fn can_player_move(&self, card: usize, destination: usize, seat: usize) -> bool {
        self.can_player_move_without_tie_check(card, destination, seat)
            && (cards::reference_rules_enabled()
                || !self.move_source_tie_unknown(card, destination, seat))
    }

    pub(crate) fn can_player_move_without_tie_check(
        &self,
        card: usize,
        destination: usize,
        seat: usize,
    ) -> bool {
        let target = &self.cards[card];
        let Some(source) = target.lane else {
            return false;
        };
        if target.owner != seat
            || !target.revealed
            || target.move_staged
            || self.immune_to_movement(card)
            || destination >= 3
            || source == destination
            || !self.has_move_space(card, destination)
        {
            return false;
        }
        if self.ongoing_blocks_move(card, source) {
            return false;
        }
        if !self.jeff_unrestricted(card)
            && self.location_abilities_active(source)
            && let Some(test) = self.lanes[source].def.allows_move_from
            && {
                self.record_location_hook(source, "allows_move_from");
                !test(self, source, card, destination, seat)
            }
        {
            return false;
        }
        if !self.jeff_unrestricted(card)
            && self.location_abilities_active(destination)
            && let Some(test) = self.lanes[destination].def.allows_addition
            && {
                self.record_location_hook(destination, "allows_addition");
                !test(self, destination, card, destination, seat)
            }
        {
            return false;
        }
        if let Some(mask) = target.observed_move_targets {
            return mask & (1 << destination) != 0;
        }
        // HasMovesRemainingCondition::Evaluate (0x7d26688): zero stacks are unlimited;
        // Card::IsMovableByPlayer (0x7cb0cac) rejects a card already staged to move.
        (!target.abilities_removed
            && (cards::reference_rules_enabled()
                || self.lanes[source].ability_flags.hidden_ongoing)
            && (target.player_moves_used < target.default_ability().move_limit
                || target.default_ability().move_each_turn
                    && target.player_move_turn != Some(self.turn)))
            || (!cards::reference_rules_enabled()
                && target.composed_abilities.iter().any(|ability| {
                    ability.kind == AbilityKind::Ongoing
                        && (if ability.hidden {
                            self.lanes[source].ability_flags.hidden_ongoing
                        } else {
                            self.lanes[source].ability_flags.ongoing
                        })
                        && (ability.player_moves_used < ability.def.move_limit
                            || ability.def.move_each_turn
                                && ability.player_move_turn != Some(self.turn))
                }))
            || ((if cards::reference_rules_enabled() {
                !self.lanes[source].revealed || !self.lanes[source].def.disables_ongoing
            } else {
                self.lanes[source].ability_flags.hidden_ongoing
            }) && target.external_moves.iter().any(|grant| {
                grant.start <= self.turn
                    && self.turn <= grant.end
                    && grant.remaining.is_none_or(|remaining| remaining > 0)
            }))
            || self
                .matching_move_grant(card, seat, Some(destination))
                .is_some()
            || self
                .lanes
                .iter()
                .flat_map(|lane| lane.cards.iter().copied())
                .any(|provider| {
                    self.cards[provider].revealed
                        && self.ongoing_active(provider)
                        && self.cards[provider]
                            .ongoing_guard_abilities()
                            .filter(|a| {
                                a.supports(AbilityKind::Ongoing)
                                    && (a.kind != AbilityKind::Default
                                        || !self.cards[provider].abilities_removed)
                            })
                            .any(|a| {
                                a.def.grants_move_globally.is_some_and(|test| {
                                    self.record_card_hook(a.def.id, "grants_move_globally");
                                    test(self, provider, card)
                                })
                            })
                })
            || self.lanes[source].cards.iter().copied().any(|provider| {
                self.cards[provider].revealed
                    && self.ongoing_active(provider)
                    && self.cards[provider]
                        .ongoing_guard_abilities()
                        .filter(|a| {
                            a.supports(AbilityKind::Ongoing)
                                && (a.kind != AbilityKind::Default
                                    || !self.cards[provider].abilities_removed)
                        })
                        .any(|a| {
                            a.def.grants_move_from.is_some_and(|test| {
                                self.record_card_hook(a.def.id, "grants_move_from");
                                test(self, provider, card)
                            })
                        })
            })
            || (self.location_abilities_active(destination)
                && self.lanes[destination]
                    .def
                    .grants_move_to
                    .is_some_and(|test| {
                        self.record_location_hook(destination, "grants_move_to");
                        test(self, destination, card, source, seat)
                    }))
            || (self.location_abilities_active(source)
                && self.lanes[source].def.grants_move_from.is_some_and(|test| {
                    self.record_location_hook(source, "grants_move_from");
                    test(self, source, card, destination, seat)
                }))
    }

    // Movable ability stacks are consumed by a player move while active.
    fn consume_external_move(&mut self, card: usize, source: usize) {
        let active = if cards::reference_rules_enabled() {
            !self.lanes[source].revealed || !self.lanes[source].def.disables_ongoing
        } else {
            self.lanes[source].ability_flags.hidden_ongoing
        };
        if active {
            for grant in &mut self.cards[card].external_moves {
                if grant.start <= self.turn
                    && self.turn <= grant.end
                    && let Some(remaining) = &mut grant.remaining
                {
                    *remaining = remaining.saturating_sub(1);
                }
            }
        }
    }

    // MoveSystem::ConsumeMovableSourceStack (0x7c9dcd0) consumes only
    // the ability identified by the staged movement component.
    fn consume_move_source(
        &mut self,
        card: usize,
        seat: usize,
        source: Option<MoveSource>,
    ) -> Option<MoveGrant> {
        match source {
            Some(MoveSource::Intrinsic) => {
                if self.cards[card].default_ability().move_limit > 0 {
                    self.cards[card].player_moves_used =
                        self.cards[card].player_moves_used.saturating_add(1);
                }
                if self.cards[card].default_ability().move_each_turn {
                    self.cards[card].player_move_turn = Some(self.turn);
                }
                None
            }
            Some(MoveSource::Attached(index)) => {
                let ability = &mut self.cards[card].composed_abilities[index];
                if ability.def.move_limit > 0 {
                    ability.player_moves_used = ability.player_moves_used.saturating_add(1);
                }
                if ability.def.move_each_turn {
                    ability.player_move_turn = Some(self.turn);
                }
                None
            }
            Some(MoveSource::External(index)) => {
                if let Some(remaining) = &mut self.cards[card].external_moves[index].remaining {
                    *remaining = remaining.saturating_sub(1);
                }
                None
            }
            Some(MoveSource::Player(index)) => {
                let grant = &mut self.players[seat].move_grants[index];
                if !grant.unlimited {
                    grant.remaining -= 1;
                }
                Some(grant.clone())
            }
            Some(MoveSource::Passive(_)) | None => None,
        }
    }

    /// Resolve one voluntary movement, consuming its intrinsic and granted stacks.
    pub fn move_player_card(&mut self, card: usize, destination: usize, seat: usize) -> bool {
        if !self.can_player_move(card, destination, seat) {
            return false;
        }
        let reference = cards::reference_rules_enabled();
        let (selected, ambiguous) = self.select_move_source(card, destination, seat);
        if !reference && ambiguous {
            return false;
        }
        let grant = reference
            .then(|| self.matching_move_grant(card, seat, Some(destination)))
            .flatten();
        let source = self.cards[card].lane.unwrap();
        if !self.move_card(card, destination) {
            return false;
        }
        self.players[seat].non_play_action_turn = Some(self.turn);
        self.players[seat].move_from_turn[source] = Some(self.turn);
        let grant = if reference {
            self.consume_external_move(card, source);
            let _ = self.consume_move_source(card, seat, Some(MoveSource::Intrinsic));
            self.consume_move_source(card, seat, grant.map(MoveSource::Player))
        } else {
            self.consume_move_source(card, seat, selected)
        };
        if let Some(grant) = grant {
            let source = grant.source;
            let callback = grant.on_move;
            if let Some(hook) = callback {
                let _ = self.counted_trigger(|game| {
                    game.record_card_hook(game.cards[source].def.id, "move_grant");
                    hook(game, source, card)
                });
            }
        }
        true
    }
    /// Location::CanMoveCard permits checking staged face-down cards as source.
    pub fn can_move_card(&self, card: usize, destination: usize) -> bool {
        let target = &self.cards[card];
        let Some(source) = target.lane else {
            return false;
        };
        if destination >= 3 || destination == source || self.lanes[destination].destroyed {
            return false;
        }
        // Location::CanMoveCard (0x7ddba50) accepts the latest staged
        // action's original location before checking capacity or move stats.
        if !cards::reference_rules_enabled()
            && self.is_wait_for_staging
            && matches!(
                self.staged_undo_option(card, target.owner),
                Some(crate::engine_ext::autoplay::Undo::Move { lane }) if lane == destination
            )
        {
            return true;
        }
        if (target.revealed && self.immune_to_movement(card))
            || !self.has_move_space(card, destination)
        {
            return false;
        }
        if self.ongoing_blocks_move(card, source) {
            return false;
        }
        if !self.jeff_unrestricted(card)
            && self.location_abilities_active(source)
            && let Some(test) = self.lanes[source].def.allows_move_from
            && {
                self.record_location_hook(source, "allows_move_from");
                !test(self, source, card, destination, target.owner)
            }
        {
            return false;
        }
        self.jeff_unrestricted(card)
            || !self.location_abilities_active(destination)
            || self.lanes[destination]
                .def
                .allows_addition
                .is_none_or(|test| {
                    self.record_location_hook(destination, "allows_addition");
                    test(self, destination, card, destination, target.owner)
                })
    }

    // Game::GetRandomLocationDefId (0x7db248c): one Game::Roll over the location
    // pool weighted by rarity (`LocationRarityDefList::.cctor` 0x7ac3c60 stores
    // weights 200/125/75/15 for rarity ids 1..4, read by `FindWeight` 0x7c67e94 and
    // `<.cctor>b__5_0` 0x7c68250), skipping the definition of `excluded`.
    // [INFERENCE] ids 1..4 are common, uncommon, rare, ultra rare. Cumulative
    // order follows `location_pool`; the client's own order (WeightedTable at
    // 0x60708f0, generic code) is not decoded, so only the distribution is exact.
    fn location_def_drawable(&self, id: &str, excluded: Option<&str>) -> bool {
        Some(id) != excluded && !self.starting_location_ids.contains(&id)
    }

    pub(crate) fn random_location_def_id(&self, excluded: Option<&str>) -> Option<&'static str> {
        static WEIGHTS: std::sync::OnceLock<std::collections::HashMap<String, u32>> =
            std::sync::OnceLock::new();
        let weights = WEIGHTS.get_or_init(|| {
            crate::data::locations()
                .into_iter()
                .map(|row| {
                    let weight = match row.rarity.as_str() {
                        "uncommon" => 125,
                        "rare" => 75,
                        "ultra rare" => 15,
                        _ => 200,
                    };
                    (row.carddefid, weight)
                })
                .collect()
        });
        let weight = |id: &str| weights.get(id).copied().unwrap_or(200) as usize;
        let total: usize = self
            .location_pool
            .iter()
            .filter(|def| self.location_def_drawable(def.id, excluded))
            .map(|def| weight(def.id))
            .sum();
        if total == 0 {
            return None;
        }
        let mut roll = self.rng.borrow_mut().randbelow(total);
        for def in self
            .location_pool
            .iter()
            .filter(|def| self.location_def_drawable(def.id, excluded))
        {
            let w = weight(def.id);
            if roll < w {
                if let Some(hooks) = &self.replay_hooks {
                    hooks
                        .borrow_mut()
                        .random_locations
                        .push((excluded.map(str::to_owned), def.id));
                }
                return Some(def.id);
            }
            roll -= w;
        }
        None
    }

    // Game::GetRandomLocationToMoveCard (0x7db2f50): try legal locations
    // other than the excluded location, then optionally all other locations.
    pub(crate) fn random_location_to_move_card(
        &self,
        card: usize,
        excluded: usize,
        fallback_to_any: bool,
    ) -> Option<usize> {
        let mut candidates = [0; 3];
        let mut count = 0;
        for lane in 0..self.lanes.len() {
            if lane != excluded && self.can_move_card(card, lane) {
                candidates[count] = lane;
                count += 1;
            }
        }
        if count == 0 && fallback_to_any {
            for lane in 0..self.lanes.len() {
                if lane != excluded {
                    candidates[count] = lane;
                    count += 1;
                }
            }
        }
        (count != 0).then(|| candidates[self.rng.borrow_mut().randbelow(count)])
    }

    // Game::GetRandomLocationToSpawnCard (0x7db2b64): one roll over the
    // locations other than `excluded` that can spawn `card`; with
    // `fallback_to_any` and no spawnable location, over all other locations.
    // The caller must still check that the card can be added at the result.
    pub(crate) fn random_location_to_spawn_card(
        &self,
        card: usize,
        seat: usize,
        excluded: Option<usize>,
        fallback_to_any: bool,
    ) -> Option<usize> {
        let mut candidates = [0; 3];
        let mut count = 0;
        for lane in 0..self.lanes.len() {
            if Some(lane) != excluded && self.can_add(card, lane, seat) {
                candidates[count] = lane;
                count += 1;
            }
        }
        if count == 0 && fallback_to_any {
            for lane in 0..self.lanes.len() {
                if Some(lane) != excluded {
                    candidates[count] = lane;
                    count += 1;
                }
            }
        }
        (count != 0).then(|| candidates[self.rng.borrow_mut().randbelow(count)])
    }

    fn activation_definition(&self, card: usize, seat: usize) -> Option<&'static CardDef> {
        let target = &self.cards[card];
        target
            .abilities()
            .filter(|ability| ability.supports(AbilityKind::Activate))
            .filter(|ability| ability.id.is_some() || !target.abilities_removed)
            // Card::HasActivateAbility (0x7cb36b8) passes includeHidden=false.
            .filter(|ability| cards::reference_rules_enabled() || !ability.hidden)
            .map(|ability| ability.def)
            .find(|def| {
                def.on_activate.is_some()
                    && (!cards::reference_rules_enabled()
                        || (self.players[seat].energy >= def.activate_cost
                            && def.can_activate.is_none_or(|test| {
                                self.record_card_hook(def.id, "can_activate");
                                test(self, card)
                            })))
            })
    }

    fn activation_cost(&self, card: usize, seat: usize) -> i32 {
        if cards::reference_rules_enabled() {
            self.activation_definition(card, seat)
                .unwrap()
                .activate_cost
        } else {
            // Card::CanActivate (0x7cb9a24) and StageForActivate
            // (0x7cb9678) read ActivateCost from the card definition.
            self.cards[card].def.activate_cost
        }
    }

    pub fn can_activate(&self, card: usize, seat: usize) -> bool {
        let target = &self.cards[card];
        target.owner == seat
            && target.lane.is_some()
            && target.revealed
            && (!cards::reference_rules_enabled() || self.card_triggers_allowed(card))
            && !target.activated
            && !target.activate_staged
            && target.observed_can_activate.unwrap_or(true)
            && target.lane.is_some_and(|lane| {
                if cards::reference_rules_enabled() {
                    !self.lanes[lane].revealed || !self.lanes[lane].def.blocks_activate
                } else {
                    self.lanes[lane].ability_flags.activate
                }
            })
            && self.activation_definition(card, seat).is_some()
            && self.players[seat].energy >= self.activation_cost(card, seat)
    }

    /// Resolve an activation and dispatch its post-activation trigger.
    pub fn activate_card(&mut self, card: usize, seat: usize) -> bool {
        if !self.can_activate(card, seat) {
            return false;
        }
        let cost = self.activation_cost(card, seat);
        self.set_energy(seat, self.players[seat].energy - cost);
        self.players[seat].energy_spent_this_turn += cost;
        self.cards[card].activated = true;
        self.players[seat].non_play_action_turn = Some(self.turn);
        self.ongoing();
        if self.card_triggers_allowed(card) {
            self.dispatch_card_activation(card);
        }
        self.emit(Event::CardActivated { card });
        true
    }
    /// Stage a play, move or activation, preserving the player's action order.
    pub fn stage_action(&mut self, seat: usize, action: Action) -> Result<(), String> {
        self.last_move_source = None;
        let cost = match action {
            Action::Play { card, lane } => {
                if !self.players[seat].hand.contains(&card)
                    || !self.can_play(card, lane, seat, self.players[seat].energy, 0)
                {
                    return Err("illegal staged play".into());
                }
                self.card_cost(card)
            }
            Action::Move { card, lane } => {
                if !self.can_player_move(card, lane, seat) {
                    if !cards::reference_rules_enabled()
                        && self.can_player_move_without_tie_check(card, lane, seat)
                        && self.move_source_tie_unknown(card, lane, seat)
                    {
                        return Err("move_source_tie_unknown".into());
                    }
                    return Err("illegal staged move".into());
                }
                0
            }
            Action::Activate { card } => {
                if !self.can_activate(card, seat) {
                    return Err("illegal staged activation".into());
                }
                self.activation_cost(card, seat)
            }
            Action::Return { card } => {
                if !self.staged_returnable(card, seat) {
                    return Err("illegal staged return".into());
                }
                0
            }
        };
        self.apply_staged_action(seat, action, cost);
        Ok(())
    }

    /// Card::IsStageUndoable (0x7cb0e74): only the latest staged action can be undone.
    pub(crate) fn staged_returnable(&self, card: usize, seat: usize) -> bool {
        self.players[seat].hand.len() < 7
            && self.players[seat].staged_actions.last().is_some_and(|staged| {
                matches!(staged.action, Action::Play { card: staged_card, .. } if staged_card == card)
            })
            && self.cards[card].lane.is_some()
    }

    pub(crate) fn staged_undo_option(
        &self,
        card: usize,
        seat: usize,
    ) -> Option<crate::engine_ext::autoplay::Undo> {
        use crate::engine_ext::autoplay::Undo;
        match self.players[seat].staged_actions.last()?.action {
            Action::Play { card: id, .. } if id == card => Some(Undo::Play),
            Action::Move { card: id, .. } if id == card => {
                self.cards[card].last_lane.map(|lane| Undo::Move { lane })
            }
            Action::Activate { card: id } if id == card => Some(Undo::Activate),
            _ => None,
        }
    }

    pub(crate) fn apply_staged_action(&mut self, seat: usize, action: Action, cost: i32) {
        match action {
            Action::Play { card, lane } => {
                let slot = self.players[seat]
                    .hand
                    .iter()
                    .position(|&id| id == card)
                    .unwrap();
                self.emit(Event::CardStaged { card, seat });
                self.take_hand(seat, slot);
                self.set_energy(seat, self.players[seat].energy - cost);
                self.players[seat].energy_spent_this_turn += cost;
                self.cards[card].lane = Some(lane);
                self.cards[card].turn_played = self.turn;
                let printed_cost = self.printed_cost(card);
                self.players[seat].staged_history.push(StagedPlay {
                    turn: self.turn,
                    card,
                    def_id: self.cards[card].def.id,
                    printed_cost,
                    created: self.cards[card].created,
                    resolved: false,
                });
                self.players[seat]
                    .staged_lane_history
                    .push((self.turn, lane));
                self.played[seat].push((card, lane));
                self.push_location(lane, card);
                self.notify_added(card, lane);
                self.players[seat].staged_actions.push(StagedAction {
                    action,
                    cost,
                    hand_index: Some(slot),
                });
                if !cards::reference_rules_enabled() {
                    self.ongoing();
                }
            }
            Action::Move { card, lane } => {
                let reference = cards::reference_rules_enabled();
                let (selected, _) = self.select_move_source(card, lane, seat);
                self.last_move_source = selected.map(|source| match source {
                    MoveSource::Intrinsic => self.cards[card].default_ability().id,
                    MoveSource::Attached(index) => {
                        self.cards[card].composed_abilities[index].def.id
                    }
                    MoveSource::External(index) => {
                        self.cards[card].external_moves[index].owner_def_id
                    }
                    MoveSource::Player(index) => self.players[seat].move_grants[index]
                        .owner_def_id
                        .unwrap_or(
                            self.cards[self.players[seat].move_grants[index].source]
                                .default_ability()
                                .id,
                        ),
                    MoveSource::Passive(id) => id,
                });
                let old_grant = reference
                    .then(|| self.matching_move_grant(card, seat, Some(lane)))
                    .flatten();
                self.players[seat].non_play_action_turn = Some(self.turn);
                let from = self.cards[card].lane.unwrap();
                self.players[seat].move_from_turn[from] = Some(self.turn);
                let grant = if reference {
                    self.consume_external_move(card, from);
                    let _ = self.consume_move_source(card, seat, Some(MoveSource::Intrinsic));
                    self.consume_move_source(card, seat, old_grant.map(MoveSource::Player))
                } else {
                    self.consume_move_source(card, seat, selected)
                };
                if let Some(grant) = grant
                    && let Some(hook) = grant.on_move
                {
                    self.cards[card].attach_ability(AttachedAbility {
                        id: "WebSlingThisCardMovedTrigger",
                        owner_def: "WebSling",
                        origin: Some(card),
                        index: 0,
                        kind: AttachedAbilityKind::MovePower {
                            source: grant.source,
                            turn: self.turn,
                            hook,
                        },
                    });
                }
                self.cards[card].move_staged = true;
                self.cards[card].last_lane = Some(from);
                self.detach_location(from, card);
                self.push_location(lane, card);
                self.cards[card].lane = Some(lane);
                self.players[seat].staged_actions.push(StagedAction {
                    action,
                    cost,
                    hand_index: None,
                });
                self.ongoing();
            }
            Action::Activate { card } => {
                self.players[seat].non_play_action_turn = Some(self.turn);
                self.set_energy(seat, self.players[seat].energy - cost);
                self.players[seat].energy_spent_this_turn += cost;
                self.cards[card].activate_staged = true;
                self.players[seat].staged_actions.push(StagedAction {
                    action,
                    cost,
                    hand_index: None,
                });
                self.ongoing();
            }
            Action::Return { card } => {
                // Location::StageForReturn (0x7dde240) removes the original
                // stage and refunds its recorded energy, not the card's current cost.
                let staged = self.players[seat].staged_actions.pop().unwrap();
                let Action::Play { lane, .. } = staged.action else {
                    unreachable!()
                };
                self.detach_location(lane, card);
                self.cards[card].lane = None;
                self.cards[card].turn_played = 0;
                self.put_hand(seat, staged.hand_index.unwrap(), card);
                self.set_energy(seat, self.players[seat].energy + staged.cost);
                self.players[seat].energy_spent_this_turn -= staged.cost;
                if let Some(index) = self.played[seat].iter().rposition(|&(id, _)| id == card) {
                    self.played[seat].remove(index);
                }
                if let Some(index) = self.players[seat]
                    .staged_history
                    .iter()
                    .rposition(|play| play.card == card && play.turn == self.turn && !play.resolved)
                {
                    self.players[seat].staged_history.remove(index);
                }
                if let Some(index) = self.players[seat]
                    .staged_lane_history
                    .iter()
                    .rposition(|&entry| entry == (self.turn, lane))
                {
                    self.players[seat].staged_lane_history.remove(index);
                }
                self.ongoing();
            }
        }
    }

    fn location_staging_end(&mut self) {
        // JosiesBarBehavior::BuffUnrevealedCards (0x7d5b898) is registered
        // on EndOfStagingTrigger before any staged cards resolve.
        for lane in 0..self.lanes.len() {
            if self.location_abilities_active(lane)
                && let Some(hook) = self.lanes[lane].def.on_staging_end
            {
                let _ = self.counted_trigger(|game| {
                    game.record_location_hook(lane, "on_staging_end");
                    hook(game, lane)
                });
            }
        }
    }

    /// Resolve each player's staged actions in priority order and original stage order.
    pub fn resolve_staged_actions(&mut self, priority: usize) {
        self.is_wait_for_staging = false;
        self.location_staging_end();
        let pending = std::mem::take(&mut self.delayed_reveals);
        for seat in [priority, 1 - priority] {
            for queued in &pending[seat] {
                self.reveal_with_staging(queued.card, queued.staging);
            }
            let actions = std::mem::take(&mut self.players[seat].staged_actions);
            for staged in actions {
                match staged.action {
                    Action::Play { card, lane } => {
                        self.reveal_with_staging(card, Some(StagingContext { seat, lane }));
                    }
                    Action::Move { card, .. } => {
                        self.cards[card].move_staged = false;
                        // Card::CanResolveForMove (0x7cb9cbc) checks that both
                        // previous and current zones are locations.
                        if let (Some(from), Some(lane)) =
                            (self.cards[card].last_lane, self.cards[card].lane)
                        {
                            if !crate::cards::reference_rules_enabled() {
                                self.emit(Event::BeforeCardMoved {
                                    card,
                                    from,
                                    to: lane,
                                });
                            }
                            if self.cards[card].lane == Some(lane) {
                                self.finish_move(card, from, lane);
                            }
                        }
                    }
                    Action::Activate { card } => {
                        self.cards[card].activate_staged = false;
                        // Card::CanResolveForActivate (0x7cb9d4c) does not
                        // re-check energy or the original ability restrictions.
                        if self.cards[card].lane.is_some()
                            && self.cards[card].revealed
                            && !self.cards[card].activated
                        {
                            self.cards[card].activated = true;
                            self.ongoing();
                            if self.card_triggers_allowed(card) {
                                self.dispatch_card_activation(card);
                            }
                            self.emit(Event::CardActivated { card });
                        }
                    }
                    Action::Return { .. } => unreachable!("returns undo a staged action"),
                }
            }
        }
    }
    pub fn move_card(&mut self, card: usize, destination: usize) -> bool {
        if !self.cards[card].revealed {
            return false;
        }
        self.move_card_including_staged(card, destination)
    }

    /// JuggernautBehavior::MoveOpposingCardsPlayedThisTurn 0x7ed6d34
    /// can move unrevealed staged cards as well as revealed cards.
    pub fn move_card_including_staged(&mut self, card: usize, destination: usize) -> bool {
        let Some(source) = self.cards[card].lane else {
            return false;
        };
        if (self.cards[card].revealed && self.immune_to_movement(card))
            || destination >= 3
            || source == destination
            || !self.has_move_space(card, destination)
        {
            return false;
        }
        if self.ongoing_blocks_move(card, source) {
            return false;
        }
        if !self.jeff_unrestricted(card)
            && self.location_abilities_active(source)
            && let Some(test) = self.lanes[source].def.allows_move_from
            && {
                self.record_location_hook(source, "allows_move_from");
                !test(self, source, card, destination, self.cards[card].owner)
            }
        {
            return false;
        }
        if !self.jeff_unrestricted(card)
            && self.location_abilities_active(destination)
            && let Some(test) = self.lanes[destination].def.allows_addition
            && {
                self.record_location_hook(destination, "allows_addition");
                !test(self, destination, card, destination, self.cards[card].owner)
            }
        {
            return false;
        }
        if !crate::cards::reference_rules_enabled() {
            self.emit(Event::BeforeCardMoved {
                card,
                from: source,
                to: destination,
            });
        }
        if self.cards[card].lane != Some(source) {
            return false;
        }
        self.detach_location(source, card);
        self.cards[card].last_lane = Some(source);
        self.push_location(destination, card);
        self.cards[card].lane = Some(destination);
        self.finish_move(card, source, destination);
        true
    }

    fn finish_move(&mut self, card: usize, source: usize, destination: usize) {
        // Game::UpdatePlayerMoveStats (0x7db05a0) counts all location transfers.
        self.players[self.cards[card].owner].cards_moved_this_game += 1;
        self.players[self.cards[card].owner].last_moved_card = Some(card);
        self.cards[card].move_turns.push(self.turn);
        if self.cards[card].revealed
            && self.card_triggers_allowed(card)
            && let Some(hook) = self.cards[card].default_ability().on_move
        {
            let _ = self.counted_trigger(|game| {
                game.record_card_hook(game.cards[card].default_ability().id, "on_move");
                hook(game, card, source)
            });
        }
        let abilities = std::mem::take(&mut self.cards[card].attached_abilities);
        for ability in abilities {
            if !self.cards[card].abilities_removed
                && self.card_triggers_allowed(card)
                && let AttachedAbilityKind::MovePower { source, turn, hook } = ability.kind
                && turn == self.turn
            {
                let _ = self.counted_trigger(|game| {
                    game.record_card_hook(game.cards[source].def.id, "move_power");
                    hook(game, source, card)
                });
            } else {
                self.cards[card].attached_abilities.push(ability);
            }
        }
        let observers = self.lanes[destination].cards.clone();
        for observer in observers {
            if observer != card
                && self.cards[observer].revealed
                && self.card_triggers_allowed(observer)
                && let Some(hook) = self.cards[observer].default_ability().on_move_here
            {
                let _ = self.counted_trigger(|game| {
                    game.record_card_hook(
                        game.cards[observer].default_ability().id,
                        "on_move_here",
                    );
                    hook(game, observer, card)
                });
            }
        }
        if self.location_abilities_active(destination)
            && let Some(hook) = self.lanes[destination].def.on_card_moved
        {
            let _ = self.counted_trigger(|game| {
                game.record_location_hook(destination, "on_card_moved");
                hook(game, destination, card, source)
            });
        }
        self.emit(Event::CardMoved {
            card,
            from: source,
            to: destination,
        });
        self.ongoing();
    }
    pub(crate) fn notify_added(&mut self, card: usize, lane: usize) {
        if self.card_triggers_allowed(card)
            && let Some(hook) = self.cards[card].default_ability().on_added
        {
            let _ = self.counted_trigger(|game| {
                game.record_card_hook(game.cards[card].default_ability().id, "on_added");
                hook(game, card)
            });
        }
        if self.location_abilities_active(lane)
            && let Some(hook) = self.lanes[lane].def.on_card_added
        {
            let _ = self.counted_trigger(|game| {
                game.record_location_hook(lane, "on_card_added");
                hook(game, lane, card)
            });
        }
        self.emit(Event::CardAdded { card, lane });
    }
    /// StardustBehavior::Restrict (0x7cc357c) disables spawning at all locations.
    pub fn spawns_allowed(&self) -> bool {
        !*HAS_SPAWN_BLOCKERS
            || !self.lanes.iter().flat_map(|lane| &lane.cards).any(|&id| {
                self.cards[id].revealed
                    && self.ongoing_active(id)
                    && self.cards[id]
                        .ongoing_guard_abilities()
                        .any(|a| a.supports(AbilityKind::Ongoing) && a.def.blocks_spawns)
            })
    }
    pub fn can_add(&self, card: usize, lane: usize, seat: usize) -> bool {
        self.cards[card].lane.is_none()
            && self.has_space(lane, seat, 0)
            && self.spawns_allowed()
            && !self.lane_locked(lane)
            && (!self.location_abilities_active(lane)
                || self.lanes[lane].def.allows_addition.is_none_or(|test| {
                    self.record_location_hook(lane, "allows_addition");
                    test(self, lane, card, lane, seat)
                }))
    }

    pub fn add_card(
        &mut self,
        card: usize,
        lane: usize,
        seat: usize,
        trigger_reveal: bool,
    ) -> bool {
        if !self.can_add(card, lane, seat) {
            return false;
        }
        self.finish_creation_zone(card, false, seat);
        self.detach_graveyards(card);
        self.cards[card].owner = seat;
        self.cards[card].lane = Some(lane);
        self.cards[card].turn_played = 0;
        self.cards[card].revealed = false;
        self.push_location(lane, card);
        self.notify_added(card, lane);
        if trigger_reveal {
            self.reveal(card);
        } else {
            self.cards[card].revealed = true;
            self.cards[card].turn_revealed = Some(self.turn);
            self.ongoing();
        }
        true
    }

    /// Stage an added card for a caller-managed batch of post-ability reveals.
    pub fn add_card_deferred(&mut self, card: usize, lane: usize, seat: usize) -> bool {
        if !self.can_add(card, lane, seat) {
            return false;
        }
        self.finish_creation_zone(card, false, seat);
        self.detach_graveyards(card);
        self.cards[card].owner = seat;
        self.cards[card].lane = Some(lane);
        self.cards[card].turn_played = 0;
        self.cards[card].revealed = false;
        self.push_location(lane, card);
        self.notify_added(card, lane);
        self.deferred_added_reveals.push(card);
        true
    }

    /// Resolve the current batch in insertion order; nested batches drain independently.
    pub fn resolve_deferred_cards(&mut self) {
        let cards = std::mem::take(&mut self.deferred_added_reveals);
        for card in cards {
            self.reveal(card);
        }
    }

    pub fn add_created(&mut self, id: &str, lane: usize, seat: usize) -> bool {
        let card = self.create_card(id, seat);
        if !self.add_card(card, lane, seat, true) {
            self.cards.pop();
            return false;
        }
        true
    }

    /// GrowZombieHorde (0x7e0f96c) buffs every owned Horde already in play.
    pub fn horde(&mut self, origin: usize, seat: usize, power: i32) -> Option<usize> {
        let hordes: Vec<_> = self
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&card| {
                self.cards[card].owner == seat && self.cards[card].def.id == "ZombieHorde"
            })
            .collect();
        if let Some(&first) = hordes.first() {
            for card in hordes {
                self.add_power(card, power, seat);
            }
            if power != 0 {
                self.ongoing();
            }
            return Some(first);
        }
        let card = self.create_card("ZombieHorde", seat);
        // LocationCreateNewZombie (0x7e0feac) and CardCreateNewZombie (0x7e0fd58) call
        // Game::GetRandomLocationToSpawnCard (0x7db2b64) excluding the source location with
        // fallbackToAnyLocation = true: with no location that can take the Horde they still
        // roll over every other location and the creation fails there. The reference chose
        // only among spawnable locations and never rolled when there were none.
        let target = if cards::reference_rules_enabled() {
            let targets: Vec<_> = (0..3)
                .filter(|&lane| lane != origin && self.can_add(card, lane, seat))
                .collect();
            (!targets.is_empty()).then(|| targets[self.rng.borrow_mut().randbelow(targets.len())])
        } else {
            self.random_location_to_spawn_card(card, seat, Some(origin), true)
        };
        // A new Horde is placed first (RandomCreateNewZombie and its siblings) and then grows
        // through AddPowerPermanently, so its growth triggers in play (e.g. Venus).
        if !target.is_some_and(|lane| self.add_card(card, lane, seat, true)) {
            self.cards.pop();
            return None;
        }
        self.add_power(card, power, seat);
        if power != 0 {
            self.ongoing();
        }
        Some(card)
    }
    pub fn add_hand(&mut self, seat: usize, card: usize, from_deck: bool) -> bool {
        if self.players[seat].hand.len() >= 7 {
            return false;
        }
        self.finish_creation_zone(card, !from_deck, seat);
        self.cards[card].owner = seat;
        self.push_hand(seat, card);
        if from_deck {
            self.cards[card].drawn_turn = Some(self.turn);
            self.cards[card].draw_extends_quickdraw = false;
            self.players[seat].draws.push((self.turn, card));
        }
        if !from_deck {
            self.notify_hand_added(seat);
        }
        self.ongoing();
        if from_deck {
            self.emit(Event::CardDrawn { card, seat });
            if !cards::reference_rules_enabled() {
                self.ongoing();
            }
        }
        true
    }
    /// Insert a created card now, but postpone its PostCardCreated observers.
    pub fn add_hand_deferred(&mut self, seat: usize, card: usize) -> bool {
        if self.players[seat].hand.len() >= 7 {
            return false;
        }
        self.finish_creation_zone(card, true, seat);
        self.cards[card].owner = seat;
        self.push_hand(seat, card);
        self.ongoing();
        true
    }

    pub fn notify_hand_added(&mut self, seat: usize) {
        let observers: Vec<_> = self
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| self.cards[id].revealed && self.cards[id].owner == seat)
            .collect();
        for observer in observers {
            if self.card_triggers_allowed(observer)
                && let Some(hook) = self.cards[observer].default_ability().on_hand_added
            {
                let _ = self.counted_trigger(|game| {
                    game.record_card_hook(
                        game.cards[observer].default_ability().id,
                        "on_hand_added",
                    );
                    hook(game, observer, seat)
                });
            }
        }
    }

    pub fn swap_hands(&mut self) {
        let [first, second] = &mut self.players;
        std::mem::swap(&mut first.hand, &mut second.hand);
        for seat in 0..2 {
            for &card in &self.players[seat].hand {
                self.cards[card].owner = seat;
            }
            let count = self.players[seat].hand.len();
            let observers: Vec<_> = self
                .lanes
                .iter()
                .flat_map(|lane| lane.cards.iter().copied())
                .filter(|&card| self.cards[card].revealed && self.cards[card].owner == seat)
                .collect();
            for observer in observers {
                if self.card_triggers_allowed(observer)
                    && let Some(hook) = self.cards[observer].default_ability().on_hand_added
                {
                    for _ in 0..count {
                        let _ = self.counted_trigger(|game| {
                            game.record_card_hook(
                                game.cards[observer].default_ability().id,
                                "on_hand_added",
                            );
                            hook(game, observer, seat)
                        });
                    }
                }
            }
            self.ongoing();
        }
    }

    pub fn take_from_board(&mut self, card: usize) -> bool {
        let Some(lane) = self.cards[card].lane else {
            return false;
        };
        if self.lane_locked(lane) {
            return false;
        }
        self.detach_location(lane, card);
        self.cards[card].last_lane = Some(lane);
        self.cards[card].lane = None;
        self.cards[card].revealed = false;
        self.cards[card].turn_played = 0;
        self.cards[card].ongoing = 0;
        self.cards[card].location_power = 0;
        self.cards[card]
            .attached_abilities
            .retain(|a| !matches!(a.kind, AttachedAbilityKind::Watch { .. }));
        if let Some(clear) = self.cards[card].default_ability().clear_reveal_state {
            clear(&mut self.cards[card]);
        }
        self.ongoing();
        true
    }
    /// FalconBehavior::ReturnOneCostCards (0x7ea603c) moves a board card
    /// back to its owner's hand, retaining its permanent modifications.
    pub fn return_board_card_to_hand(&mut self, card: usize) -> bool {
        let seat = self.cards[card].owner;
        self.return_board_card_to_seat_hand(card, seat)
    }
    /// Luke's Bar may return a card to its staging player's hand.
    pub fn return_board_card_to_seat_hand(&mut self, card: usize, seat: usize) -> bool {
        if self.players[seat].hand.len() >= 7 || !self.take_from_board(card) {
            return false;
        }
        if self.add_hand(seat, card, false) {
            self.emit(Event::CardReturnedToHand { card });
            true
        } else {
            false
        }
    }

    /// Return whether the discard succeeded, including cards whose discard
    /// trigger subsequently returns them to hand. Game::DiscardCard 0x7dabe68
    /// checks the card's zone is Hand before the Soul Stone check and returns
    /// false otherwise, e.g. for a card an earlier discard trigger already moved.
    pub fn discard_card(&mut self, seat: usize, card: usize) -> bool {
        let Some(slot) = self.players[seat].hand.iter().position(|&id| id == card) else {
            return false;
        };
        if self.thanos_stone_active(card, "SoulStone") {
            return false;
        }
        self.take_hand(seat, slot);
        self.transition_card_zone(card, crate::zones::CardZone::Discard(seat));
        // MorbiusBehavior::GetPowerChange 0x7f01d3c and
        // ColonelAmericaBehavior::UpdateDiscardTurnCount 0x7e82934 use events,
        // not the mutable discard zone. Record before the card's discard hook.
        self.players[seat].discard_history.push(CardZoneEvent {
            turn: self.turn,
            card,
            power: self.cards[card].power(),
            lane: None,
        });
        self.players[seat].graveyard_order.push(card);
        if let Some(hook) = self.cards[card].default_ability().on_discard {
            let _ = self.counted_trigger(|game| {
                game.record_card_hook(game.cards[card].default_ability().id, "on_discard");
                hook(game, card)
            });
        } else {
            self.push_discard(seat, card);
        }
        if !self.players[seat].discard.contains(&card)
            && !self.players[seat]
                .graveyard
                .iter()
                .any(|&(id, _)| id == card)
        {
            self.players[seat].graveyard_order.retain(|&id| id != card);
        }
        self.emit(Event::CardDiscarded { card, seat });
        self.ongoing();
        true
    }
    /// Game::BanishCard (0x7dac084) moves the card to its owner's banished zone.
    pub fn banish_card(&mut self, card: usize) -> bool {
        let owner = self.cards[card].owner;
        if self.players[owner].banished.contains(&card) {
            return false;
        }
        if let Some(lane) = self.cards[card].lane.take() {
            self.detach_location(lane, card);
            self.cards[card].last_lane = Some(lane);
        }
        self.detach_player_zones(card);
        self.push_banished(owner, card);
        self.ongoing();
        for seat in 0..2 {
            self.trigger_player_effects(seat, PlayerTrigger::CardBanished, Some(card));
        }
        true
    }

    /// ResurrectCardAndMultiplyPowerPermanently resets transient state, applies
    /// the multiplier, then moves the card from its graveyard to the location.
    pub fn revive_card(
        &mut self,
        card: usize,
        lane: usize,
        multiplier: i32,
        source: usize,
    ) -> bool {
        let owner = self.cards[card].owner;
        if !self.players[owner]
            .graveyard
            .iter()
            .any(|&(id, _)| id == card)
            || !self.can_add(card, lane, owner)
        {
            return false;
        }
        self.cards[card].state = self.cards[card]
            .default_ability()
            .initial_state
            .map(|init| init());
        self.cards[card]
            .attached_abilities
            .retain(|a| !matches!(a.kind, AttachedAbilityKind::Watch { .. }));
        self.cards[card].turn_played = 0;
        self.cards[card].ongoing = 0;
        self.cards[card].location_power = 0;
        if multiplier != 1 {
            self.multiply_power(card, multiplier, source);
        }
        if !self.add_card(card, lane, owner, true) {
            return false;
        }
        self.emit(Event::CardResurrected { card, lane });
        true
    }

    pub fn destroy_card(&mut self, card: usize) -> bool {
        self.destroy_card_impl(card, false)
    }

    pub(crate) fn force_destroy_card(&mut self, card: usize) -> bool {
        self.destroy_card_impl(card, true)
    }

    fn destroy_card_impl(&mut self, card: usize, force: bool) -> bool {
        let Some(lane) = self.cards[card].lane else {
            return false;
        };
        if !force && !self.can_destroy_card(card, lane) {
            return false;
        }
        if force {
            let owner = self.cards[card].owner;
            if let Some(play) = self.players[owner]
                .staged_history
                .iter_mut()
                .rev()
                .find(|play| play.card == card)
            {
                play.resolved = true;
            }
            self.cards[card].revealed = true;
        }
        self.finish_destroy_card(card, lane)
    }

    fn can_destroy_card(&self, card: usize, lane: usize) -> bool {
        if self.cards[card].revealed && self.thanos_stone_active(card, "SoulStone") {
            return false;
        }
        if self.cards[card].revealed
            && self.cards[card]
                .ongoing_guard_abilities()
                .any(|a| a.supports(AbilityKind::Ongoing) && a.def.immune_destroy_move)
        {
            return false;
        }
        if self.lane_locked(lane)
            || (self.location_abilities_active(lane) && self.lanes[lane].def.no_destroy)
        {
            return false;
        }
        if self.lanes[lane].cards.iter().any(|&id| {
            self.cards[id].revealed
                && self.ongoing_active(id)
                && self.cards[id]
                    .ongoing_guard_abilities()
                    .any(|a| a.supports(AbilityKind::Ongoing) && a.def.prevents_destroy)
        }) {
            return false;
        }
        // CaieraBehavior::ProtectCardsByCost (0x7e79584).
        if self
            .lanes
            .iter()
            .flat_map(|location| &location.cards)
            .any(|&source| {
                self.cards[source].revealed
                    && self.ongoing_active(source)
                    && self.cards[source]
                        .ongoing_guard_abilities()
                        .filter(|a| a.supports(AbilityKind::Ongoing))
                        .any(|a| {
                            a.def.destroy_guard.is_some_and(|guard| {
                                self.record_card_hook(a.def.id, "destroy_guard");
                                !guard(self, source, card)
                            })
                        })
            })
        {
            return false;
        }
        true
    }

    fn finish_destroy_card(&mut self, card: usize, lane: usize) -> bool {
        // ArchangelHorsemanBehavior::GetValidCount 0x7e697a0 and
        // DeathBehavior::DiscountSelf 0x7e89764 count destruction events.
        let owner = self.cards[card].owner;
        self.players[owner].destruction_history.push(CardZoneEvent {
            turn: self.turn,
            card,
            power: self.cards[card].power(),
            lane: Some(lane),
        });
        self.detach_location(lane, card);
        self.cards[card].last_lane = Some(lane);
        self.push_graveyard(owner, card, Some(lane));
        self.players[owner].graveyard_order.push(card);
        if let Some(destroyed) = &mut self.destroyed {
            destroyed.push((card, lane));
        }
        self.cards[card].lane = None;
        if self.cards[card].revealed
            && !self.cards[card].abilities_removed
            && let Some(hook) = self.cards[card].default_ability().on_destroy
        {
            let _ = self.counted_trigger(|game| {
                game.record_card_hook(game.cards[card].default_ability().id, "on_destroy");
                hook(game, card)
            });
        }
        self.emit(Event::CardDestroyed { card, lane });
        if !self.lanes[lane].destroyed {
            cards::angel_replace(self, self.cards[card].owner, lane);
        }
        self.ongoing();
        true
    }
    /// The deck slot `draw` takes next: a card due this turn, else the first eligible one.
    pub(crate) fn next_draw_slot(&self, seat: usize) -> Option<usize> {
        let deck = &self.players[seat].deck;
        deck.iter()
            .position(|&id| self.cards[id].def.draw_turn == Some(self.turn))
            .or_else(|| {
                deck.iter().position(|&id| {
                    self.cards[id]
                        .def
                        .draw_turn
                        .is_none_or(|turn| turn <= self.turn)
                })
            })
    }
    /// Deck::GetTopCard (0x7cc0b28) returns the first card, irrespective of draw timing.
    pub fn top_card(&self, seat: usize) -> Option<usize> {
        self.players[seat].deck.first().copied()
    }
    pub fn draw(&mut self, seat: usize) {
        self.draw_from_deck(seat, seat);
    }
    pub fn start_turn(&mut self) {
        if let Some(queue) = &mut self.staging_order {
            queue.clear();
        }
        if !cards::reference_rules_enabled() {
            self.is_wait_for_staging = false;
        }
        self.is_start_turn = true;
        self.expire_location_silence(false);
        for player in &mut self.players {
            player.energy_spent_this_turn = 0;
        }
        for card in &mut self.cards {
            card.observed_can_activate = None;
            card.observed_move_targets = None;
            card.external_moves.retain(|grant| grant.end >= self.turn);
        }
        self.played_on_reveal = [false; 2];
        self.delays_reveals = false;
        for seat in 0..2 {
            let permanent_energy = self.players[seat].permanent_energy;
            self.set_energy(
                seat,
                i32::from(self.turn) + self.players[seat].default_energy_offset + permanent_energy,
            );
            self.trigger_player_effects(seat, PlayerTrigger::StartTurn, None);
            let bonus: i32 = self.players[seat]
                .scheduled_energy
                .iter()
                .filter(|&&(turn, _)| turn == self.turn)
                .map(|&(_, amount)| amount)
                .sum();
            self.players[seat]
                .scheduled_energy
                .retain(|&(turn, _)| turn > self.turn);
            if bonus != 0 {
                self.set_energy(seat, self.players[seat].energy + bonus);
            }
            self.draw(seat);
        }
        let copied_schedule = self
            .lanes
            .iter()
            .any(|lane| lane.copied_reveal_turn.is_some());
        if copied_schedule {
            let scheduled = if self.current_location < usize::from(self.turn).min(self.lanes.len())
            {
                let lane = self.location_reveal_order[self.current_location];
                self.current_location += 1;
                Some(lane)
            } else {
                None
            };
            // Game::ActivateTriggerAbility (0x7da93a0) visits the location
            // list in order for StartOfTurn, including simultaneous reveals.
            self.end_turn_lane_order = Some([0, 1, 2]);
            for original in 0..self.lanes.len() {
                let lane = self.end_turn_lane_order.unwrap()[original];
                let copied = self.lanes[lane].copied_reveal_turn;
                if copied.is_some_and(|turn| turn <= self.turn)
                    || (copied.is_none() && scheduled == Some(original))
                {
                    self.reveal_scheduled_location(lane);
                }
                if self.location_abilities_active(lane)
                    && let Some(hook) = self.lanes[lane].def.on_turn_start
                {
                    let _ = self.counted_trigger(|game| {
                        game.record_location_hook(lane, "on_turn_start");
                        hook(game, lane)
                    });
                    self.ongoing();
                }
            }
            self.end_turn_lane_order = None;
        } else if self.current_location < usize::from(self.turn).min(self.lanes.len()) {
            let lane = self.location_reveal_order[self.current_location];
            self.current_location += 1;
            self.reveal_scheduled_location(lane);
        }
        self.ongoing();
        if !copied_schedule {
            for lane in 0..3 {
                if self.location_abilities_active(lane)
                    && let Some(hook) = self.lanes[lane].def.on_turn_start
                {
                    let _ = self.counted_trigger(|game| {
                        game.record_location_hook(lane, "on_turn_start");
                        hook(game, lane)
                    });
                    self.ongoing();
                }
            }
        }
        let observers: Vec<_> = if cards::reference_rules_enabled() {
            self.lanes
                .iter()
                .flat_map(|lane| lane.cards.iter().copied())
                .filter(|&id| self.cards[id].revealed)
                .collect()
        } else {
            self.revealed_cards_in_resolution_order()
        };
        for id in observers {
            if !self.card_triggers_allowed(id) {
                continue;
            }
            let default = self.cards[id].default_ability();
            let attached: Vec<_> = self.cards[id]
                .composed_abilities
                .iter()
                .filter(|ability| ability.kind == AbilityKind::Trigger)
                .filter_map(|ability| {
                    ability
                        .def
                        .start_turn
                        .map(|hook| (ability.id, ability.def.id, hook))
                })
                .collect();
            if !self.cards[id].abilities_removed
                && let Some(hook) = default.start_turn
            {
                self.execute_default_ability(id, |game, card| {
                    game.record_card_hook(default.id, "start_turn");
                    hook(game, card)
                });
            }
            for (instance, def_id, hook) in attached {
                self.execute_ability(id, instance, |game, card| {
                    game.record_card_hook(def_id, "start_turn");
                    hook(game, card)
                });
            }
        }
        for seat in 0..2 {
            for lane in 0..3 {
                if self.location_abilities_active(lane)
                    && let Some(hook) = self.lanes[lane].def.energy_bonus
                {
                    self.record_location_hook(lane, "energy_bonus");
                    let energy = self.players[seat].energy + hook(self, lane, seat);
                    self.set_energy(seat, energy);
                }
            }
        }
        let observers: Vec<_> = if cards::reference_rules_enabled() {
            self.lanes
                .iter()
                .flat_map(|lane| lane.cards.iter().copied())
                .filter(|&id| self.cards[id].revealed)
                .collect()
        } else {
            self.revealed_cards_in_resolution_order()
        };
        for id in observers {
            if self.cards[id].lane.is_some()
                && self.cards[id].revealed
                && !self.cards[id].abilities_removed
                && let Some(hook) = self.cards[id].default_ability().post_start_turn
            {
                let _ = self.counted_trigger(|game| {
                    game.record_card_hook(game.cards[id].default_ability().id, "post_start_turn");
                    hook(game, id)
                });
            }
        }
        self.emit(Event::TurnStart);
        // StartTurn::TriggerStartOfTurn and TriggerPostStartOfTurn refresh
        // ongoing stats (0x7e4a36c, 0x7e4ab00).
        if !cards::reference_rules_enabled() {
            self.ongoing();
        }
        self.is_start_turn = false;
        self.is_wait_for_staging = true;
    }
    pub fn powers(&self) -> [[i32; 2]; 3] {
        let mut totals = self.raw_location_powers();
        if self.score_overrides == [None; 2] {
            return totals;
        }
        for (lane, powers) in totals.iter_mut().enumerate() {
            if !self.lanes[lane].destroyed {
                for (seat, value) in powers.iter_mut().enumerate() {
                    if let Some(override_power) = self.score_overrides[seat] {
                        *value = override_power;
                    }
                }
            }
        }
        totals
    }

    /// Raw calculated totals ignore score overrides but include Nexus contributions.
    pub(crate) fn raw_location_powers(&self) -> [[i32; 2]; 3] {
        let base: [[i32; 2]; 3] = std::array::from_fn(|lane| {
            if self.lanes[lane].destroyed {
                return [0, 0];
            }
            std::array::from_fn(|seat| {
                (self.lanes[lane]
                    .cards
                    .iter()
                    .filter(|&&card| self.cards[card].revealed && self.cards[card].owner == seat)
                    .map(|&card| self.cards[card].power())
                    .sum::<i32>()
                    + self.lanes[lane].bonus[seat])
                    * self.lanes[lane].multiplier[seat]
            })
        });
        // NexusBehavior::AddPowerToOtherLocations (0x7d8b0b4): each
        // contribution uses the unmodified lane total, not other Nexus bonuses.
        let mut result = base;
        for (source, location) in self.lanes.iter().enumerate() {
            if location.def.contributes_to_other_lanes
                && if cards::reference_rules_enabled() {
                    location.revealed
                } else {
                    self.location_abilities_active(source)
                }
            {
                for (destination, totals) in result.iter_mut().enumerate() {
                    if source != destination && !self.lanes[destination].destroyed {
                        for seat in 0..2 {
                            totals[seat] += base[source][seat];
                        }
                    }
                }
            }
        }
        result
    }
    /// Winner of a lane, or None on a tie. Game::UpdateLocationWinner
    /// 0x7da9bf4 uses the location's forced-tie and fewest-points flags.
    pub fn lane_winner(&self, lane: usize) -> Option<usize> {
        self.lane_winner_with_power(lane, self.powers()[lane])
    }

    fn lane_winner_with_power(&self, lane: usize, power: [i32; 2]) -> Option<usize> {
        let location = &self.lanes[lane];
        let active = if cards::reference_rules_enabled() {
            location.revealed
        } else {
            self.location_abilities_active(lane)
        };
        if location.destroyed || (active && location.def.id == "Cancun") {
            return None;
        }
        let ordering = power[0].cmp(&power[1]);
        let ordering = if active && location.def.id == "BarWithNoName" {
            ordering.reverse()
        } else {
            ordering
        };
        match ordering {
            std::cmp::Ordering::Greater => Some(0),
            std::cmp::Ordering::Less => Some(1),
            std::cmp::Ordering::Equal => None,
        }
    }

    pub fn winner(&self) -> i32 {
        let powers = self.powers();
        let mut wins = [0, 0];
        let mut total = [0, 0];
        for (lane, p) in powers.into_iter().enumerate() {
            if let Some(winner) = self.lane_winner_with_power(lane, p) {
                wins[winner] += 1;
            }
            // GenerateGameResultDataByPowerGameBehavior::Get 0x7dce440
            // excludes a forced-tie lane from the power tiebreak.
            if !self.lanes[lane].destroyed
                && !(self.lanes[lane].def.id == "Cancun"
                    && if cards::reference_rules_enabled() {
                        self.lanes[lane].revealed
                    } else {
                        self.location_abilities_active(lane)
                    })
            {
                total[0] += p[0];
                total[1] += p[1];
            }
        }
        if (wins[0], total[0]) > (wins[1], total[1]) {
            0
        } else if (wins[1], total[1]) > (wins[0], total[0]) {
            1
        } else {
            2
        }
    }
    fn client_ongoing_metadata(
        def: &'static CardDef,
        hand: bool,
    ) -> &'static cards::ongoing_priorities::OngoingMetadata {
        // The client commonly labels the stat-changing hand ability "Ongoing";
        // a second "InHandOngoing" on the same card may only produce VFX.
        let rows = cards::ongoing_priorities::ONGOING_PRIORITIES;
        let start = rows.partition_point(|row| row.0 < def.id);
        let matching = || rows[start..].iter().take_while(|row| row.0 == def.id);
        matching()
            .find(|row| row.2 == "Ongoing")
            .or_else(|| {
                hand.then(|| matching().find(|row| row.2 == "InHandOngoing"))
                    .flatten()
            })
            .unwrap_or_else(|| panic!("missing ongoing stat priority for {}", def.id))
    }

    fn clear_ongoing_source(&mut self, source: OngoingSource) -> Vec<(usize, Vec<usize>)> {
        let mut positions = Vec::new();
        for (id, card) in self.cards.iter_mut().enumerate() {
            let indices: Vec<_> = card
                .stat_updates
                .iter()
                .enumerate()
                .filter_map(|(index, update)| {
                    (update.ongoing_source == Some(source)).then_some(index)
                })
                .collect();
            if indices.is_empty() {
                continue;
            }
            let mut power = 0;
            card.stat_updates.retain(|update| {
                if update.ongoing_source != Some(source) {
                    return true;
                }
                if update.stat == Stat::Power {
                    power += update.value;
                }
                false
            });
            card.ongoing -= power;
            Self::recalculate_ongoing_cost(card);
            positions.push((id, indices));
        }
        positions
    }

    fn recalculate_ongoing_cost(card: &mut Card) {
        let mut cost = card.cost;
        for update in card
            .stat_updates
            .iter()
            .filter(|update| update.ongoing && update.stat == Stat::Cost)
        {
            cost = match update.operation {
                StatOperation::Add => cost + update.value,
                StatOperation::Multiply => cost * update.value,
                StatOperation::Set => update.value,
            };
        }
        card.ongoing_cost = cost - card.cost;
    }

    fn restore_ongoing_positions(
        &mut self,
        source: OngoingSource,
        positions: Vec<(usize, Vec<usize>)>,
    ) {
        for (id, indices) in positions {
            let card = &mut self.cards[id];
            let mut replacements = Vec::new();
            card.stat_updates.retain(|update| {
                if update.ongoing_source == Some(source) {
                    replacements.push(*update);
                    false
                } else {
                    true
                }
            });
            for (ordinal, update) in replacements.into_iter().enumerate() {
                let index = indices
                    .get(ordinal)
                    .copied()
                    .unwrap_or_else(|| indices[indices.len() - 1] + ordinal + 1 - indices.len());
                card.stat_updates
                    .insert(index.min(card.stat_updates.len()), update);
            }
            Self::recalculate_ongoing_cost(card);
        }
    }

    fn activate_scheduled_ongoing(&mut self, scheduled: ScheduledOngoing) -> bool {
        let ScheduledOngoing {
            card,
            instance,
            hook,
            def_id,
            kind,
            runs,
            hand,
        } = scheduled;
        let source = OngoingSource::Card {
            card,
            instance,
            hand,
        };
        let positions = if self.is_wait_for_staging {
            self.clear_ongoing_source(source)
        } else {
            Vec::new()
        };
        let lane_positions = if self.is_wait_for_staging {
            self.take_lane_contributions(source)
        } else {
            Vec::new()
        };
        let previous_source = self.current_ongoing_source.replace(source);
        let mut succeeded = false;
        for _ in 0..runs {
            let mut action_result = false;
            let action = |game: &mut Self, card| {
                game.record_card_hook(def_id, kind);
                action_result = match hook {
                    ScheduledOngoingHook::Ordinary(hook) => {
                        hook(game, card);
                        true
                    }
                    ScheduledOngoingHook::Conditional(hook) => hook(game, card),
                };
            };
            let executed = if let Some(instance) = instance {
                self.execute_ability(card, instance, action)
            } else {
                self.execute_default_ability(card, action)
            };
            succeeded = executed && action_result;
        }
        self.current_ongoing_source = previous_source;
        self.restore_ongoing_positions(source, positions);
        self.restore_lane_contributions(source, lane_positions);
        succeeded
    }

    fn run_client_ongoing_card(
        &mut self,
        card: usize,
        priority: u8,
        hand: bool,
        runs: usize,
        pending: &mut Vec<PendingConditional>,
    ) {
        let hooks: Vec<_> = self.cards[card]
            .abilities()
            .filter(|ability| ability.supports(AbilityKind::Ongoing))
            .filter(|ability| ability.id.is_some() || !self.cards[card].abilities_removed)
            .filter_map(|ability| {
                let (hook, kind) = if hand {
                    ability
                        .def
                        .conditional_in_hand_ongoing
                        .map(|hook| {
                            (
                                ScheduledOngoingHook::Conditional(hook),
                                "conditional_in_hand_ongoing",
                            )
                        })
                        .or_else(|| {
                            ability.def.in_hand_ongoing.map(|hook| {
                                (ScheduledOngoingHook::Ordinary(hook), "in_hand_ongoing")
                            })
                        })
                        .or_else(|| {
                            ability.def.late_in_hand_ongoing.map(|hook| {
                                (ScheduledOngoingHook::Ordinary(hook), "late_in_hand_ongoing")
                            })
                        })
                } else {
                    ability
                        .def
                        .late_ongoing
                        .map(|hook| (ScheduledOngoingHook::Conditional(hook), "late_ongoing"))
                        .or_else(|| {
                            ability
                                .def
                                .ongoing
                                .map(|hook| (ScheduledOngoingHook::Ordinary(hook), "ongoing"))
                        })
                }?;
                let metadata = Self::client_ongoing_metadata(ability.def, hand);
                let staged_play = self.players.iter().any(|player| {
                    player
                        .staged_history
                        .iter()
                        .any(|play| play.card == card && play.turn == self.turn && !play.resolved)
                });
                (metadata.3 == priority
                    && (!self.is_wait_for_staging || metadata.4)
                    && (!staged_play || metadata.5)
                    && (!self.cards[card].move_staged || metadata.6))
                    .then_some(ScheduledOngoing {
                        card,
                        instance: ability.id,
                        hook,
                        def_id: ability.def.id,
                        kind,
                        runs,
                        hand,
                    })
            })
            .collect();
        for scheduled in hooks {
            if !self.activate_scheduled_ongoing(scheduled)
                && matches!(scheduled.hook, ScheduledOngoingHook::Conditional(_))
            {
                pending.push(PendingConditional::Card(scheduled));
            }
        }
    }

    /// Whether the staging updater can change a scheduled contribution or
    /// regenerate copied Ongoing abilities. This is deliberately conservative.
    fn staging_refresh_needed(&self) -> bool {
        if self.players.iter().any(|player| {
            player
                .cost_effects
                .iter()
                .any(|effect| effect.kind.ongoing_schedule().1)
        }) || self.lanes.iter().any(|lane| {
            locations::ongoing_stat_schedule(lane.def.id).is_some_and(|(_, active, _, _)| active)
        }) || self.cards.iter().any(|card| {
            !card.copied_ongoing_guards.is_empty()
                || !card.virtual_ongoing_ids.is_empty()
                || card
                    .composed_abilities
                    .iter()
                    .any(|ability| ability.parent.is_some())
        }) {
            return true;
        }
        let scheduled = |card: usize, hand: bool| {
            if !hand
                && matches!(
                    self.cards[card].default_ability().id,
                    "Moonstone" | "SuperSkrull"
                )
            {
                return true;
            }
            self.cards[card]
                .abilities()
                .filter(|ability| ability.supports(AbilityKind::Ongoing))
                .filter(|ability| ability.id.is_some() || !self.cards[card].abilities_removed)
                .any(|ability| {
                    if !hand
                        && matches!(ability.def.id, "Moonstone" | "SuperSkrull" | "RedOnslaught")
                    {
                        return true;
                    }
                    let def = ability.def;
                    let hook = if hand {
                        def.conditional_in_hand_ongoing.is_some()
                            || def.in_hand_ongoing.is_some()
                            || def.late_in_hand_ongoing.is_some()
                    } else {
                        def.late_ongoing.is_some() || def.ongoing.is_some()
                    };
                    hook && Self::client_ongoing_metadata(def, hand).4
                })
        };
        self.players
            .iter()
            .any(|player| player.hand.iter().any(|&card| scheduled(card, true)))
            || self.lanes.iter().any(|lane| {
                lane.cards
                    .iter()
                    .any(|&card| self.cards[card].revealed && scheduled(card, false))
            })
    }

    fn finish_client_ongoing(&mut self) {
        for lane in 0..3 {
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if !self.cards[id].abilities_removed
                    && (self.cards[id].revealed || self.cards[id].turn_played > 0)
                    && let Some(hook) = self.cards[id].default_ability().objective_update
                {
                    self.execute_default_ability(id, |game, card| {
                        game.record_card_hook(
                            game.cards[card].default_ability().id,
                            "objective_update",
                        );
                        hook(game, card)
                    });
                }
            }
        }
        self.emit(Event::Ongoing);
    }

    // AbilitySystem::UpdateOngoingStats 0x7c94474: Init, modifiers,
    // Default, ConditionalCard, ConditionalLocation, PostConditional.
    fn client_ongoing(&mut self) {
        let staging = self.is_wait_for_staging;
        if staging && !self.staging_refresh_needed() {
            self.finish_client_ongoing();
            return;
        }
        self.client_ongoing_scheduled();
    }

    fn client_ongoing_scheduled(&mut self) {
        let staging = self.is_wait_for_staging;
        // Card::ClearOngoingStatsWhileStaging 0x7cb4cec clears only
        // movement/play restriction caches. Those guards are evaluated on
        // demand here, so there are no equivalent cached fields to clear.
        if !staging {
            self.score_overrides = [None; 2];
            for lane in &mut self.lanes {
                lane.bonus = [0; 2];
                lane.contributions.clear();
                lane.multiplier = [1; 2];
            }
            for card in &mut self.cards {
                card.ongoing = 0;
                card.ongoing_cost = 0;
                card.location_power = 0;
                card.stat_updates.retain(|update| !update.ongoing);
            }
        }
        let copied = self.prepare_copied_ongoing();
        for priority in [1, 0, 2, 3, 4] {
            let mut pending = Vec::new();
            if !staging && priority == 0 {
                for card in 0..self.cards.len() {
                    self.cards[card].ongoing += self.luke_power_correction(card);
                }
            }
            // ActivateOngoingAbilities visits each player's abilities in attachment order.
            for seat in 0..2 {
                for index in 0..self.players[seat].cost_effects.len() {
                    let effect = self.players[seat].cost_effects[index];
                    let (order, active_staging, _, _) = effect.kind.ongoing_schedule();
                    if order != priority || (staging && !active_staging) {
                        continue;
                    }
                    let previous_source =
                        self.current_ongoing_source.replace(OngoingSource::Player {
                            seat,
                            sequence: effect.sequence,
                        });
                    match effect.kind {
                        PlayerCostEffectKind::Sandman { turn } if turn == self.turn => {
                            for hand_index in 0..self.players[seat].hand.len() {
                                let card = self.players[seat].hand[hand_index];
                                let cost = self.cards[card].cost + self.cards[card].ongoing_cost;
                                // SandmanBehavior::IncreaseCardCost (0x7f2e084)
                                // uses AddCostWithMax(+1, max(current, 6)).
                                if cost < 6 {
                                    self.add_ongoing_cost(card, 1, seat);
                                } else {
                                    self.set_ongoing_cost(card, cost, seat);
                                }
                            }
                        }
                        PlayerCostEffectKind::BaronMordo { card, until, value }
                            if self.turn <= until && self.players[seat].hand.contains(&card) =>
                        {
                            // BaronMordoBehavior::SetCardCost (0x7e6ed1c)
                            // checks that the target is still in this player's hand.
                            self.set_ongoing_cost(card, value, seat);
                        }
                        PlayerCostEffectKind::BlackSwan {
                            until,
                            printed_cost,
                            value,
                        } if self.turn <= until => {
                            for hand_index in 0..self.players[seat].hand.len() {
                                let card = self.players[seat].hand[hand_index];
                                if self.printed_cost(card) == printed_cost {
                                    self.set_ongoing_cost(card, value, seat);
                                }
                            }
                        }
                        // Skill01DrDoomChampionBehavior::.cctor (0x7d09b34)
                        // assigns its player-owned cost setter priority 4.
                        PlayerCostEffectKind::ChampionDoom { turn } if turn == self.turn => {
                            for hand_index in 0..self.players[seat].hand.len() {
                                let card = self.players[seat].hand[hand_index];
                                if self.printed_cost(card) == 6 {
                                    self.set_ongoing_cost(card, 3, seat);
                                }
                            }
                        }
                        PlayerCostEffectKind::Wave { until } if self.turn <= until => {
                            for hand_index in 0..self.players[seat].hand.len() {
                                let card = self.players[seat].hand[hand_index];
                                let cost = self.cards[card].cost + self.cards[card].ongoing_cost;
                                // WaveBehavior::SetCardCost (0x7cef16c)
                                // sets the cap only when current cost exceeds it.
                                if cost > 4 {
                                    self.set_ongoing_cost(card, 4, seat);
                                }
                            }
                        }
                        _ => {}
                    }
                    self.current_ongoing_source = previous_source;
                }
            }
            for lane in 0..3 {
                let location = self.lanes[lane].def;
                if !self.location_abilities_active(lane)
                    || !locations::ongoing_stat_schedule(location.id).is_some_and(
                        |(order, active_staging, _, _)| {
                            order == priority && (!staging || active_staging)
                        },
                    )
                {
                    continue;
                }
                let previous_source = self
                    .current_ongoing_source
                    .replace(OngoingSource::Location { lane });
                if let Some(hook) = location.modify_cost {
                    for seat in 0..2 {
                        for index in 0..self.players[seat].hand.len() {
                            let card = self.players[seat].hand[index];
                            let cost = self.cards[card].cost + self.cards[card].ongoing_cost;
                            self.record_location_hook(lane, "modify_cost");
                            match hook(self, lane, card, cost) {
                                Some(locations::CostUpdate::Add(delta)) => {
                                    self.add_ongoing_cost(card, delta, seat);
                                }
                                Some(locations::CostUpdate::Set(value)) => {
                                    self.set_ongoing_cost(card, value, seat);
                                }
                                None => {}
                            }
                        }
                    }
                }
                for index in 0..self.lanes[lane].cards.len() {
                    let card = self.lanes[lane].cards[index];
                    if self.cards[card].revealed
                        && location.power != 0
                        && (location.power >= 0 || self.can_reduce_card_power(card))
                    {
                        self.add_location_power(card, location.power);
                    }
                }
                if let Some(hook) = location.ongoing {
                    let _ = self.counted_trigger(|game| {
                        game.record_location_hook(lane, "ongoing");
                        hook(game, lane)
                    });
                }
                if let Some(hook) = location.conditional_ongoing
                    && !self
                        .counted_trigger(|game| {
                            game.record_location_hook(lane, "conditional_ongoing");
                            hook(game, lane)
                        })
                        .unwrap_or(false)
                {
                    pending.push(PendingConditional::Location { lane, hook });
                }
                self.current_ongoing_source = previous_source;
            }
            // Tribunal declares PlayOngoingAbilityBeforeCards = [OmegaRed].
            for tribunal_first in [true, false] {
                for lane in 0..3 {
                    if !self.lane_ongoing_allowed(lane) {
                        continue;
                    }
                    let base = if self.location_abilities_active(lane) {
                        usize::from(self.lanes[lane].def.ongoing_repeats)
                    } else {
                        1
                    };
                    let mut extra = [0usize; 2];
                    for &id in &self.lanes[lane].cards {
                        if self.cards[id].revealed && self.cards[id].has_ongoing(true) {
                            extra[self.cards[id].owner] += base
                                * self.cards[id]
                                    .abilities()
                                    .filter(|ability| ability.supports(AbilityKind::Ongoing))
                                    .filter(|ability| {
                                        ability.id.is_some() || !self.cards[id].abilities_removed
                                    })
                                    .map(|ability| usize::from(ability.def.extra_ongoing_runs))
                                    .sum::<usize>();
                        }
                    }
                    for index in 0..self.lanes[lane].cards.len() {
                        let card = self.lanes[lane].cards[index];
                        if !self.cards[card].revealed || !self.cards[card].has_ongoing(true) {
                            continue;
                        }
                        let tribunal = self.cards[card]
                            .abilities()
                            .any(|ability| ability.def.id == "LivingTribunal");
                        if tribunal != tribunal_first {
                            continue;
                        }
                        let own_extra: usize = self.cards[card]
                            .abilities()
                            .filter(|ability| ability.supports(AbilityKind::Ongoing))
                            .filter(|ability| {
                                ability.id.is_some() || !self.cards[card].abilities_removed
                            })
                            .map(|ability| usize::from(ability.def.extra_ongoing_runs))
                            .sum();
                        let runs = base + extra[self.cards[card].owner] - base * own_extra;
                        self.run_client_ongoing_card(card, priority, false, runs, &mut pending);
                    }
                }
            }
            for seat in 0..2 {
                for index in 0..self.players[seat].hand.len() {
                    let card = self.players[seat].hand[index];
                    self.run_client_ongoing_card(card, priority, true, 1, &mut pending);
                    if !staging && priority == 0 {
                        for effect_index in 0..self.cards[card].cost_effects.len() {
                            let effect = self.cards[card].cost_effects[effect_index];
                            let previous_source =
                                self.current_ongoing_source
                                    .replace(OngoingSource::CardCost {
                                        card,
                                        sequence: effect.sequence,
                                    });
                            match effect.kind {
                                CardCostEffectKind::NextTurnDelta { turn, delta }
                                    if turn == self.turn =>
                                {
                                    self.add_ongoing_cost(card, delta, seat);
                                }
                                CardCostEffectKind::TemporarySet { turn, value }
                                    if turn == self.turn =>
                                {
                                    self.set_ongoing_cost(card, value, seat);
                                }
                                _ => {}
                            }
                            self.current_ongoing_source = previous_source;
                        }
                    }
                    if !staging && priority == 4 {
                        for effect_index in 0..self.cards[card].cost_effects.len() {
                            let effect = self.cards[card].cost_effects[effect_index];
                            if let CardCostEffectKind::WhiteQueenWinning { lane, seat: owner } =
                                effect.kind
                                && seat == owner
                                && !self.lanes[lane].destroyed
                                && self.lane_winner(lane) == Some(owner)
                            {
                                let previous =
                                    self.current_ongoing_source
                                        .replace(OngoingSource::CardCost {
                                            card,
                                            sequence: effect.sequence,
                                        });
                                self.add_ongoing_cost(card, -2, owner);
                                self.current_ongoing_source = previous;
                            }
                        }
                    }
                    if !staging
                        && priority == 4
                        && !self.cards[card].abilities_removed
                        && self.cards[card].default_ability().id == "SuperiorIronMan"
                    {
                        let previous_source =
                            self.current_ongoing_source.replace(OngoingSource::Card {
                                card,
                                instance: None,
                                hand: true,
                            });
                        let cost = self.card_cost(card);
                        let next = self.double_cost_reductions(card, cost);
                        self.add_ongoing_cost(card, next - cost, seat);
                        self.current_ongoing_source = previous_source;
                    }
                }
            }
            // RerunConditionalOngoingAbilities 0x7c982e8 retries failed
            // activations until a full traversal succeeds at none.
            if matches!(priority, 2 | 3) {
                loop {
                    let mut succeeded = None;
                    for (index, &scheduled) in pending.iter().enumerate() {
                        let activated = match scheduled {
                            PendingConditional::Card(card) => self.activate_scheduled_ongoing(card),
                            PendingConditional::Location { lane, hook } => {
                                let old = self
                                    .current_ongoing_source
                                    .replace(OngoingSource::Location { lane });
                                let result = self
                                    .counted_trigger(|game| {
                                        game.record_location_hook(lane, "conditional_ongoing");
                                        hook(game, lane)
                                    })
                                    .unwrap_or(false);
                                self.current_ongoing_source = old;
                                result
                            }
                        };
                        if activated {
                            succeeded = Some(index);
                            break;
                        }
                    }
                    if let Some(index) = succeeded {
                        pending.remove(index);
                    } else {
                        break;
                    }
                }
            }
        }
        self.finish_copied_ongoing(&copied);
        self.finish_client_ongoing();
    }

    pub fn ongoing(&mut self) {
        if !cards::reference_rules_enabled() {
            self.client_ongoing();
            return;
        }
        self.score_overrides = [None; 2];
        for lane in &mut self.lanes {
            lane.bonus = [0; 2];
            lane.multiplier = [1; 2];
            lane.contributions.clear();
        }
        for card in &mut self.cards {
            card.ongoing = 0;
            card.ongoing_cost = 0;
            card.location_power = 0;
            card.stat_updates.retain(|update| !update.ongoing);
        }
        for id in 0..self.cards.len() {
            self.cards[id].ongoing += self.luke_power_correction(id);
        }
        for seat in 0..2 {
            for index in 0..self.players[seat].hand.len() {
                let card = self.players[seat].hand[index];
                if !self.cards[card].abilities_removed
                    && let Some(hook) = self.cards[card].default_ability().in_hand_ongoing
                {
                    let _ = self.counted_trigger(|game| {
                        game.record_card_hook(
                            game.cards[card].default_ability().id,
                            "in_hand_ongoing",
                        );
                        hook(game, card)
                    });
                }
            }
        }
        for lane in 0..3 {
            if !self.lane_ongoing_allowed(lane) {
                continue;
            }
            let base = if self.lanes[lane].revealed {
                usize::from(self.lanes[lane].def.ongoing_repeats)
            } else {
                1
            };
            let mut extra = [0usize; 2];
            for &id in &self.lanes[lane].cards {
                if self.cards[id].revealed && self.cards[id].has_ongoing(true) {
                    extra[self.cards[id].owner] +=
                        base * usize::from(self.cards[id].default_ability().extra_ongoing_runs);
                }
            }
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if self.cards[id].revealed
                    && self.cards[id].has_ongoing(true)
                    && let Some(hook) = self.cards[id].default_ability().ongoing
                {
                    let runs = base + extra[self.cards[id].owner]
                        - base * usize::from(self.cards[id].default_ability().extra_ongoing_runs);
                    for _ in 0..runs {
                        let _ = self.counted_trigger(|game| {
                            game.record_card_hook(game.cards[id].default_ability().id, "ongoing");
                            hook(game, id)
                        });
                    }
                }
            }
        }
        for lane in 0..3 {
            if self.lanes[lane].revealed {
                let power = self.lanes[lane].def.power;
                for index in 0..self.lanes[lane].cards.len() {
                    let id = self.lanes[lane].cards[index];
                    if self.cards[id].revealed
                        && power != 0
                        && (power >= 0 || self.can_reduce_card_power(id))
                    {
                        self.add_location_power(id, power);
                    }
                }
                if let Some(hook) = self.lanes[lane].def.ongoing {
                    let _ = self.counted_trigger(|game| {
                        game.record_location_hook(lane, "ongoing");
                        hook(game, lane)
                    });
                }
                if let Some(hook) = self.lanes[lane].def.conditional_ongoing {
                    let _ = self.counted_trigger(|game| {
                        game.record_location_hook(lane, "conditional_ongoing");
                        hook(game, lane)
                    });
                }
            }
        }
        // SandmanBehavior::IncreaseCardCost (0x7f2e084): each effect adds
        // one up to max(current cost, 6), to the current hand.
        for seat in 0..2 {
            let effects = self.players[seat].cost_effects.iter()
                .filter(|effect| matches!(effect.kind, PlayerCostEffectKind::Sandman { turn } if turn == self.turn)).count();
            for index in 0..self.players[seat].hand.len() {
                let card = self.players[seat].hand[index];
                for _ in 0..effects {
                    let cost = self.card_cost(card);
                    let next = (cost + 1).min(cost.max(6));
                    if self.cost_change_allowed(card, cost, next) {
                        self.cards[card].ongoing_cost += next - cost;
                    }
                }
            }
        }
        for lane in 0..3 {
            if !self.lane_ongoing_allowed(lane) {
                continue;
            }
            let base = if self.lanes[lane].revealed {
                usize::from(self.lanes[lane].def.ongoing_repeats)
            } else {
                1
            };
            let mut extra = [0usize; 2];
            for &id in &self.lanes[lane].cards {
                if self.cards[id].revealed && self.cards[id].has_ongoing(true) {
                    extra[self.cards[id].owner] +=
                        base * usize::from(self.cards[id].default_ability().extra_ongoing_runs);
                }
            }
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if self.cards[id].revealed
                    && self.cards[id].has_ongoing(true)
                    && let Some(hook) = self.cards[id].default_ability().late_ongoing
                {
                    let runs = base + extra[self.cards[id].owner]
                        - base * usize::from(self.cards[id].default_ability().extra_ongoing_runs);
                    for _ in 0..runs {
                        let _ = self.counted_trigger(|game| {
                            game.record_card_hook(
                                game.cards[id].default_ability().id,
                                "late_ongoing",
                            );
                            hook(game, id)
                        });
                    }
                }
            }
        }
        for seat in 0..2 {
            for index in 0..self.players[seat].hand.len() {
                let card = self.players[seat].hand[index];
                if !self.cards[card].abilities_removed
                    && let Some(hook) = self.cards[card].default_ability().late_in_hand_ongoing
                {
                    let _ = self.counted_trigger(|game| {
                        game.record_card_hook(
                            game.cards[card].default_ability().id,
                            "late_in_hand_ongoing",
                        );
                        hook(game, card)
                    });
                }
                if !self.cards[card].abilities_removed
                    && let Some(hook) = self.cards[card]
                        .default_ability()
                        .conditional_in_hand_ongoing
                {
                    let _ = self.counted_trigger(|game| {
                        game.record_card_hook(
                            game.cards[card].default_ability().id,
                            "conditional_in_hand_ongoing",
                        );
                        hook(game, card)
                    });
                }
            }
        }
        for lane in 0..3 {
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if !self.cards[id].abilities_removed
                    && (self.cards[id].revealed || self.cards[id].turn_played > 0)
                    && let Some(hook) = self.cards[id].default_ability().objective_update
                {
                    self.execute_default_ability(id, |game, card| {
                        game.record_card_hook(
                            game.cards[card].default_ability().id,
                            "objective_update",
                        );
                        hook(game, card)
                    });
                }
            }
        }
        self.emit(Event::Ongoing);
    }
    pub fn printed_cost(&self, card: usize) -> i32 {
        CATALOG[self.cards[card].def.id].cost
    }
    pub(crate) fn printed_power(&self, card: usize) -> i32 {
        CATALOG[self.cards[card].def.id].power
    }

    pub fn trigger_on_reveal(&mut self, card: usize) {
        self.trigger_reveal_abilities(card, false);
    }

    // Card::<ActivateTextResolveAbility>b__474_0 (0x7cbefdc) selects
    // ThisCardResolvedTrigger definitions only when IsHidden is false.
    fn trigger_reveal_abilities(&mut self, card: usize, initial: bool) {
        let attached: Vec<_> = self.cards[card]
            .composed_abilities
            .iter()
            .filter(|ability| {
                ability.kind == AbilityKind::Reveal
                    && (initial || !ability.hidden || cards::reference_rules_enabled())
            })
            .filter_map(|ability| {
                ability
                    .def
                    .reveal
                    .map(|hook| (ability.id, ability.def.id, hook, ability.hidden))
            })
            .collect();
        self.trigger_text_reveal(card);
        for (instance, def_id, hook, hidden) in attached {
            self.execute_ability(card, instance, |game, card| {
                game.trigger_reveal_hook_inner(
                    card,
                    hook,
                    !hidden || cards::reference_rules_enabled(),
                    def_id,
                )
            });
        }
    }
    /// Card::ActivateTextResolveAbility (0x7cbb424) selects the first
    /// resolve from GetDefaultAbilities (0x7cb1dcc), not foreign attachments.
    pub(crate) fn trigger_text_reveal(&mut self, card: usize) {
        if let Some(hook) = self.cards[card].default_ability().reveal {
            let reentrant = self.cards[card]
                .abilities()
                .next()
                .unwrap()
                .resolve_reentrant(&self.cards[card]);
            self.execute_default_resolve(card, reentrant, |game, card| {
                game.trigger_reveal_hook(card, hook, game.cards[card].default_ability().id)
            });
        }
    }
    pub(crate) fn trigger_reveal_hook(
        &mut self,
        card: usize,
        hook: cards::CardHook,
        def_id: &'static str,
    ) {
        self.trigger_reveal_hook_inner(card, hook, true, def_id);
    }

    fn trigger_reveal_hook_inner(
        &mut self,
        card: usize,
        hook: cards::CardHook,
        repeat: bool,
        def_id: &'static str,
    ) {
        let Some(lane) = self.cards[card].lane else {
            return;
        };
        if (if cards::reference_rules_enabled() {
            self.lanes[lane].revealed && self.lanes[lane].def.disables_on_reveal
        } else {
            !self.lanes[lane].ability_flags.on_reveal
        }) || self.lanes[lane].cards.iter().any(|&id| {
            self.cards[id].revealed
                && self.ongoing_active(id)
                && self.cards[id]
                    .ongoing_guard_abilities()
                    .any(|a| a.supports(AbilityKind::Ongoing) && a.def.blocks_reveal)
        }) {
            return;
        }
        let active = self.location_abilities_active(lane);
        let ongoing_base = if active {
            usize::from(self.lanes[lane].def.ongoing_repeats)
        } else {
            1
        };
        let mut runs = if active {
            usize::from(self.lanes[lane].def.reveal_repeats)
        } else {
            1
        };
        for &source in &self.lanes[lane].cards {
            if !self.cards[source].revealed || !self.ongoing_active(source) {
                continue;
            }
            for (def_id, repetitions) in self.cards[source]
                .ongoing_guard_abilities()
                .filter(|a| a.supports(AbilityKind::Ongoing))
                .filter_map(|a| a.def.reveal_repetitions.map(|hook| (a.def.id, hook)))
            {
                self.record_card_hook(def_id, "reveal_repetitions");
                let additional = usize::from(repetitions(self, source, card));
                let extra: usize = self.lanes[lane]
                    .cards
                    .iter()
                    .copied()
                    .filter(|&other| {
                        other != source
                            && self.cards[other].revealed
                            && self.ongoing_active(other)
                            && self.cards[other].owner == self.cards[source].owner
                    })
                    .flat_map(|other| self.cards[other].ongoing_guard_abilities())
                    .filter(|a| a.supports(AbilityKind::Ongoing))
                    .map(|a| usize::from(a.def.extra_ongoing_runs))
                    .sum();
                runs += additional * ongoing_base * (1 + extra);
            }
        }
        if !repeat {
            runs = 1;
        }
        let original = self.cards[card].def.id;
        for _ in 0..runs {
            self.record_card_hook(def_id, "reveal");
            hook(self, card);
            if self.cards[card].lane.is_none() || self.cards[card].def.id != original {
                break;
            }
        }
    }
    pub fn reveal(&mut self, id: usize) {
        self.reveal_with_staging(id, None);
    }

    pub(crate) fn reveal_with_staging(&mut self, id: usize, staging: Option<StagingContext>) {
        let Some(lane) = self.cards[id].lane else {
            return;
        };
        if self.cards[id].revealed {
            return;
        }
        if self.cards[id]
            .attached_abilities
            .iter()
            .any(|ability| matches!(ability.kind, AttachedAbilityKind::RevealAtGameEnd))
        {
            return;
        }
        // DarkDimensionBehavior::MakeCardsHidden @ 0x7d4d94c stops
        // hiding cards once the client enters EndGame.
        let staging_owner = staging.map_or(self.cards[id].owner, |context| context.seat);
        let owner_concealed = !self.ending_reveals
            && (self.players[staging_owner]
                .supergiant_hidden_turns
                .iter()
                .any(|&(turn, source)| turn == self.cards[id].turn_played && source != id)
                || self.lanes[lane].cards.iter().copied().any(|source| {
                    source != id
                        && self.cards[source].owner == self.cards[id].owner
                        && self.cards[source].revealed
                        && self.ongoing_active(source)
                        && self.cards[source].ongoing_guard_abilities().any(|ability| {
                            ability.supports(AbilityKind::Ongoing)
                                && ability.def.conceals_owner_cards
                        })
                }));
        let lane_delayed = !self.ending_reveals
            && self.location_abilities_active(lane)
            && self.lanes[lane].def.delays_reveals_until_end;
        if self.delays_reveals || owner_concealed || lane_delayed {
            let owner = if cards::reference_rules_enabled() {
                self.cards[id].owner
            } else {
                staging.map_or(self.cards[id].owner, |context| context.seat)
            };
            let pending = &mut self.delayed_reveals[owner];
            if !pending.iter().any(|queued| queued.card == id) {
                pending.push(QueuedReveal {
                    card: id,
                    staging,
                    persistent: owner_concealed || lane_delayed,
                });
            }
            return;
        }
        let mut left_moves = 0;
        if self.cards[id].turn_played > 0 {
            let seat = self.cards[id].owner;
            let power = std::mem::take(&mut self.players[seat].next_play_power);
            if power != 0 {
                self.add_power(id, power, seat);
            }
            left_moves = std::mem::take(&mut self.players[seat].pending_left_moves);
        }
        if self.location_abilities_active(lane)
            && let Some(hook) = self.lanes[lane].def.before_card_reveal
        {
            let _ = self.counted_trigger(|game| {
                game.record_location_hook(lane, "before_card_reveal");
                hook(game, lane, id)
            });
            if self.cards[id].lane.is_none() {
                self.ongoing();
                return;
            }
        }
        self.emit(Event::BeforeCardRevealed { card: id });
        if self.cards[id].turn_played > 0
            && let Some(context) = staging
        {
            self.emit(Event::StagedCardResolving {
                card: id,
                seat: context.seat,
                lane: context.lane,
            });
        }
        if self.cards[id].lane.is_none() {
            return;
        }
        if self.cards[id].turn_played > 0 {
            self.trigger_player_effects(
                self.cards[id].owner,
                PlayerTrigger::BeforeCardRevealed,
                Some(id),
            );
            if self.cards[id].lane.is_none() {
                return;
            }
        }
        // ThanosFracturedFrontierBehavior::SelfLoadShot 0x7cd7ec8 consumes
        // PreCardResolvedTrigger, before this card's resolve abilities.
        if self.cards[id].turn_played > 0 {
            self.quickdraw_card_resolved(id, true);
        }
        if self.cards[id].turn_played > 0 && self.cards[id].default_ability().reveal.is_some() {
            self.played_on_reveal[self.cards[id].owner] = true;
        }
        if self.cards[id].turn_played > 0 {
            let seat = self.cards[id].owner;
            if (self.cards[id].default_ability().move_limit > 0
                || self.cards[id].default_ability().move_each_turn
                || self.matching_move_grant(id, seat, None).is_some())
                && !self.players[seat].movable_cards_played.contains(&id)
            {
                self.players[seat].movable_cards_played.push(id);
            }
        }
        // Card::ResolveForPlay @ 0x7cb9d90 can change the card's owner while
        // resolving; the play remains attributed to the staging player.
        let staging_owner = if cards::reference_rules_enabled() {
            self.cards[id].owner
        } else {
            staging.map_or(self.cards[id].owner, |context| context.seat)
        };
        let was_played = self.cards[id].turn_played > 0;
        let resolved_def_id = self.cards[id].def.id;
        let resolved_printed_cost = self.printed_cost(id);
        let resolved_lane = self.cards[id].lane;
        let post_resolve_start = self.post_resolve_plays.len();
        self.cards[id].latest_resolution_order = self.next_resolution_order;
        self.next_resolution_order += 1;
        self.trigger_reveal_abilities(id, true);
        // Card::ResolveForPlay 0x7cb9d90 calls TrackCardsPlayedStats
        // 0x7cba364 after the reveal, even when the card left its location.
        if was_played && self.cards[id].has_resolve_trigger() {
            self.players[staging_owner].on_reveal_cards_played += 1;
        }
        self.cards[id].revealed = true;
        self.cards[id].turn_revealed = Some(self.turn);
        // ResolveForPlay (0x7cb9d90) updates ongoing after SetRevealed;
        // TriggerAbility::Activate (0x7c93fd8) refreshes after each resolve.
        self.ongoing();
        self.emit(Event::CardRevealed { card: id });
        if self.cards[id].turn_played > 0 {
            let observers: Vec<_> = self
                .lanes
                .iter()
                .flat_map(|lane| lane.cards.iter().copied())
                .filter(|&other| other != id && self.cards[other].revealed)
                .collect();
            for other in observers {
                if self.card_triggers_allowed(other)
                    && let Some(hook) = self.cards[other].default_ability().on_play
                {
                    let _ = self.counted_trigger(|game| {
                        game.record_card_hook(game.cards[other].default_ability().id, "on_play");
                        hook(game, other, id)
                    });
                }
            }
        }
        if was_played {
            self.players[staging_owner]
                .resolved_play_history
                .push(ResolvedPlayEvent {
                    turn: self.turn,
                    card: id,
                    def_id: resolved_def_id,
                    printed_cost: resolved_printed_cost,
                    lane: resolved_lane,
                    staged_lane: staging.map(|context| context.lane),
                    power: Some(self.cards[id].power()),
                });
            if self.cards[id].turn_played == 0
                && let Some(play) = self.players[staging_owner]
                    .staged_history
                    .iter_mut()
                    .rev()
                    .find(|play| play.card == id && !play.resolved)
            {
                play.resolved = true;
            }
        }
        if self.cards[id].turn_played > 0 {
            self.players[staging_owner]
                .play_history
                .push((self.turn, id));
            let seat = staging_owner;
            if let Some(play) = self.players[seat]
                .staged_history
                .iter_mut()
                .rev()
                .find(|play| play.card == id && !play.resolved)
            {
                play.resolved = true;
            }
            self.emit(Event::CardPlayed { card: id });
            self.trigger_player_effects(self.cards[id].owner, PlayerTrigger::CardPlayed, Some(id));
        }
        self.ongoing();
        if self.is_skill(id)
            && let Some(lane) = self.cards[id].lane
        {
            if cards::reference_rules_enabled() {
                self.detach_location(lane, id);
                self.cards[id].last_lane = Some(lane);
                self.cards[id].lane = None;
                self.push_banished(self.cards[id].owner, id);
                self.ongoing();
            } else {
                self.banish_card(id);
            }
        }
        // Card::ResolveForPlay @ 0x7cb9d90: banish skills before
        // PostCardResolvedTrigger, which reaches cards and locations.
        self.emit(Event::CardResolved { card: id });
        if was_played && let Some(context) = staging {
            self.emit(Event::StagedCardResolved {
                card: id,
                seat: context.seat,
                lane: context.lane,
            });
        }
        if self.cards[id].turn_played > 0 {
            // LoadShot 0x7cd723c consumes PostCardResolvedTrigger for other cards.
            self.quickdraw_card_resolved(id, false);
            self.trigger_player_effects(staging_owner, PlayerTrigger::CardResolved, Some(id));
        }
        if self.cards[id].turn_played > 0
            && let Some(lane) = self.cards[id].lane
            && self.location_abilities_active(lane)
            && let Some(hook) = self.lanes[lane].def.after_card_play
        {
            let _ = self.counted_trigger(|game| {
                game.record_location_hook(lane, "after_card_play");
                hook(game, lane, id)
            });
            self.ongoing();
        }
        // SinisterLondon::CreateCopy (0x7d7fe8c) uses CardStaged's
        // original location even after a resolved skill has been banished.
        if !cards::reference_rules_enabled()
            && was_played
            && self.is_skill(id)
            && let Some(context) = staging
            && self.location_abilities_active(context.lane)
            && self.lanes[context.lane].def.id == "SinisterLondon"
            && let Some(hook) = self.lanes[context.lane].def.after_card_play
        {
            let _ = self.counted_trigger(|game| {
                game.record_location_hook(context.lane, "after_card_play");
                hook(game, context.lane, id)
            });
            self.ongoing();
        }
        for _ in 0..left_moves {
            let Some(lane) = self.cards[id].lane else {
                break;
            };
            if lane == 0 || !self.move_card(id, lane - 1) {
                break;
            }
        }
        self.resolve_effect_plays(post_resolve_start);
    }

    /// Resolve the effect-staged batch after its enclosing action has finished.
    pub(crate) fn resolve_effect_plays(&mut self, start: usize) {
        let post_resolve = self.post_resolve_plays.split_off(start);
        for queued in post_resolve {
            self.reveal_with_staging(queued.card, queued.staging);
        }
    }
    /// Start the next turn: clear last turn's plays and draw, reveal and refresh.
    pub(crate) fn begin_turn(&mut self) {
        self.played = Default::default();
        self.pending_plays = std::array::from_fn(|seat| {
            self.delayed_reveals[seat]
                .iter()
                .filter_map(|queued| self.cards[queued.card].lane.map(|lane| (queued.card, lane)))
                .collect()
        });
        self.start_turn();
    }
    pub(crate) fn action_priority(&self, priority: Option<usize>) -> usize {
        priority.unwrap_or_else(|| {
            let winner = self.winner();
            if winner == 2 {
                self.rng.borrow_mut().randbelow(2)
            } else {
                winner as usize
            }
        })
    }

    pub(crate) fn resolve_action_plans(
        &mut self,
        plans: [Vec<Action>; 2],
        priority: usize,
    ) -> Result<TurnTrace, String> {
        self.resolve_action_plans_state(plans, priority, false)
    }

    /// Resolve actions staged by ordered phases (EndSinglePlayerStaging::OnEnter
    /// 0x7e08b20) without applying their stage-time effects a second time.
    pub(crate) fn resolve_already_staged(
        &mut self,
        plans: [Vec<Action>; 2],
        priority: usize,
    ) -> Result<TurnTrace, String> {
        self.resolve_action_plans_state(plans, priority, true)
    }

    fn resolve_action_plans_state(
        &mut self,
        plans: [Vec<Action>; 2],
        priority: usize,
        already_staged: bool,
    ) -> Result<TurnTrace, String> {
        let plays = std::array::from_fn(|seat| {
            let mut staged = Vec::new();
            for &action in &plans[seat] {
                match action {
                    Action::Play { card, lane } => staged.push((card, lane)),
                    Action::Return { card } => {
                        if let Some(index) = staged.iter().rposition(|&(id, _)| id == card) {
                            staged.remove(index);
                        }
                    }
                    _ => {}
                }
            }
            staged
                .into_iter()
                .map(|(card, lane)| Played {
                    card: self.cards[card].def.id,
                    lane,
                })
                .collect()
        });
        // Card::IsMovableByPlayer (0x7cb0cac) and Card::CanActivate
        // (0x7cb9a24) evaluate a player's own staging state. Validate each
        // plan on a separate copy, before the opponent's hidden staging can
        // move ongoing effects or otherwise change those checks.
        if !already_staged {
            let mut costs: [Vec<i32>; 2] = std::array::from_fn(|_| Vec::new());
            for seat in 0..2 {
                let mut rng = self.fork_rng();
                let mut view = self.fork(rng.as_mut());
                for &action in &plans[seat] {
                    let cost = match action {
                        Action::Play { card, .. } => view.card_cost(card),
                        Action::Activate { card } => view
                            .activation_definition(card, seat)
                            .map_or(0, |_| view.activation_cost(card, seat)),
                        Action::Move { .. } | Action::Return { .. } => 0,
                    };
                    view.stage_action(seat, action).map_err(|error| {
                    let lanes: Vec<_> = view.lanes.iter().enumerate().map(|(lane, loc)| {
                        (if loc.revealed { loc.def.id } else { "<hidden>" }, loc.revealed, loc.destroyed, view.lane_slot_capacity(lane),
                         loc.cards.iter().map(|&id| {
                             let card = &view.cards[id];
                             (if card.revealed || view.human_seat == Some(card.owner) { card.def.id } else { "<hidden>" }, card.owner, card.revealed)
                         }).collect::<Vec<_>>())
                    }).collect();
                    format!("{error}: turn {}, seat {seat}, action {action:?}, energy {:?}, cost {:?}, hand {:?}, lanes {lanes:?}",
                        view.turn, (view.human_seat == Some(seat)).then_some(view.players[seat].energy),
                        (view.human_seat == Some(seat)).then_some(cost),
                        view.players[seat].hand.iter().map(|&id| if view.human_seat == Some(seat) { view.cards[id].def.id } else { "<hidden>" }).collect::<Vec<_>>())
                })?;
                    costs[seat].push(cost);
                }
            }
            if !crate::cards::reference_rules_enabled() {
                self.reveal_priority = Some(priority);
            }
            for (seat, plan) in plans.iter().enumerate() {
                for (&action, &cost) in plan.iter().zip(&costs[seat]) {
                    self.apply_staged_action(seat, action, cost);
                }
            }
        }
        if already_staged && !crate::cards::reference_rules_enabled() {
            self.reveal_priority = Some(priority);
        }
        self.ongoing();
        self.resolve_staged_actions(priority);
        for player in &mut self.players {
            player.energy_history.push((
                self.turn,
                player.energy_spent_this_turn,
                player.energy == 0,
            ));
            player.turn_energy_history.push(TurnEnergyEvent {
                turn: self.turn,
                spent: player.energy_spent_this_turn,
                remaining: player.energy,
                bonus: player.energy - i32::from(self.turn) - player.default_energy_offset,
            });
        }
        self.end_turn_effects();
        let result = TurnTrace {
            turn: self.turn,
            priority,
            plays,
            powers: self.powers(),
        };
        if self.turn >= self.total_turns {
            self.finish_reveals();
            self.emit(Event::GameEnd);
            self.resolve_endgame_hidden_cards();
            self.ongoing();
        }
        if let Some(stakes) = &mut self.stakes {
            stakes.advance_turn();
        }
        self.turn += 1;
        Ok(result)
    }

    /// DaredevilBehavior::ForceOpponentToStageFirst @ 0x7e87de0.
    pub(crate) fn daredevil_viewer(&self) -> Option<usize> {
        if self.turn != 5 || cards::reference_rules_enabled() {
            return None;
        }
        let mut owners = [false; 2];
        for &id in self.lanes.iter().flat_map(|lane| &lane.cards) {
            let card = &self.cards[id];
            if card.revealed && !card.abilities_removed && card.default_ability().id == "Daredevil"
            {
                owners[card.owner] = true;
            }
        }
        match owners {
            [true, false] => Some(0),
            [false, true] => Some(1),
            _ => None,
        }
    }

    /// Play a new turn with player moves and activations enabled.
    pub fn play_turn_actions(
        &mut self,
        policy: crate::policy::Policy,
    ) -> Result<TurnTrace, String> {
        self.begin_turn();
        let priority = self.action_priority(None);
        let plans = {
            let mut rng = self.rng.borrow_mut();
            if let Some(viewer) = self.daredevil_viewer() {
                let first = 1 - viewer;
                let opponent = policy.plan_actions(self, first, &mut **rng);
                let mut fork_rng = rng.clone_rng();
                let mut view = self.fork(fork_rng.as_mut());
                for &action in &opponent {
                    view.stage_action(first, action)?;
                }
                let response = policy.plan_actions(&view, viewer, &mut **rng);
                if viewer == 0 {
                    [response, opponent]
                } else {
                    [opponent, response]
                }
            } else {
                [
                    policy.plan_actions(self, 0, &mut **rng),
                    policy.plan_actions(self, 1, &mut **rng),
                ]
            }
        };
        self.resolve_action_plans(plans, priority)
    }
    /// Start a turn, then allow stake decisions before either player stages cards.
    /// Returning false from `stakes` terminates the turn without resolving plays.
    pub fn play_turn_with_stakes<F, S>(
        &mut self,
        mut stakes: S,
        mut planner: F,
    ) -> Result<Option<TurnTrace>, String>
    where
        F: for<'b> FnMut(&Game<'b>, usize, usize) -> Result<Vec<Action>, String>,
        S: for<'b> FnMut(&mut Game<'b>) -> Result<bool, String>,
    {
        self.begin_turn();
        if !stakes(self)? {
            return Ok(None);
        }
        let priority = self.action_priority(None);
        let mut choose = |game: &Game<'_>, seat, priority| {
            if game
                .stakes
                .as_ref()
                .is_some_and(|stakes| stakes.pending_retreat[seat])
            {
                return Ok(Vec::new());
            }
            if let Some(controller) = game.autoplay_controller(seat) {
                let mut rng = self.rng.borrow_mut();
                Ok(game.autoplay_plan_with_rng(seat, controller, &mut **rng))
            } else {
                planner(game, seat, priority)
            }
        };
        let plans = if let Some(viewer) = self.daredevil_viewer() {
            let first = 1 - viewer;
            let opponent = choose(self, first, priority)?;
            let mut rng = self.fork_rng();
            let mut view = self.fork(rng.as_mut());
            for &action in &opponent {
                view.stage_action(first, action)?;
            }
            let response = choose(&view, viewer, priority)?;
            if viewer == 0 {
                [response, opponent]
            } else {
                [opponent, response]
            }
        } else {
            [choose(self, 0, priority)?, choose(self, 1, priority)?]
        };
        if self
            .stakes
            .as_ref()
            .is_some_and(|stakes| stakes.pending_retreat.iter().any(|&v| v))
        {
            for (seat, plan) in plans.into_iter().enumerate() {
                for action in plan {
                    self.stage_action(seat, action)?;
                }
            }
            self.ongoing();
            return Ok(None);
        }
        self.resolve_action_plans(plans, priority).map(Some)
    }

    pub fn play_turn_with_actions<F>(&mut self, mut planner: F) -> Result<TurnTrace, String>
    where
        F: for<'b> FnMut(&Game<'b>, usize, usize) -> Result<Vec<Action>, String>,
    {
        self.begin_turn();
        let priority = self.action_priority(None);
        let mut choose = |game: &Game<'_>, seat, priority| {
            if let Some(controller) = game.autoplay_controller(seat) {
                let mut rng = self.rng.borrow_mut();
                Ok(game.autoplay_plan_with_rng(seat, controller, &mut **rng))
            } else {
                planner(game, seat, priority)
            }
        };
        let plans = if let Some(viewer) = self.daredevil_viewer() {
            let first = 1 - viewer;
            let opponent = choose(self, first, priority)?;
            let mut rng = self.fork_rng();
            let mut view = self.fork(rng.as_mut());
            for &action in &opponent {
                view.stage_action(first, action)?;
            }
            let response = choose(&view, viewer, priority)?;
            if viewer == 0 {
                [response, opponent]
            } else {
                [opponent, response]
            }
        } else {
            [choose(self, 0, priority)?, choose(self, 1, priority)?]
        };
        self.resolve_action_plans(plans, priority)
    }

    /// Resolve an already-started position with both seats controlled by the policy.
    pub(crate) fn play_position_policy_turn(
        &mut self,
        policy: crate::policy::Policy,
        priority: Option<usize>,
        plays_only: bool,
    ) -> Result<TurnTrace, String> {
        self.pending_plays = std::array::from_fn(|seat| {
            self.delayed_reveals[seat]
                .iter()
                .filter_map(|queued| self.cards[queued.card].lane.map(|lane| (queued.card, lane)))
                .collect()
        });
        if plays_only && (0..2).any(|seat| self.autoplay_controller(seat).is_some()) {
            let priority = self.action_priority(priority);
            let plans = {
                let mut rng = self.rng.borrow_mut();
                if let Some(viewer) = self.daredevil_viewer() {
                    let first = 1 - viewer;
                    let plan = self.autoplay_or_plays(first, policy, &mut **rng);
                    let mut fork_rng = rng.clone_rng();
                    let mut view = self.fork(fork_rng.as_mut());
                    for &action in &plan {
                        view.stage_action(first, action)?;
                    }
                    let response = view.autoplay_or_plays(viewer, policy, &mut **rng);
                    if viewer == 0 {
                        [response, plan]
                    } else {
                        [plan, response]
                    }
                } else {
                    [
                        self.autoplay_or_plays(0, policy, &mut **rng),
                        self.autoplay_or_plays(1, policy, &mut **rng),
                    ]
                }
            };
            return self.resolve_action_plans(plans, priority);
        }
        if plays_only {
            return self.resolve_started_turn(priority, |game, _| {
                let mut rng = game.rng.borrow_mut();
                if let Some(viewer) = game.daredevil_viewer() {
                    let first = 1 - viewer;
                    let plan = policy.plan(game, first, &mut **rng);
                    let mut fork_rng = rng.clone_rng();
                    let mut view = game.fork(fork_rng.as_mut());
                    for &(card, lane) in &plan {
                        view.stage_action(first, Action::Play { card, lane })?;
                    }
                    let response = policy.plan(&view, viewer, &mut **rng);
                    Ok(if viewer == 0 {
                        [response, plan]
                    } else {
                        [plan, response]
                    })
                } else {
                    Ok([
                        policy.plan(game, 0, &mut **rng),
                        policy.plan(game, 1, &mut **rng),
                    ])
                }
            });
        }
        let priority = self.action_priority(priority);
        let plans = {
            let mut rng = self.rng.borrow_mut();
            if let Some(viewer) = self.daredevil_viewer() {
                let first = 1 - viewer;
                let plan = policy.plan_actions(self, first, &mut **rng);
                let mut fork_rng = rng.clone_rng();
                let mut view = self.fork(fork_rng.as_mut());
                for &action in &plan {
                    view.stage_action(first, action)?;
                }
                let response = policy.plan_actions(&view, viewer, &mut **rng);
                if viewer == 0 {
                    [response, plan]
                } else {
                    [plan, response]
                }
            } else {
                [
                    policy.plan_actions(self, 0, &mut **rng),
                    policy.plan_actions(self, 1, &mut **rng),
                ]
            }
        };
        self.resolve_action_plans(plans, priority)
    }

    /// Resolve a planning position with player moves and activations enabled.
    pub fn resolve_position_actions(
        &mut self,
        actions: &[Action],
        policy: crate::policy::Policy,
        priority: Option<usize>,
    ) -> Result<TurnTrace, String> {
        self.pending_plays = std::array::from_fn(|seat| {
            self.delayed_reveals[seat]
                .iter()
                .filter_map(|queued| self.cards[queued.card].lane.map(|lane| (queued.card, lane)))
                .collect()
        });
        let priority = self.action_priority(priority);
        let own = self.autoplay_controller(0).map_or_else(
            || actions.to_vec(),
            |controller| self.autoplay_plan(0, controller),
        );
        let opponent = {
            let mut rng = self.rng.borrow_mut();
            policy.plan_actions(self, 1, &mut **rng)
        };
        self.resolve_action_plans([own, opponent], priority)
    }

    pub fn play_turn(&mut self, policy: crate::policy::Policy) -> TurnTrace {
        self.play_turn_actions(policy)
            .expect("built-in action policies always plan legal actions")
    }

    /// Explicit frozen-reference playout: never voluntarily move or activate.
    pub fn play_turn_plays_only(&mut self, policy: crate::policy::Policy) -> TurnTrace {
        self.begin_turn();
        if (0..2).any(|seat| self.autoplay_controller(seat).is_some()) {
            return self
                .play_position_policy_turn(policy, None, true)
                .expect("built-in policies always plan");
        }
        self.resolve_started_turn(None, |game, _| {
            let mut rng = game.rng.borrow_mut();
            if let Some(viewer) = game.daredevil_viewer() {
                let first = 1 - viewer;
                let opponent = policy.plan(game, first, &mut **rng);
                let mut fork_rng = rng.clone_rng();
                let mut view = game.fork(fork_rng.as_mut());
                for &(card, lane) in &opponent {
                    view.stage_action(first, Action::Play { card, lane })?;
                }
                let response = policy.plan(&view, viewer, &mut **rng);
                Ok(if viewer == 0 {
                    [response, opponent]
                } else {
                    [opponent, response]
                })
            } else {
                Ok([
                    policy.plan(game, 0, &mut **rng),
                    policy.plan(game, 1, &mut **rng),
                ])
            }
        })
        .expect("built-in policies always plan")
    }

    /// Play a turn whose plans come from `planner(game, seat, priority)`, called for seat
    /// 0 and then seat 1 after the turn has started and reveal priority was chosen (the
    /// same RNG order as Python's turn loop). Plans hold arena card indices and lanes.
    pub fn play_turn_with<F>(&mut self, mut planner: F) -> Result<TurnTrace, String>
    where
        F: for<'b> FnMut(&Game<'b>, usize, usize) -> Result<Vec<(usize, usize)>, String>,
    {
        self.begin_turn();
        if (0..2).any(|seat| self.autoplay_controller(seat).is_some()) {
            let priority = self.action_priority(None);
            let mut choose = |game: &Game<'_>, seat| -> Result<Vec<Action>, String> {
                if let Some(controller) = game.autoplay_controller(seat) {
                    let mut rng = self.rng.borrow_mut();
                    Ok(game.autoplay_plan_with_rng(seat, controller, &mut **rng))
                } else {
                    Ok(planner(game, seat, priority)?
                        .into_iter()
                        .map(|(card, lane)| Action::Play { card, lane })
                        .collect())
                }
            };
            let plans = if let Some(viewer) = self.daredevil_viewer() {
                let first = 1 - viewer;
                let plan = choose(self, first)?;
                let mut rng = self.fork_rng();
                let mut view = self.fork(rng.as_mut());
                for &action in &plan {
                    view.stage_action(first, action)?;
                }
                let response = choose(&view, viewer)?;
                if viewer == 0 {
                    [response, plan]
                } else {
                    [plan, response]
                }
            } else {
                [choose(self, 0)?, choose(self, 1)?]
            };
            return self.resolve_action_plans(plans, priority);
        }
        self.resolve_started_turn(None, |game, priority| {
            if let Some(viewer) = game.daredevil_viewer() {
                let first = 1 - viewer;
                let opponent = planner(game, first, priority)?;
                let mut rng = game.fork_rng();
                let mut view = game.fork(rng.as_mut());
                for &(card, lane) in &opponent {
                    view.stage_action(first, Action::Play { card, lane })?;
                }
                let response = planner(&view, viewer, priority)?;
                Ok(if viewer == 0 {
                    [response, opponent]
                } else {
                    [opponent, response]
                })
            } else {
                Ok([planner(game, 0, priority)?, planner(game, 1, priority)?])
            }
        })
    }

    /// Resolve a materialized planning state without drawing or starting the turn again.
    /// `my_plan` contains arena card indices, not hand slots.
    pub fn resolve_position_turn(
        &mut self,
        my_plan: &[(usize, usize)],
        policy: crate::policy::Policy,
        priority: Option<usize>,
    ) -> TurnTrace {
        if (0..2).any(|seat| self.autoplay_controller(seat).is_some()) {
            let priority = self.action_priority(priority);
            let own = self.autoplay_controller(0).map_or_else(
                || {
                    my_plan
                        .iter()
                        .map(|&(card, lane)| Action::Play { card, lane })
                        .collect()
                },
                |controller| self.autoplay_plan(0, controller),
            );
            let opponent = {
                let mut rng = self.rng.borrow_mut();
                self.autoplay_or_plays(1, policy, &mut **rng)
            };
            return self
                .resolve_action_plans([own, opponent], priority)
                .expect("built-in policies always plan");
        }
        self.pending_plays = std::array::from_fn(|seat| {
            self.delayed_reveals[seat]
                .iter()
                .filter_map(|queued| self.cards[queued.card].lane.map(|lane| (queued.card, lane)))
                .collect()
        });
        self.resolve_started_turn(priority, |game, _| {
            let mut rng = game.rng.borrow_mut();
            Ok([my_plan.to_vec(), policy.plan(game, 1, &mut **rng)])
        })
        .expect("built-in policies always plan")
    }

    /// Resolve all remaining face-down cards at the completed game boundary.
    pub fn finish_reveals(&mut self) {
        self.delays_reveals = false;
        self.ending_reveals = true;
        self.expire_location_silence(true);
        let pending = std::mem::take(&mut self.delayed_reveals);
        // EndTurn::GetOrderedStagedCardsThisGameForResolve @ 0x7e0a8e0
        // enumerates players in resolution order. Preserve the reference order
        // when no persistent lane delay was involved.
        let persistent = pending.iter().flatten().any(|queued| {
            queued.persistent
                || self.cards[queued.card].lane.is_some_and(|lane| {
                    self.lanes[lane].revealed && self.lanes[lane].def.delays_reveals_until_end
                })
        });
        let first = if persistent {
            self.reveal_priority.unwrap_or(0)
        } else {
            0
        };
        for seat in [first, 1 - first] {
            for queued in &pending[seat] {
                self.reveal_with_staging(queued.card, queued.staging);
            }
        }
        let unresolved: Vec<_> = self
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| !self.cards[id].revealed)
            .collect();
        for id in unresolved {
            self.reveal(id);
        }
        self.ending_reveals = false;
    }

    /// Dispatch card-owned proximity triggers independently of current text.
    pub fn resolve_attached_watches(&mut self, card: usize) {
        let owner = self.cards[card].owner;
        let abilities = std::mem::take(&mut self.cards[card].attached_abilities);
        for ability in abilities {
            if let AttachedAbilityKind::Watch {
                turn,
                lane,
                expect,
                power,
                staged_only,
            } = ability.kind
                && turn == self.turn
            {
                let played = if staged_only {
                    self.played[owner].iter().any(|&(id, played_lane)| {
                        played_lane == lane && self.cards[id].turn_played == self.turn
                    })
                } else {
                    self.played_at(owner, lane)
                };
                if played == expect {
                    self.add_power(card, power, owner);
                }
            } else {
                self.cards[card].attached_abilities.push(ability);
            }
        }
    }

    /// Number of executions of an eligible revealed card's EndOfTurn trigger.
    pub(crate) fn end_turn_repetitions(&self, card: usize) -> u32 {
        let target = &self.cards[card];
        let player = &self.players[target.owner];
        if target.revealed
            && target.lane.is_some()
            && player.end_turn_multiplier_turn == Some(self.turn)
        {
            1 + player.end_turn_multiplier
        } else {
            1
        }
    }

    /// Apply the effects at the end of an already-revealed turn.
    pub fn end_turn_effects(&mut self) {
        let interleave = crate::cards::reference_rules_enabled();
        let observers: Vec<_> = if interleave {
            self.lanes
                .iter()
                .flat_map(|lane| lane.cards.iter().copied())
                .filter(|&id| self.cards[id].revealed)
                .collect()
        } else {
            self.revealed_cards_in_resolution_order()
        };
        // EndTurn::OnEnter (0x7e09558) dispatches PreEndOfTurn first.
        // GetAllRevealedCards (0x7dac98c) sorts by LatestResolutionOrder
        // (key 0x7dc6bfc); Game::ActivateTriggerAbility (0x7da93a0)
        // visits locations before that card list, independent of player priority.
        // The frozen reference instead interleaves the pre/end hooks per card.
        let turn = self.turn;
        let expire = |game: &mut Self, id: usize| {
            game.cards[id].attached_abilities.retain(|ability| !matches!(
                ability.kind, AttachedAbilityKind::MovePower { turn: expiration, .. } if expiration <= turn
            ));
        };
        if !interleave {
            for &id in &observers {
                expire(self, id);
                if self.card_triggers_allowed(id) {
                    self.resolve_attached_watches(id);
                }
                if !self.cards[id].abilities_removed
                    && self.card_triggers_allowed(id)
                    && let Some(hook) = self.cards[id].default_ability().pre_end_turn
                {
                    let _ = self.counted_trigger(|game| {
                        game.record_card_hook(game.cards[id].default_ability().id, "pre_end_turn");
                        hook(game, id)
                    });
                }
            }
        }
        if interleave {
            for &id in &observers {
                expire(self, id);
                if self.card_triggers_allowed(id) {
                    self.resolve_attached_watches(id);
                }
                if !self.cards[id].abilities_removed
                    && self.card_triggers_allowed(id)
                    && let Some(hook) = self.cards[id].default_ability().end_turn
                {
                    for _ in 0..self.end_turn_repetitions(id) {
                        let _ = self.counted_trigger(|game| {
                            game.record_card_hook(game.cards[id].default_ability().id, "end_turn");
                            hook(game, id)
                        });
                    }
                }
            }
            let hand_observers: Vec<_> = self
                .players
                .iter()
                .flat_map(|player| player.hand.iter().copied())
                .filter(|&id| self.cards[id].default_ability().end_turn_in_hand)
                .collect();
            for id in hand_observers {
                let owner = self.cards[id].owner;
                if self.players[owner].hand.contains(&id)
                    && !self.cards[id].abilities_removed
                    && let Some(hook) = self.cards[id].default_ability().end_turn
                {
                    let _ = self.counted_trigger(|game| {
                        game.record_card_hook(game.cards[id].default_ability().id, "end_turn");
                        hook(game, id)
                    });
                }
            }
            self.ongoing();
        }
        self.end_turn_lane_order = Some([0, 1, 2]);
        for original in 0..3 {
            let lane = self.end_turn_lane_order.unwrap()[original];
            if self.location_abilities_active(lane) {
                if let Some(hook) = self.lanes[lane].def.on_turn_end {
                    let _ = self.counted_trigger(|game| {
                        game.record_location_hook(lane, "on_turn_end");
                        hook(game, lane)
                    });
                    self.ongoing();
                }
                if !interleave && let Some(hook) = self.lanes[lane].def.on_event {
                    let _ = self.counted_trigger(|game| {
                        game.record_location_hook(lane, "on_event");
                        hook(game, lane, Event::TurnEnd)
                    });
                }
            }
        }
        self.end_turn_lane_order = None;
        if interleave {
            self.emit(Event::TurnEnd);
        } else {
            for id in self.revealed_cards_in_resolution_order() {
                if self.cards[id].lane.is_none() || !self.card_triggers_allowed(id) {
                    continue;
                }
                if !self.cards[id].abilities_removed
                    && let Some(hook) = self.cards[id].default_ability().end_turn
                {
                    for _ in 0..self.end_turn_repetitions(id) {
                        self.execute_default_ability(id, |game, card| {
                            game.record_card_hook(
                                game.cards[card].default_ability().id,
                                "end_turn",
                            );
                            hook(game, card)
                        });
                    }
                }
                let attached: Vec<_> = self.cards[id]
                    .composed_abilities
                    .iter()
                    .filter(|ability| ability.kind == AbilityKind::Trigger)
                    .filter_map(|ability| {
                        ability
                            .def
                            .end_turn
                            .map(|hook| (ability.id, ability.def.id, hook, ability.hidden))
                    })
                    .collect();
                for (instance, def_id, hook, hidden) in attached {
                    let repeats = if hidden {
                        1
                    } else {
                        self.end_turn_repetitions(id)
                    };
                    for _ in 0..repeats {
                        self.execute_ability(id, instance, |game, card| {
                            game.record_card_hook(def_id, "end_turn");
                            hook(game, card)
                        });
                    }
                }
                self.dispatch_card_event(id, Event::TurnEnd, true, false);
            }
            let observers: Vec<_> = self
                .players
                .iter()
                .flat_map(|player| player.hand.iter().chain(&player.deck).copied())
                .collect();
            for id in observers {
                let owner = self.cards[id].owner;
                if self.players[owner].hand.contains(&id)
                    && !self.cards[id].abilities_removed
                    && self.cards[id].default_ability().end_turn_in_hand
                    && let Some(hook) = self.cards[id].default_ability().end_turn
                {
                    self.execute_default_ability(id, |game, card| {
                        game.record_card_hook(game.cards[card].default_ability().id, "end_turn");
                        hook(game, card)
                    });
                }
                self.dispatch_card_event(id, Event::TurnEnd, true, false);
            }
        }
        for seat in 0..2 {
            self.trigger_player_effects(seat, PlayerTrigger::EndTurn, None);
        }
        self.ongoing();
        for lane in 0..3 {
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if self.cards[id].revealed
                    && !self.cards[id].abilities_removed
                    && let Some(hook) = self.cards[id].default_ability().objective_freeze
                {
                    self.execute_default_ability(id, |game, card| {
                        game.record_card_hook(
                            game.cards[card].default_ability().id,
                            "objective_freeze",
                        );
                        hook(game, card)
                    });
                }
            }
        }
        self.emit(Event::AfterTurnEnd);
        // SandcastleBehavior (0x7d7e0fc) and TVA behavior (0x7d8f724)
        // end the match after this turn's effects, without shortening an
        // already-started turn.
        if self.lanes.iter().enumerate().any(|(index, lane)| {
            self.location_abilities_active(index)
                && lane.def.maximum_turns != 0
                && if cards::reference_rules_enabled() {
                    self.turn >= lane.def.maximum_turns
                } else {
                    self.turn == lane.def.maximum_turns
                }
        }) {
            self.set_total_turns(self.turn);
        }
    }

    fn resolve_started_turn<P>(
        &mut self,
        priority: Option<usize>,
        plan: P,
    ) -> Result<TurnTrace, String>
    where
        P: FnOnce(&Self, usize) -> Result<[Vec<(usize, usize)>; 2], String>,
    {
        let pending = std::mem::take(&mut self.delayed_reveals);
        let winner = self.winner();
        let priority = priority.unwrap_or_else(|| {
            if winner == 2 {
                self.rng.borrow_mut().randbelow(2)
            } else {
                winner as usize
            }
        });
        let plans = plan(self, priority)?;
        if !crate::cards::reference_rules_enabled() {
            self.reveal_priority = Some(priority);
        }
        let plays = std::array::from_fn(|seat| {
            plans[seat]
                .iter()
                .map(|&(card, lane)| Played {
                    card: self.cards[card].def.id,
                    lane,
                })
                .collect()
        });
        for (seat, plan) in plans.into_iter().enumerate() {
            self.played[seat].extend(plan);
        }
        for seat in 0..2 {
            for index in 0..self.played[seat].len() {
                let (id, lane) = self.played[seat][index];
                if self.cards[id].lane.is_some() {
                    continue;
                }
                let slot = self.players[seat]
                    .hand
                    .iter()
                    .position(|&card| card == id)
                    .expect("planned card not in hand");
                self.emit(Event::CardStaged { card: id, seat });
                self.take_hand(seat, slot);
                let spent = self.card_cost(id);
                self.players[seat].energy_spent_this_turn += spent;
                let energy = self.players[seat].energy - spent;
                self.set_energy(seat, energy);
                self.cards[id].lane = Some(lane);
                self.cards[id].turn_played = self.turn;
                let entry = StagedPlay {
                    turn: self.turn,
                    card: id,
                    def_id: self.cards[id].def.id,
                    printed_cost: self.printed_cost(id),
                    created: self.cards[id].created,
                    resolved: false,
                };
                self.players[seat].staged_history.push(entry);
                self.players[seat]
                    .staged_lane_history
                    .push((self.turn, lane));
                self.push_location(lane, id);
                self.notify_added(id, lane);
            }
        }
        if self.played.iter().any(|plays| !plays.is_empty()) {
            self.ongoing();
        }
        self.is_wait_for_staging = false;
        self.location_staging_end();
        for seat in [priority, 1 - priority] {
            for queued in &pending[seat] {
                self.reveal_with_staging(queued.card, queued.staging);
            }
            for index in 0..self.played[seat].len() {
                let (id, lane) = self.played[seat][index];
                self.reveal_with_staging(id, Some(StagingContext { seat, lane }));
            }
        }
        for player in &mut self.players {
            player.energy_history.push((
                self.turn,
                player.energy_spent_this_turn,
                player.energy == 0,
            ));
            player.turn_energy_history.push(TurnEnergyEvent {
                turn: self.turn,
                spent: player.energy_spent_this_turn,
                remaining: player.energy,
                bonus: player.energy - i32::from(self.turn) - player.default_energy_offset,
            });
        }
        self.end_turn_effects();
        let result = TurnTrace {
            turn: self.turn,
            priority,
            plays,
            powers: self.powers(),
        };
        if self.turn >= self.total_turns {
            self.finish_reveals();
            self.emit(Event::GameEnd);
            self.resolve_endgame_hidden_cards();
            self.ongoing();
        }
        if let Some(stakes) = &mut self.stakes {
            stakes.advance_turn();
        }
        self.turn += 1;
        Ok(result)
    }
}

#[cfg(test)]
mod staging_tests {
    use super::*;
    use crate::random::PythonRandom;

    /// Exercises dynamically chosen card IDs with debug assertions enabled.
    #[test]
    #[ignore = "runs 3000 full random games"]
    fn seeded_registered_card_and_location_smoke() {
        let pool = crate::cards::deck_eligible_ids();
        let location_pool = crate::locations::REGISTRY;
        assert!(pool.len() >= 12 && location_pool.len() >= 3);
        for seed in 0..3000_i128 {
            let mut setup = PythonRandom::seed(seed);
            let mut choices = pool.to_vec();
            setup.shuffle(&mut choices);
            let decks: [Vec<String>; 2] = std::array::from_fn(|seat| {
                (0..12)
                    .map(|i| {
                        choices[(seed as usize * 24 + seat * 12 + i) % choices.len()].to_owned()
                    })
                    .collect()
            });
            let mut locations: Vec<_> = location_pool.iter().map(|def| def.id).collect();
            setup.shuffle(&mut locations);
            let lanes: Vec<String> = locations[..3].iter().map(|id| (*id).to_owned()).collect();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut rng = PythonRandom::seed(seed);
                let mut game = Game::new([&decks[0], &decks[1]], &lanes, &mut rng).unwrap();
                while game.turn < game.total_turns {
                    game.play_turn(crate::policy::Policy::Spread);
                    for (lane, location) in game.lanes.iter().enumerate() {
                        for &card in &location.cards {
                            assert_eq!(game.cards[card].lane, Some(lane));
                            assert!(game.cards[card].owner < 2);
                        }
                    }
                }
                game.finish_reveals();
            }));
            if let Err(error) = result {
                std::panic::resume_unwind(Box::new(format!(
                    "smoke seed {seed}, decks {decks:?}, locations {lanes:?}: {error:?}"
                )));
            }
        }
    }

    #[test]
    fn literal_card_creation_and_transformation_targets_are_registered() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        const IDENTITY_ONLY: &[(&str, &str)] = &[
            ("AgathaHarkness", "excluded from random card selection"),
            (
                "CaptainAmericaAvengers",
                "identity check for compatible Captain America",
            ),
            (
                "CaptainAmericaChampion",
                "identity check for compatible Captain America",
            ),
            ("Thanos", "identity check for Infinity Stone effects"),
            (
                "ThanosChampion",
                "identity check for Infinity Stone effects",
            ),
        ];
        let mut missing = Vec::new();
        for directory in ["cards", "locations"] {
            for entry in std::fs::read_dir(root.join(directory)).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().is_none_or(|ext| ext != "rs") {
                    continue;
                }
                // Client-wide generated inventories, not effect target lists:
                // parity/generate_no_text.py and parity/generate_ongoing_priorities.py.
                if directory == "cards"
                    && path
                        .file_name()
                        .is_some_and(|name| name == "no_text.rs" || name == "ongoing_priorities.rs")
                {
                    continue;
                }
                let contents = std::fs::read_to_string(&path).unwrap();
                let source = contents.split("#[cfg(test)]").next().unwrap();
                for literal in source.split('"').skip(1).step_by(2) {
                    if CATALOG.contains_key(literal)
                        && cards::get(literal).is_none()
                        && !IDENTITY_ONLY.iter().any(|(id, _)| *id == literal)
                    {
                        missing.push(format!("{}: {literal}", path.display()));
                    }
                }
                for (method, argument) in [
                    (".create_card(", 0),
                    (".transform_card(", 1),
                    (".transform_card_without_modifiers(", 1),
                ] {
                    for tail in source.split(method).skip(1) {
                        let mut depth = 0;
                        let mut start = 0;
                        let mut current = 0;
                        for (index, byte) in tail.bytes().enumerate() {
                            match byte {
                                b'(' | b'[' | b'{' => depth += 1,
                                b')' | b']' | b'}' if depth > 0 => depth -= 1,
                                b',' | b')' if depth == 0 => {
                                    if current == argument {
                                        let value = tail[start..index].trim();
                                        if let Some(id) = value
                                            .strip_prefix('"')
                                            .and_then(|value| value.strip_suffix('"'))
                                            && (cards::get(id).is_none()
                                                || !CATALOG.contains_key(id))
                                        {
                                            missing.push(format!("{}: {id}", path.display()));
                                        }
                                        break;
                                    }
                                    if byte == b')' {
                                        break;
                                    }
                                    current += 1;
                                    start = index + 1;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "unregistered card literals: {missing:?}"
        );
    }

    fn game(rng: &mut PythonRandom) -> Game<'_> {
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned)
        .to_vec();
        let locations = ["Ruins", "Xandar", "KamarTaj"].map(str::to_owned).to_vec();
        let mut game = Game::new([&deck, &deck], &locations, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = crate::locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game.players[0].energy = 6;
        game.players[1].energy = 6;
        game
    }

    #[test]
    fn ongoing_metadata_lookup_matches_original_order_and_fallback() {
        let rows = crate::cards::ongoing_priorities::ONGOING_PRIORITIES;
        assert!(rows.windows(2).all(|pair| pair[0].0 <= pair[1].0));
        for def in crate::cards::registry() {
            for hand in [false, true] {
                let expected = rows
                    .iter()
                    .find(|row| row.0 == def.id && row.2 == "Ongoing")
                    .or_else(|| {
                        hand.then(|| {
                            rows.iter()
                                .find(|row| row.0 == def.id && row.2 == "InHandOngoing")
                        })
                        .flatten()
                    });
                if let Some(expected) = expected {
                    assert_eq!(Game::client_ongoing_metadata(def, hand), expected);
                }
            }
        }
    }

    #[test]
    fn empty_staging_refresh_matches_full_scheduler() {
        let mut rng = PythonRandom::seed(400);
        let mut source = game(&mut rng);
        source.lanes[0].def = crate::locations::get("Elysium").unwrap();
        let blue = source.create_card("BlueMarvel", 0);
        let target = source.create_card("Hulk", 0);
        assert!(source.add_card(blue, 1, 0, false));
        assert!(source.add_card(target, 2, 0, false));
        source.is_wait_for_staging = false;
        source.ongoing();
        source.is_wait_for_staging = true;
        assert!(!source.staging_refresh_needed());
        let mut fast_rng = source.fork_rng();
        let mut full_rng = source.fork_rng();
        let mut fast = source.fork(fast_rng.as_mut());
        let mut full = source.fork(full_rng.as_mut());
        fast.client_ongoing();
        full.client_ongoing_scheduled();
        assert_eq!(
            crate::position::state_json(&fast),
            crate::position::state_json(&full)
        );
        for id in 0..fast.cards.len() {
            assert_eq!(fast.card_cost(id), full.card_cost(id));
            assert_eq!(fast.cards[id].power(), full.cards[id].power());
            assert_eq!(
                format!("{:?}", fast.cards[id].stat_updates),
                format!("{:?}", full.cards[id].stat_updates)
            );
        }
        assert_eq!(
            fast.rng.borrow_mut().getrandbits(64),
            full.rng.borrow_mut().getrandbits(64)
        );
    }

    #[test]
    fn staging_preserves_non_staging_power_and_cost_contributions() {
        let mut rng = PythonRandom::seed(401);
        let mut game = game(&mut rng);
        game.lanes[0].def = crate::locations::get("Elysium").unwrap();
        let source = game.create_card("BlueMarvel", 0);
        let target = game.create_card("Hulk", 0);
        let hand = game.create_card("Hulk", 0);
        assert!(game.add_card(source, 1, 0, false));
        assert!(game.add_card(target, 2, 0, false));
        assert!(game.add_hand(0, hand, false));
        game.is_wait_for_staging = false;
        game.ongoing();
        let power = game.cards[target].power();
        let cost = game.card_cost(hand);
        assert_eq!(power, game.printed_power(target) + 1);
        assert_eq!(cost, game.printed_cost(hand) - 1);
        game.is_wait_for_staging = true;
        game.ongoing();
        game.ongoing();
        assert_eq!(game.cards[target].power(), power);
        assert_eq!(game.card_cost(hand), cost);
    }

    #[test]
    fn staging_replaces_ability_contributions_by_source() {
        fn contribution(game: &mut Game<'_>, card: usize) {
            game.add_ongoing_power(card, i32::from(game.turn), 0);
            game.add_ongoing_cost(card, i32::from(game.turn), 0);
        }
        // Agony's client ongoing metadata is active while staging.
        static TEST_ABILITY: CardDef = CardDef {
            ongoing: Some(contribution),
            ..CardDef::vanilla("Agony")
        };
        let mut rng = PythonRandom::seed(402);
        let mut game = game(&mut rng);
        let card = game.create_card("Agony", 0);
        game.cards[card].override_ability = Some(&TEST_ABILITY);
        assert!(game.add_card(card, 0, 0, false));
        game.turn = 1;
        game.is_wait_for_staging = false;
        game.ongoing();
        game.turn = 2;
        game.is_wait_for_staging = true;
        assert!(game.staging_refresh_needed());
        game.ongoing();
        let power = game.cards[card].power();
        let cost = game.card_cost(card);
        game.ongoing();
        assert_eq!(game.cards[card].power(), power);
        assert_eq!(game.card_cost(card), cost);
        assert_eq!(power, game.printed_power(card) + 2);
        assert_eq!(cost, game.printed_cost(card) + 2);
        // A later, independent Set must remain after this ability's Add.
        game.set_ongoing_cost(card, 7, 0);
        game.turn = 3;
        game.ongoing();
        assert_eq!(game.card_cost(card), 7);
    }

    #[test]
    fn failed_conditional_ability_retries_after_later_success() {
        // Lancelot's HasTargets condition (0x7d29874) initially fails;
        // Captain Carter (0x7e7d22c) then increases the back-row card.
        let mut rng = PythonRandom::seed(403);
        let mut game = game(&mut rng);
        let lancelot = game.create_card("Lancelot", 0);
        let carter = game.create_card("CaptainCarter", 0);
        let filler = game.create_card("Wasp", 0);
        let target = game.create_card("MistyKnight", 0);
        assert!(game.add_card(lancelot, 0, 0, false));
        assert!(game.add_card(carter, 1, 0, false));
        assert!(game.add_card(filler, 1, 0, false));
        assert!(game.add_card(target, 1, 0, false));
        game.is_wait_for_staging = false;
        game.ongoing();
        assert_eq!(
            game.cards[target].power(),
            game.printed_power(target) + game.cards[carter].power() + 1
        );
    }
    #[test]
    fn copied_conditional_retries_after_later_success() {
        let mut rng = PythonRandom::seed(404);
        let mut game = game(&mut rng);
        let skrull = game.create_card("SuperSkrull", 0);
        let lancelot = game.create_card("Lancelot", 1);
        let carter = game.create_card("CaptainCarter", 0);
        let filler = game.create_card("Wasp", 0);
        let target = game.create_card("MistyKnight", 0);
        assert!(game.add_card(skrull, 0, 0, false));
        assert!(game.add_card(lancelot, 2, 1, false));
        assert!(game.add_card(carter, 1, 0, false));
        assert!(game.add_card(filler, 1, 0, false));
        assert!(game.add_card(target, 1, 0, false));
        game.is_wait_for_staging = false;
        game.ongoing();
        assert_eq!(
            game.cards[target].power(),
            game.printed_power(target) + game.cards[carter].power() + 1
        );
    }

    #[test]
    fn attached_reveals_run_in_independent_context_even_after_default_removal() {
        fn increment(game: &mut Game<'_>, card: usize) {
            let value = game.cards[card]
                .ability_state_mut()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<u32>()
                .unwrap();
            *value += 1;
        }
        static REVEAL: CardDef = CardDef {
            reveal: Some(increment),
            ..CardDef::vanilla("RevealContextTest")
        };
        let mut rng = PythonRandom::seed(5);
        let mut game = game(&mut rng);
        let card = place(&mut game, "Wasp", 0, 0);
        game.cards[card].abilities_removed = true;
        game.cards[card]
            .composed_abilities
            .push(AbilityInstance::new(
                &REVEAL,
                AbilityKind::Reveal,
                None,
                false,
                true,
                Some(Box::new(3_u32)),
            ));
        assert!(game.cards[card].has_resolve_trigger());
        game.trigger_on_reveal(card);
        assert_eq!(
            game.cards[card].composed_abilities[0]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<u32>(),
            Some(&4)
        );
    }

    #[test]
    fn start_turn_triggers_follow_latest_resolution_order() {
        fn add(game: &mut Game<'_>, _: usize) {
            game.players[0].energy += 1;
        }
        fn double(game: &mut Game<'_>, _: usize) {
            game.players[0].energy *= 2;
        }
        static ADD: CardDef = CardDef {
            start_turn: Some(add),
            ..CardDef::vanilla("StartOrderAdd")
        };
        static DOUBLE: CardDef = CardDef {
            start_turn: Some(double),
            ..CardDef::vanilla("StartOrderDouble")
        };
        for (add_order, double_order, expected) in [(1, 2, 4), (2, 1, 3)] {
            let mut rng = PythonRandom::seed(19);
            let mut game = game(&mut rng);
            let add_card = place(&mut game, "Wasp", 0, 0);
            let double_card = place(&mut game, "Wasp", 0, 1);
            game.cards[add_card].override_ability = Some(&ADD);
            game.cards[double_card].override_ability = Some(&DOUBLE);
            game.cards[add_card].latest_resolution_order = add_order;
            game.cards[double_card].latest_resolution_order = double_order;
            game.turn = 1;
            game.start_turn();
            assert_eq!(game.players[0].energy, expected);
        }
    }

    #[test]
    fn draw_observers_follow_latest_resolution_order() {
        fn add(game: &mut Game<'_>, _: usize, event: Event) {
            if matches!(event, Event::CardDrawn { .. }) {
                game.players[0].energy += 1;
            }
        }
        fn double(game: &mut Game<'_>, _: usize, event: Event) {
            if matches!(event, Event::CardDrawn { .. }) {
                game.players[0].energy *= 2;
            }
        }
        static ADD: CardDef = CardDef {
            on_event: Some(add),
            ..CardDef::vanilla("DrawOrderAdd")
        };
        static DOUBLE: CardDef = CardDef {
            on_event: Some(double),
            ..CardDef::vanilla("DrawOrderDouble")
        };
        for (add_order, double_order, expected) in [(1, 2, 4), (2, 1, 3)] {
            let mut rng = PythonRandom::seed(20);
            let mut game = game(&mut rng);
            let add_card = place(&mut game, "Wasp", 0, 0);
            let double_card = place(&mut game, "Wasp", 0, 1);
            game.cards[add_card].override_ability = Some(&ADD);
            game.cards[double_card].override_ability = Some(&DOUBLE);
            game.cards[add_card].latest_resolution_order = add_order;
            game.cards[double_card].latest_resolution_order = double_order;
            game.players[0].energy = 1;
            game.emit(Event::CardDrawn {
                card: add_card,
                seat: 0,
            });
            assert_eq!(game.players[0].energy, expected);
        }
    }

    #[test]
    fn attached_start_and_activation_use_own_context() {
        fn increment(game: &mut Game<'_>, card: usize) {
            *game.cards[card]
                .ability_state_mut()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<u32>()
                .unwrap() += 1;
        }
        static START: CardDef = CardDef {
            start_turn: Some(increment),
            ..CardDef::vanilla("AttachedStartTest")
        };
        static ACTIVATE: CardDef = CardDef {
            on_activate: Some(increment),
            activate_cost: 2,
            ..CardDef::vanilla("AttachedActivateTest")
        };
        let mut rng = PythonRandom::seed(15);
        let mut game = game(&mut rng);
        let card = place(&mut game, "Wasp", 0, 0);
        game.cards[card].abilities_removed = true;
        game.cards[card]
            .composed_abilities
            .push(AbilityInstance::new(
                &START,
                AbilityKind::Trigger,
                None,
                false,
                true,
                Some(Box::new(0_u32)),
            ));
        game.cards[card]
            .composed_abilities
            .push(AbilityInstance::new(
                &ACTIVATE,
                AbilityKind::Activate,
                None,
                false,
                true,
                Some(Box::new(0_u32)),
            ));
        game.start_turn();
        assert_eq!(
            game.cards[card].composed_abilities[0]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<u32>(),
            Some(&1)
        );
        game.players[0].energy = 2;
        assert!(game.activate_card(card, 0));
        assert_eq!(
            game.players[0].energy,
            if cards::reference_rules_enabled() {
                0
            } else {
                2
            }
        );
        assert_eq!(
            game.cards[card].composed_abilities[1]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<u32>(),
            Some(&1)
        );
    }

    #[test]
    fn activation_uses_printed_cost_and_visible_ability() {
        if cards::reference_rules_enabled() {
            return;
        }
        fn increment(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 1;
        }
        fn no_targets(_: &Game<'_>, _: usize) -> bool {
            false
        }
        static PRINTED: CardDef = CardDef {
            activate_cost: 2,
            ..CardDef::vanilla("PrintedActivateCostTest")
        };
        static ATTACHED: CardDef = CardDef {
            on_activate: Some(increment),
            can_activate: Some(no_targets),
            ..CardDef::vanilla("AttachedActivateCostTest")
        };
        let mut rng = PythonRandom::seed(151);
        let mut game = game(&mut rng);
        let card = place(&mut game, "Wasp", 0, 0);
        let initial_power = game.cards[card].permanent;
        game.cards[card].def = &PRINTED;
        game.cards[card]
            .composed_abilities
            .push(AbilityInstance::new(
                &ATTACHED,
                AbilityKind::Activate,
                None,
                true,
                true,
                None,
            ));
        game.players[0].energy = 2;
        assert!(!game.can_activate(card, 0));
        game.cards[card].composed_abilities[0].hidden = false;
        game.players[0].energy = 1;
        assert!(!game.can_activate(card, 0));
        game.players[0].energy = 2;
        assert!(game.stage_action(0, Action::Activate { card }).is_ok());
        assert_eq!(game.players[0].energy, 0);
        assert!(!game.can_activate(card, 0));
        game.resolve_staged_actions(0);
        assert!(game.cards[card].activated);
        assert_eq!(game.cards[card].permanent, initial_power + 1);
    }

    #[test]
    fn trigger_suppression_does_not_alone_block_staging_activation() {
        if cards::reference_rules_enabled() {
            return;
        }
        fn increment(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 1;
        }
        static ACTIVATE: CardDef = CardDef {
            on_activate: Some(increment),
            ..CardDef::vanilla("SuppressedActivateTest")
        };
        let mut rng = PythonRandom::seed(152);
        let mut game = game(&mut rng);
        let card = place(&mut game, "Wasp", 0, 0);
        game.cards[card].override_ability = Some(&ACTIVATE);
        game.lanes[0].ability_flags.trigger = false;
        assert!(game.can_activate(card, 0));
    }

    #[test]
    fn deep_space_blocks_trigger_and_activate() {
        fn increment(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 1;
        }
        static DEEP: LocationDef = locations::location("DeepSpace", 0, 0, 1, false);
        static ABILITY: CardDef = CardDef {
            end_turn: Some(increment),
            on_activate: Some(increment),
            ..CardDef::vanilla("DeepSpaceTest")
        };
        let mut rng = PythonRandom::seed(17);
        let mut game = game(&mut rng);
        let card = place(&mut game, "Wasp", 0, 0);
        game.cards[card].override_ability = Some(&ABILITY);
        game.replace_location_def(0, &DEEP, None);
        let before = game.cards[card].permanent;
        assert!(!game.can_activate(card, 0));
        game.end_turn_effects();
        assert_eq!(game.cards[card].permanent, before);
        game.replace_location(0, "Ruins");
        assert!(game.can_activate(card, 0));
        game.end_turn_effects();
        assert_eq!(game.cards[card].permanent, before + 1);
    }

    #[test]
    fn initial_hand_prioritizes_self_draw_only_during_opening() {
        let mut rng = PythonRandom::seed(8);
        let mut game = game(&mut rng);
        game.players[0].hand.clear();
        game.players[0].deck.clear();
        let hulk = game.create_card("Hulk", 0);
        let rings = game.create_card("TenRings", 0);
        let cyclops = game.create_card("Cyclops", 0);
        let thing = game.create_card("TheThing", 0);
        game.players[0].deck.extend([hulk, cyclops, thing, rings]);
        game.draw_initial_hand(0);
        assert_eq!(game.players[0].hand, [rings, hulk, cyclops]);
        game.players[0].hand.clear();
        game.players[0].deck.clear();
        game.players[0].deck.extend([hulk, rings]);
        game.draw(0);
        assert_eq!(game.players[0].hand, [hulk]);
    }

    #[test]
    fn initial_hand_without_self_draw_uses_existing_deck_order() {
        let mut rng = PythonRandom::seed(9);
        let mut game = game(&mut rng);
        game.players[0].hand.clear();
        game.players[0].deck.clear();
        let hulk = game.create_card("Hulk", 0);
        let cyclops = game.create_card("Cyclops", 0);
        let thing = game.create_card("TheThing", 0);
        game.players[0].deck.extend([hulk, cyclops, thing]);
        game.draw_initial_hand(0);
        assert_eq!(game.players[0].hand, [hulk, cyclops, thing]);
    }

    fn place(game: &mut Game<'_>, name: &str, seat: usize, lane: usize) -> usize {
        let card = game.create_card(name, seat);
        game.cards[card].lane = Some(lane);
        game.cards[card].revealed = true;
        game.lanes[lane].cards.push(card);
        card
    }

    fn check_sinister_staged_skill(expected_gain: i32) {
        let mut rng = PythonRandom::seed(73);
        let mut game = game(&mut rng);
        game.turn = 3;
        game.lanes[0].def = locations::get("SinisterLondon").unwrap();
        let horde = place(&mut game, "ZombieHorde", 0, 1);
        let before = game.cards[horde].power();
        let skill = place(&mut game, "TheHunger", 0, 0);
        game.cards[skill].revealed = false;
        game.cards[skill].turn_played = game.turn;
        game.reveal_with_staging(skill, Some(StagingContext { seat: 0, lane: 0 }));
        assert!(game.players[0].banished.contains(&skill));
        assert_eq!(game.cards[horde].power(), before + expected_gain);
    }

    #[test]
    fn sinister_london_copies_staged_skill_after_banishment() {
        check_sinister_staged_skill(6);
    }

    #[test]
    fn reference_sinister_london_does_not_copy_banished_skill() {
        const CHILD: &str = "SNAPSIM_TEST_REFERENCE_SINISTER_SKILL";
        if std::env::var_os(CHILD).is_some() {
            check_sinister_staged_skill(3);
            return;
        }
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "game::staging_tests::reference_sinister_london_does_not_copy_banished_skill",
            ])
            .env(CHILD, "1")
            .env("SNAPSIM_REFERENCE_RULES", "1")
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn copied_objective_updates_with_new_default_context() {
        let mut rng = PythonRandom::seed(74);
        let mut game = game(&mut rng);
        let rogue = place(&mut game, "RogueScionOfDivision", 0, 0);
        let objective = game.create_card("PolarisHorseman", 1);
        game.players[1].hand = vec![objective];
        game.trigger_on_reveal(rogue);
        assert_eq!(game.cards[rogue].default_ability().id, "PolarisHorseman");
        assert!(game.cards[rogue].ability_frames.is_empty());
        assert!(game.cards[rogue].state.is_some());
    }

    #[test]
    fn delayed_reveal_preserves_original_staging_context() {
        let mut rng = PythonRandom::seed(13);
        let mut game = game(&mut rng);
        game.turn = 2;
        let card = place(&mut game, "Wasp", 1, 1);
        game.cards[card].revealed = false;
        game.cards[card].turn_played = 2;
        game.delays_reveals = true;
        let staging = StagingContext { seat: 0, lane: 0 };
        game.reveal_with_staging(card, Some(staging));
        assert_eq!(
            game.delayed_reveals[0],
            [QueuedReveal {
                card,
                staging: Some(staging),
                persistent: false
            }]
        );
        game.delays_reveals = false;
        game.finish_reveals();
        let event = game.players[0].resolved_play_history.last().unwrap();
        assert_eq!(event.card, card);
        assert_eq!(event.staged_lane, Some(0));
        assert_eq!(event.lane, Some(1));
    }

    #[test]
    fn swapping_locations_moves_occupants_but_not_slot_history() {
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        game.lanes[1].def = locations::get("Xandar").unwrap();
        let card = place(&mut game, "Wasp", 0, 1);
        game.played[0].push((card, 1));
        game.swap_other_locations(0);
        assert_eq!(game.lanes[2].def.id, "Xandar");
        assert_eq!(game.cards[card].lane, Some(2));
        assert_eq!(game.lanes[2].cards, [card]);
        assert_eq!(game.played[0], [(card, 1)]);
    }

    #[test]
    fn permanent_energy_refreshes_on_next_turn_and_survives_fork() {
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        game.turn = 4;
        game.players[0].energy = 2;
        game.add_permanent_energy(0, 3);
        assert_eq!(game.players[0].energy, 2);
        let mut fork_rng = game.fork_rng();
        let mut fork = game.fork(fork_rng.as_mut());
        fork.turn = 5;
        fork.start_turn();
        assert_eq!(fork.players[0].energy, 8);
        assert_eq!(fork.players[1].energy, 5);
    }

    #[test]
    fn has_no_text_uses_effective_definition_and_removed_text() {
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        let card = game.create_card("Wasp", 0);
        assert_eq!(game.cards[card].has_no_text(), Some(true));
        game.cards[card].override_ability = cards::get("Hawkeye");
        assert_eq!(game.cards[card].has_no_text(), Some(false));
        game.cards[card].abilities_removed = true;
        assert_eq!(game.cards[card].has_no_text(), Some(true));
    }

    #[test]
    fn discard_reports_immunity_and_success() {
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        let stone = place(&mut game, "SoulStone", 0, 0);
        let thanos = game.cards.len();
        game.cards
            .push(crate::position::make_card("Thanos", 0).unwrap());
        game.players[0].hand.push(thanos);
        assert!(!game.discard_card(0, thanos));
        assert!(game.players[0].hand.contains(&thanos));
        game.cards[stone].abilities_removed = true;
        assert!(game.discard_card(0, thanos));
        assert!(!game.players[0].hand.contains(&thanos));
    }

    #[test]
    fn resolved_play_snapshots_on_reveal_before_later_text_changes() {
        let mut rng = PythonRandom::seed(31);
        let mut game = game(&mut rng);
        let card = game.create_card("Medusa", 0);
        game.players[0].hand.push(card);
        game.stage_action(0, Action::Play { card, lane: 1 })
            .unwrap();
        game.resolve_staged_actions(0);
        assert_eq!(game.players[0].on_reveal_cards_played, 1);
        game.cards[card].abilities_removed = true;
        game.cards[card].override_ability = cards::get("Hulk");
        game.ongoing();
        assert_eq!(game.players[0].on_reveal_cards_played, 1);
        let mut other_rng = PythonRandom::seed(32);
        assert_eq!(
            game.fork(&mut other_rng).players[0].on_reveal_cards_played,
            1
        );
    }

    #[test]
    fn played_reveal_snapshot_uses_post_reveal_text_even_after_bounce() {
        fn lose_text(game: &mut Game<'_>, card: usize) {
            game.cards[card].override_ability = cards::get("Hulk");
        }
        fn bounce(game: &mut Game<'_>, card: usize) {
            assert!(game.return_board_card_to_hand(card));
        }
        static CHANGE: CardDef = CardDef {
            reveal: Some(lose_text),
            ..CardDef::vanilla("ChangeTest")
        };
        static BOUNCE: CardDef = CardDef {
            reveal: Some(bounce),
            ..CardDef::vanilla("BounceTest")
        };
        let mut rng = PythonRandom::seed(33);
        let mut game = game(&mut rng);
        let first = game.create_card("Wasp", 0);
        game.cards[first].override_ability = Some(&CHANGE);
        game.players[0].hand.push(first);
        game.stage_action(
            0,
            Action::Play {
                card: first,
                lane: 0,
            },
        )
        .unwrap();
        game.resolve_staged_actions(0);
        assert_eq!(game.players[0].on_reveal_cards_played, 0);
        let second = game.create_card("Wasp", 0);
        game.cards[second].override_ability = Some(&BOUNCE);
        game.players[0].hand.push(second);
        game.stage_action(
            0,
            Action::Play {
                card: second,
                lane: 1,
            },
        )
        .unwrap();
        game.resolve_staged_actions(0);
        assert_eq!(game.players[0].on_reveal_cards_played, 1);
    }

    #[test]
    fn zone_events_survive_return_and_resurrection() {
        let mut rng = PythonRandom::seed(9);
        let mut game = game(&mut rng);
        let discarded = game.create_card("Wasp", 0);
        game.players[0].hand.push(discarded);
        assert!(game.discard_card(0, discarded));
        game.players[0].discard.clear();
        game.players[0].graveyard_order.clear();
        game.players[0].hand.push(discarded);
        assert!(game.discard_card(0, discarded));
        assert_eq!(game.players[0].discard_history.len(), 2);
        let destroyed = place(&mut game, "Cyclops", 1, 0);
        assert!(game.destroy_card(destroyed));
        assert!(game.revive_card(destroyed, 1, 1, destroyed));
        let event = game.players[1].destruction_history[0];
        assert_eq!(
            (event.card, event.lane, event.turn),
            (destroyed, Some(0), game.turn)
        );
        assert!(game.players[1].graveyard.is_empty());
    }

    #[test]
    fn graveyard_order_interleaves_and_removes_resurrected_cards() {
        let mut rng = PythonRandom::seed(17);
        let mut game = game(&mut rng);
        let a = game.create_card("Wasp", 0);
        let b = place(&mut game, "Cyclops", 0, 0);
        let c = game.create_card("Hulk", 0);
        game.players[0].hand.extend([a, c]);
        assert!(game.discard_card(0, a));
        assert!(game.destroy_card(b));
        assert!(game.discard_card(0, c));
        assert_eq!(game.players[0].graveyard_order, [a, b, c]);
        assert!(game.revive_card(b, 1, 1, b));
        assert_eq!(game.players[0].graveyard_order, [a, c]);
        assert!(game.destroy_card(b));
        assert_eq!(game.players[0].graveyard_order, [a, c, b]);
    }

    #[test]
    fn end_turn_trigger_uses_effective_text_and_removal_state() {
        static EVENT_TRIGGER: CardDef = CardDef {
            end_turn_trigger: true,
            ..CardDef::vanilla("EventTriggerTest")
        };
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        let card = game.create_card("Wasp", 0);
        assert!(!game.cards[card].has_end_turn_trigger());
        game.cards[card].override_ability = Some(&EVENT_TRIGGER);
        assert!(game.cards[card].has_end_turn_trigger());
        game.cards[card].abilities_removed = true;
        assert!(!game.cards[card].has_end_turn_trigger());
    }

    #[test]
    fn global_play_predicate_applies_across_lanes() {
        fn only_first(_: &Game<'_>, _: usize, _: usize, queued: usize) -> bool {
            queued == 0
        }
        static GLOBAL: LocationDef = LocationDef {
            allows_play_globally_with_count: Some(only_first),
            ..locations::location("GlobalTest", 0, 0, 1, false)
        };
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        game.lanes[0].def = &GLOBAL;
        let card = game.create_card("Wasp", 0);
        assert!(game.can_play_with_count(card, 2, 0, 6, 0, 0));
        assert!(!game.can_play_with_count(card, 2, 0, 6, 0, 1));
        game.lanes[0].revealed = false;
        assert!(game.can_play_with_count(card, 2, 0, 6, 0, 1));
    }

    #[test]
    fn lane_play_and_global_destruction_guards_follow_effective_ongoing() {
        fn protect(game: &Game<'_>, source: usize, target: usize) -> bool {
            game.cards[source].owner != game.cards[target].owner
        }
        static GOOSE: CardDef = CardDef {
            blocks_expensive_plays: true,
            ..CardDef::vanilla("GooseTest")
        };
        static PROTECT: CardDef = CardDef {
            destroy_guard: Some(protect),
            ..CardDef::vanilla("ProtectTest")
        };
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        let goose = place(&mut game, "Wasp", 0, 0);
        game.cards[goose].override_ability = Some(&GOOSE);
        let hulk = game.create_card("Hulk", 1);
        assert!(!game.can_play(hulk, 0, 1, 6, 0));
        assert!(game.can_play(hulk, 1, 1, 6, 0));
        game.cards[goose].abilities_removed = true;
        assert!(game.can_play(hulk, 0, 1, 6, 0));
        game.cards[goose].abilities_removed = false;
        game.cards[goose].override_ability = Some(&PROTECT);
        let ally = place(&mut game, "Cyclops", 0, 1);
        assert!(!game.destroy_card(ally));
        game.cards[goose].abilities_removed = true;
        assert!(game.destroy_card(ally));
    }

    #[test]
    fn hand_draw_blocker_stops_draw_until_removed() {
        static BITE: CardDef = CardDef {
            blocks_draw_in_hand: true,
            ..CardDef::vanilla("BiteTest")
        };
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        game.players[0].hand.clear();
        game.players[0].deck.clear();
        let bite = game.create_card("Wasp", 0);
        let drawn = game.create_card("Hulk", 0);
        game.cards[bite].override_ability = Some(&BITE);
        game.players[0].hand.push(bite);
        game.players[0].deck.push(drawn);
        game.draw(0);
        assert_eq!(game.players[0].deck, [drawn]);
        game.cards[bite].abilities_removed = true;
        game.draw(0);
        assert_eq!(game.players[0].hand, [bite, drawn]);
    }

    #[test]
    fn previous_action_queries_include_observed_and_card_history() {
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        game.players[0].observed_play_turns.push(2);
        game.players[0].observed_move_turns.push(2);
        assert!(game.has_played_on_turn(0, 2));
        assert!(game.has_moved_on_turn(0, 2));
        assert!(!game.has_played_on_turn(1, 2));
        let card = place(&mut game, "Wasp", 0, 0);
        game.turn = 3;
        assert!(game.move_card(card, 1));
        assert!(game.take_from_board(card));
        assert!(game.has_moved_on_turn(0, 3));
    }

    #[test]
    fn return_to_hand_respects_capacity_and_preserves_power() {
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        let card = place(&mut game, "Wasp", 0, 1);
        game.cards[card].permanent = 9;
        game.players[0].hand = vec![card; 7];
        assert!(!game.return_board_card_to_hand(card));
        assert_eq!(game.cards[card].lane, Some(1));
        game.players[0].hand.clear();
        assert!(game.return_board_card_to_hand(card));
        assert_eq!(game.cards[card].permanent, 9);
        assert!(!game.cards[card].revealed);
        assert_eq!(game.players[0].hand, [card]);
    }

    #[test]
    fn location_scoring_uses_base_power_and_turn_cap_applies_at_end() {
        static NEXUS: LocationDef = LocationDef {
            contributes_to_other_lanes: true,
            ..locations::location("TestNexus", 0, 0, 1, false)
        };
        static END: LocationDef = LocationDef {
            maximum_turns: 4,
            ..locations::location("TestEnd", 0, 0, 1, false)
        };
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        game.lanes[0].def = &NEXUS;
        game.lanes[1].def = &NEXUS;
        game.lanes[2].def = &END;
        game.lanes[0].bonus = [2, 1];
        game.lanes[1].bonus = [3, 4];
        assert_eq!(game.powers(), [[5, 5], [5, 5], [5, 5]]);
        game.turn = 3;
        game.end_turn_effects();
        assert_eq!(game.total_turns, 6);
        game.turn = 4;
        game.end_turn_effects();
        assert_eq!(game.total_turns, 4);
    }

    #[test]
    fn end_turn_location_then_card_resolution_order_across_hook_types() {
        fn location(game: &mut Game<'_>, _: usize) {
            game.players[0].energy = game.players[0].energy * 10 + 1;
        }
        fn direct(game: &mut Game<'_>, _: usize) {
            game.players[0].energy = game.players[0].energy * 10 + 3;
        }
        fn event(game: &mut Game<'_>, _: usize, event: Event) {
            if matches!(event, Event::TurnEnd) {
                game.players[0].energy = game.players[0].energy * 10 + 2;
            }
        }
        static LOCATION: LocationDef = LocationDef {
            on_turn_end: Some(location),
            ..locations::location("EndOrderTest", 0, 0, 1, false)
        };
        static DIRECT: CardDef = CardDef {
            end_turn: Some(direct),
            ..CardDef::vanilla("EndDirectTest")
        };
        static EVENT: CardDef = CardDef {
            on_event: Some(event),
            end_turn_trigger: true,
            ..CardDef::vanilla("EndEventTest")
        };
        let mut rng = PythonRandom::seed(42);
        let mut game = game(&mut rng);
        game.lanes[0].def = &LOCATION;
        let later = place(&mut game, "Wasp", 0, 0);
        let earlier = place(&mut game, "Wasp", 1, 2);
        game.cards[later].override_ability = Some(&DIRECT);
        game.cards[earlier].override_ability = Some(&EVENT);
        game.cards[later].latest_resolution_order = 8;
        game.cards[earlier].latest_resolution_order = 2;
        game.players[0].energy = 0;
        game.end_turn_effects();
        assert_eq!(game.players[0].energy, 123);
    }

    fn check_pre_end_order(expected: i32) {
        let mut rng = PythonRandom::seed(33);
        let mut game = game(&mut rng);
        game.turn = 3;
        let isca = place(&mut game, "Isca", 0, 0);
        let hawkeye = place(&mut game, "Hawkeye", 1, 0);
        game.cards[hawkeye].attach_watch(hawkeye, game.turn, 0, true, 4);
        game.played[1].push((hawkeye, 0));
        game.cards[hawkeye].turn_played = game.turn;
        game.end_turn_effects();
        assert_eq!(game.cards[hawkeye].power(), 5);
        assert_eq!(game.cards[isca].power(), expected);
    }

    #[test]
    fn pre_end_watches_resolve_before_end_turn_hooks() {
        check_pre_end_order(4);
    }

    #[test]
    fn reference_mode_preserves_interleaved_end_turn_order() {
        const CHILD: &str = "SNAPSIM_TEST_REFERENCE_END_ORDER";
        if std::env::var_os(CHILD).is_some() {
            check_pre_end_order(2);
            return;
        }
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "game::staging_tests::reference_mode_preserves_interleaved_end_turn_order",
            ])
            .env(CHILD, "1")
            .env("SNAPSIM_REFERENCE_RULES", "1")
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn intrinsic_each_turn_movement_resets_without_resetting_lifetime_stacks() {
        let mut rng = PythonRandom::seed(31);
        let mut game = game(&mut rng);
        game.turn = 3;
        let vision = place(&mut game, "Vision", 0, 0);
        let crawler = place(&mut game, "Nightcrawler", 0, 0);
        assert!(game.move_player_card(vision, 1, 0));
        assert!(!game.can_player_move(vision, 2, 0));
        assert!(game.move_player_card(crawler, 1, 0));
        game.turn = 4;
        assert!(game.can_player_move(vision, 2, 0));
        assert!(!game.can_player_move(crawler, 2, 0));
        assert!(game.move_player_card(vision, 2, 0));
    }

    #[test]
    fn opponent_staged_move_does_not_change_own_staging_legality() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let professor = place(&mut game, "ProfessorX", 0, 0);
        game.grant_move(MoveGrant {
            seat: 0,
            source: professor,
            owner_def_id: None,
            card: Some(professor),
            lane: None,
            destination: None,
            start_turn: game.turn,
            end_turn: game.turn,
            remaining: 1,
            unlimited: false,
            condition: None,
            on_move: None,
        });
        let card = game.create_card("Wasp", 1);
        game.players[1].hand.push(card);
        let movement = Action::Move {
            card: professor,
            lane: 1,
        };
        let play = Action::Play { card, lane: 1 };
        let mut fork_rng = game.fork_rng();
        let mut combined = game.fork(fork_rng.as_mut());
        combined.stage_action(0, movement).unwrap();
        assert!(combined.stage_action(1, play).is_err());
        assert!(
            game.resolve_action_plans([vec![movement], vec![play]], 0)
                .is_ok()
        );
    }

    #[test]
    fn removed_cards_fizzle_staged_play_move_and_activation() {
        let mut rng = PythonRandom::seed(2);
        let mut game = game(&mut rng);
        let mover = place(&mut game, "Nightcrawler", 0, 0);
        let activator = place(&mut game, "KidOmega", 0, 0);
        let played = game.create_card("Wasp", 0);
        game.players[0].hand.push(played);
        game.stage_action(
            0,
            Action::Move {
                card: mover,
                lane: 1,
            },
        )
        .unwrap();
        game.stage_action(0, Action::Activate { card: activator })
            .unwrap();
        game.stage_action(
            0,
            Action::Play {
                card: played,
                lane: 2,
            },
        )
        .unwrap();
        for card in [mover, activator, played] {
            assert!(game.take_from_board(card));
        }
        game.resolve_staged_actions(0);
        assert!(!game.cards[activator].activated);
        assert!(!game.cards[played].revealed);
        assert!(!game.cards[mover].move_staged);
    }
}
