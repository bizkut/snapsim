use crate::game::{AttachedAbilityKind, Event, Game};

impl Game<'_> {
    /// Game::ResurrectCard moves a discarded or destroyed card to a location,
    /// optionally changing its owner, then resolves it and emits resurrection.
    pub fn resurrect_from_zone(&mut self, card: usize, lane: usize, seat: usize) -> bool {
        self.resurrect_from_zone_add_power(card, lane, seat, 0)
    }

    /// ArchangelHorsemanBehavior::ResurrectCard (0x7e697b4) adds power before resolution.
    pub fn resurrect_from_zone_add_power(
        &mut self,
        card: usize,
        lane: usize,
        seat: usize,
        amount: i32,
    ) -> bool {
        let in_zone = self.players.iter().any(|player| {
            player.discard.contains(&card) || player.graveyard.iter().any(|&(id, _)| id == card)
        });
        if !in_zone || !self.can_add(card, lane, seat) {
            return false;
        }
        self.cards[card].state = self.cards[card]
            .ability_def()
            .initial_state
            .map(|init| init());
        self.cards[card]
            .attached_abilities
            .retain(|ability| !matches!(ability.kind, AttachedAbilityKind::Watch { .. }));
        self.cards[card].turn_played = 0;
        self.cards[card].ongoing = 0;
        self.cards[card].location_power = 0;
        for player in &mut self.players {
            player.discard.retain(|&id| id != card);
            player.graveyard_order.retain(|&id| id != card);
        }
        if amount != 0 {
            self.add_power(card, amount, seat);
        }
        if !self.add_card(card, lane, seat, true) {
            return false;
        }
        self.emit(Event::CardResurrected { card, lane });
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::random::PythonRandom;

    #[test]
    fn resurrection_transfers_opponent_discard_and_clears_zone() {
        let mut rng = PythonRandom::seed(47);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Wasp", 1);
        game.players[1].discard.push(card);
        assert!(game.resurrect_from_zone(card, 0, 0));
        assert!(!game.players[1].discard.contains(&card));
        assert_eq!(game.cards[card].owner, 0);
        assert_eq!(game.cards[card].lane, Some(0));
    }

    #[test]
    fn resurrection_does_not_remove_card_when_lane_full() {
        let mut rng = PythonRandom::seed(48);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Wasp", 1);
        game.players[1].discard.push(card);
        for _ in 0..4 {
            let filler = game.create_card("Wasp", 0);
            game.add_card(filler, 0, 0, false);
        }
        assert!(!game.resurrect_from_zone(card, 0, 0));
        assert!(game.players[1].discard.contains(&card));
    }
}
