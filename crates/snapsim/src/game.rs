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

pub struct Card {
    pub def: &'static CardDef,
    pub owner: usize,
    pub cost: i32,
    pub ongoing_cost: i32,
    /// Lasting cost before location modifiers.
    pub created: bool,
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
    pub turn_played: u8,
    pub drawn_turn: Option<u8>,
    pub move_turns: Vec<u8>,
    pub merged_to: Option<usize>,
    pub watches: Vec<(u8, usize)>,
    pub state: Option<Box<dyn CardState>>,
}
impl Card {
    pub fn power(&self) -> i32 {
        self.permanent + self.ongoing + self.location_power
    }
    pub fn has_ongoing(&self) -> bool {
        !self.abilities_removed
            && (self.def.ongoing.is_some()
                || self.def.late_ongoing.is_some()
                || self.def.ongoing_ability
                || self.def.cost_guard.is_some()
                || self.def.blocks_spawns
                || self.def.blocks_reveal
                || self.def.prevents_destroy
                || self.def.locks_lane)
    }
    pub fn can_reduce_power(&self) -> bool {
        !self.revealed || !self.def.immune_destroy_move
    }

    pub fn reduce_power(&mut self, amount: i32) {
        if self.can_reduce_power() {
            self.permanent -= amount;
        }
    }
    pub fn has_no_ability(&self) -> bool {
        self.abilities_removed
            || CATALOG
                .get(self.def.id)
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
    ThisTurn,
    UntilTriggered,
}

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

#[derive(Default)]
pub struct Player {
    pub hand: Vec<usize>,
    pub deck: Vec<usize>,
    pub discard: Vec<usize>,
    pub banished: Vec<usize>,
    /// Cards currently in the graveyard, in destruction order, with their death location.
    pub graveyard: Vec<(usize, usize)>,
    pub pending_effects: Vec<PendingEffect>,
    /// Drawn cards, preserving the turn and draw order.
    pub draws: Vec<(u8, usize)>,
    /// Resolved played cards, preserving turn and resolution order.
    pub play_history: Vec<(u8, usize)>,
    /// (turn, energy spent, all available energy spent).
    pub energy_history: Vec<(u8, i32, bool)>,
    pub energy_spent_this_turn: i32,
    pub energy: i32,
    pub next_play_power: i32,
    pub pending_left_moves: u32,
}
pub struct Lane {
    pub def: &'static LocationDef,
    /// Original ID when an unsupported location is represented by an inert rule.
    pub observed_id: Option<String>,
    pub revealed: bool,
    pub revealed_turn: Option<u8>,
    pub cards: Vec<usize>,
    pub bonus: [i32; 2],
    pub multiplier: [i32; 2],
    pub state: Option<Box<dyn std::any::Any>>,
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
    CardAdded {
        card: usize,
        lane: usize,
    },
    CardMoved {
        card: usize,
        from: usize,
        to: usize,
    },
    CardDestroyed {
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
    pub turn: u8,
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
    fn emit(&mut self, event: Event) {
        if !*HAS_EVENT_HOOKS {
            return;
        }
        let observers: Vec<_> = self
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| self.cards[id].revealed && self.cards[id].def.on_event.is_some())
            .collect();
        for card in observers {
            if self.cards[card].lane.is_some()
                && let Some(hook) = self.cards[card].def.on_event
            {
                hook(self, card, event);
            }
        }
        if matches!(
            event,
            Event::GameStart | Event::Empower { .. } | Event::CardDrawn { .. }
        ) {
            let observers: Vec<_> = self
                .players
                .iter()
                .flat_map(|player| player.hand.iter().chain(&player.deck).copied())
                .filter(|&id| self.cards[id].def.on_event.is_some())
                .collect();
            for card in observers {
                if let Some(hook) = self.cards[card].def.on_event {
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
    /// Install an independent instance; repeated reveals can install repeated effects.
    pub fn add_player_effect(&mut self, seat: usize, effect: PendingEffect) {
        self.players[seat].pending_effects.push(effect);
    }

    fn trigger_player_effects(&mut self, seat: usize, trigger: PlayerTrigger, card: Option<usize>) {
        let effects = std::mem::take(&mut self.players[seat].pending_effects);
        let mut survivors = Vec::new();
        for mut effect in effects {
            let fired = effect.trigger == trigger
                && !(card == Some(effect.source) && effect.created_turn == self.turn);
            let completed = fired && (effect.apply)(self, &mut effect, card);
            let expired = fired && effect.lifetime == EffectLifetime::NextCard
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

    fn change_stat(
        &mut self,
        card: usize,
        stat: Stat,
        operation: StatOperation,
        value: i32,
        source: usize,
        is_steal: bool,
    ) {
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
        if amount >= 0 || self.cards[card].can_reduce_power() {
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
                    self.cards[card].ongoing += amount;
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
            .retain(|update| update.stat != Stat::Power);
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
                    owner: seat,
                    cost: data.cost,
                    ongoing_cost: 0,
                    created: false,
                    permanent: data.power,
                    stat_updates: Vec::new(),
                    ongoing: 0,
                    location_power: 0,
                    lane: None,
                    last_lane: None,
                    revealed: false,
                    abilities_removed: false,
                    repeating: false,
                    turn_played: 0,
                    drawn_turn: None,
                    move_turns: Vec::new(),
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
            owner,
            ongoing_cost: 0,
            cost: row.cost,
            permanent: row.power,
            stat_updates: Vec::new(),
            created: true,
            ongoing: 0,
            location_power: 0,
            lane: None,
            last_lane: None,
            revealed: false,
            abilities_removed: false,
            repeating: false,
            turn_played: 0,
            drawn_turn: None,
            move_turns: Vec::new(),
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
        self.cards[card].state = self.cards[source]
            .state
            .as_deref()
            .map(CardState::clone_box);
        if let Some(clear) = self.cards[card].def.clear_reveal_state {
            clear(&mut self.cards[card]);
        }
        card
    }

    pub fn lane_locked(&self, lane: usize) -> bool {
        self.lanes[lane].cards.iter().any(|&card| {
            self.cards[card].revealed
                && self.cards[card].has_ongoing()
                && self.cards[card].def.locks_lane
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
                && let Some(hook) = self.cards[card].def.modify_final_cost
            {
                let next = hook(self, card, cost);
                if self.cost_change_allowed(card, cost, next) {
                    cost = next;
                }
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
    pub fn move_card(&mut self, card: usize, destination: usize) -> bool {
        let Some(source) = self.cards[card].lane else {
            return false;
        };
        if !self.cards[card].revealed
            || self.cards[card].def.immune_destroy_move
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
        self.cards[card].move_turns.push(self.turn);
        if let Some(hook) = self.cards[card].def.on_move {
            hook(self, card, source);
        }
        let observers = self.lanes[destination].cards.clone();
        for observer in observers {
            if observer != card
                && self.cards[observer].revealed
                && let Some(hook) = self.cards[observer].def.on_move_here
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
        true
    }
    fn notify_added(&mut self, card: usize, lane: usize) {
        if let Some(hook) = self.cards[card].def.on_added {
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
                    && self.cards[id].def.blocks_spawns
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

    pub fn horde(&mut self, origin: usize, seat: usize, power: i32) -> Option<usize> {
        for lane in &self.lanes {
            for &card in &lane.cards {
                if self.cards[card].owner == seat && self.cards[card].def.id == "ZombieHorde" {
                    self.add_power(card, power, seat);
                    if power != 0 {
                        self.ongoing();
                    }
                    return Some(card);
                }
            }
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
        self.add_power(card, power, seat);
        self.add_card(card, targets[choice], seat, true);
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
            if let Some(hook) = self.cards[observer].def.on_hand_added {
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
                if let Some(hook) = self.cards[observer].def.on_hand_added {
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
        if let Some(clear) = self.cards[card].def.clear_reveal_state {
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
        if let Some(hook) = self.cards[card].def.on_discard {
            hook(self, card);
        } else {
            self.players[seat].discard.push(card);
        }
        self.emit(Event::CardDiscarded { card, seat });
    }
    pub fn destroy_card(&mut self, card: usize) -> bool {
        let Some(lane) = self.cards[card].lane else {
            return false;
        };
        if self.cards[card].revealed && self.cards[card].def.immune_destroy_move {
            return false;
        }
        if self.lane_locked(lane) || (self.lanes[lane].revealed && self.lanes[lane].def.no_destroy)
        {
            return false;
        }
        if self.lanes[lane].cards.iter().any(|&id| {
            self.cards[id].revealed
                && self.cards[id].has_ongoing()
                && self.cards[id].def.prevents_destroy
        }) {
            return false;
        }
        self.lanes[lane].cards.retain(|&id| id != card);
        self.cards[card].last_lane = Some(lane);
        self.players[self.cards[card].owner]
            .graveyard
            .push((card, lane));
        if let Some(destroyed) = &mut self.destroyed {
            destroyed.push((card, lane));
        }
        self.cards[card].lane = None;
        if self.cards[card].revealed
            && let Some(hook) = self.cards[card].def.on_destroy
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
    /// The top card of a deck: the card `draw` would take next.
    pub fn top_card(&self, seat: usize) -> Option<usize> {
        self.next_draw_slot(seat)
            .map(|slot| self.players[seat].deck[slot])
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
        self.played_on_reveal = [false; 2];
        self.delays_reveals = false;
        for seat in 0..2 {
            self.set_energy(seat, i32::from(self.turn));
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
            if let Some(hook) = self.cards[id].def.start_turn {
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
                    && let Some(hook) = self.cards[card].def.in_hand_ongoing
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
                        base * usize::from(self.cards[id].def.extra_ongoing_runs);
                }
            }
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if self.cards[id].revealed
                    && self.cards[id].has_ongoing()
                    && let Some(hook) = self.cards[id].def.ongoing
                {
                    let runs = base + extra[self.cards[id].owner]
                        - base * usize::from(self.cards[id].def.extra_ongoing_runs);
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
                        base * usize::from(self.cards[id].def.extra_ongoing_runs);
                }
            }
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if self.cards[id].revealed
                    && self.cards[id].has_ongoing()
                    && let Some(hook) = self.cards[id].def.late_ongoing
                {
                    let runs = base + extra[self.cards[id].owner]
                        - base * usize::from(self.cards[id].def.extra_ongoing_runs);
                    for _ in 0..runs {
                        hook(self, id);
                    }
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
        let Some(hook) = self.cards[card].def.reveal else {
            return;
        };
        if self.cards[card].abilities_removed
            || self.lanes[lane].cards.iter().any(|&id| {
                self.cards[id].revealed
                    && self.cards[id].has_ongoing()
                    && self.cards[id].def.blocks_reveal
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
            if let Some(repetitions) = self.cards[source].def.reveal_repetitions {
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
                    .map(|other| usize::from(self.cards[other].def.extra_ongoing_runs))
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
        if self.cards[id].turn_played > 0 && self.cards[id].def.reveal.is_some() {
            self.played_on_reveal[self.cards[id].owner] = true;
        }
        self.trigger_on_reveal(id);
        self.cards[id].revealed = true;
        self.emit(Event::CardRevealed { card: id });
        if self.cards[id].turn_played > 0 {
            let observers: Vec<_> = self
                .lanes
                .iter()
                .flat_map(|lane| lane.cards.iter().copied())
                .filter(|&other| other != id && self.cards[other].revealed)
                .collect();
            for other in observers {
                if let Some(hook) = self.cards[other].def.on_play {
                    hook(self, other, id);
                }
            }
        }
        if self.cards[id].turn_played > 0 {
            self.players[self.cards[id].owner]
                .play_history
                .push((self.turn, id));
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

    pub fn play_turn(&mut self, policy: crate::policy::Policy) -> TurnTrace {
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
            if let Some(hook) = self.cards[id].def.end_turn {
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
                self.lanes[lane].cards.push(id);
                self.notify_added(id, lane);
            }
        }
        if self.played.iter().any(|plays| !plays.is_empty()) {
            self.ongoing();
        }
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
