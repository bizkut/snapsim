//! Game state and turn lifecycle. Board and hand entries are stable arena indices.
use crate::{
    cards::{self, CardDef},
    data,
    locations::{self, LocationDef},
    random::GameRng,
};
use serde::Serialize;
use std::{cell::RefCell, collections::HashMap, sync::LazyLock};
pub trait CardState: std::any::Any {
    fn clone_box(&self) -> Box<dyn CardState>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: std::any::Any + Clone> CardState for T {
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

#[derive(Clone, Copy, Debug)]
pub struct CardStatUpdate {
    pub stat: Stat,
    pub operation: StatOperation,
    pub value: i32,
    pub source: usize,
    pub ongoing: bool,
}

#[derive(Clone)]
pub struct Card {
    pub def: &'static CardDef,
    /// Effective text; card identity, printed stats and ownership remain unchanged.
    pub override_ability: Option<&'static CardDef>,
    pub morphed_from: Option<&'static str>,
    pub morphed_from_entity: Option<usize>,
    pub owner: usize,
    pub cost: i32,
    pub ongoing_cost: i32,
    pub temporary_hand_cost: Option<(u8, i32)>,
    /// Lasting cost before location modifiers.
    pub created: bool,
    /// Visible client provenance; absent when not reported.
    pub created_by_card_id: Option<String>,
    pub created_by_location_id: Option<String>,
    pub created_by_player: Option<usize>,
    pub observed_entity_id: Option<i64>,
    pub merged_to_entity_id: Option<i64>,
    pub permanent: i32,
    pub stat_updates: Vec<CardStatUpdate>,
    pub ongoing: i32,
    pub location_power: i32,
    pub lane: Option<usize>,
    /// Last location this card left, updated before movement or destruction hooks.
    pub last_lane: Option<usize>,
    pub revealed: bool,
    pub abilities_removed: bool,
    pub repeating: bool,
    pub latest_resolution_order: u32,
    pub turn_played: u8,
    pub drawn_turn: Option<u8>,
    pub draw_extends_quickdraw: bool,
    pub move_turns: Vec<u8>,
    /// Voluntary movement and activation state (forced movement does not consume stacks).
    pub player_moves_used: u8,
    pub activated: bool,
    pub move_staged: bool,
    pub activate_staged: bool,
    /// Optional current-turn legality reported by the live client.
    pub observed_can_activate: Option<bool>,
    pub observed_move_targets: Option<u8>,
    pub merged_to: Option<usize>,
    pub watches: Vec<(u8, usize)>,
    pub state: Option<Box<dyn CardState>>,
}
impl Card {
    pub fn ability_def(&self) -> &'static CardDef {
        self.override_ability.unwrap_or(self.def)
    }
    pub fn power(&self) -> i32 {
        self.permanent + self.ongoing + self.location_power
    }
    pub fn has_ongoing(&self) -> bool {
        !self.abilities_removed
            && (self.ability_def().ongoing.is_some()
                || self.ability_def().late_ongoing.is_some()
                || self.ability_def().ongoing_ability
                || self.ability_def().cost_guard.is_some()
                || self.ability_def().blocks_spawns
                || self.ability_def().blocks_reveal
                || self.ability_def().prevents_destroy
                || self.ability_def().locks_lane)
    }
    pub fn can_reduce_power(&self) -> bool {
        !self.revealed || !self.ability_def().immune_destroy_move
    }

    pub fn reduce_power(&mut self, amount: i32) {
        if self.can_reduce_power() {
            self.permanent -= amount;
        }
    }
    pub fn has_no_ability(&self) -> bool {
        self.abilities_removed
            || CATALOG
                .get(self.ability_def().id)
                .is_some_and(|data| data.ability.is_empty())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerTrigger {
    BeforeCardRevealed,
    CardPlayed,
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

/// A player-owned movement permission; one grant may be shared by a whole lane.
#[derive(Clone)]
pub struct MoveGrant {
    pub seat: usize,
    pub source: usize,
    pub card: Option<usize>,
    pub lane: Option<usize>,
    pub start_turn: u8,
    pub end_turn: u8,
    pub remaining: u8,
    /// Invoked with (source card, moved card) after the granted move resolves.
    pub on_move: Option<crate::cards::PlayHook>,
}

/// One player action in the order it was staged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    Play { card: usize, lane: usize },
    Move { card: usize, lane: usize },
    Activate { card: usize },
}

#[derive(Clone)]
struct StagedAction {
    action: Action,
    grant: Option<MoveGrant>,
}

#[derive(Clone, Default)]
pub struct Player {
    pub hand: Vec<usize>,
    pub deck: Vec<usize>,
    pub discard: Vec<usize>,
    pub banished: Vec<usize>,
    pub set_aside: Vec<usize>,
    /// Cards in destruction order and their death lane, if it was observed.
    pub graveyard: Vec<(usize, Option<usize>)>,
    pub pending_effects: Vec<PendingEffect>,
    pub move_grants: Vec<MoveGrant>,
    staged_actions: Vec<StagedAction>,
    pub scheduled_energy: Vec<(u8, i32)>,
    pub movable_cards_played: Vec<usize>,
    /// Drawn cards, preserving the turn and draw order.
    pub draws: Vec<(u8, usize)>,
    /// Resolved played cards, preserving turn and resolution order.
    pub play_history: Vec<(u8, usize)>,
    /// Snapshot of all staged plays, including those not yet resolved.
    pub staged_history: Vec<StagedPlay>,
    /// Turns on which this player snapped.
    pub snapped_turns: Vec<u8>,
    /// (turn, energy spent, all available energy spent).
    pub energy_history: Vec<(u8, i32, bool)>,
    pub energy_spent_this_turn: i32,
    pub energy: i32,
    pub next_play_power: i32,
    pub pending_left_moves: u32,
}
#[derive(Clone)]
pub struct Lane {
    pub def: &'static LocationDef,
    /// Original ID when an unsupported location is represented by an inert rule.
    pub observed_id: Option<String>,
    pub revealed: bool,
    pub revealed_turn: Option<u8>,
    pub cards: Vec<usize>,
    pub bonus: [i32; 2],
    pub multiplier: [i32; 2],
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
    BeforeCardRevealed {
        card: usize,
    },
    AfterTurnEnd,
    CardRevealed {
        card: usize,
    },
    CardPlayed {
        card: usize,
    },
    CardActivated {
        card: usize,
    },
    CardAdded {
        card: usize,
        lane: usize,
    },
    CardMoved {
        card: usize,
        from: usize,
        to: usize,
    },
    CardMerged {
        card: usize,
        absorbed: usize,
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

pub struct Game<'a> {
    pub cards: Vec<Card>,
    pub players: [Player; 2],
    pub lanes: [Lane; 3],
    pub location_pool: Vec<&'static LocationDef>,
    pub next_resolution_order: u32,
    pub turn: u8,
    pub is_wait_for_staging: bool,
    pub reveal_priority: Option<usize>,
    pub total_turns: u8,
    pub played: [Vec<(usize, usize)>; 2],
    pub pending_plays: [Vec<(usize, usize)>; 2],
    pub played_on_reveal: [bool; 2],
    pub delays_reveals: bool,
    pub delayed_reveals: [Vec<usize>; 2],
    pub(crate) deferred_added_reveals: Vec<usize>,
    /// Python's `current_location`: scheduled reveals consumed so far. A lane that a
    /// card already revealed (Scarlet Witch, Magik) still uses up its scheduled turn.
    pub current_location: usize,
    /// Python's `_location_reveal_order`: lanes in the order the schedule reveals them.
    pub location_reveal_order: [usize; 3],
    /// Optional replay observer, populated only for explicitly observed games.
    pub destroyed: Option<Vec<(usize, usize)>>,
    pub rng: RefCell<&'a mut dyn GameRng>,
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
    /// Clone all simulation state against a separately cloned RNG stream.
    pub fn fork<'b>(&self, rng: &'b mut dyn GameRng) -> Game<'b> {
        Game {
            cards: self.cards.clone(),
            players: self.players.clone(),
            lanes: self.lanes.clone(),
            location_pool: self.location_pool.clone(),
            next_resolution_order: self.next_resolution_order,
            turn: self.turn,
            is_wait_for_staging: self.is_wait_for_staging,
            reveal_priority: self.reveal_priority,
            total_turns: self.total_turns,
            played: self.played.clone(),
            pending_plays: self.pending_plays.clone(),
            played_on_reveal: self.played_on_reveal,
            delays_reveals: self.delays_reveals,
            delayed_reveals: self.delayed_reveals.clone(),
            deferred_added_reveals: self.deferred_added_reveals.clone(),
            current_location: self.current_location,
            location_reveal_order: self.location_reveal_order,
            destroyed: self.destroyed.clone(),
            rng: RefCell::new(rng),
        }
    }

    pub fn fork_rng(&self) -> Box<dyn GameRng> {
        self.rng.borrow().clone_rng()
    }
    fn emit(&mut self, event: Event) {
        if !*HAS_EVENT_HOOKS {
            return;
        }
        let observers: Vec<_> = self
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| {
                self.cards[id].revealed && self.cards[id].ability_def().on_event.is_some()
            })
            .collect();
        for card in observers {
            if self.cards[card].lane.is_some()
                && !self.cards[card].abilities_removed
                && let Some(hook) = self.cards[card].ability_def().on_event
            {
                hook(self, card, event);
            }
        }
        if matches!(
            event,
            Event::GameStart | Event::TurnStart | Event::Empower { .. } | Event::CardDrawn { .. }
        ) {
            let observers: Vec<_> = self
                .players
                .iter()
                .flat_map(|player| player.hand.iter().chain(&player.deck).copied())
                .filter(|&id| self.cards[id].ability_def().on_event.is_some())
                .collect();
            for card in observers {
                if !self.cards[card].abilities_removed
                    && let Some(hook) = self.cards[card].ability_def().on_event
                {
                    hook(self, card, event);
                }
            }
        }
        for lane in 0..3 {
            if self.lanes[lane].revealed
                && let Some(hook) = self.lanes[lane].def.on_event
            {
                hook(self, lane, event);
            }
        }
    }
    pub(crate) fn emit_merged(&mut self, card: usize, absorbed: usize) {
        self.emit(Event::CardMerged { card, absorbed });
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
            let fired = effect.trigger == trigger
                && !(card == Some(effect.source)
                    && (effect.created_turn == self.turn
                        || effect.lifetime == EffectLifetime::NextOtherCard));
            let completed = fired && (effect.apply)(self, &mut effect, card);
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

    pub fn set_total_turns(&mut self, turns: u8) {
        let old = self.total_turns;
        self.total_turns = turns;
        if old != turns {
            self.emit(Event::TurnCountChanged { old, new: turns });
        }
    }
    pub fn set_energy(&mut self, seat: usize, energy: i32) {
        let old = self.players[seat].energy;
        self.players[seat].energy = energy;
        if old != energy {
            self.emit(Event::EnergyChanged {
                seat,
                old,
                new: energy,
            });
        }
    }
    pub fn schedule_energy(&mut self, seat: usize, turn: u8, amount: i32) {
        self.players[seat].scheduled_energy.push((turn, amount));
    }
    pub fn set_temporary_hand_cost(&mut self, card: usize, turn: u8, value: i32) {
        self.cards[card].temporary_hand_cost = Some((turn, value));
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
        let target = &mut self.cards[card];
        target.stat_updates.push(CardStatUpdate {
            stat,
            operation,
            value,
            source,
            ongoing: false,
        });
        match stat {
            Stat::Power => target.permanent = new,
            Stat::Cost => target.cost = new,
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
        let target = &mut self.cards[card];
        if amount < 0 && !target.can_reduce_power() {
            return;
        }
        target.permanent += amount;
        target.stat_updates.push(CardStatUpdate {
            stat: Stat::Power,
            operation: StatOperation::Add,
            value: amount,
            source,
            ongoing: false,
        });
    }
    pub fn set_power(&mut self, card: usize, value: i32, source: usize) {
        self.change_stat(card, Stat::Power, StatOperation::Set, value, source, false);
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
    pub fn steal_power_many(&mut self, from: &[usize], to: usize, amount: i32, source: usize) {
        let mut stolen = 0;
        for &card in from {
            let before = self.cards[card].permanent;
            self.add_power(card, -amount, source);
            stolen += before - self.cards[card].permanent;
        }
        if stolen > 0 {
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
        if self.is_character(card) && (amount >= 0 || self.cards[card].can_reduce_power()) {
            self.cards[card].ongoing += amount;
            self.cards[card].stat_updates.push(CardStatUpdate {
                stat: Stat::Power,
                operation: StatOperation::Add,
                value: amount,
                source,
                ongoing: true,
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
        });
    }
    pub fn add_ongoing_cost(&mut self, card: usize, amount: i32, source: usize) {
        let old = self.cards[card].cost + self.cards[card].ongoing_cost;
        if self.cost_change_allowed(card, old, old + amount) {
            self.cards[card].ongoing_cost += amount;
            self.cards[card].stat_updates.push(CardStatUpdate {
                stat: Stat::Cost,
                operation: StatOperation::Add,
                value: amount,
                source,
                ongoing: true,
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
            });
        }
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
                    || !self.cards[id].has_ongoing()
                    || self.cards[id]
                        .def
                        .cost_guard
                        .is_none_or(|guard| guard(self, id, card, old, new))
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
            if location.revealed
                && let Some(hook) = location.def.modify_cost
            {
                let next = hook(self, lane, card, current);
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
        target.permanent = CATALOG[target.def.id].power;
        target
            .stat_updates
            .retain(|update| update.stat != Stat::Power || update.ongoing);
    }
    pub fn is_character(&self, card: usize) -> bool {
        CATALOG[self.cards[card].def.id].card_type == "Character"
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
                    owner: seat,
                    cost: data.cost,
                    ongoing_cost: 0,
                    temporary_hand_cost: None,
                    created: false,
                    created_by_card_id: None,
                    created_by_location_id: None,
                    created_by_player: None,
                    observed_entity_id: None,
                    merged_to_entity_id: None,
                    permanent: data.power,
                    stat_updates: Vec::new(),
                    ongoing: 0,
                    location_power: 0,
                    lane: None,
                    last_lane: None,
                    revealed: false,
                    abilities_removed: false,
                    repeating: false,
                    latest_resolution_order: 0,
                    turn_played: 0,
                    drawn_turn: None,
                    draw_extends_quickdraw: false,
                    move_turns: Vec::new(),
                    player_moves_used: 0,
                    activated: false,
                    move_staged: false,
                    activate_staged: false,
                    observed_can_activate: None,
                    observed_move_targets: None,
                    merged_to: None,
                    watches: Vec::new(),
                    state: def.initial_state.map(|init| init()),
                });
                players[seat].deck.push(index);
            }
            rng.shuffle(&mut players[seat].deck);
        }
        let chosen = rng.sample_indices(available.len(), 3);
        let lanes = std::array::from_fn(|i| Lane {
            def: available[chosen[i]],
            observed_id: None,
            revealed: false,
            revealed_turn: None,
            cards: Vec::new(),
            bonus: [0; 2],
            multiplier: [1; 2],
            state: available[chosen[i]].initial_state.map(|init| init()),
        });
        let mut game = Self {
            cards,
            players,
            lanes,
            location_pool: available,
            turn: 0,
            next_resolution_order: 1,
            is_wait_for_staging: false,
            total_turns: 6,
            reveal_priority: None,
            played: Default::default(),
            pending_plays: Default::default(),
            played_on_reveal: [false; 2],
            delays_reveals: false,
            delayed_reveals: Default::default(),
            deferred_added_reveals: Vec::new(),
            current_location: 0,
            location_reveal_order: [0, 1, 2],
            destroyed: None,
            rng: RefCell::new(rng),
        };
        game.emit(Event::GameStart);
        // StartGame::OnEnter (0x7e47c68) triggers game-start before the opening draw.
        for seat in 0..2 {
            if let Some(slot) = game.players[seat]
                .deck
                .iter()
                .position(|&index| game.cards[index].def.id == "Quicksilver")
            {
                let card = game.players[seat].deck.remove(slot);
                game.add_hand(seat, card, true);
            }
            while game.players[seat].hand.len() < 3 {
                let Some(slot) = game.players[seat]
                    .deck
                    .iter()
                    .position(|&index| game.cards[index].def.draw_turn.is_none())
                else {
                    break;
                };
                let card = game.players[seat].deck.remove(slot);
                game.add_hand(seat, card, true);
            }
        }
        game.turn = 1;
        Ok(game)
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

    pub fn create_card(&mut self, id: &str, owner: usize) -> usize {
        let def = crate::position::card_def(id).expect("unregistered created card");
        let row = CATALOG.get(id).expect("missing created card data");
        let index = self.cards.len();
        self.cards.push(Card {
            def,
            override_ability: None,
            morphed_from: None,
            morphed_from_entity: None,
            owner,
            ongoing_cost: 0,
            temporary_hand_cost: None,
            cost: row.cost,
            permanent: row.power,
            stat_updates: Vec::new(),
            created: true,
            created_by_card_id: None,
            created_by_location_id: None,
            created_by_player: None,
            observed_entity_id: None,
            merged_to_entity_id: None,
            ongoing: 0,
            location_power: 0,
            lane: None,
            last_lane: None,
            revealed: false,
            abilities_removed: false,
            repeating: false,
            latest_resolution_order: 0,
            turn_played: 0,
            drawn_turn: None,
            draw_extends_quickdraw: false,
            move_turns: Vec::new(),
            player_moves_used: 0,
            activated: false,
            move_staged: false,
            activate_staged: false,
            observed_can_activate: None,
            observed_move_targets: None,
            merged_to: None,
            watches: Vec::new(),
            state: def.initial_state.map(|init| init()),
        });
        index
    }

    /// Copy lasting card attributes, clearing board-only reveal state.
    pub fn copy_card(&mut self, source: usize, owner: usize) -> usize {
        let card = self.create_card(self.cards[source].def.id, owner);
        self.cards[card].cost = self.cards[source].cost;
        self.cards[card].permanent = self.cards[source].permanent;
        self.cards[card].stat_updates = self.cards[source]
            .stat_updates
            .iter()
            .copied()
            .filter(|update| !update.ongoing)
            .collect();
        self.cards[card].abilities_removed = self.cards[source].abilities_removed;
        self.cards[card].override_ability = self.cards[source].override_ability;
        self.cards[card].morphed_from = self.cards[source].morphed_from;
        self.cards[card].morphed_from_entity = self.cards[source].morphed_from_entity;
        self.cards[card].state = self.cards[source]
            .state
            .as_deref()
            .map(CardState::clone_box);
        if let Some(clear) = self.cards[card].ability_def().clear_reveal_state {
            clear(&mut self.cards[card]);
        }
        card
    }
    /// Copy the source's current text without changing this card's identity or stats.
    /// CopyTextAndActivateResolveAbility (0x7cbb3f4) refreshes ongoing before resolve.
    pub fn copy_text(&mut self, card: usize, source: usize, activate_reveal: bool) -> bool {
        let def = self.cards[source].ability_def();
        // Card::CopyText (0x7cbadb4) rejects an already installed override.
        if self.cards[card]
            .override_ability
            .is_some_and(|old| old.id == def.id)
        {
            return false;
        }
        let removed = self.cards[source].abilities_removed;
        let state = self.cards[source]
            .state
            .as_deref()
            .map(CardState::clone_box);
        self.cards[card].override_ability = Some(def);
        self.cards[card].abilities_removed = removed;
        self.cards[card].state = state.or_else(|| def.initial_state.map(|init| init()));
        self.ongoing();
        if activate_reveal
            && !removed
            && let Some(hook) = def.reveal
        {
            hook(self, card);
        }
        true
    }

    /// MorphIntoNewCard removes Set modifiers but retains Add and Multiply.
    pub fn transform_card(&mut self, card: usize, target_id: &str) -> bool {
        let Some(def) = cards::get(target_id) else {
            return false;
        };
        let Some(row) = CATALOG.get(target_id) else {
            return false;
        };
        let old = self.cards[card].def.id;
        // Card::<MorphIntoNewCard>b__477_0 (0x7cbf0cc) matches OperationType::Set.
        self.cards[card]
            .stat_updates
            .retain(|u| u.ongoing || u.operation != StatOperation::Set);
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
        target.override_ability = None;
        target.abilities_removed = false;
        target.state = def.initial_state.map(|init| init());
        target.permanent = power;
        target.cost = cost;
        target.watches.clear();
        self.ongoing();
        // CompleteMorph (0x7cbb714) activates the new resolve text when eligible.
        if self.cards[card].lane.is_some() && self.cards[card].revealed {
            self.trigger_on_reveal(card);
        }
        true
    }
    /// MorphIntoNewCardWithoutModifiers (0x7cbbe80).
    pub fn transform_card_without_modifiers(&mut self, card: usize, target_id: &str) -> bool {
        if cards::get(target_id).is_none() || !CATALOG.contains_key(target_id) {
            return false;
        }
        self.cards[card]
            .stat_updates
            .retain(|update| update.ongoing);
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
            .state
            .as_deref()
            .map(CardState::clone_box);
        let updates = self.cards[source]
            .stat_updates
            .iter()
            .copied()
            .filter(|update| !update.ongoing)
            .collect();
        let cost = self.cards[source].cost;
        let power = self.cards[source].permanent;
        let removed = self.cards[source].abilities_removed;
        let target = &mut self.cards[card];
        target.def = def;
        target.morphed_from = Some(old);
        target.morphed_from_entity = Some(source);
        target.override_ability = override_ability;
        target.state = state;
        target.stat_updates = updates;
        target.cost = cost;
        target.permanent = power;
        target.abilities_removed = removed;
        target.watches.clear();
        self.ongoing();
        if self.cards[card].lane.is_some() && self.cards[card].revealed {
            self.trigger_on_reveal(card);
        }
        true
    }

    pub fn lane_locked(&self, lane: usize) -> bool {
        self.lanes[lane].cards.iter().any(|&card| {
            self.cards[card].revealed
                && self.cards[card].has_ongoing()
                && self.cards[card].ability_def().locks_lane
        })
    }

    /// Effective cost, Python's `energy_cost`: revealed location modifiers apply in lane
    /// order to cards in a hand only. Python resets that adjustment when a card is
    /// played, added to the board or discarded, so cards elsewhere use their lasting cost.
    pub fn card_cost(&self, card: usize) -> i32 {
        let mut cost = self.cards[card].cost + self.cards[card].ongoing_cost;
        if self
            .players
            .iter()
            .any(|player| player.hand.contains(&card))
        {
            for (lane, location) in self.lanes.iter().enumerate() {
                if location.revealed
                    && let Some(hook) = location.def.modify_cost
                {
                    let next = hook(self, lane, card, cost);
                    if self.cost_change_allowed(card, cost, next) {
                        cost = next;
                    }
                }
            }
            if !self.cards[card].abilities_removed
                && let Some(hook) = self.cards[card].ability_def().modify_final_cost
            {
                let next = hook(self, card, cost);
                if self.cost_change_allowed(card, cost, next) {
                    cost = next;
                }
            }
            if let Some((turn, value)) = self.cards[card].temporary_hand_cost
                && turn == self.turn
                && self.cost_change_allowed(card, cost, value)
            {
                cost = value;
            }
        }
        cost.max(0)
    }

    /// Replace and immediately reveal a location without moving its cards.
    pub fn replace_location(&mut self, lane: usize, id: &str) {
        let def = locations::get(id).expect("unregistered replacement location");
        self.lanes[lane].def = def;
        self.lanes[lane].observed_id = None;
        self.lanes[lane].state = def.initial_state.map(|init| init());
        self.lanes[lane].revealed = true;
        self.lanes[lane].revealed_turn = Some(self.turn);
        self.delays_reveals |= def.delays_reveals;
        self.emit(Event::LocationRevealed { lane });
        if let Some(hook) = def.on_reveal {
            hook(self, lane);
        }
        let minimum = self
            .lanes
            .iter()
            .filter(|location| location.revealed)
            .map(|location| location.def.minimum_turns)
            .max()
            .unwrap_or(0);
        self.set_total_turns(6.max(minimum));
        for seat in 0..2 {
            let energy = self.players[seat].energy + def.reveal_energy;
            self.set_energy(seat, energy);
        }
        self.ongoing();
    }

    pub fn has_space(&self, lane: usize, seat: usize, queued: usize) -> bool {
        self.lanes[lane]
            .cards
            .iter()
            .filter(|&&card| self.cards[card].owner == seat)
            .count()
            + queued
            < 4
    }
    pub fn can_play(
        &self,
        card: usize,
        lane: usize,
        seat: usize,
        energy: i32,
        queued: usize,
    ) -> bool {
        if self.card_cost(card) > energy
            || !self.has_space(lane, seat, queued)
            || self.lane_locked(lane)
            || (!self.cards[card].abilities_removed
                && self.cards[card]
                    .ability_def()
                    .can_play
                    .is_some_and(|test| !test(self, card, lane, seat)))
        {
            return false;
        }
        for (source, location) in self.lanes.iter().enumerate() {
            if !location.revealed {
                continue;
            }
            let predicate = if source == lane {
                location.def.allows_play
            } else {
                location.def.allows_play_elsewhere
            };
            if let Some(test) = predicate
                && !test(self, source, card, lane, seat)
            {
                return false;
            }
            if source == lane
                && let Some(test) = location.def.allows_addition
                && !test(self, source, card, lane, seat)
            {
                return false;
            }
        }
        true
    }
    /// Install a movement permission without moving or revealing a card.
    pub fn grant_move(&mut self, grant: MoveGrant) {
        self.players[grant.seat].move_grants.push(grant);
    }

    fn matching_move_grant(&self, card: usize, seat: usize) -> Option<usize> {
        let lane = self.cards[card].lane?;
        self.players[seat].move_grants.iter().position(|grant| {
            grant.remaining != 0
                && grant.start_turn <= self.turn
                && self.turn <= grant.end_turn
                && grant.card.is_none_or(|target| target == card)
                && grant.lane.is_none_or(|target| target == lane)
        })
    }

    /// Whether a player can voluntarily move this card to the destination now.
    pub fn can_player_move(&self, card: usize, destination: usize, seat: usize) -> bool {
        let target = &self.cards[card];
        let Some(source) = target.lane else {
            return false;
        };
        if target.owner != seat
            || !target.revealed
            || target.move_staged
            || target.ability_def().immune_destroy_move
            || destination >= 3
            || source == destination
            || !self.has_space(destination, seat, 0)
        {
            return false;
        }
        if self.lanes[destination].revealed
            && let Some(test) = self.lanes[destination].def.allows_addition
            && !test(self, destination, card, destination, seat)
        {
            return false;
        }
        if let Some(mask) = target.observed_move_targets {
            return mask & (1 << destination) != 0;
        }
        (!target.abilities_removed && target.player_moves_used < target.ability_def().move_limit)
            || self.matching_move_grant(card, seat).is_some()
    }

    /// Resolve one voluntary movement, consuming its intrinsic and granted stacks.
    pub fn move_player_card(&mut self, card: usize, destination: usize, seat: usize) -> bool {
        if !self.can_player_move(card, destination, seat) {
            return false;
        }
        let grant = self.matching_move_grant(card, seat);
        if !self.move_card(card, destination) {
            return false;
        }
        if self.cards[card].ability_def().move_limit > 0 {
            self.cards[card].player_moves_used =
                self.cards[card].player_moves_used.saturating_add(1);
        }
        if let Some(index) = grant {
            let grant = &mut self.players[seat].move_grants[index];
            grant.remaining -= 1;
            let source = grant.source;
            let callback = grant.on_move;
            if let Some(hook) = callback {
                hook(self, source, card);
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
        if destination >= 3
            || destination == source
            || (target.revealed && target.ability_def().immune_destroy_move)
            || !self.has_space(destination, target.owner, 0)
        {
            return false;
        }
        !self.lanes[destination].revealed
            || self.lanes[destination]
                .def
                .allows_addition
                .is_none_or(|test| test(self, destination, card, destination, target.owner))
    }

    pub fn can_activate(&self, card: usize, seat: usize) -> bool {
        let target = &self.cards[card];
        let def = target.ability_def();
        target.owner == seat
            && target.lane.is_some()
            && target.revealed
            && !target.abilities_removed
            && !target.activated
            && !target.activate_staged
            && target.observed_can_activate.unwrap_or(true)
            && target.lane.is_some_and(|lane| {
                !self.lanes[lane].revealed || !self.lanes[lane].def.blocks_activate
            })
            && def.on_activate.is_some()
            && self.players[seat].energy >= def.activate_cost
            && def.can_activate.is_none_or(|test| test(self, card))
    }

    /// Resolve an activation and dispatch its post-activation trigger.
    pub fn activate_card(&mut self, card: usize, seat: usize) -> bool {
        if !self.can_activate(card, seat) {
            return false;
        }
        let def = self.cards[card].ability_def();
        self.set_energy(seat, self.players[seat].energy - def.activate_cost);
        self.players[seat].energy_spent_this_turn += def.activate_cost;
        self.cards[card].activated = true;
        self.ongoing();
        if let Some(hook) = def.on_activate {
            hook(self, card);
        }
        self.emit(Event::CardActivated { card });
        true
    }
    /// Stage a play, move or activation, preserving the player's action order.
    pub fn stage_action(&mut self, seat: usize, action: Action) -> Result<(), String> {
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
                    return Err("illegal staged move".into());
                }
                0
            }
            Action::Activate { card } => {
                if !self.can_activate(card, seat) {
                    return Err("illegal staged activation".into());
                }
                self.cards[card].ability_def().activate_cost
            }
        };
        self.apply_staged_action(seat, action, cost);
        Ok(())
    }

    fn apply_staged_action(&mut self, seat: usize, action: Action, cost: i32) {
        match action {
            Action::Play { card, lane } => {
                let slot = self.players[seat]
                    .hand
                    .iter()
                    .position(|&id| id == card)
                    .unwrap();
                self.players[seat].hand.remove(slot);
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
                self.played[seat].push((card, lane));
                self.lanes[lane].cards.push(card);
                self.notify_added(card, lane);
                self.players[seat].staged_actions.push(StagedAction {
                    action,
                    grant: None,
                });
            }
            Action::Move { card, lane } => {
                let from = self.cards[card].lane.unwrap();
                let grant = self.matching_move_grant(card, seat).map(|index| {
                    let grant = &mut self.players[seat].move_grants[index];
                    grant.remaining -= 1;
                    grant.clone()
                });
                if self.cards[card].ability_def().move_limit > 0 {
                    self.cards[card].player_moves_used =
                        self.cards[card].player_moves_used.saturating_add(1);
                }
                self.cards[card].move_staged = true;
                self.cards[card].last_lane = Some(from);
                self.lanes[from].cards.retain(|&id| id != card);
                self.lanes[lane].cards.push(card);
                self.cards[card].lane = Some(lane);
                self.players[seat]
                    .staged_actions
                    .push(StagedAction { action, grant });
                self.ongoing();
            }
            Action::Activate { card } => {
                self.set_energy(seat, self.players[seat].energy - cost);
                self.players[seat].energy_spent_this_turn += cost;
                self.cards[card].activate_staged = true;
                self.players[seat].staged_actions.push(StagedAction {
                    action,
                    grant: None,
                });
                self.ongoing();
            }
        }
    }

    /// Resolve each player's staged actions in priority order and original stage order.
    pub fn resolve_staged_actions(&mut self, priority: usize) {
        self.is_wait_for_staging = false;
        let pending = std::mem::take(&mut self.delayed_reveals);
        for seat in [priority, 1 - priority] {
            for &card in &pending[seat] {
                self.reveal(card);
            }
            let actions = std::mem::take(&mut self.players[seat].staged_actions);
            for staged in actions {
                match staged.action {
                    Action::Play { card, .. } => self.reveal(card),
                    Action::Move { card, .. } => {
                        self.cards[card].move_staged = false;
                        // Card::CanResolveForMove (0x7cb9cbc) checks that both
                        // previous and current zones are locations.
                        if let (Some(from), Some(lane)) =
                            (self.cards[card].last_lane, self.cards[card].lane)
                        {
                            self.finish_move(card, from, lane);
                            if let Some(grant) = staged.grant
                                && let Some(hook) = grant.on_move
                            {
                                hook(self, grant.source, card);
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
                            if !self.cards[card].abilities_removed
                                && let Some(hook) = self.cards[card].ability_def().on_activate
                            {
                                hook(self, card);
                            }
                            self.emit(Event::CardActivated { card });
                        }
                    }
                }
            }
        }
    }
    pub fn move_card(&mut self, card: usize, destination: usize) -> bool {
        let Some(source) = self.cards[card].lane else {
            return false;
        };
        if !self.cards[card].revealed
            || self.cards[card].ability_def().immune_destroy_move
            || destination >= 3
            || source == destination
            || !self.has_space(destination, self.cards[card].owner, 0)
        {
            return false;
        }
        if self.lanes[destination].revealed
            && let Some(test) = self.lanes[destination].def.allows_addition
            && !test(self, destination, card, destination, self.cards[card].owner)
        {
            return false;
        }
        self.lanes[source].cards.retain(|&id| id != card);
        self.cards[card].last_lane = Some(source);
        self.lanes[destination].cards.push(card);
        self.cards[card].lane = Some(destination);
        self.finish_move(card, source, destination);
        true
    }

    fn finish_move(&mut self, card: usize, source: usize, destination: usize) {
        self.cards[card].move_turns.push(self.turn);
        if let Some(hook) = self.cards[card].ability_def().on_move {
            hook(self, card, source);
        }
        let observers = self.lanes[destination].cards.clone();
        for observer in observers {
            if observer != card
                && self.cards[observer].revealed
                && let Some(hook) = self.cards[observer].ability_def().on_move_here
            {
                hook(self, observer, card);
            }
        }
        if self.lanes[destination].revealed
            && let Some(hook) = self.lanes[destination].def.on_card_moved
        {
            hook(self, destination, card, source);
        }
        self.emit(Event::CardMoved {
            card,
            from: source,
            to: destination,
        });
        self.ongoing();
    }
    fn notify_added(&mut self, card: usize, lane: usize) {
        if let Some(hook) = self.cards[card].ability_def().on_added {
            hook(self, card);
        }
        if self.lanes[lane].revealed
            && let Some(hook) = self.lanes[lane].def.on_card_added
        {
            hook(self, lane, card);
        }
        self.emit(Event::CardAdded { card, lane });
    }
    /// StardustBehavior::Restrict (0x7cc357c) disables spawning at all locations.
    pub fn spawns_allowed(&self) -> bool {
        !*HAS_SPAWN_BLOCKERS
            || !self.lanes.iter().flat_map(|lane| &lane.cards).any(|&id| {
                self.cards[id].revealed
                    && self.cards[id].has_ongoing()
                    && self.cards[id].ability_def().blocks_spawns
            })
    }
    pub fn can_add(&self, card: usize, lane: usize, seat: usize) -> bool {
        self.cards[card].lane.is_none()
            && self.has_space(lane, seat, 0)
            && self.spawns_allowed()
            && !self.lane_locked(lane)
            && (!self.lanes[lane].revealed
                || self.lanes[lane]
                    .def
                    .allows_addition
                    .is_none_or(|test| test(self, lane, card, lane, seat)))
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
        for player in &mut self.players {
            player.graveyard.retain(|&(id, _)| id != card);
        }
        self.cards[card].owner = seat;
        self.cards[card].lane = Some(lane);
        self.cards[card].turn_played = 0;
        self.cards[card].revealed = false;
        self.lanes[lane].cards.push(card);
        self.notify_added(card, lane);
        if trigger_reveal {
            self.reveal(card);
        } else {
            self.cards[card].revealed = true;
            self.ongoing();
        }
        true
    }

    /// Stage an added card for a caller-managed batch of post-ability reveals.
    pub fn add_card_deferred(&mut self, card: usize, lane: usize, seat: usize) -> bool {
        if !self.can_add(card, lane, seat) {
            return false;
        }
        for player in &mut self.players {
            player.graveyard.retain(|&(id, _)| id != card);
        }
        self.cards[card].owner = seat;
        self.cards[card].lane = Some(lane);
        self.cards[card].turn_played = 0;
        self.cards[card].revealed = false;
        self.lanes[lane].cards.push(card);
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
        let targets: Vec<_> = (0..3)
            .filter(|&lane| lane != origin && self.can_add(card, lane, seat))
            .collect();
        if targets.is_empty() {
            self.cards.pop();
            return None;
        }
        let choice = self.rng.borrow_mut().randbelow(targets.len());
        // A new Horde is placed first (RandomCreateNewZombie and its siblings) and then grows
        // through AddPowerPermanently, so its growth triggers in play (e.g. Venus).
        self.add_card(card, targets[choice], seat, true);
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
        self.cards[card].owner = seat;
        self.players[seat].hand.push(card);
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
        }
        true
    }
    /// Insert a created card now, but postpone its PostCardCreated observers.
    pub fn add_hand_deferred(&mut self, seat: usize, card: usize) -> bool {
        if self.players[seat].hand.len() >= 7 {
            return false;
        }
        self.cards[card].owner = seat;
        self.players[seat].hand.push(card);
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
            if let Some(hook) = self.cards[observer].ability_def().on_hand_added {
                hook(self, observer, seat);
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
                if let Some(hook) = self.cards[observer].ability_def().on_hand_added {
                    for _ in 0..count {
                        hook(self, observer, seat);
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
        self.lanes[lane].cards.retain(|&id| id != card);
        self.cards[card].last_lane = Some(lane);
        self.cards[card].lane = None;
        self.cards[card].revealed = false;
        self.cards[card].turn_played = 0;
        self.cards[card].ongoing = 0;
        self.cards[card].location_power = 0;
        self.cards[card].watches.clear();
        if let Some(clear) = self.cards[card].ability_def().clear_reveal_state {
            clear(&mut self.cards[card]);
        }
        self.ongoing();
        true
    }
    pub fn discard_card(&mut self, seat: usize, card: usize) {
        let slot = self.players[seat]
            .hand
            .iter()
            .position(|&id| id == card)
            .expect("discard card must be in hand");
        self.players[seat].hand.remove(slot);
        if let Some(hook) = self.cards[card].ability_def().on_discard {
            hook(self, card);
        } else {
            self.players[seat].discard.push(card);
        }
        self.emit(Event::CardDiscarded { card, seat });
    }
    /// Game::BanishCard (0x7dac084) moves the card to its owner's banished zone.
    pub fn banish_card(&mut self, card: usize) -> bool {
        let owner = self.cards[card].owner;
        if self.players[owner].banished.contains(&card) {
            return false;
        }
        if let Some(lane) = self.cards[card].lane.take() {
            self.lanes[lane].cards.retain(|&id| id != card);
            self.cards[card].last_lane = Some(lane);
        }
        for player in &mut self.players {
            player.hand.retain(|&id| id != card);
            player.deck.retain(|&id| id != card);
            player.discard.retain(|&id| id != card);
            player.graveyard.retain(|&(id, _)| id != card);
        }
        self.players[owner].banished.push(card);
        self.ongoing();
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
            .ability_def()
            .initial_state
            .map(|init| init());
        self.cards[card].watches.clear();
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
        let Some(lane) = self.cards[card].lane else {
            return false;
        };
        if self.cards[card].revealed && self.cards[card].ability_def().immune_destroy_move {
            return false;
        }
        if self.lane_locked(lane) || (self.lanes[lane].revealed && self.lanes[lane].def.no_destroy)
        {
            return false;
        }
        if self.lanes[lane].cards.iter().any(|&id| {
            self.cards[id].revealed
                && self.cards[id].has_ongoing()
                && self.cards[id].ability_def().prevents_destroy
        }) {
            return false;
        }
        self.lanes[lane].cards.retain(|&id| id != card);
        self.cards[card].last_lane = Some(lane);
        self.players[self.cards[card].owner]
            .graveyard
            .push((card, Some(lane)));
        if let Some(destroyed) = &mut self.destroyed {
            destroyed.push((card, lane));
        }
        self.cards[card].lane = None;
        if self.cards[card].revealed
            && let Some(hook) = self.cards[card].ability_def().on_destroy
        {
            hook(self, card);
        }
        self.emit(Event::CardDestroyed { card, lane });
        cards::angel_replace(self, self.cards[card].owner, lane);
        true
    }
    /// The deck slot `draw` takes next: a card due this turn, else the first eligible one.
    fn next_draw_slot(&self, seat: usize) -> Option<usize> {
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
        if self.players[seat].hand.len() >= 7 {
            return;
        }
        if let Some(slot) = self.next_draw_slot(seat) {
            let card = self.players[seat].deck.remove(slot);
            self.add_hand(seat, card, true);
        }
    }
    pub fn start_turn(&mut self) {
        for player in &mut self.players {
            player.energy_spent_this_turn = 0;
        }
        for card in &mut self.cards {
            card.observed_can_activate = None;
            card.observed_move_targets = None;
        }
        self.played_on_reveal = [false; 2];
        self.delays_reveals = false;
        for seat in 0..2 {
            self.set_energy(seat, i32::from(self.turn));
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
        // Python: reveal the next scheduled lane unless it was already revealed.
        if self.current_location < usize::from(self.turn).min(self.lanes.len()) {
            let lane = self.location_reveal_order[self.current_location];
            self.current_location += 1;
            if !self.lanes[lane].revealed {
                self.lanes[lane].revealed = true;
                self.lanes[lane].revealed_turn = Some(self.turn);
                self.delays_reveals |= self.lanes[lane].def.delays_reveals;
                self.emit(Event::LocationRevealed { lane });
                if let Some(hook) = self.lanes[lane].def.on_reveal {
                    hook(self, lane);
                }
                self.set_total_turns(self.total_turns.max(self.lanes[lane].def.minimum_turns));
                for seat in 0..2 {
                    let energy = self.players[seat].energy + self.lanes[lane].def.reveal_energy;
                    self.set_energy(seat, energy);
                }
            }
        }
        self.ongoing();
        for lane in 0..3 {
            if self.lanes[lane].revealed
                && let Some(hook) = self.lanes[lane].def.on_turn_start
            {
                hook(self, lane);
                self.ongoing();
            }
        }
        let observers: Vec<_> = self
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| self.cards[id].revealed)
            .collect();
        for id in observers {
            if let Some(hook) = self.cards[id].ability_def().start_turn {
                hook(self, id);
            }
        }
        for seat in 0..2 {
            for lane in 0..3 {
                if self.lanes[lane].revealed
                    && let Some(hook) = self.lanes[lane].def.energy_bonus
                {
                    let energy = self.players[seat].energy + hook(self, lane, seat);
                    self.set_energy(seat, energy);
                }
            }
        }
        self.emit(Event::TurnStart);
        self.is_wait_for_staging = true;
    }
    pub fn powers(&self) -> [[i32; 2]; 3] {
        std::array::from_fn(|lane| {
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
        })
    }
    pub fn winner(&self) -> i32 {
        let powers = self.powers();
        let mut wins = [0, 0];
        let mut total = [0, 0];
        for p in powers {
            wins[0] += i32::from(p[0] > p[1]);
            wins[1] += i32::from(p[1] > p[0]);
            total[0] += p[0];
            total[1] += p[1];
        }
        if (wins[0], total[0]) > (wins[1], total[1]) {
            0
        } else if (wins[1], total[1]) > (wins[0], total[0]) {
            1
        } else {
            2
        }
    }
    pub fn ongoing(&mut self) {
        for lane in &mut self.lanes {
            lane.bonus = [0; 2];
            lane.multiplier = [1; 2];
        }
        for card in &mut self.cards {
            card.ongoing = 0;
            card.ongoing_cost = 0;
            card.location_power = 0;
            card.stat_updates.retain(|update| !update.ongoing);
        }
        for seat in 0..2 {
            for index in 0..self.players[seat].hand.len() {
                let card = self.players[seat].hand[index];
                if !self.cards[card].abilities_removed
                    && let Some(hook) = self.cards[card].ability_def().in_hand_ongoing
                {
                    hook(self, card);
                }
            }
        }
        for lane in 0..3 {
            let base = if self.lanes[lane].revealed {
                usize::from(self.lanes[lane].def.ongoing_repeats)
            } else {
                1
            };
            let mut extra = [0usize; 2];
            for &id in &self.lanes[lane].cards {
                if self.cards[id].revealed && self.cards[id].has_ongoing() {
                    extra[self.cards[id].owner] +=
                        base * usize::from(self.cards[id].ability_def().extra_ongoing_runs);
                }
            }
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if self.cards[id].revealed
                    && self.cards[id].has_ongoing()
                    && let Some(hook) = self.cards[id].ability_def().ongoing
                {
                    let runs = base + extra[self.cards[id].owner]
                        - base * usize::from(self.cards[id].ability_def().extra_ongoing_runs);
                    for _ in 0..runs {
                        hook(self, id);
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
                        && (power >= 0 || self.cards[id].can_reduce_power())
                    {
                        self.add_location_power(id, power);
                    }
                }
                if let Some(hook) = self.lanes[lane].def.ongoing {
                    hook(self, lane);
                }
            }
        }
        for lane in 0..3 {
            let base = if self.lanes[lane].revealed {
                usize::from(self.lanes[lane].def.ongoing_repeats)
            } else {
                1
            };
            let mut extra = [0usize; 2];
            for &id in &self.lanes[lane].cards {
                if self.cards[id].revealed && self.cards[id].has_ongoing() {
                    extra[self.cards[id].owner] +=
                        base * usize::from(self.cards[id].ability_def().extra_ongoing_runs);
                }
            }
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if self.cards[id].revealed
                    && self.cards[id].has_ongoing()
                    && let Some(hook) = self.cards[id].ability_def().late_ongoing
                {
                    let runs = base + extra[self.cards[id].owner]
                        - base * usize::from(self.cards[id].ability_def().extra_ongoing_runs);
                    for _ in 0..runs {
                        hook(self, id);
                    }
                }
            }
        }
        for lane in 0..3 {
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if !self.cards[id].abilities_removed
                    && (self.cards[id].revealed || self.cards[id].turn_played > 0)
                    && let Some(hook) = self.cards[id].ability_def().objective_update
                {
                    hook(self, id);
                }
            }
        }
        self.emit(Event::Ongoing);
    }
    pub fn printed_cost(&self, card: usize) -> i32 {
        CATALOG[self.cards[card].def.id].cost
    }

    pub fn trigger_on_reveal(&mut self, card: usize) {
        let Some(lane) = self.cards[card].lane else {
            return;
        };
        let Some(hook) = self.cards[card].ability_def().reveal else {
            return;
        };
        if self.cards[card].abilities_removed
            || self.lanes[lane].cards.iter().any(|&id| {
                self.cards[id].revealed
                    && self.cards[id].has_ongoing()
                    && self.cards[id].ability_def().blocks_reveal
            })
        {
            return;
        }
        let active = self.lanes[lane].revealed;
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
            if !self.cards[source].revealed || !self.cards[source].has_ongoing() {
                continue;
            }
            if let Some(repetitions) = self.cards[source].ability_def().reveal_repetitions {
                let additional = usize::from(repetitions(self, source, card));
                let extra: usize = self.lanes[lane]
                    .cards
                    .iter()
                    .copied()
                    .filter(|&other| {
                        other != source
                            && self.cards[other].revealed
                            && self.cards[other].has_ongoing()
                            && self.cards[other].owner == self.cards[source].owner
                    })
                    .map(|other| usize::from(self.cards[other].ability_def().extra_ongoing_runs))
                    .sum();
                runs += additional * ongoing_base * (1 + extra);
            }
        }
        let original = self.cards[card].def.id;
        for _ in 0..runs {
            hook(self, card);
            if self.cards[card].lane.is_none() || self.cards[card].def.id != original {
                break;
            }
        }
    }
    pub fn reveal(&mut self, id: usize) {
        let Some(lane) = self.cards[id].lane else {
            return;
        };
        if self.cards[id].revealed {
            return;
        }
        if self.delays_reveals {
            let pending = &mut self.delayed_reveals[self.cards[id].owner];
            if !pending.contains(&id) {
                pending.push(id);
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
        if self.lanes[lane].revealed
            && let Some(hook) = self.lanes[lane].def.before_card_reveal
        {
            hook(self, lane, id);
            if self.cards[id].lane.is_none() {
                self.ongoing();
                return;
            }
        }
        self.emit(Event::BeforeCardRevealed { card: id });
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
        if self.cards[id].turn_played > 0 && self.cards[id].ability_def().reveal.is_some() {
            self.played_on_reveal[self.cards[id].owner] = true;
        }
        if self.cards[id].turn_played > 0 {
            let seat = self.cards[id].owner;
            if (self.cards[id].ability_def().move_limit > 0
                || self.matching_move_grant(id, seat).is_some())
                && !self.players[seat].movable_cards_played.contains(&id)
            {
                self.players[seat].movable_cards_played.push(id);
            }
        }
        self.cards[id].latest_resolution_order = self.next_resolution_order;
        self.next_resolution_order += 1;
        self.trigger_on_reveal(id);
        self.cards[id].revealed = true;
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
                if let Some(hook) = self.cards[other].ability_def().on_play {
                    hook(self, other, id);
                }
            }
        }
        if self.cards[id].turn_played > 0 {
            self.players[self.cards[id].owner]
                .play_history
                .push((self.turn, id));
            let seat = self.cards[id].owner;
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
        if self.cards[id].turn_played > 0
            && let Some(lane) = self.cards[id].lane
            && self.lanes[lane].revealed
            && let Some(hook) = self.lanes[lane].def.after_card_play
        {
            hook(self, lane, id);
            self.ongoing();
        }
        if self.is_skill(id)
            && let Some(lane) = self.cards[id].lane
        {
            self.lanes[lane].cards.retain(|&card| card != id);
            self.cards[id].last_lane = Some(lane);
            self.cards[id].lane = None;
            self.players[self.cards[id].owner].banished.push(id);
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
    }
    /// Start the next turn: clear last turn's plays and draw, reveal and refresh.
    fn begin_turn(&mut self) {
        self.played = Default::default();
        self.pending_plays = std::array::from_fn(|seat| {
            self.delayed_reveals[seat]
                .iter()
                .filter_map(|&id| self.cards[id].lane.map(|lane| (id, lane)))
                .collect()
        });
        self.start_turn();
    }
    fn action_priority(&self, priority: Option<usize>) -> usize {
        priority.unwrap_or_else(|| {
            let winner = self.winner();
            if winner == 2 {
                self.rng.borrow_mut().randbelow(2)
            } else {
                winner as usize
            }
        })
    }

    fn resolve_action_plans(
        &mut self,
        plans: [Vec<Action>; 2],
        priority: usize,
    ) -> Result<TurnTrace, String> {
        let plays = std::array::from_fn(|seat| {
            plans[seat]
                .iter()
                .filter_map(|action| {
                    if let Action::Play { card, lane } = *action {
                        Some(Played {
                            card: self.cards[card].def.id,
                            lane,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        });
        // Card::IsMovableByPlayer (0x7cb0cac) and Card::CanActivate
        // (0x7cb9a24) evaluate a player's own staging state. Validate each
        // plan on a separate copy, before the opponent's hidden staging can
        // move ongoing effects or otherwise change those checks.
        let mut costs: [Vec<i32>; 2] = std::array::from_fn(|_| Vec::new());
        for seat in 0..2 {
            let mut rng = self.fork_rng();
            let mut view = self.fork(rng.as_mut());
            for &action in &plans[seat] {
                let cost = match action {
                    Action::Play { card, .. } => view.card_cost(card),
                    Action::Activate { card } => view.cards[card].ability_def().activate_cost,
                    Action::Move { .. } => 0,
                };
                view.stage_action(seat, action)?;
                costs[seat].push(cost);
            }
        }
        for (seat, plan) in plans.into_iter().enumerate() {
            for (action, cost) in plan.into_iter().zip(costs[seat].iter().copied()) {
                self.apply_staged_action(seat, action, cost);
            }
        }
        self.ongoing();
        self.resolve_staged_actions(priority);
        for player in &mut self.players {
            player.energy_history.push((
                self.turn,
                player.energy_spent_this_turn,
                player.energy == 0,
            ));
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
            self.ongoing();
        }
        self.turn += 1;
        Ok(result)
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
            [
                policy.plan_actions(self, 0, &mut **rng),
                policy.plan_actions(self, 1, &mut **rng),
            ]
        };
        self.resolve_action_plans(plans, priority)
    }
    pub fn play_turn_with_actions<F>(&mut self, mut planner: F) -> Result<TurnTrace, String>
    where
        F: FnMut(&Game<'a>, usize, usize) -> Result<Vec<Action>, String>,
    {
        self.begin_turn();
        let priority = self.action_priority(None);
        let plans = [planner(self, 0, priority)?, planner(self, 1, priority)?];
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
                .filter_map(|&id| self.cards[id].lane.map(|lane| (id, lane)))
                .collect()
        });
        let priority = self.action_priority(priority);
        let opponent = {
            let mut rng = self.rng.borrow_mut();
            policy.plan_actions(self, 1, &mut **rng)
        };
        self.resolve_action_plans([actions.to_vec(), opponent], priority)
    }

    pub fn play_turn(&mut self, policy: crate::policy::Policy) -> TurnTrace {
        self.play_turn_actions(policy)
            .expect("built-in action policies always plan legal actions")
    }

    /// Explicit frozen-reference playout: never voluntarily move or activate.
    pub fn play_turn_plays_only(&mut self, policy: crate::policy::Policy) -> TurnTrace {
        self.begin_turn();
        self.resolve_started_turn(None, |game, _| {
            let mut rng = game.rng.borrow_mut();
            Ok([
                policy.plan(game, 0, &mut **rng),
                policy.plan(game, 1, &mut **rng),
            ])
        })
        .expect("built-in policies always plan")
    }

    /// Play a turn whose plans come from `planner(game, seat, priority)`, called for seat
    /// 0 and then seat 1 after the turn has started and reveal priority was chosen (the
    /// same RNG order as Python's turn loop). Plans hold arena card indices and lanes.
    pub fn play_turn_with<F>(&mut self, mut planner: F) -> Result<TurnTrace, String>
    where
        F: FnMut(&Game<'a>, usize, usize) -> Result<Vec<(usize, usize)>, String>,
    {
        self.begin_turn();
        self.resolve_started_turn(None, |game, priority| {
            Ok([planner(game, 0, priority)?, planner(game, 1, priority)?])
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
        self.pending_plays = std::array::from_fn(|seat| {
            self.delayed_reveals[seat]
                .iter()
                .filter_map(|&id| self.cards[id].lane.map(|lane| (id, lane)))
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
        let pending = std::mem::take(&mut self.delayed_reveals);
        for seat in pending {
            for id in seat {
                self.reveal(id);
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
    }

    /// Apply the effects at the end of an already-revealed turn.
    pub fn end_turn_effects(&mut self) {
        let observers: Vec<_> = self
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| self.cards[id].revealed)
            .collect();
        for id in observers {
            if let Some(hook) = self.cards[id].ability_def().end_turn {
                hook(self, id);
            }
        }
        self.ongoing();
        for lane in 0..3 {
            if self.lanes[lane].revealed
                && let Some(hook) = self.lanes[lane].def.on_turn_end
            {
                hook(self, lane);
                self.ongoing();
            }
        }
        self.emit(Event::TurnEnd);
        for seat in 0..2 {
            self.trigger_player_effects(seat, PlayerTrigger::EndTurn, None);
        }
        self.ongoing();
        for lane in 0..3 {
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if self.cards[id].revealed
                    && !self.cards[id].abilities_removed
                    && let Some(hook) = self.cards[id].ability_def().objective_freeze
                {
                    hook(self, id);
                }
            }
        }
        self.emit(Event::AfterTurnEnd);
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
                let hand = &mut self.players[seat].hand;
                let slot = hand
                    .iter()
                    .position(|&card| card == id)
                    .expect("planned card not in hand");
                hand.remove(slot);
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
                self.lanes[lane].cards.push(id);
                self.notify_added(id, lane);
            }
        }
        if self.played.iter().any(|plays| !plays.is_empty()) {
            self.ongoing();
        }
        self.is_wait_for_staging = false;
        for seat in [priority, 1 - priority] {
            for &id in &pending[seat] {
                self.reveal(id);
            }
            for index in 0..self.played[seat].len() {
                let id = self.played[seat][index].0;
                self.reveal(id);
            }
        }
        for player in &mut self.players {
            player.energy_history.push((
                self.turn,
                player.energy_spent_this_turn,
                player.energy == 0,
            ));
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
            self.ongoing();
        }
        self.turn += 1;
        Ok(result)
    }
}

#[cfg(test)]
mod staging_tests {
    use super::*;
    use crate::random::PythonRandom;

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

    fn place(game: &mut Game<'_>, name: &str, seat: usize, lane: usize) -> usize {
        let card = game.create_card(name, seat);
        game.cards[card].lane = Some(lane);
        game.cards[card].revealed = true;
        game.lanes[lane].cards.push(card);
        card
    }

    #[test]
    fn opponent_staged_move_does_not_change_own_staging_legality() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let professor = place(&mut game, "ProfessorX", 0, 0);
        game.grant_move(MoveGrant {
            seat: 0,
            source: professor,
            card: Some(professor),
            lane: None,
            start_turn: game.turn,
            end_turn: game.turn,
            remaining: 1,
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
