use crate::game::Game;

impl Game<'_> {
    /// Draw into `seat`'s hand from the selected player's deck.
    pub fn draw_from_deck(&mut self, seat: usize, deck_seat: usize) {
        if self.players[seat].hand.len() >= 7 {
            return;
        }
        // WidowsBiteBehavior::StopCardDraw (0x7cf2bd8).
        if self.players[seat].hand.iter().any(|&card| {
            !self.cards[card].abilities_removed
                && self.cards[card].ability_def().blocks_draw_in_hand
        }) {
            return;
        }
        for lane in 0..3 {
            if self.lanes[lane].revealed
                && let Some(hook) = self.lanes[lane].def.before_draw
                && let Some(card) = hook(self, lane, seat)
            {
                self.add_hand(seat, card, true);
                return;
            }
        }
        if let Some(slot) = self.next_draw_slot(deck_seat) {
            let card = self.players[deck_seat].deck.remove(slot);
            self.add_hand(seat, card, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::random::PythonRandom;

    #[test]
    fn draws_from_other_deck_and_transfers_owner() {
        let mut rng = PythonRandom::seed(42);
        let mut game = super::super::test_game(&mut rng);
        game.players[0].hand.clear();
        game.players[1].deck.clear();
        let card = game.create_card("Hulk", 1);
        game.players[1].deck.push(card);
        game.turn = 2;
        game.draw_from_deck(0, 1);
        assert_eq!(game.players[0].hand, [card]);
        assert!(game.players[1].deck.is_empty());
        assert_eq!(game.cards[card].owner, 0);
        assert_eq!(game.players[0].draws.last(), Some(&(2, card)));
    }

    #[test]
    fn full_hand_does_not_consume_other_deck() {
        let mut rng = PythonRandom::seed(43);
        let mut game = super::super::test_game(&mut rng);
        let card = game.create_card("Hulk", 1);
        game.players[1].deck.clear();
        game.players[1].deck.push(card);
        while game.players[0].hand.len() < 7 {
            let created = game.create_card("Wasp", 0);
            game.players[0].hand.push(created);
        }
        game.draw_from_deck(0, 1);
        assert_eq!(game.players[1].deck, [card]);
    }
}
