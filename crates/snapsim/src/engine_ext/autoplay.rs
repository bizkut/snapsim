//! Client staging-option inventory and autonomous staging.
use crate::game::{Action, Game};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AutoController {
    Ego,
    Agatha(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Undo {
    Play,
    Move { lane: usize },
    Activate,
}

#[derive(Debug, PartialEq, Eq)]
struct StagingOption {
    card: usize,
    targets: Vec<Action>,
    activate: bool,
    undo: Option<Undo>,
}

impl Game<'_> {
    pub(crate) fn autoplay_controller(&self, seat: usize) -> Option<AutoController> {
        if self.lanes.iter().enumerate().any(|(lane, location)| {
            location.def.id == "Ego" && self.location_abilities_active(lane)
        }) {
            return Some(AutoController::Ego);
        }
        if self.turn.is_multiple_of(2) {
            return self.players[seat]
                .hand
                .iter()
                .copied()
                .find(|&card| {
                    !self.cards[card].abilities_removed
                        && self.cards[card].default_ability().id == "AgathaHarkness"
                })
                .map(AutoController::Agatha);
        }
        None
    }

    // Game::GatherOptions (0x7da8e90) enumerates hand first, then the
    // player's cards in location order. The valid-options predicate
    // (0x7c63cc4) accepts a target list or an activation, not undo metadata.
    fn autoplay_options(&self, seat: usize) -> Vec<StagingOption> {
        let mut options = Vec::new();
        for &card in &self.players[seat].hand {
            let targets = (0..self.lanes.len())
                .filter(|&lane| self.can_play(card, lane, seat, self.players[seat].energy, 0))
                .map(|lane| Action::Play { card, lane })
                .collect();
            options.push(StagingOption {
                card,
                targets,
                activate: false,
                undo: None,
            });
        }
        for lane in &self.lanes {
            for &card in &lane.cards {
                if self.cards[card].owner != seat {
                    continue;
                }
                let targets: Vec<_> = (0..self.lanes.len())
                    .filter(|&to| self.can_player_move(card, to, seat))
                    .map(|lane| Action::Move { card, lane })
                    .collect();
                // CanBeReturnedToHand defaults false (Card::.ctor 0x7cad8b4).
                // A staged play's undo is separate metadata, not a valid target.
                let undo = self.staged_undo_option(card, seat);
                let activate = self.can_activate(card, seat);
                if !targets.is_empty() || activate || undo.is_some() {
                    options.push(StagingOption {
                        card,
                        targets,
                        activate,
                        undo,
                    });
                }
            }
        }
        options
    }

    pub(crate) fn autoplay_or_plays(
        &self,
        seat: usize,
        policy: crate::policy::Policy,
        rng: &mut dyn crate::random::GameRng,
    ) -> Vec<Action> {
        if let Some(controller) = self.autoplay_controller(seat) {
            self.autoplay_plan_with_rng(seat, controller, rng)
        } else {
            policy
                .plan(self, seat, rng)
                .into_iter()
                .map(|(card, lane)| Action::Play { card, lane })
                .collect()
        }
    }

    /// Generate a plan against a private staging view, while consuming the
    /// caller's RNG in the same order as the client's staging callbacks.
    pub(crate) fn autoplay_plan(&self, seat: usize, controller: AutoController) -> Vec<Action> {
        let mut rng = self.rng.borrow_mut();
        self.autoplay_plan_with_rng(seat, controller, &mut **rng)
    }

    pub(crate) fn autoplay_plan_with_rng(
        &self,
        seat: usize,
        controller: AutoController,
        rng: &mut dyn crate::random::GameRng,
    ) -> Vec<Action> {
        let mut view = self.fork(rng);
        let mut actions = Vec::new();
        loop {
            let options: Vec<_> = view
                .autoplay_options(seat)
                .into_iter()
                .filter(|option| {
                    debug_assert!(option.undo.is_none_or(|undo| match undo {
                        Undo::Move { lane } => lane < self.lanes.len(),
                        Undo::Play | Undo::Activate => true,
                    }));
                    !option.targets.is_empty() || option.activate
                })
                .collect();
            if options.is_empty() {
                break;
            }
            // AgathaHarknessBehavior::AutoPlay (0x7e6091c) takes her own
            // option without rolling; otherwise both behaviors roll an option.
            let preferred = match controller {
                AutoController::Agatha(card) => {
                    options.iter().position(|option| option.card == card)
                }
                AutoController::Ego => None,
            };
            let index = preferred.unwrap_or_else(|| view.rng.borrow_mut().randbelow(options.len()));
            let option = &options[index];
            // GetRandomStagingOptionTargetEntityId (0x7db313c) rolls even for
            // one target; an empty target list selects activation without a roll.
            let action = if option.targets.is_empty() {
                Action::Activate { card: option.card }
            } else {
                option.targets[view.rng.borrow_mut().randbelow(option.targets.len())]
            };
            view.stage_action(seat, action)
                .expect("gathered staging option must remain legal");
            actions.push(action);
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn inventory_and_undo_play_restore_energy_and_hand() {
        let mut rng = PythonRandom::seed(11);
        let mut game = crate::engine_ext::test_game(&mut rng);
        game.players[0].energy = 6;
        let card = game.players[0].hand[0];
        let hand = game.players[0].hand.clone();
        let cost = game.card_cost(card);
        let option = game
            .autoplay_options(0)
            .into_iter()
            .find(|o| o.card == card)
            .unwrap();
        let action = option.targets[0];
        game.stage_action(0, action).unwrap();
        let option = game
            .autoplay_options(0)
            .into_iter()
            .find(|o| o.card == card)
            .unwrap();
        assert_eq!(option.undo, Some(Undo::Play));
        assert_eq!(game.players[0].energy, 6 - cost);
        game.stage_action(0, Action::Return { card }).unwrap();
        assert_eq!(game.players[0].hand, hand);
        assert_eq!(game.players[0].energy, 6);
        assert!(!game.played[0].iter().any(|&(id, _)| id == card));
    }

    #[test]
    fn inventory_lists_move_and_activate() {
        let mut rng = PythonRandom::seed(12);
        let mut game = crate::engine_ext::test_game(&mut rng);
        game.players[0].energy = 6;
        let mover = game.create_card("SpiderManBrandNewDay", 0);
        let activator = game.create_card("KarimaShapandar", 0);
        assert!(game.add_card(mover, 0, 0, false));
        assert!(game.add_card(activator, 1, 0, false));
        let options = game.autoplay_options(0);
        let movement = options.iter().find(|option| option.card == mover).unwrap();
        assert!(movement.targets.contains(&Action::Move {
            card: mover,
            lane: 2
        }));
        let activate = options
            .iter()
            .find(|option| option.card == activator)
            .unwrap();
        assert!(activate.activate);
        assert!(activate.targets.is_empty());
        game.stage_action(0, Action::Activate { card: activator })
            .unwrap();
        let option = game
            .autoplay_options(0)
            .into_iter()
            .find(|option| option.card == activator)
            .unwrap();
        assert_eq!(option.undo, Some(Undo::Activate));
        game.stage_action(
            0,
            Action::Move {
                card: mover,
                lane: 2,
            },
        )
        .unwrap();
        let option = game
            .autoplay_options(0)
            .into_iter()
            .find(|option| option.card == mover)
            .unwrap();
        assert_eq!(option.undo, Some(Undo::Move { lane: 0 }));
    }

    #[test]
    fn agatha_controls_even_turn_and_selects_herself_first() {
        let mut rng = PythonRandom::seed(13);
        let mut game = crate::engine_ext::test_game(&mut rng);
        game.players[0].energy = 6;
        let agatha = game.create_card("AgathaHarkness", 0);
        assert!(game.add_hand(0, agatha, false));
        game.turn = 1;
        assert_eq!(game.autoplay_controller(0), None);
        game.turn = 2;
        let controller = game.autoplay_controller(0).unwrap();
        assert_eq!(controller, AutoController::Agatha(agatha));
        let plan = game.autoplay_plan(0, controller);
        assert!(matches!(plan.first(), Some(Action::Play { card, .. }) if *card == agatha));
    }

    #[test]
    fn ego_rolls_for_single_option_then_single_target() {
        let mut rng = PythonRandom::seed(15);
        let mut game = crate::engine_ext::test_game(&mut rng);
        game.truncate_sampled_hand(0, 0);
        for lane in 1..3 {
            for _ in 0..4 {
                let filler = game.create_card("Hulk", 0);
                assert!(game.add_card(filler, lane, 0, false));
            }
        }
        let card = game.create_card("Wasp", 0);
        assert!(game.add_hand(0, card, false));
        game.players[0].energy = 0;
        let mut expected = game.fork_rng();
        expected.randbelow(1);
        expected.randbelow(1);
        let expected_next = expected.getrandbits(32);
        assert_eq!(
            game.autoplay_plan(0, AutoController::Ego),
            vec![Action::Play { card, lane: 0 }]
        );
        assert_eq!(game.rng.borrow_mut().getrandbits(32), expected_next);
    }

    #[test]
    fn ego_controls_both_seats_after_reveal() {
        let mut rng = PythonRandom::seed(14);
        let mut game = crate::engine_ext::test_game(&mut rng);
        game.lanes[0].def = crate::locations::get("Ego").unwrap();
        game.lanes[0].revealed = true;
        for seat in 0..2 {
            assert_eq!(game.autoplay_controller(seat), Some(AutoController::Ego));
            game.players[seat].energy = 6;
            let plan = game.autoplay_plan(seat, AutoController::Ego);
            assert!(!plan.is_empty());
        }
    }
}
