//! Client zone operations that card rules share: moving existing cards between zones and
//! making basic copies, mirroring the 59.15.1 client.

use crate::game::Game;

/// Identity of a client zone, including its seat where applicable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardZone {
    Deck(usize),
    Hand(usize),
    Location(usize),
    Discard(usize),
    Graveyard(usize),
    Banished(usize),
    SetAside(usize),
}

/// Initial deck assembly occurs before the Game object exists.
pub(crate) fn initialize_deck_card(player: &mut crate::game::Player, card: usize) {
    player.deck.push(card);
}
pub(crate) fn swap_initial_deck(player: &mut crate::game::Player, a: usize, b: usize) {
    player.deck.swap(a, b);
}

impl Game<'_> {
    /// Record one successful client zone transition before dispatching observers.
    pub fn transition_card_zone(&mut self, card: usize, destination: CardZone) {
        let card = &mut self.cards[card];
        if card.zone != Some(destination) {
            card.previous_zone = card.zone;
            card.zone = Some(destination);
        }
    }

    /// Removal alone leaves the last zone identity intact until insertion into
    /// the destination; this also makes failed remove/restore attempts inert.
    pub(crate) fn take_deck(&mut self, seat: usize, index: usize) -> usize {
        self.players[seat].deck.remove(index)
    }
    pub(crate) fn put_deck(&mut self, seat: usize, index: usize, card: usize) {
        self.players[seat].deck.insert(index, card);
        self.transition_card_zone(card, CardZone::Deck(seat));
    }
    pub(crate) fn push_deck(&mut self, seat: usize, card: usize) {
        self.put_deck(seat, self.players[seat].deck.len(), card);
    }
    pub(crate) fn swap_deck(&mut self, seat: usize, a: usize, b: usize) {
        self.players[seat].deck.swap(a, b);
    }
    pub(crate) fn take_hand(&mut self, seat: usize, index: usize) -> usize {
        self.players[seat].hand.remove(index)
    }
    pub(crate) fn put_hand(&mut self, seat: usize, index: usize, card: usize) {
        self.players[seat].hand.insert(index, card);
        self.transition_card_zone(card, CardZone::Hand(seat));
    }
    pub(crate) fn push_hand(&mut self, seat: usize, card: usize) {
        self.put_hand(seat, self.players[seat].hand.len(), card);
    }
    /// Remove surplus synthetic hidden-hand samples, not client zone moves.
    pub(crate) fn truncate_sampled_hand(&mut self, seat: usize, count: usize) {
        self.players[seat].hand.truncate(count);
    }
    pub(crate) fn push_graveyard(&mut self, seat: usize, card: usize, lane: Option<usize>) {
        self.players[seat].graveyard.push((card, lane));
        self.transition_card_zone(card, CardZone::Graveyard(seat));
    }
    pub(crate) fn take_graveyard(&mut self, seat: usize, index: usize) -> (usize, Option<usize>) {
        self.players[seat].graveyard.remove(index)
    }
    pub(crate) fn push_banished(&mut self, seat: usize, card: usize) {
        self.players[seat].banished.push(card);
        self.transition_card_zone(card, CardZone::Banished(seat));
    }
    pub(crate) fn push_set_aside(&mut self, seat: usize, card: usize) {
        self.players[seat].set_aside.push(card);
        self.transition_card_zone(card, CardZone::SetAside(seat));
    }
    pub(crate) fn push_location(&mut self, lane: usize, card: usize) {
        self.lanes[lane].cards.push(card);
        self.transition_card_zone(card, CardZone::Location(lane));
    }
    /// Detach without changing zone identity; insertion records the transition.
    pub(crate) fn detach_location(&mut self, lane: usize, card: usize) {
        self.lanes[lane].cards.retain(|&id| id != card);
    }
    pub(crate) fn push_discard(&mut self, seat: usize, card: usize) {
        self.players[seat].discard.push(card);
        self.transition_card_zone(card, CardZone::Discard(seat));
    }
    pub(crate) fn take_discard(&mut self, seat: usize, index: usize) -> usize {
        self.players[seat].discard.remove(index)
    }
    pub(crate) fn detach_discards(&mut self, card: usize) {
        for player in &mut self.players {
            player.discard.retain(|&id| id != card);
            player.graveyard_order.retain(|&id| id != card);
        }
    }
    pub(crate) fn detach_graveyards(&mut self, card: usize) {
        for player in &mut self.players {
            player.graveyard.retain(|&(id, _)| id != card);
            player.graveyard_order.retain(|&id| id != card);
        }
    }
    pub(crate) fn detach_player_zones(&mut self, card: usize) {
        for player in &mut self.players {
            player.hand.retain(|&id| id != card);
            player.deck.retain(|&id| id != card);
            player.discard.retain(|&id| id != card);
            player.graveyard.retain(|&(id, _)| id != card);
            player.banished.retain(|&id| id != card);
            player.set_aside.retain(|&id| id != card);
            player.graveyard_order.retain(|&id| id != card);
        }
    }
    /// Loki's ReplaceDeck moves every old deck entity to SetAside.
    pub(crate) fn set_aside_deck(&mut self, seat: usize) {
        let cards = std::mem::take(&mut self.players[seat].deck);
        if crate::cards::reference_rules_enabled() {
            return;
        }
        for card in cards {
            self.players[seat].set_aside.push(card);
            self.transition_card_zone(card, CardZone::SetAside(seat));
        }
    }

    /// Preserve hand order while transferring all cards into the deck.
    pub(crate) fn move_hand_to_deck(&mut self, seat: usize) {
        let cards = std::mem::take(&mut self.players[seat].hand);
        for card in cards {
            self.push_deck(seat, card);
        }
    }
}

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
        self.take_hand(seat, slot);
        if self.add_card(card, lane, seat, true) {
            true
        } else {
            self.put_hand(seat, slot, card);
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
        self.take_deck(seat, slot);
        if self.add_card(card, lane, seat, true) {
            true
        } else {
            self.put_deck(seat, slot, card);
            false
        }
    }

    /// Move an ally from its location to its owner's deck without shuffling.
    pub fn move_card_to_deck(&mut self, card: usize) -> bool {
        let seat = self.cards[card].owner;
        if !self.take_from_board(card) {
            return false;
        }
        self.push_deck(seat, card);
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
        let on_location =
            self.cards[receiver].lane.is_some() || self.cards[absorbed].lane.is_some();
        self.merge_card_without_event(receiver, absorbed);
        if crate::cards::reference_rules_enabled() || on_location {
            self.emit_merged(receiver, absorbed);
        }
        true
    }

    /// Card::Merge (0x7cbbf50) emits one post-merge trigger for the entire list.
    pub fn merge_cards(&mut self, receiver: usize, absorbed: &[usize]) -> bool {
        if absorbed.is_empty() || absorbed.contains(&receiver) {
            return false;
        }
        let on_location = self.cards[receiver].lane.is_some()
            || absorbed.iter().any(|&id| self.cards[id].lane.is_some());
        for &id in absorbed {
            self.merge_card_without_event(receiver, id);
        }
        if on_location {
            self.emit_merged_batch(receiver, absorbed);
        }
        true
    }

    fn merge_card_without_event(&mut self, receiver: usize, absorbed: usize) {
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
            player.set_aside.retain(|&id| id != absorbed);
            player.graveyard.retain(|&(id, _)| id != absorbed);
            player.graveyard_order.retain(|&id| id != absorbed);
        }
        let grants = self.cards[absorbed].external_moves.clone();
        if !grants.is_empty() {
            self.cards[receiver].observed_move_targets = None;
        }
        self.cards[receiver].external_moves.extend(grants);
        self.players[self.cards[absorbed].owner]
            .set_aside
            .push(absorbed);
        self.transition_card_zone(absorbed, CardZone::SetAside(self.cards[absorbed].owner));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Action;
    use crate::random::PythonRandom;

    #[test]
    fn staged_move_tracks_previous_zone_and_fork_rollback() {
        let mut rng = PythonRandom::seed(17);
        let deck = [
            "Nightcrawler",
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let card = game.create_card("Nightcrawler", 0);
        assert!(game.add_card(card, 0, 0, false));
        let original_zone = game.cards[card].zone;
        let original_previous = game.cards[card].previous_zone;
        {
            let mut rng = game.fork_rng();
            let mut trial = game.fork(rng.as_mut());
            trial
                .stage_action(0, Action::Move { card, lane: 1 })
                .unwrap();
            assert_eq!(trial.cards[card].zone, Some(CardZone::Location(1)));
            assert_eq!(trial.cards[card].previous_zone, Some(CardZone::Location(0)));
        }
        assert_eq!(game.cards[card].zone, original_zone);
        assert_eq!(game.cards[card].previous_zone, original_previous);
        game.stage_action(0, Action::Move { card, lane: 1 })
            .unwrap();
        game.resolve_staged_actions(0);
        assert_eq!(game.cards[card].zone, Some(CardZone::Location(1)));
        assert_eq!(game.cards[card].previous_zone, Some(CardZone::Location(0)));
    }
}
