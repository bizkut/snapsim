//! Game state and turn lifecycle. Board and hand entries are stable arena indices.
use crate::{
    cards::{self, CardDef},
    data,
    locations::{self, LocationDef},
    random::GameRng,
};
use serde::Serialize;
use std::{cell::RefCell, collections::HashMap, sync::LazyLock};

pub struct Card {
    pub def: &'static CardDef,
    pub owner: usize,
    pub cost: i32,
    pub permanent: i32,
    pub ongoing: i32,
    pub location_power: i32,
    pub lane: Option<usize>,
    pub revealed: bool,
    pub turn_played: u8,
    pub watches: Vec<(u8, usize)>,
    pub state: Option<Box<dyn std::any::Any>>,
}
impl Card {
    pub fn power(&self) -> i32 {
        self.permanent + self.ongoing + self.location_power
    }
}
#[derive(Default)]
pub struct Player {
    pub hand: Vec<usize>,
    pub deck: Vec<usize>,
    pub discard: Vec<usize>,
    pub energy: i32,
}
pub struct Lane {
    pub def: &'static LocationDef,
    pub revealed: bool,
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
    pub turn: u8,
    pub total_turns: u8,
    pub played: [Vec<(usize, usize)>; 2],
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
                let def = cards::get(id).ok_or_else(|| format!("unsupported card {id}"))?;
                let data = CATALOG
                    .get(id)
                    .ok_or_else(|| format!("missing catalog card {id}"))?;
                let index = cards.len();
                cards.push(Card {
                    def,
                    owner: seat,
                    cost: data.cost,
                    permanent: data.power,
                    ongoing: 0,
                    location_power: 0,
                    lane: None,
                    revealed: false,
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
            cards: Vec::new(),
            bonus: [0; 2],
            multiplier: [1; 2],
            state: available[chosen[i]].initial_state.map(|init| init()),
        });
        Ok(Self {
            cards,
            players,
            lanes,
            turn: 1,
            total_turns: 6,
            played: Default::default(),
            rng: RefCell::new(rng),
        })
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
            ongoing: 0,
            location_power: 0,
            lane: None,
            revealed: false,
            turn_played: 0,
            watches: Vec::new(),
            state: def.initial_state.map(|init| init()),
        });
        index
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
        if self.cards[card].cost > energy || !self.has_space(lane, seat, queued) {
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
            || destination >= 3
            || source == destination
            || !self.has_space(destination, self.cards[card].owner, 0)
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
    pub fn add_created(&mut self, id: &str, lane: usize, seat: usize) -> bool {
        if !self.has_space(lane, seat, 0) {
            return false;
        }
        let card = self.create_card(id, seat);
        if self.lanes[lane].revealed
            && let Some(test) = self.lanes[lane].def.allows_addition
            && !test(self, lane, card, lane, seat)
        {
            self.cards.pop();
            return false;
        }
        self.cards[card].lane = Some(lane);
        self.lanes[lane].cards.push(card);
        self.notify_added(card, lane);
        self.reveal(card);
        true
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
        if self.lanes[lane].revealed && self.lanes[lane].def.no_destroy {
            return false;
        }
        if self.lanes[lane]
            .cards
            .iter()
            .any(|&id| self.cards[id].revealed && self.cards[id].def.prevents_destroy)
        {
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
        for seat in 0..2 {
            self.set_energy(seat, i32::from(self.turn));
            self.draw(seat);
        }
        if self.turn <= 3 {
            let lane = usize::from(self.turn - 1);
            self.lanes[lane].revealed = true;
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
            for index in 0..self.lanes[lane].cards.len() {
                let id = self.lanes[lane].cards[index];
                if self.cards[id].revealed
                    && let Some(hook) = self.cards[id].def.ongoing
                {
                    hook(self, id);
                }
            }
        }
        for lane in 0..3 {
            if self.lanes[lane].revealed {
                let power = self.lanes[lane].def.power;
                for &id in &self.lanes[lane].cards {
                    if self.cards[id].revealed {
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
    pub fn reveal(&mut self, id: usize) {
        let Some(lane) = self.cards[id].lane else {
            return;
        };
        if self.cards[id].revealed {
            return;
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
        let blocked = self.lanes[lane]
            .cards
            .iter()
            .any(|&source| self.cards[source].revealed && self.cards[source].def.blocks_reveal);
        if !blocked {
            let runs = if self.lanes[lane].revealed {
                self.lanes[lane].def.reveal_repeats
            } else {
                1
            };
            if let Some(hook) = self.cards[id].def.reveal {
                for _ in 0..runs {
                    hook(self, id);
                }
            }
        }
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
    }
    pub fn play_turn(&mut self, policy: crate::policy::Policy) -> TurnTrace {
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
                let energy = self.players[seat].energy - self.cards[id].cost;
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
        self.turn += 1;
        result
    }
}
