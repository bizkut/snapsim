use crate::game::{Event, Game};

impl Game<'_> {
    /// Location::CanCardChangeOwner (0x7ddc3b4) checks that the location
    /// exists and the other player has a slot before moving the card.
    pub fn can_change_card_owner(&self, card: usize) -> bool {
        let Some(lane) = self.cards[card].lane else {
            return false;
        };
        !self.lanes[lane].destroyed && self.has_space(lane, 1 - self.cards[card].owner, 0)
    }

    /// Card::SetOwner followed by Location::AddCardToPlayer, retaining lane
    /// and abilities while appending to the recipient's slots.
    pub fn change_card_owner(&mut self, card: usize) -> bool {
        if !self.can_change_card_owner(card) {
            return false;
        }
        let lane = self.cards[card].lane.unwrap();
        let old = self.cards[card].owner;
        let new = 1 - old;
        self.lanes[lane].cards.retain(|&id| id != card);
        self.cards[card].owner = new;
        self.lanes[lane].cards.push(card);
        self.ongoing();
        self.emit(Event::CardOwnerChanged { card, old, new });
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::random::PythonRandom;

    #[test]
    fn owner_change_preserves_lane_and_uses_last_recipient_slot() {
        let mut rng = PythonRandom::seed(50);
        let mut game = super::super::test_game(&mut rng);
        let target = game.create_card("Wasp", 0);
        let other = game.create_card("Hulk", 1);
        game.add_card(target, 0, 0, false);
        game.add_card(other, 0, 1, false);
        assert!(game.change_card_owner(target));
        assert_eq!(game.cards[target].owner, 1);
        assert_eq!(game.cards[target].lane, Some(0));
        assert_eq!(game.zone_position(target), Some(1));
    }

    #[test]
    fn owner_change_rejects_full_recipient_side() {
        let mut rng = PythonRandom::seed(51);
        let mut game = super::super::test_game(&mut rng);
        let target = game.create_card("Wasp", 0);
        game.add_card(target, 0, 0, false);
        for _ in 0..4 {
            let card = game.create_card("Wasp", 1);
            game.add_card(card, 0, 1, false);
        }
        assert!(!game.change_card_owner(target));
        assert_eq!(game.cards[target].owner, 0);
    }
}
