use crate::game::{Event, Game};

/// External MovableOngoingAbilityDef instance with zero (unlimited) stacks.
#[derive(Clone)]
pub(crate) struct CardMoveGrant {
    pub origin: usize,
    pub owner_def_id: &'static str,
    pub start: u8,
    pub end: u8,
    /// None is unlimited; Some is the remaining number of player moves.
    pub remaining: Option<u8>,
}

impl Game<'_> {
    pub(crate) fn emit_merged_batch(&mut self, card: usize, absorbed: &[usize]) {
        let batch = self.merge_batches.len();
        self.merge_batches.push(absorbed.to_vec());
        let event = Event::CardMergedBatch { card, batch };
        self.emit(event);
        for &id in absorbed {
            self.dispatch_absorbed_merge(id, event);
        }
        self.merge_batches.pop();
        self.ongoing();
    }

    pub(crate) fn dispatch_absorbed_merge(&mut self, card: usize, event: Event) {
        self.dispatch_card_event(card, event, true, false);
    }

    pub(crate) fn merged_member(&self, event: Event, observer: usize) -> Option<usize> {
        match event {
            Event::CardMerged { card, absorbed } if observer == card || observer == absorbed => {
                Some(card)
            }
            Event::CardMergedBatch { card, batch }
                if observer == card || self.merge_batches[batch].contains(&observer) =>
            {
                Some(card)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::abilities::AbilityKind;
    use super::*;
    use crate::cards::CardDef;
    use crate::game::AbilityInstance;
    use crate::random::PythonRandom;

    static TRIGGER: CardDef = CardDef {
        on_event: Some(count_merge),
        ..CardDef::vanilla("Wasp")
    };

    static REENTRANT_TRIGGER: CardDef = CardDef {
        on_event: Some(reentrant_merge),
        ..CardDef::vanilla("Wasp")
    };

    static ABSORB_SELF_TRIGGER: CardDef = CardDef {
        on_event: Some(absorb_self),
        ..CardDef::vanilla("Wasp")
    };

    fn absorb_self(game: &mut Game<'_>, card: usize, event: Event) {
        if game.merged_member(event, card).is_none() {
            return;
        }
        *game.cards[card]
            .ability_state_mut()
            .unwrap()
            .as_any_mut()
            .downcast_mut::<i32>()
            .unwrap() += 1;
        let receiver = game.lanes[0]
            .cards
            .iter()
            .copied()
            .find(|&id| id != card)
            .unwrap();
        game.merge_card(receiver, card);
    }

    fn reentrant_merge(game: &mut Game<'_>, card: usize, event: Event) {
        if game.merged_member(event, card).is_none() {
            return;
        }
        let count = game.cards[card]
            .ability_state_mut()
            .unwrap()
            .as_any_mut()
            .downcast_mut::<i32>()
            .unwrap();
        *count += 1;
        let first = *count == 1;
        if first {
            let next = game.lanes[0]
                .cards
                .iter()
                .copied()
                .find(|&id| id != card)
                .unwrap();
            game.merge_card(card, next);
        }
    }

    fn count_merge(game: &mut Game<'_>, card: usize, event: Event) {
        if game.merged_member(event, card).is_some() {
            let state = game.cards[card]
                .ability_state_mut()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<i32>()
                .unwrap();
            *state += 1;
        }
    }

    fn add_trigger(game: &mut Game<'_>, card: usize, value: i32) {
        game.cards[card]
            .composed_abilities
            .push(AbilityInstance::new(
                &TRIGGER,
                AbilityKind::Trigger,
                None,
                false,
                true,
                Some(Box::new(value)),
            ));
    }

    fn trigger_state(game: &Game<'_>, card: usize, index: usize) -> i32 {
        *game.cards[card].composed_abilities[index]
            .state
            .as_deref()
            .unwrap()
            .as_any()
            .downcast_ref::<i32>()
            .unwrap()
    }

    #[test]
    fn absorbed_single_dispatch_preserves_each_instance_context() {
        let mut rng = PythonRandom::seed(81);
        let mut game = super::super::test_game(&mut rng);
        let receiver = game.create_card("Hulk", 0);
        let absorbed = game.create_card("Wasp", 0);
        assert!(game.add_card(receiver, 0, 0, false));
        assert!(game.add_card(absorbed, 0, 0, false));
        game.cards[absorbed].state = Some(Box::new(100_i32));
        add_trigger(&mut game, absorbed, 5);
        add_trigger(&mut game, absorbed, 10);
        game.cards[absorbed].abilities_removed = true;
        assert!(game.merge_card(receiver, absorbed));
        assert_eq!(trigger_state(&game, absorbed, 0), 6);
        assert_eq!(trigger_state(&game, absorbed, 1), 11);
        assert_eq!(
            *game.cards[absorbed]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<i32>()
                .unwrap(),
            100
        );
    }

    #[test]
    fn absorbed_batch_dispatch_runs_each_instance_once() {
        let mut rng = PythonRandom::seed(82);
        let mut game = super::super::test_game(&mut rng);
        let receiver = game.create_card("Hulk", 0);
        let first = game.create_card("Wasp", 0);
        let second = game.create_card("Wasp", 0);
        assert!(game.add_card(receiver, 0, 0, false));
        assert!(game.add_card(first, 0, 0, false));
        assert!(game.add_card(second, 0, 0, false));
        add_trigger(&mut game, first, 5);
        add_trigger(&mut game, second, 10);
        assert!(game.merge_cards(receiver, &[first, second]));
        assert_eq!(trigger_state(&game, first, 0), 6);
        assert_eq!(trigger_state(&game, second, 0), 11);
    }

    #[test]
    fn receiver_dispatch_uses_attached_context() {
        let mut rng = PythonRandom::seed(83);
        let mut game = super::super::test_game(&mut rng);
        let receiver = game.create_card("Hulk", 0);
        let absorbed = game.create_card("Wasp", 0);
        assert!(game.add_card(receiver, 0, 0, false));
        assert!(game.add_card(absorbed, 0, 0, false));
        add_trigger(&mut game, receiver, 9);
        assert!(game.merge_card(receiver, absorbed));
        assert_eq!(trigger_state(&game, receiver, 0), 10);
    }

    #[test]
    fn nested_merge_reuses_the_active_instance_state() {
        let mut rng = PythonRandom::seed(84);
        let mut game = super::super::test_game(&mut rng);
        let receiver = game.create_card("Hulk", 0);
        let first = game.create_card("Wasp", 0);
        let second = game.create_card("Wasp", 0);
        assert!(game.add_card(receiver, 0, 0, false));
        assert!(game.add_card(first, 0, 0, false));
        assert!(game.add_card(second, 0, 0, false));
        game.cards[receiver]
            .composed_abilities
            .push(AbilityInstance::new(
                &REENTRANT_TRIGGER,
                AbilityKind::Trigger,
                None,
                false,
                true,
                Some(Box::new(0_i32)),
            ));
        assert!(game.merge_card(receiver, first));
        assert_eq!(game.cards[second].merged_to, Some(receiver));
        assert_eq!(trigger_state(&game, receiver, 0), 2);
    }

    #[test]
    fn nested_absorption_runs_default_in_its_own_context() {
        let mut rng = PythonRandom::seed(85);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Wasp", 0);
        let other = game.create_card("Hulk", 0);
        let first = game.create_card("Wasp", 0);
        assert!(game.add_card(card, 0, 0, false));
        assert!(game.add_card(other, 0, 0, false));
        assert!(game.add_card(first, 0, 0, false));
        game.cards[card].override_ability = Some(&TRIGGER);
        game.cards[card].state = Some(Box::new(100_i32));
        game.cards[card]
            .composed_abilities
            .push(AbilityInstance::new(
                &ABSORB_SELF_TRIGGER,
                AbilityKind::Trigger,
                None,
                false,
                false,
                Some(Box::new(5_i32)),
            ));
        assert!(game.merge_card(card, first));
        assert_eq!(game.cards[card].merged_to, Some(other));
        assert_eq!(trigger_state(&game, card, 0), 6);
        assert_eq!(
            *game.cards[card]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<i32>()
                .unwrap(),
            102
        );
    }
}
