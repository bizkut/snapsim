use crate::game::Game;

impl Game<'_> {
    /// Draw into `seat`'s hand from the selected player's deck.
    pub fn draw_from_deck(&mut self, seat: usize, deck_seat: usize) {
        if crate::cards::reference_rules_enabled() && self.players[seat].hand.len() >= 7 {
            return;
        }
        // Player::DrawCard (0x7de6f0c) selects a card before the pre-draw trigger.
        if !crate::cards::reference_rules_enabled() && self.players[deck_seat].deck.is_empty() {
            return;
        }
        // WidowsBiteBehavior::StopCardDraw (0x7cf2bd8).
        if self.players[seat].hand.iter().any(|&card| {
            !self.cards[card].abilities_removed
                && self.cards[card].ongoing_guard_abilities().any(|a| {
                    a.supports(super::abilities::AbilityKind::Ongoing) && a.def.blocks_draw_in_hand
                })
        }) {
            return;
        }
        for lane in 0..3 {
            if self.location_abilities_active(lane)
                && let Some(hook) = self.lanes[lane].def.before_draw
                && let Some(card) = self
                    .counted_trigger(|game| {
                        game.record_location_hook(lane, "before_draw");
                        hook(game, lane, seat)
                    })
                    .flatten()
            {
                self.add_hand(seat, card, true);
                return;
            }
        }
        // The client attempts the move only after PreCardDrawTrigger.
        if self.players[seat].hand.len() >= 7 {
            return;
        }
        if let Some(slot) = self.next_draw_slot(deck_seat) {
            let card = self.take_deck(deck_seat, slot);
            self.add_hand(seat, card, true);
        }
    }

    /// Player::DrawCard(Card) (0x7de7abc) draws a selected deck card,
    /// running the same pre-draw interception as an ordinary draw.
    pub fn draw_specific(&mut self, seat: usize, card: usize) {
        let Some(slot) = self.players[seat].deck.iter().position(|&id| id == card) else {
            return;
        };
        if (crate::cards::reference_rules_enabled() && self.players[seat].hand.len() >= 7)
            || self.players[seat].hand.iter().any(|&id| {
                !self.cards[id].abilities_removed
                    && self.cards[id].ongoing_guard_abilities().any(|a| {
                        a.supports(super::abilities::AbilityKind::Ongoing)
                            && a.def.blocks_draw_in_hand
                    })
            })
        {
            return;
        }
        for lane in 0..3 {
            if self.location_abilities_active(lane)
                && let Some(hook) = self.lanes[lane].def.before_draw
                && let Some(replacement) = self
                    .counted_trigger(|game| {
                        game.record_location_hook(lane, "before_draw");
                        hook(game, lane, seat)
                    })
                    .flatten()
            {
                self.add_hand(seat, replacement, true);
                return;
            }
        }
        if self.players[seat].hand.len() >= 7 {
            return;
        }
        self.take_deck(seat, slot);
        self.add_hand(seat, card, true);
    }
}

#[cfg(test)]
mod tests {
    use crate::random::PythonRandom;

    #[test]
    fn full_hand_still_consumes_time_theater_interception() {
        for selected in [false, true] {
            let mut rng = PythonRandom::seed(45);
            let mut game = super::super::test_game(&mut rng);
            let def = crate::locations::get("TimeTheater").unwrap();
            game.lanes[0].def = def;
            game.lanes[0].state = def.initial_state.map(|init| init());
            game.lanes[0].revealed = true;
            game.players[0].hand.clear();
            game.players[0].deck.clear();
            let previous = game.create_card("Wasp", 0);
            game.players[0].draws.push((1, previous));
            let card = game.create_card("Hulk", 0);
            game.players[0].deck.push(card);
            while game.players[0].hand.len() < 7 {
                let filler = game.create_card("Cyclops", 0);
                game.players[0].hand.push(filler);
            }
            if selected {
                game.draw_specific(0, card);
            } else {
                game.draw_from_deck(0, 0);
            }
            assert_eq!(game.players[0].deck, [card]);
            assert_eq!(game.players[0].hand.len(), 7);
            game.players[0].hand.pop();
            if selected {
                game.draw_specific(0, card);
            } else {
                game.draw_from_deck(0, 0);
            }
            assert!(game.players[0].hand.contains(&card));
            assert!(game.players[0].deck.is_empty());
        }
    }

    #[test]
    fn selected_draw_removes_only_selected_card() {
        let mut rng = PythonRandom::seed(44);
        let mut game = super::super::test_game(&mut rng);
        game.players[0].hand.clear();
        game.players[0].deck.clear();
        let first = game.create_card("Wasp", 0);
        let selected = game.create_card("Hulk", 0);
        game.players[0].deck.extend([first, selected]);
        game.draw_specific(0, selected);
        assert_eq!(game.players[0].deck, [first]);
        assert_eq!(game.players[0].hand, [selected]);
        assert_eq!(
            game.players[0].draws.last().map(|draw| draw.1),
            Some(selected)
        );
    }

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
