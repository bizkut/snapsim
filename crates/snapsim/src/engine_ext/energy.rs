use crate::game::Game;

impl Game<'_> {
    /// PlayerEnergySystem::SetPermanentEnergy (0x7ca0d1c) sets the
    /// permanent total in update order, active this turn only in StartTurn.
    pub fn set_permanent_energy(&mut self, seat: usize, value: i32) {
        let current_default = i32::from(self.turn) + self.players[seat].default_energy_offset;
        let new = value.max(0) - current_default;
        self.players[seat].permanent_energy = new;
        if self.is_start_turn {
            self.set_energy(seat, value.max(0));
        }
    }

    /// Player::get_BonusEnergy (0x7de3d64): current minus default energy.
    pub fn bonus_energy(&self, seat: usize) -> i32 {
        self.players[seat].energy - i32::from(self.turn) - self.players[seat].default_energy_offset
    }

    /// PlayerEnergySystem::CanReducePermanentEnergy (0x7ca0a24) checks
    /// the active permanent total before accepting a negative update.
    pub fn can_reduce_permanent_energy(&self, seat: usize, amount: i32) -> bool {
        let total = i64::from(self.turn)
            + i64::from(self.players[seat].default_energy_offset)
            + i64::from(self.players[seat].permanent_energy);
        total.max(0) + i64::from(amount) >= 0
    }
}

#[cfg(test)]
mod tests {
    use crate::random::PythonRandom;

    #[test]
    fn observed_default_energy_changes_bonus_and_future_refresh() {
        let mut rng = PythonRandom::seed(9);
        let mut game = super::super::test_game(&mut rng);
        game.turn = 3;
        game.players[0].default_energy_offset = 2;
        game.players[0].permanent_energy = 1;
        game.players[0].energy = 8;
        assert_eq!(game.bonus_energy(0), 3);
        game.turn = 4;
        game.start_turn();
        assert_eq!(game.players[0].energy, 7);
        assert_eq!(game.bonus_energy(0), 1);
    }

    #[test]
    fn permanent_set_respects_order_and_future_turn_growth() {
        let mut rng = PythonRandom::seed(3);
        let mut game = super::super::test_game(&mut rng);
        game.turn = 4;
        game.players[0].energy = 4;
        game.add_permanent_energy(0, 2);
        game.set_permanent_energy(0, 8);
        game.add_permanent_energy(0, 1);
        assert_eq!(game.players[0].energy, 4);
        game.turn = 5;
        game.start_turn();
        assert_eq!(game.players[0].energy, 10);
        assert_eq!(game.bonus_energy(0), 5);
    }

    #[test]
    fn permanent_set_during_start_turn_applies_immediately() {
        let mut rng = PythonRandom::seed(4);
        let mut game = super::super::test_game(&mut rng);
        game.turn = 3;
        game.players[0].energy = 5;
        game.players[0].permanent_energy = 1;
        game.is_start_turn = true;
        game.set_permanent_energy(0, 7);
        assert_eq!(game.players[0].energy, 7);
    }

    #[test]
    fn permanent_reduction_cannot_make_total_negative() {
        let mut rng = PythonRandom::seed(5);
        let mut game = super::super::test_game(&mut rng);
        game.turn = 2;
        game.add_permanent_energy(0, -1);
        assert!(game.can_reduce_permanent_energy(0, -1));
        assert!(!game.can_reduce_permanent_energy(0, -2));
        game.set_permanent_energy(0, 0);
        assert!(!game.can_reduce_permanent_energy(0, -1));
    }

    #[test]
    fn post_start_hook_observes_location_energy() {
        fn bonus(_: &crate::game::Game<'_>, _: usize, _: usize) -> i32 {
            2
        }
        fn observe(game: &mut crate::game::Game<'_>, card: usize) {
            game.players[0].next_play_power = game.bonus_energy(game.cards[card].owner);
        }
        static LOCATION: crate::locations::LocationDef = crate::locations::LocationDef {
            energy_bonus: Some(bonus),
            ..crate::locations::location("EnergyTest", 0, 0, 1, false)
        };
        static CARD: crate::cards::CardDef = crate::cards::CardDef {
            post_start_turn: Some(observe),
            ..crate::cards::CardDef::vanilla("EnergyTest")
        };
        let mut rng = PythonRandom::seed(6);
        let mut game = super::super::test_game(&mut rng);
        game.lanes[0].def = &LOCATION;
        game.lanes[0].revealed = true;
        let card = game.create_card("Wasp", 0);
        game.cards[card].override_ability = Some(&CARD);
        game.add_card(card, 0, 0, true);
        game.turn = 2;
        game.current_location = 3;
        game.start_turn();
        assert_eq!(game.players[0].next_play_power, 2);
    }
}
