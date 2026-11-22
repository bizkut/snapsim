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

    /// Move an existing deck card to a location; its resolve ability happens immediately.
    pub fn move_deck_card_to_location(&mut self, card: usize, lane: usize) -> bool {
        let seat = self.cards[card].owner;
        let Some(slot) = self.players[seat].deck.iter().position(|&id| id == card) else {
            return false;
        };
        if !self.can_add(card, lane, seat) {
            return false;
        }
        self.players[seat].deck.remove(slot);
        if self.add_card(card, lane, seat, true) {
            true
        } else {
            self.players[seat].deck.insert(slot, card);
            false
        }
    }

    /// Move an ally from its location to its owner's deck without shuffling.
    pub fn move_card_to_deck(&mut self, card: usize) -> bool {
        let seat = self.cards[card].owner;
        if !self.take_from_board(card) {
            return false;
        }
        self.players[seat].deck.push(card);
        true
    }

    pub fn shuffle_deck(&mut self, seat: usize) {
        let mut rng = self.rng.borrow_mut();
        let deck = &mut self.players[seat].deck;
        // Deck::Shuffle (0x7cc0ac0) uses Game::Shuffle's forward pass.
        for index in 0..deck.len().saturating_sub(1) {
            let other = index + rng.randbelow(deck.len() - index);
            deck.swap(index, other);
        }
    }

    /// Card::Merge (0x7cbbf50) adds modifier-only power, then moves the
    /// absorbed card to SetAside and emits its post-merge trigger.
    pub fn merge_card(&mut self, receiver: usize, absorbed: usize) -> bool {
        if receiver == absorbed {
            return false;
        }
        let owner = self.cards[receiver].owner;
        let power = self.cards[absorbed].permanent;
        self.cards[absorbed].merged_to = Some(receiver);
        self.add_power(receiver, power, owner);
        if let Some(lane) = self.cards[absorbed].lane.take() {
            self.cards[absorbed].last_lane = Some(lane);
            self.lanes[lane].cards.retain(|&id| id != absorbed);
        }
        for player in &mut self.players {
            player.hand.retain(|&id| id != absorbed);
            player.deck.retain(|&id| id != absorbed);
            player.discard.retain(|&id| id != absorbed);
            player.graveyard.retain(|&(id, _)| id != absorbed);
            player.graveyard_order.retain(|&id| id != absorbed);
        }
        self.players[self.cards[absorbed].owner]
            .set_aside
            .push(absorbed);
        self.emit_merged(receiver, absorbed);
        true
    }

    /// Card::CloneExternalAbilities (0x7cb244c), predicate 0x7cbf514:
    /// retain instance context but allocate a new ability index on the receiver.
    pub fn clone_external_abilities(&mut self, receiver: usize, source: usize) {
        if self.cards[source].abilities_removed {
            return;
        }
        let replaced = self.cards[source].replaced_card_def_id;
        let copies: Vec<_> = self.cards[source]
            .attached_abilities
            .iter()
            .filter(|ability| {
                replaced != Some(ability.owner_def)
                    && !self.cards[receiver]
                        .attached_abilities
                        .iter()
                        .any(|existing| {
                            existing.id == ability.id
                                && self.is_default_attached(receiver, existing)
                        })
            })
            .cloned()
            .collect();
        for ability in copies {
            self.cards[receiver].attach_ability(ability);
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
