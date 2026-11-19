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

pub struct Card {
    pub def: &'static CardDef,
    pub owner: usize,
    pub cost: i32,
    /// Lasting cost before location modifiers.
    pub created: bool,
    pub permanent: i32,
    pub ongoing: i32,
    pub location_power: i32,
    pub lane: Option<usize>,
    pub revealed: bool,
    pub abilities_removed: bool,
    pub repeating: bool,
    pub turn_played: u8,
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
                || self.def.ongoing_ability
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
#[derive(Default)]
pub struct Player {
    pub hand: Vec<usize>,
    pub deck: Vec<usize>,
    pub discard: Vec<usize>,
    pub energy: i32,
    pub next_play_power: i32,
    pub pending_left_moves: u32,
}
pub struct Lane {
    pub def: &'static LocationDef,
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
    TurnStart,
    TurnEnd,
    Ongoing,
    CardRevealed { card: usize },
    CardPlayed { card: usize },
    CardAdded { card: usize, lane: usize },
    CardMoved { card: usize, from: usize, to: usize },
    CardDestroyed { card: usize, lane: usize },
    CardDiscarded { card: usize, seat: usize },
    LocationRevealed { lane: usize },
    EnergyChanged { seat: usize, old: i32, new: i32 },
    TurnCountChanged { old: u8, new: u8 },
}

pub struct Game<'a> {
    pub cards: Vec<Card>,
    pub players: [Player; 2],
    pub lanes: [Lane; 3],
    pub location_pool: Vec<&'static LocationDef>,
    pub turn: u8,
    pub total_turns: u8,
    pub played: [Vec<(usize, usize)>; 2],
    pub pending_plays: [Vec<(usize, usize)>; 2],
    pub played_on_reveal: [bool; 2],
    pub delays_reveals: bool,
    pub delayed_reveals: [Vec<usize>; 2],
    pub rng: RefCell<&'a mut dyn GameRng>,
}

static CATALOG: LazyLock<HashMap<String, data::CardData>> = LazyLock::new(|| {
    data::cards()
        .into_iter()
        .map(|row| (row.carddefid.clone(), row))
        .collect()
});
static HAS_EVENT_HOOKS: LazyLock<bool> = LazyLock::new(|| {
    cards::REGISTRY.iter().any(|entry| entry.on_event.is_some())
        || locations::REGISTRY
            .iter()
            .any(|entry| entry.on_event.is_some())
});

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
        for lane in 0..3 {
            if self.lanes[lane].revealed
                && let Some(hook) = self.lanes[lane].def.on_event
            {
                hook(self, lane, event);
            }
        }
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
                if cards::NON_OWNABLE.contains(&id.as_str()) {
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
                    created: false,
                    permanent: data.power,
                    ongoing: 0,
                    location_power: 0,
                    lane: None,
                    revealed: false,
                    abilities_removed: false,
                    repeating: false,
                    turn_played: 0,
                    watches: Vec::new(),
                    state: def.initial_state.map(|init| init()),
                });
                players[seat].deck.push(index);
            }
            rng.shuffle(&mut players[seat].deck);
            if let Some(slot) = players[seat]
                .deck
                .iter()
                .position(|&index| cards[index].def.id == "Quicksilver")
            {
                let card = players[seat].deck.remove(slot);
                players[seat].hand.push(card);
            }
            while players[seat].hand.len() < 3 {
                let Some(slot) = players[seat]
                    .deck
                    .iter()
                    .position(|&index| cards[index].def.draw_turn.is_none())
                else {
                    break;
                };
                let card = players[seat].deck.remove(slot);
                players[seat].hand.push(card);
            }
        }
        let chosen = rng.sample_indices(available.len(), 3);
        let lanes = std::array::from_fn(|i| Lane {
            def: available[chosen[i]],
            revealed: false,
            revealed_turn: None,
            cards: Vec::new(),
            bonus: [0; 2],
            multiplier: [1; 2],
            state: available[chosen[i]].initial_state.map(|init| init()),
        });
        Ok(Self {
            cards,
            players,
            lanes,
            location_pool: available,
            turn: 1,
            total_turns: 6,
            played: Default::default(),
            pending_plays: Default::default(),
            played_on_reveal: [false; 2],
            delays_reveals: false,
            delayed_reveals: Default::default(),
            rng: RefCell::new(rng),
        })
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
        let def = cards::get(id).expect("unregistered created card");
        let row = CATALOG.get(id).expect("missing created card data");
        let index = self.cards.len();
        self.cards.push(Card {
            def,
            owner,
            cost: row.cost,
            permanent: row.power,
            created: true,
            ongoing: 0,
            location_power: 0,
            lane: None,
            revealed: false,
            abilities_removed: false,
            repeating: false,
            turn_played: 0,
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
        let mut cost = self.cards[card].cost;
        if self
            .players
            .iter()
            .any(|player| player.hand.contains(&card))
        {
            for (lane, location) in self.lanes.iter().enumerate() {
                if location.revealed
                    && let Some(hook) = location.def.modify_cost
                {
                    cost = hook(self, lane, card, cost);
                }
            }
        }
        cost.max(0)
    }

    /// Replace and immediately reveal a location without moving its cards.
    pub fn replace_location(&mut self, lane: usize, id: &str) {
        let def = locations::get(id).expect("unregistered replacement location");
        self.lanes[lane].def = def;
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
        self.lanes[destination].cards.push(card);
        self.cards[card].lane = Some(destination);
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
    pub fn can_add(&self, card: usize, lane: usize, seat: usize) -> bool {
        self.cards[card].lane.is_none()
            && self.has_space(lane, seat, 0)
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
                    self.cards[card].permanent += power;
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
        self.cards[card].permanent += power;
        self.add_card(card, targets[choice], seat, true);
        Some(card)
    }
    pub fn add_hand(&mut self, seat: usize, card: usize, from_deck: bool) -> bool {
        if self.players[seat].hand.len() >= 7 {
            return false;
        }
        self.cards[card].owner = seat;
        self.players[seat].hand.push(card);
        if !from_deck {
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
        self.ongoing();
        true
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
    pub fn draw(&mut self, seat: usize) {
        if self.players[seat].hand.len() >= 7 {
            return;
        }
        let deck = &self.players[seat].deck;
        let chosen = deck
            .iter()
            .position(|&id| self.cards[id].def.draw_turn == Some(self.turn))
            .or_else(|| {
                deck.iter().position(|&id| {
                    self.cards[id]
                        .def
                        .draw_turn
                        .is_none_or(|turn| turn <= self.turn)
                })
            });
        if let Some(slot) = chosen {
            let card = self.players[seat].deck.remove(slot);
            self.add_hand(seat, card, true);
        }
    }
    pub fn start_turn(&mut self) {
        self.played_on_reveal = [false; 2];
        self.delays_reveals = false;
        for seat in 0..2 {
            self.set_energy(seat, i32::from(self.turn));
            self.draw(seat);
        }
        if self.turn <= 3 && !self.lanes[usize::from(self.turn - 1)].revealed {
            let lane = usize::from(self.turn - 1);
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
            card.location_power = 0;
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
                for &id in &self.lanes[lane].cards {
                    if self.cards[id].revealed && (power >= 0 || self.cards[id].can_reduce_power())
                    {
                        self.cards[id].location_power += power;
                    }
                }
                if let Some(hook) = self.lanes[lane].def.ongoing {
                    hook(self, lane);
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
            self.cards[id].permanent += self.players[seat].next_play_power;
            self.players[seat].next_play_power = 0;
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
            self.emit(Event::CardPlayed { card: id });
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
        for _ in 0..left_moves {
            let Some(lane) = self.cards[id].lane else {
                break;
            };
            if lane == 0 || !self.move_card(id, lane - 1) {
                break;
            }
        }
    }
    pub fn play_turn(&mut self, policy: crate::policy::Policy) -> TurnTrace {
        let pending = std::mem::take(&mut self.delayed_reveals);
        self.pending_plays = std::array::from_fn(|seat| {
            pending[seat]
                .iter()
                .filter_map(|&id| self.cards[id].lane.map(|lane| (id, lane)))
                .collect()
        });
        self.start_turn();
        let winner = self.winner();
        let priority = if winner == 2 {
            self.rng.borrow_mut().randbelow(2)
        } else {
            winner as usize
        };
        let plans = {
            let mut rng = self.rng.borrow_mut();
            [
                policy.plan(self, 0, &mut **rng),
                policy.plan(self, 1, &mut **rng),
            ]
        };
        let plays = std::array::from_fn(|seat| {
            plans[seat]
                .iter()
                .map(|&(card, lane)| Played {
                    card: self.cards[card].def.id,
                    lane,
                })
                .collect()
        });
        self.played = plans;
        for seat in 0..2 {
            for index in 0..self.played[seat].len() {
                let (id, lane) = self.played[seat][index];
                let hand = &mut self.players[seat].hand;
                let slot = hand
                    .iter()
                    .position(|&card| card == id)
                    .expect("planned card not in hand");
                hand.remove(slot);
                let energy = self.players[seat].energy - self.card_cost(id);
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
        let result = TurnTrace {
            turn: self.turn,
            priority,
            plays,
            powers: self.powers(),
        };
        if self.turn >= self.total_turns {
            self.delays_reveals = false;
            let pending = std::mem::take(&mut self.delayed_reveals);
            for seat in pending {
                for id in seat {
                    self.reveal(id);
                }
            }
        }
        self.turn += 1;
        result
    }
}
