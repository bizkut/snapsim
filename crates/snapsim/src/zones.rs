//! Client zone operations that card rules share: moving existing cards between zones and
//! making basic copies, mirroring the 59.15.1 client.

use crate::game::Game;

impl Game<'_> {
    /// Move an existing hand card to a location without recording a player play.
    /// ResolveMoveToZone (0x7db04fc) resolves hand-to-location moves immediately.
    pub(crate) fn move_hand_card_to_location(&mut self, card: usize, lane: usize) -> bool {
        let seat = self.cards[card].owner;
        let Some(slot) = self.players[seat].hand.iter().position(|&id| id == card) else {
            return false;
        };
        if !self.can_add(card, lane, seat) {
            return false;
        }
        self.players[seat].hand.remove(slot);
        if self.add_card(card, lane, seat, true) {
            true
        } else {
            self.players[seat].hand.insert(slot, card);
            false
        }
    }

    /// Copy the persistent card state to a location, with resolution deferred until
    /// the enclosing trigger has finished spawning cards (CompleteCardCreation 0x7daae34).
    pub(crate) fn copy_card_to_location(
        &mut self,
        source: usize,
        lane: usize,
        seat: usize,
    ) -> Option<usize> {
        if !self.has_space(lane, seat, 0) {
            return None;
        }
        // The copy constructor (0x7caecc8) clones persistent abilities and modifiers.
        let copy = self.copy_card(source, seat);
        if self.add_card_deferred(copy, lane, seat) {
            Some(copy)
        } else {
            self.cards.pop();
            None
        }
    }

    /// Copy a card into the owner's hand. A full hand prevents creation entirely.
    pub(crate) fn copy_card_to_hand(&mut self, source: usize, seat: usize) -> Option<usize> {
        if self.players[seat].hand.len() >= 7 {
            return None;
        }
        let copy = self.copy_card(source, seat);
        if self.add_hand_deferred(seat, copy) {
            Some(copy)
        } else {
            self.cards.pop();
            None
        }
    }
}
