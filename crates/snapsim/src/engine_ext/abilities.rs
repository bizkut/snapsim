//! Per-card ability instances and their independent execution context.
use crate::cards::CardDef;
use crate::game::{Card, CardState, Event, Game};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

/// The client ability category represented by an attached instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbilityKind {
    /// The card's original definition, whose hooks span several categories.
    Default,
    Ongoing,
    Reveal,
    Activate,
    Trigger,
}

/// An additional ability attached to a card, independent of its printed text.
#[derive(Clone)]
pub struct AbilityInstance {
    pub id: u64,
    pub def: &'static CardDef,
    pub kind: AbilityKind,
    pub origin: Option<usize>,
    /// Source card, source instance (None for default), and default generation.
    pub(crate) copied_from: Option<(usize, Option<u64>, u64)>,
    pub(crate) parent: Option<(u64, u64)>,
    pub hidden: bool,
    pub reentrant: bool,
    pub state: Option<Box<dyn CardState>>,
}

impl AbilityInstance {
    pub fn new(
        def: &'static CardDef,
        kind: AbilityKind,
        origin: Option<usize>,
        hidden: bool,
        reentrant: bool,
        state: Option<Box<dyn CardState>>,
    ) -> Self {
        Self {
            id: NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
            def,
            kind,
            origin,
            copied_from: None,
            parent: None,
            hidden,
            reentrant,
            state,
        }
    }
}

#[derive(Clone)]
pub(crate) struct AbilityFrame {
    pub instance: u64,
    pub generation: u64,
    pub root: usize,
    pub state: Option<Box<dyn CardState>>,
}

/// Borrowed view of a default or attached ability.
#[derive(Clone, Copy)]
pub struct AbilityRef<'a> {
    pub id: Option<u64>,
    pub def: &'static CardDef,
    pub kind: AbilityKind,
    pub origin: Option<usize>,
    pub hidden: bool,
    pub state: Option<&'a dyn CardState>,
}

impl AbilityRef<'_> {
    pub fn supports(self, kind: AbilityKind) -> bool {
        self.kind == AbilityKind::Default || self.kind == kind
    }

    pub(crate) fn cloned_state(self) -> Option<Box<dyn CardState>> {
        self.state.map(CardState::clone_box)
    }

    /// Reentrancy of the single default resolve trigger, from 59.15.1 defs.json.
    pub(crate) fn resolve_reentrant(self, source: &Card) -> bool {
        self.id
            .and_then(|id| {
                source
                    .composed_abilities
                    .iter()
                    .find(|entry| entry.id == id)
                    .map(|entry| entry.reentrant)
            })
            .unwrap_or(!matches!(
                self.def.id,
                "Chameleon" | "DrDoomDoomsday" | "Misery" | "Odin" | "RamaTut" | "Spell03Agamotto"
            ))
    }
}

impl Card {
    /// Attach a generated ongoing ability to the currently executing instance.
    pub(crate) fn attach_owned_ongoing(&mut self, mut ability: AbilityInstance) {
        ability.parent = Some(
            self.ability_frames
                .last()
                .map_or((0, self.default_ability_generation), |frame| {
                    (frame.instance, frame.generation)
                }),
        );
        self.composed_abilities.push(ability);
    }

    /// State owned by the currently executing ability, or the default text.
    pub fn ability_state(&self) -> Option<&(dyn CardState + 'static)> {
        match self.ability_frames.last() {
            Some(frame) => self.ability_frames[frame.root].state.as_deref(),
            None => self.state.as_deref(),
        }
    }

    /// Mutable state owned by the currently executing ability, or the default text.
    pub fn ability_state_mut(&mut self) -> Option<&mut (dyn CardState + 'static)> {
        match self.ability_frames.last().map(|frame| frame.root) {
            Some(root) => self.ability_frames[root].state.as_deref_mut(),
            None => self.state.as_deref_mut(),
        }
    }

    pub fn set_ability_state(&mut self, state: Option<Box<dyn CardState>>) {
        match self.ability_frames.last().map(|frame| frame.root) {
            Some(root) => self.ability_frames[root].state = state,
            None => self.state = state,
        }
    }

    /// State of the default ability even while another ability is executing.
    pub(crate) fn default_ability_state(&self) -> Option<&(dyn CardState + 'static)> {
        self.ability_frames
            .iter()
            .find(|frame| {
                frame.instance == 0 && frame.generation == self.default_ability_generation
            })
            .map_or_else(
                || self.state.as_deref(),
                |frame| self.ability_frames[frame.root].state.as_deref(),
            )
    }

    /// Card::CopyEffects (0x7caf324) clones abilities, including active contexts.
    pub(crate) fn copy_composed_abilities(&self) -> Vec<AbilityInstance> {
        let mut copies: Vec<_> = self
            .composed_abilities
            .iter()
            .map(|ability| {
                let state = self
                    .ability_frames
                    .iter()
                    .find(|frame| frame.instance == ability.id)
                    .map_or_else(
                        || ability.state.as_deref(),
                        |frame| self.ability_frames[frame.root].state.as_deref(),
                    );
                AbilityInstance::new(
                    ability.def,
                    ability.kind,
                    ability.origin,
                    ability.hidden,
                    ability.reentrant,
                    state.map(CardState::clone_box),
                )
            })
            .collect();
        for index in 0..copies.len() {
            let parent = self.composed_abilities[index].parent.and_then(|(id, _)| {
                if id == 0 {
                    Some((0, 0))
                } else {
                    self.composed_abilities
                        .iter()
                        .position(|ability| ability.id == id)
                        .map(|source_index| (copies[source_index].id, 0))
                        .or(Some((id, 0)))
                }
            });
            copies[index].parent = parent;
        }
        copies
    }

    pub(crate) fn has_event_ability(&self, skip_nico: bool, face_down: bool) -> bool {
        self.abilities().any(|ability| {
            (ability.id.is_some() || !self.abilities_removed)
                && ability.supports(AbilityKind::Trigger)
                && (!skip_nico || ability.def.id != "NicoMinoru")
                && (!face_down || ability.def.event_active_when_unrevealed)
                && ability.def.on_event.is_some()
        })
    }

    /// Return the effective default definition, excluding attached instances.
    pub fn default_ability(&self) -> &'static CardDef {
        self.abilities().next().unwrap().def
    }

    /// Iterate the effective default text followed by independently attached abilities.
    pub fn abilities(&self) -> impl Iterator<Item = AbilityRef<'_>> {
        std::iter::once(AbilityRef {
            id: None,
            def: self.override_ability.unwrap_or(self.def),
            kind: AbilityKind::Default,
            origin: None,
            hidden: false,
            state: self.default_ability_state(),
        })
        .chain(self.composed_abilities.iter().map(|ability| {
            AbilityRef {
                id: Some(ability.id),
                def: ability.def,
                kind: ability.kind,
                origin: ability.origin,
                hidden: ability.hidden,
                state: self
                    .ability_frames
                    .iter()
                    .find(|frame| frame.instance == ability.id)
                    .map_or_else(
                        || ability.state.as_deref(),
                        |frame| self.ability_frames[frame.root].state.as_deref(),
                    ),
            }
        }))
    }
}

impl Game<'_> {
    /// Preserve the independent contexts of copied transient ongoing parents.
    pub(crate) fn copy_virtual_ongoing_contexts(&mut self, source: usize, target: usize) {
        let generation = self.cards[target].default_ability_generation;
        let mut copied = Vec::with_capacity(self.cards[source].virtual_ongoing_ids.len());
        for (entity, source_id, source_generation, _, old) in
            &self.cards[source].virtual_ongoing_ids
        {
            let mut ability = AbilityInstance::new(
                old.def,
                old.kind,
                old.origin,
                old.hidden,
                old.reentrant,
                old.state.as_deref().map(CardState::clone_box),
            );
            ability.copied_from = old.copied_from;
            copied.push((old.id, *entity, *source_id, *source_generation, ability));
        }
        let card = &mut self.cards[target];
        card.virtual_ongoing_ids.clear();
        for (old_id, entity, source_id, source_generation, ability) in copied {
            for child in &mut card.composed_abilities {
                if child.parent.is_some_and(|parent| parent.0 == old_id) {
                    child.parent = Some((ability.id, 0));
                }
            }
            card.virtual_ongoing_ids.push((
                entity,
                source_id,
                source_generation,
                generation,
                ability,
            ));
        }
    }

    /// RemoveDefaultAbilities (0x7cbe110) preserves foreign-owned abilities.
    pub(crate) fn remove_default_composed(&mut self, card: usize) {
        let receiver = self.cards[card].def.id;
        let mut abilities = std::mem::take(&mut self.cards[card].composed_abilities);
        abilities.retain(|ability| {
            ability.def.id != receiver
                && ability
                    .origin
                    .and_then(|origin| self.cards.get(origin))
                    .is_none_or(|origin| origin.def.id != receiver)
        });
        self.cards[card].composed_abilities = abilities;
    }
    /// Count a player, location, or legacy trigger without changing its state context.
    /// CheckReentrant (0x7ca61e8) blocks repeated non-reentrant instances;
    /// distinct non-reentrant instances can still nest up to the global limit.
    pub(crate) fn counted_trigger<T>(&mut self, action: impl FnOnce(&mut Self) -> T) -> Option<T> {
        if self.ability_call_depth >= 32 {
            return None;
        }
        self.ability_call_depth += 1;
        let result = action(self);
        self.ability_call_depth -= 1;
        Some(result)
    }

    /// Execute an attached instance. Nested re-entry shares the same root
    /// context; a removed instance remains alive until its last frame returns.
    pub fn execute_ability(
        &mut self,
        card: usize,
        instance: u64,
        hook: impl FnOnce(&mut Self, usize),
    ) -> bool {
        // Ability::CanActivate (0x7c8a6e0) checks the global active
        // ability stack against AbilityDef::MaxAbilityCallDepth (32).
        if self.ability_call_depth >= 32 {
            return false;
        }
        let Some(index) = self.cards[card]
            .composed_abilities
            .iter()
            .position(|ability| ability.id == instance)
        else {
            return false;
        };
        let root = self.cards[card]
            .ability_frames
            .iter()
            .find(|frame| frame.instance == instance)
            .map(|frame| frame.root);
        if root.is_some() && !self.cards[card].composed_abilities[index].reentrant {
            return false;
        }
        let frame_index = self.cards[card].ability_frames.len();
        let state = if root.is_none() {
            self.cards[card].composed_abilities[index].state.take()
        } else {
            None
        };
        self.cards[card].ability_frames.push(AbilityFrame {
            instance,
            generation: 0,
            root: root.unwrap_or(frame_index),
            state,
        });
        self.ability_call_depth += 1;
        hook(self, card);
        self.ability_call_depth -= 1;
        let frame = self.cards[card]
            .ability_frames
            .pop()
            .expect("ability frame");
        if frame.root == frame_index
            && let Some(ability) = self.cards[card]
                .composed_abilities
                .iter_mut()
                .find(|ability| ability.id == instance)
        {
            ability.state = frame.state;
        }
        true
    }

    /// Execute the default ability without borrowing another active frame's state.
    pub fn execute_default_ability(
        &mut self,
        card: usize,
        hook: impl FnOnce(&mut Self, usize),
    ) -> bool {
        self.execute_default_ability_inner(card, false, hook)
    }

    pub(crate) fn execute_default_resolve(
        &mut self,
        card: usize,
        reentrant: bool,
        hook: impl FnOnce(&mut Self, usize),
    ) -> bool {
        self.execute_default_ability_inner(card, !reentrant, hook)
    }

    fn execute_default_ability_inner(
        &mut self,
        card: usize,
        non_reentrant: bool,
        hook: impl FnOnce(&mut Self, usize),
    ) -> bool {
        if self.cards[card].abilities_removed {
            return false;
        }
        if self.ability_call_depth >= 32 {
            return false;
        }
        let generation = self.cards[card].default_ability_generation;
        let root = self.cards[card]
            .ability_frames
            .iter()
            .find(|frame| frame.instance == 0 && frame.generation == generation)
            .map(|frame| frame.root);
        if non_reentrant && root.is_some() {
            return false;
        }
        let frame_index = self.cards[card].ability_frames.len();
        let state = if root.is_none() {
            self.cards[card].state.take()
        } else {
            None
        };
        self.cards[card].ability_frames.push(AbilityFrame {
            instance: 0,
            generation,
            root: root.unwrap_or(frame_index),
            state,
        });
        self.ability_call_depth += 1;
        hook(self, card);
        self.ability_call_depth -= 1;
        let frame = self.cards[card]
            .ability_frames
            .pop()
            .expect("ability frame");
        if frame.root == frame_index && self.cards[card].default_ability_generation == generation {
            self.cards[card].state = frame.state;
        }
        true
    }

    /// Activate a snapshot of this card's trigger instances without consuming
    /// its action; Jocasta uses the same dispatch as a normal activation.
    pub(crate) fn dispatch_card_activation(&mut self, card: usize) {
        let default = self.cards[card].default_ability();
        let attached: Vec<_> = self.cards[card]
            .composed_abilities
            .iter()
            .filter(|ability| ability.kind == AbilityKind::Activate)
            .filter_map(|ability| {
                ability
                    .def
                    .on_activate
                    .map(|hook| (ability.id, ability.def.id, hook))
            })
            .collect();
        if let Some(hook) = default.on_activate {
            self.execute_default_resolve(card, default.id != "Jocasta", |game, card| {
                game.record_card_hook(default.id, "on_activate");
                hook(game, card);
            });
        }
        for (id, def_id, hook) in attached {
            self.execute_ability(card, id, |game, card| {
                game.record_card_hook(def_id, "on_activate");
                hook(game, card);
            });
        }
    }

    /// Dispatch a snapshot of trigger instances with their own contexts.
    /// TriggerAbility::Activate (0x7c93fd8) returns for a hidden definition
    /// only when the activation is a repeat.
    pub(crate) fn dispatch_card_event(
        &mut self,
        card: usize,
        event: Event,
        skip_nico: bool,
        face_down: bool,
    ) {
        let default = self.cards[card].default_ability();
        let default_hook = (!self.cards[card].abilities_removed
            && (!skip_nico || default.id != "NicoMinoru")
            && (!face_down || default.event_active_when_unrevealed))
            .then_some(default.on_event)
            .flatten();
        let attached: Vec<_> = self.cards[card]
            .composed_abilities
            .iter()
            .filter(|ability| {
                ability.kind == AbilityKind::Trigger
                    && (!skip_nico || ability.def.id != "NicoMinoru")
                    && (!face_down || ability.def.event_active_when_unrevealed)
            })
            .filter_map(|ability| {
                ability.def.on_event.map(|hook| {
                    (
                        ability.id,
                        ability.def.id,
                        hook,
                        ability.def.end_turn_trigger,
                        ability.hidden || ability.def.end_turn_trigger_hidden,
                    )
                })
            })
            .collect();
        if let Some(hook) = default_hook {
            let repeats = if matches!(event, Event::TurnEnd)
                && default.end_turn_trigger
                && (!default.end_turn_trigger_hidden || crate::cards::reference_rules_enabled())
            {
                self.end_turn_repetitions(card)
            } else {
                1
            };
            for _ in 0..repeats {
                self.execute_default_ability(card, |game, card| {
                    game.record_card_hook(default.id, "on_event");
                    hook(game, card, event);
                });
            }
        }
        for (id, def_id, hook, end_turn_trigger, hidden) in attached {
            let repeats = if matches!(event, Event::TurnEnd)
                && end_turn_trigger
                && (!hidden || crate::cards::reference_rules_enabled())
            {
                self.end_turn_repetitions(card)
            } else {
                1
            };
            for _ in 0..repeats {
                self.execute_ability(card, id, |game, card| {
                    game.record_card_hook(def_id, "on_event");
                    hook(game, card, event);
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn attachment_preserves_kind_and_independent_context() {
        let mut rng = PythonRandom::seed(1);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Hawkeye", 0);
        game.cards[card]
            .composed_abilities
            .push(AbilityInstance::new(
                crate::cards::get("IronMan").unwrap(),
                AbilityKind::Ongoing,
                Some(42),
                false,
                true,
                Some(Box::new(7_u32)),
            ));
        let entries: Vec<_> = game.cards[card].abilities().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].def.id, "Hawkeye");
        assert!(entries[0].supports(AbilityKind::Reveal));
        assert!(entries[1].supports(AbilityKind::Ongoing));
        assert!(!entries[1].supports(AbilityKind::Reveal));
        assert_eq!(entries[1].origin, Some(42));
        assert_eq!(
            entries[1].state.unwrap().as_any().downcast_ref::<u32>(),
            Some(&7)
        );
    }

    #[test]
    fn hidden_movement_ability_only_counts_when_requested() {
        let mut rng = PythonRandom::seed(2);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Wasp", 0);
        game.cards[card]
            .external_moves
            .push(super::super::merge::CardMoveGrant {
                origin: card,
                start: 1,
                end: u8::MAX,
                remaining: None,
            });
        assert!(!game.cards[card].has_ongoing(false));
        assert!(game.cards[card].has_ongoing(true));
    }

    fn attach_counter(game: &mut Game<'_>, card: usize, value: u32, reentrant: bool) -> u64 {
        let instance = AbilityInstance::new(
            crate::cards::get("Hawkeye").unwrap(),
            AbilityKind::Reveal,
            Some(card),
            false,
            reentrant,
            Some(Box::new(value)),
        );
        let id = instance.id;
        game.cards[card].composed_abilities.push(instance);
        id
    }

    fn increment(game: &mut Game<'_>, card: usize) {
        *game.cards[card]
            .ability_state_mut()
            .unwrap()
            .as_any_mut()
            .downcast_mut::<u32>()
            .unwrap() += 1;
    }

    #[test]
    fn nonreentrant_default_resolve_blocks_only_nested_same_instance() {
        let mut rng = PythonRandom::seed(11);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Wasp", 0);
        game.cards[card].state = Some(Box::new(0_u32));
        let nested = attach_counter(&mut game, card, 10, true);
        assert!(game.execute_default_resolve(card, false, |game, card| {
            increment(game, card);
            assert!(!game.execute_default_resolve(card, false, increment));
            assert!(game.execute_ability(card, nested, increment));
            increment(game, card);
        }));
        assert_eq!(
            game.cards[card]
                .ability_state()
                .unwrap()
                .as_any()
                .downcast_ref::<u32>(),
            Some(&2)
        );
        assert_eq!(
            game.cards[card].composed_abilities[0]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<u32>(),
            Some(&11)
        );
    }

    #[test]
    fn nested_same_instance_shares_one_context() {
        let mut rng = PythonRandom::seed(3);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Wasp", 0);
        let id = attach_counter(&mut game, card, 0, true);
        game.execute_ability(card, id, |game, card| {
            increment(game, card);
            game.execute_ability(card, id, increment);
            increment(game, card);
        });
        assert_eq!(
            game.cards[card].composed_abilities[0]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<u32>(),
            Some(&3)
        );
        assert!(game.cards[card].ability_frames.is_empty());
    }

    #[test]
    fn nested_distinct_instances_keep_separate_contexts() {
        let mut rng = PythonRandom::seed(4);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Wasp", 0);
        let a = attach_counter(&mut game, card, 1, true);
        let b = attach_counter(&mut game, card, 10, true);
        game.execute_ability(card, a, |game, card| {
            increment(game, card);
            game.execute_ability(card, b, |game, card| {
                increment(game, card);
                game.execute_ability(card, a, increment);
            });
            increment(game, card);
        });
        let states: Vec<_> = game.cards[card]
            .composed_abilities
            .iter()
            .map(|ability| {
                *ability
                    .state
                    .as_deref()
                    .unwrap()
                    .as_any()
                    .downcast_ref::<u32>()
                    .unwrap()
            })
            .collect();
        assert_eq!(states, [4, 11]);
    }

    #[test]
    fn hidden_reveal_runs_once_under_wong_and_is_skipped_by_retrigger() {
        fn hidden(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 1;
        }
        fn visible(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 10;
        }
        static HIDDEN: crate::cards::CardDef = crate::cards::CardDef {
            reveal: Some(hidden),
            ..crate::cards::CardDef::vanilla("TestHidden")
        };
        static VISIBLE: crate::cards::CardDef = crate::cards::CardDef {
            reveal: Some(visible),
            ..crate::cards::CardDef::vanilla("TestVisible")
        };
        let mut rng = PythonRandom::seed(34);
        let mut game = super::super::test_game(&mut rng);
        let wong = game.create_card("Wong", 0);
        let target = game.create_card("Wasp", 0);
        for id in [wong, target] {
            game.cards[id].lane = Some(0);
            game.lanes[0].cards.push(id);
        }
        game.cards[wong].revealed = true;
        game.lanes[0].revealed = true;
        for (def, hidden) in [(&HIDDEN, true), (&VISIBLE, false)] {
            game.cards[target]
                .composed_abilities
                .push(AbilityInstance::new(
                    def,
                    AbilityKind::Reveal,
                    None,
                    hidden,
                    true,
                    None,
                ));
        }
        game.ongoing();
        let before = game.cards[target].permanent;
        game.reveal(target);
        assert_eq!(game.cards[target].permanent, before + 21);
        game.trigger_on_reveal(target);
        assert_eq!(game.cards[target].permanent, before + 41);
    }

    #[test]
    fn borrowed_rogue_scion_does_not_retrigger_its_foreign_resolve() {
        let mut rng = PythonRandom::seed(26);
        let mut game = super::super::test_game(&mut rng);
        let receiver = game.create_card("Wasp", 0);
        let borrowed = game.create_card("RogueScionOfDivision", 0);
        let target = game.create_card("IronMan", 1);
        game.players[1].hand = vec![target];
        game.cards[receiver].lane = Some(0);
        game.cards[receiver].revealed = true;
        game.lanes[0].revealed = true;
        game.lanes[0].cards.push(receiver);
        assert!(game.trigger_borrowed_reveal(receiver, borrowed));
        assert_eq!(game.cards[receiver].default_ability().id, "IronMan");
        assert!(game.cards[target].abilities_removed);
        assert!(game.cards[receiver].ability_frames.is_empty());
        assert!(game.cards[receiver].composed_abilities.is_empty());
    }

    #[test]
    fn mirrored_rogue_resolve_stops_at_client_ability_depth_limit() {
        let mut rng = PythonRandom::seed(27);
        let mut game = super::super::test_game(&mut rng);
        let receiver = game.create_card("RogueScionOfDivision", 0);
        let target = game.create_card("RogueScionOfDivision", 1);
        game.players[1].hand = vec![target];
        game.cards[receiver].lane = Some(0);
        game.cards[receiver].revealed = true;
        game.lanes[0].revealed = true;
        game.lanes[0].cards.push(receiver);
        game.trigger_on_reveal(receiver);
        assert!(game.cards[target].abilities_removed);
        assert_eq!(game.cards[target].default_ability_generation, 32);
        assert_eq!(game.ability_call_depth, 0);
        assert!(game.cards[receiver].ability_frames.is_empty());
    }

    #[test]
    fn ability_depth_limit_is_shared_between_cards() {
        fn recurse(game: &mut Game<'_>, card: usize, first: usize, second: usize) {
            game.players[0].energy += 1;
            let next = if card == first { second } else { first };
            game.execute_default_ability(next, |game, card| recurse(game, card, first, second));
        }
        let mut rng = PythonRandom::seed(28);
        let mut game = super::super::test_game(&mut rng);
        let first = game.create_card("Wasp", 0);
        let second = game.create_card("Hulk", 0);
        game.players[0].energy = 0;
        assert!(game.execute_default_ability(first, |game, card| {
            recurse(game, card, first, second);
        }));
        assert_eq!(game.players[0].energy, 32);
        assert_eq!(game.ability_call_depth, 0);
    }

    #[test]
    fn generic_trigger_and_card_frames_share_one_depth_limit() {
        let mut rng = PythonRandom::seed(29);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Wasp", 0);
        game.ability_call_depth = 30;
        assert_eq!(
            game.counted_trigger(|game| {
                assert_eq!(game.ability_call_depth, 31);
                assert!(game.execute_default_ability(card, |game, _| {
                    assert_eq!(game.ability_call_depth, 32);
                    assert_eq!(game.counted_trigger(|_| 7), None);
                }));
                9
            }),
            Some(9)
        );
        assert_eq!(game.ability_call_depth, 30);
    }

    #[test]
    fn borrowed_morph_preserves_nested_execution_contexts_until_return() {
        let mut rng = PythonRandom::seed(12);
        let mut game = super::super::test_game(&mut rng);
        let receiver = game.create_card("Wasp", 0);
        let morph = game.create_card("Morph", 0);
        let target = game.create_card("Hulk", 1);
        game.players[1].hand = vec![target];
        game.cards[receiver].lane = Some(0);
        game.cards[receiver].revealed = true;
        game.lanes[0].revealed = true;
        game.lanes[0].cards.push(receiver);
        let outer = attach_counter(&mut game, receiver, 4, true);
        assert!(game.execute_ability(receiver, outer, |game, card| {
            increment(game, card);
            assert!(game.trigger_borrowed_reveal(card, morph));
            assert_eq!(game.cards[card].def.id, "Hulk");
            increment(game, card);
            assert_eq!(
                game.cards[card]
                    .ability_state()
                    .unwrap()
                    .as_any()
                    .downcast_ref::<u32>(),
                Some(&6)
            );
        }));
        assert!(game.cards[receiver].ability_frames.is_empty());
        assert!(
            game.cards[receiver]
                .composed_abilities
                .iter()
                .all(|ability| ability.id != outer)
        );
    }

    #[test]
    fn detached_running_instance_does_not_restore_state() {
        let mut rng = PythonRandom::seed(5);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Wasp", 0);
        let id = attach_counter(&mut game, card, 1, false);
        game.execute_ability(card, id, |game, card| {
            assert!(!game.execute_ability(card, id, increment));
            game.cards[card].composed_abilities.clear();
            increment(game, card);
        });
        assert!(game.cards[card].composed_abilities.is_empty());
        assert!(game.cards[card].ability_frames.is_empty());
    }

    #[test]
    fn default_text_replacement_discards_running_default_context() {
        let mut rng = PythonRandom::seed(6);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Wasp", 0);
        let source = game.create_card("Hawkeye", 0);
        game.cards[card].state = Some(Box::new(5_u32));
        let id = attach_counter(&mut game, card, 1, true);
        game.execute_ability(card, id, |game, card| {
            game.execute_default_ability(card, |game, card| {
                increment(game, card);
                assert!(game.copy_text(card, source, false));
            });
        });
        assert_eq!(game.cards[card].default_ability().id, "Hawkeye");
        assert!(game.cards[card].state.is_none());
    }

    #[test]
    fn copied_objective_refresh_uses_new_default_context_during_old_trigger() {
        let mut rng = PythonRandom::seed(61);
        let mut game = super::super::test_game(&mut rng);
        let receiver = game.create_card("Wasp", 0);
        let objective = game.create_card("PolarisHorseman", 1);
        game.cards[receiver].lane = Some(0);
        game.cards[receiver].revealed = true;
        game.lanes[0].revealed = true;
        game.lanes[0].cards.push(receiver);
        assert!(game.execute_default_ability(receiver, |game, card| {
            assert!(game.copy_text(card, objective, false));
        }));
        assert_eq!(game.cards[receiver].default_ability().id, "PolarisHorseman");
        assert!(game.cards[receiver].ability_state().is_some());
        assert!(game.cards[receiver].ability_frames.is_empty());
    }

    #[test]
    fn default_context_survives_each_text_replacement_until_return() {
        for operation in 0..3 {
            let mut rng = PythonRandom::seed(80 + operation);
            let mut game = super::super::test_game(&mut rng);
            let card = game.create_card("Hulk", 0);
            let source = game.create_card("Wasp", 0);
            game.cards[card].state = Some(Box::new(5_u32));
            assert!(game.execute_default_ability(card, |game, card| {
                increment(game, card);
                match operation {
                    0 => assert!(game.copy_text(card, source, false)),
                    1 => assert!(game.transform_card(card, "Wasp")),
                    _ => assert!(game.transform_into_copy(card, source)),
                }
                increment(game, card);
                assert_eq!(
                    game.cards[card]
                        .ability_state()
                        .unwrap()
                        .as_any()
                        .downcast_ref::<u32>(),
                    Some(&7)
                );
            }));
            assert!(game.cards[card].ability_frames.is_empty());
            assert!(game.cards[card].state.is_none());
        }
    }

    #[test]
    fn copy_effects_clones_active_instance_context_with_new_identity() {
        let mut rng = PythonRandom::seed(7);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Wasp", 0);
        let id = attach_counter(&mut game, card, 1, true);
        let mut copied = None;
        game.execute_ability(card, id, |game, card| {
            increment(game, card);
            copied = Some(game.copy_card(card, 0));
            increment(game, card);
        });
        let copy = copied.unwrap();
        let ability = &game.cards[copy].composed_abilities[0];
        assert_ne!(ability.id, id);
        assert_eq!(
            ability
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<u32>(),
            Some(&2)
        );
        assert_eq!(
            game.cards[card].composed_abilities[0]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<u32>(),
            Some(&3)
        );
    }

    #[test]
    fn morph_into_copy_clones_attached_context() {
        let mut rng = PythonRandom::seed(9);
        let mut game = super::super::test_game(&mut rng);
        let source = game.create_card("Wasp", 0);
        let target = game.create_card("Hulk", 0);
        let id = attach_counter(&mut game, source, 4, true);
        assert!(game.transform_into_copy(target, source));
        let copied = &game.cards[target].composed_abilities[0];
        assert_ne!(copied.id, id);
        assert_eq!(
            copied
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<u32>(),
            Some(&4)
        );
    }

    #[test]
    fn dynamic_assigned_cost_copies_with_effects_but_not_text() {
        let mut rng = PythonRandom::seed(10);
        let mut game = super::super::test_game(&mut rng);
        let source = game.create_card("Wasp", 0);
        let target = game.create_card("Hulk", 0);
        game.cards[source].dynamic_assigned_cost = Some(5);
        game.cards[target].dynamic_assigned_cost = Some(2);
        let copy = game.copy_card(source, 0);
        assert_eq!(game.cards[copy].dynamic_assigned_cost, Some(5));
        assert!(game.copy_text(target, source, false));
        assert_eq!(game.cards[target].dynamic_assigned_cost, Some(2));
        let morph = game.create_card("Hulk", 0);
        assert!(game.transform_into_copy(morph, source));
        assert_eq!(game.cards[morph].dynamic_assigned_cost, Some(5));
    }

    #[test]
    fn copy_text_preserves_destination_foreign_abilities_not_source_attachments() {
        let mut rng = PythonRandom::seed(8);
        let mut game = super::super::test_game(&mut rng);
        let target = game.create_card("Wasp", 0);
        let foreign = game.create_card("IronMan", 0);
        let source = game.create_card("Hawkeye", 0);
        let ability = AbilityInstance::new(
            crate::cards::get("IronMan").unwrap(),
            AbilityKind::Ongoing,
            Some(foreign),
            false,
            true,
            None,
        );
        let id = ability.id;
        game.cards[target].composed_abilities.push(ability);
        game.cards[source]
            .composed_abilities
            .push(AbilityInstance::new(
                crate::cards::get("Hulk").unwrap(),
                AbilityKind::Ongoing,
                Some(source),
                false,
                true,
                None,
            ));
        assert!(game.copy_text(target, source, false));
        assert_eq!(game.cards[target].composed_abilities.len(), 1);
        assert_eq!(game.cards[target].composed_abilities[0].id, id);
        game.remove_text(source);
        let second = game.create_card("Wasp", 0);
        game.cards[second]
            .composed_abilities
            .push(AbilityInstance::new(
                crate::cards::get("IronMan").unwrap(),
                AbilityKind::Ongoing,
                Some(foreign),
                false,
                true,
                None,
            ));
        assert!(game.copy_text(second, source, false));
        assert!(game.cards[second].composed_abilities.is_empty());
    }
}
