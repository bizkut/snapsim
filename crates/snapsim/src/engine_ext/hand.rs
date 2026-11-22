//! Hand and deck mechanics shared by card abilities.
use crate::game::{AttachedAbility, AttachedAbilityKind, Game, StagedPlay};

impl Game<'_> {
    pub(crate) fn finish_creation_zone(&mut self, card: usize, in_hand: bool, seat: usize) {
        if self.cards[card].creation_pending {
            self.cards[card].created_in_hand = Some(in_hand);
            self.cards[card].created_hand_seat = in_hand.then_some(seat);
            self.cards[card].creation_pending = false;
        }
    }
    /// SacredTimelineBehavior::CopyCards (0x7d79e08) copies current opening entities.
    pub fn copy_starting_hand_card(&mut self, seat: usize, index: usize) -> Option<usize> {
        if self.players[seat].hand.len() >= 7 {
            return None;
        }
        let original = *self.players[seat].starting_hand.as_ref()?.get(index)?;
        let card = self.copy_card(original, seat);
        self.add_hand(seat, card, false).then_some(card)
    }

    /// StryfeBehavior::UseHandOnReveal (0x7cca334): execute the selected hook
    /// as the source, without permanently copying its text.
    pub fn trigger_borrowed_reveal(&mut self, source: usize, selected: usize) -> bool {
        if self.cards[selected].abilities_removed {
            return false;
        }
        let Some(hook) = self.cards[selected].ability_def().reveal else {
            return false;
        };
        let context = self.cards[selected].state.clone();
        let original_def = self.cards[source].def;
        let original_ability = self.cards[source].override_ability;
        let original = std::mem::replace(&mut self.cards[source].state, context);
        self.trigger_reveal_hook(source, hook);
        // A copied reveal may permanently replace the recipient's text. In that
        // case copy_text/transform installed a new state that must not be
        // overwritten by the objective state belonging to the removed text.
        if std::ptr::eq(self.cards[source].def, original_def)
            && self.cards[source].override_ability.map(|def| def.id)
                == original_ability.map(|def| def.id)
        {
            self.cards[source].state = original;
        }
        true
    }

    /// MalekithBehavior::AddCardAndStopReveal (0x7ef0064).
    pub fn move_deck_card_hidden_until_game_end(
        &mut self,
        card: usize,
        lane: usize,
        source: usize,
    ) -> bool {
        let owner = self.cards[card].owner;
        let Some(slot) = self.players[owner].deck.iter().position(|&id| id == card) else {
            return false;
        };
        if lane >= self.lanes.len() || !self.can_add(card, lane, owner) {
            return false;
        }
        self.players[owner].deck.remove(slot);
        self.cards[card].attach_ability(AttachedAbility {
            id: "MalekithStopCardFromRevealing",
            owner_def: "Malekith",
            origin: Some(source),
            index: 0,
            kind: AttachedAbilityKind::RevealAtGameEnd,
        });
        if !self.add_card_deferred(card, lane, owner) {
            self.cards[card]
                .attached_abilities
                .retain(|a| !matches!(a.kind, AttachedAbilityKind::RevealAtGameEnd));
            self.players[owner].deck.insert(slot, card);
            return false;
        }
        self.deferred_added_reveals.retain(|&id| id != card);
        true
    }

    pub(crate) fn resolve_endgame_hidden_cards(&mut self) {
        let cards: Vec<_> = self
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| {
                self.cards[id]
                    .attached_abilities
                    .iter()
                    .any(|a| matches!(a.kind, AttachedAbilityKind::RevealAtGameEnd))
            })
            .collect();
        self.ending_reveals = true;
        for card in cards {
            self.cards[card]
                .attached_abilities
                .retain(|a| !matches!(a.kind, AttachedAbilityKind::RevealAtGameEnd));
            self.reveal(card);
        }
        self.ending_reveals = false;
    }

    /// StageForPlay (0x7ddd3f0) queues effect-played cards after the trigger resolves.
    pub fn stage_effect_play(&mut self, card: usize, lane: usize) -> bool {
        let owner = self.cards[card].owner;
        let Some(slot) = self.players[owner].hand.iter().position(|&id| id == card) else {
            return false;
        };
        if lane >= self.lanes.len()
            || !self.has_space(lane, owner, 0)
            || self.lane_locked(lane)
            || (self.lanes[lane].revealed
                && self.lanes[lane]
                    .def
                    .allows_addition
                    .is_some_and(|test| !test(self, lane, card, lane, owner)))
        {
            return false;
        }
        self.emit(crate::game::Event::CardStaged { card, seat: owner });
        self.players[owner].hand.remove(slot);
        self.cards[card].lane = Some(lane);
        self.cards[card].turn_played = self.turn;
        let printed_cost = self.printed_cost(card);
        self.players[owner].staged_history.push(StagedPlay {
            turn: self.turn,
            card,
            def_id: self.cards[card].def.id,
            printed_cost,
            created: self.cards[card].created,
            resolved: false,
        });
        self.played[owner].push((card, lane));
        self.lanes[lane].cards.push(card);
        self.post_resolve_plays.push(card);
        self.notify_added(card, lane);
        true
    }

    /// ResurrectCard (0x7dac46c), targeting hand.
    pub fn resurrect_to_hand(&mut self, card: usize) -> bool {
        let owner = self.cards[card].owner;
        self.resurrect_to_hand_multiply_power(card, 1, owner)
    }

    /// ThePeakBehavior::SwapCards (0x7d8b7cc) exchanges each hand's top card.
    pub fn exchange_top_hand_cards(&mut self) {
        let first = if self.players[0].hand.is_empty() {
            None
        } else {
            Some(self.players[0].hand.remove(0))
        };
        let second = if self.players[1].hand.is_empty() {
            None
        } else {
            Some(self.players[1].hand.remove(0))
        };
        if let Some(card) = first {
            self.cards[card].owner = 1;
        }
        if let Some(card) = second {
            self.cards[card].owner = 0;
        }
        for (card, seat) in [(first, 1), (second, 0)] {
            if let Some(card) = card {
                self.players[seat].hand.push(card);
                self.notify_hand_added(seat);
                self.ongoing();
            }
        }
    }

    /// ToxieDoxieBehavior::BuffCardsAndLockOut (0x7ce066c).
    pub fn prevent_play_next_turn(&mut self, card: usize, source: usize) {
        let turn = self.turn.saturating_add(1);
        self.cards[card].attach_ability(AttachedAbility {
            id: "ToxieDoxieCannotPlay",
            owner_def: "ToxieDoxie",
            origin: Some(source),
            index: 0,
            kind: AttachedAbilityKind::CannotPlayTurn { turn },
        });
    }

    /// BeastBehavior::ExecuteTriggerAction (0x7e0f6e4).
    pub fn add_next_turn_cost(&mut self, card: usize, delta: i32) {
        let turn = self.turn.saturating_add(1);
        let entry = &mut self.cards[card].next_turn_cost_delta;
        match entry {
            Some((existing_turn, amount)) if *existing_turn == turn => *amount += delta,
            _ => *entry = Some((turn, delta)),
        }
    }

    /// BaronMordoBehavior::AddCostSetAbilityToPlayer (0x7e6eaac).
    pub fn set_player_card_cost_until(&mut self, card: usize, value: i32, until: u8) {
        self.cards[card].player_cost_set_until = Some((until, value));
    }
    /// BlackSwanBehavior::SetCardCost (0x7e739f4) compares DefCost.
    pub fn set_hand_cost_for_printed_cost_until(
        &mut self,
        seat: usize,
        target: i32,
        value: i32,
        until: u8,
    ) {
        self.players[seat]
            .hand_cost_sets
            .push((until, target, value));
    }

    /// ResurrectCardAndMultiplyPowerPermanently (0x7dac584), targeting hand.
    pub fn resurrect_to_hand_multiply_power(
        &mut self,
        card: usize,
        multiplier: i32,
        source: usize,
    ) -> bool {
        let owner = self.cards[card].owner;
        let Some(slot) = self.players[owner]
            .graveyard
            .iter()
            .position(|&(id, _)| id == card)
        else {
            return false;
        };
        if self.players[owner].hand.len() >= 7 {
            return false;
        }
        self.players[owner].graveyard.remove(slot);
        self.players[owner].graveyard_order.retain(|&id| id != card);
        self.reset_resurrected_card(card);
        if multiplier != 1 {
            self.multiply_power(card, multiplier, source);
        }
        self.add_hand(owner, card, false)
    }
    /// ResurrectCardAndSetCostPermanently, targeting hand.
    pub fn resurrect_to_hand_set_cost(&mut self, card: usize, cost: i32, source: usize) -> bool {
        let owner = self.cards[card].owner;
        let Some(slot) = self.players[owner]
            .graveyard
            .iter()
            .position(|&(id, _)| id == card)
        else {
            return false;
        };
        if self.players[owner].hand.len() >= 7 {
            return false;
        }
        self.players[owner].graveyard.remove(slot);
        self.players[owner].graveyard_order.retain(|&id| id != card);
        self.reset_resurrected_card(card);
        self.set_cost(card, cost, source);
        self.add_hand(owner, card, false)
    }

    /// ResurrectCardAndAddCostPermanently (0x7dac7a4), during a discard hook.
    pub fn discarded_to_hand_add_cost(&mut self, card: usize, amount: i32, source: usize) -> bool {
        let owner = self.cards[card].owner;
        let slot = self.players[owner]
            .discard
            .iter()
            .position(|&id| id == card);
        if self.players[owner].hand.len() >= 7 {
            if slot.is_none() {
                self.players[owner].discard.push(card);
            }
            return false;
        }
        if let Some(slot) = slot {
            self.players[owner].discard.remove(slot);
        }
        self.players[owner].graveyard_order.retain(|&id| id != card);
        self.reset_resurrected_card(card);
        self.add_cost(card, amount, source);
        self.add_hand(owner, card, false)
    }
    /// Scorn's discarded-card resurrection with a permanent power increase.
    pub fn discarded_to_hand_add_power(&mut self, card: usize, amount: i32, source: usize) -> bool {
        let owner = self.cards[card].owner;
        let slot = self.players[owner]
            .discard
            .iter()
            .position(|&id| id == card);
        if self.players[owner].hand.len() >= 7 {
            if slot.is_none() {
                self.players[owner].discard.push(card);
            }
            return false;
        }
        if let Some(slot) = slot {
            self.players[owner].discard.remove(slot);
        }
        self.players[owner].graveyard_order.retain(|&id| id != card);
        self.reset_resurrected_card(card);
        self.add_power(card, amount, source);
        self.add_hand(owner, card, false)
    }

    /// Khonshu discard triggers resurrect the just-discarded card unmodified.
    pub fn discarded_to_hand(&mut self, card: usize) -> bool {
        let owner = self.cards[card].owner;
        let slot = self.players[owner]
            .discard
            .iter()
            .position(|&id| id == card);
        if self.players[owner].hand.len() >= 7 {
            if slot.is_none() {
                self.players[owner].discard.push(card);
            }
            return false;
        }
        if let Some(slot) = slot {
            self.players[owner].discard.remove(slot);
        }
        self.players[owner].graveyard_order.retain(|&id| id != card);
        self.reset_resurrected_card(card);
        self.add_hand(owner, card, false)
    }

    /// MorganLeFayBehavior::ReturnFromGraveyardAndBuff (0x7f02794).
    pub fn resurrect_zone_to_hand_add_power(
        &mut self,
        card: usize,
        amount: i32,
        owner: usize,
    ) -> bool {
        if self.cards[card].owner != owner || self.players[owner].hand.len() >= 7 {
            return false;
        }
        let discarded = self.players[owner]
            .discard
            .iter()
            .position(|&id| id == card);
        let destroyed = self.players[owner]
            .graveyard
            .iter()
            .position(|&(id, _)| id == card);
        if discarded.is_none() && destroyed.is_none() {
            return false;
        }
        if let Some(slot) = discarded {
            self.players[owner].discard.remove(slot);
        }
        if let Some(slot) = destroyed {
            self.players[owner].graveyard.remove(slot);
        }
        self.players[owner].graveyard_order.retain(|&id| id != card);
        self.reset_resurrected_card(card);
        self.add_power(card, amount, owner);
        self.add_hand(owner, card, false)
    }

    fn reset_resurrected_card(&mut self, card: usize) {
        self.cards[card].state = self.cards[card]
            .ability_def()
            .initial_state
            .map(|init| init());
        self.cards[card]
            .attached_abilities
            .retain(|a| !matches!(a.kind, AttachedAbilityKind::Watch { .. }));
        self.cards[card].turn_played = 0;
        self.cards[card].revealed = false;
        self.cards[card].ongoing = 0;
        self.cards[card].location_power = 0;
    }
}

#[cfg(test)]
mod tests {
    use crate::{game::Game, random::PythonRandom};

    fn game(rng: &mut PythonRandom) -> Game<'_> {
        let deck = [
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
            "Quicksilver",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        Game::new([&deck, &deck], &lanes, rng).unwrap()
    }

    #[test]
    fn next_turn_discount_stacks_and_expires() {
        let mut rng = PythonRandom::seed(4);
        let mut game = game(&mut rng);
        let card = game.create_card("Hulk", 0);
        assert!(game.add_hand(0, card, false));
        game.add_next_turn_cost(card, -1);
        game.add_next_turn_cost(card, -1);
        assert_eq!(game.card_cost(card), 6);
        game.turn += 1;
        assert_eq!(game.card_cost(card), 4);
        game.turn += 1;
        assert_eq!(game.card_cost(card), 6);
    }

    #[test]
    fn creation_zone_survives_bounce() {
        let mut rng = PythonRandom::seed(5);
        let mut game = game(&mut rng);
        let hand = game.create_card("Hulk", 0);
        assert!(game.add_hand(0, hand, false));
        assert_eq!(game.cards[hand].created_in_hand, Some(true));
        assert_eq!(game.cards[hand].created_hand_seat, Some(0));
        let board = game.create_card("Hulk", 0);
        assert!(game.add_card(board, 0, 0, false));
        assert!(game.return_board_card_to_hand(board));
        assert_eq!(game.cards[board].created_in_hand, Some(false));
        assert_eq!(game.cards[board].created_hand_seat, None);
    }

    #[test]
    fn player_cost_sets_use_printed_cost_and_expire() {
        let mut rng = PythonRandom::seed(6);
        let mut game = game(&mut rng);
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_hand(0, card, false));
        game.set_cost(card, 4, 0);
        game.set_hand_cost_for_printed_cost_until(0, 1, 0, game.turn + 1);
        assert_eq!(game.card_cost(card), 0);
        game.turn += 2;
        assert_eq!(game.card_cost(card), 4);
    }

    #[test]
    fn full_hand_preserves_graveyard_card() {
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        let card = game.create_card("Hulk", 0);
        assert!(game.add_card(card, 0, 0, false));
        assert!(game.destroy_card(card));
        while game.players[0].hand.len() < 7 {
            let filler = game.create_card("Wasp", 0);
            assert!(game.add_hand(0, filler, false));
        }
        assert!(!game.resurrect_to_hand(card));
        assert!(game.players[0].graveyard.iter().any(|&(id, _)| id == card));
        game.players[0].hand.pop();
        assert!(game.resurrect_to_hand_set_cost(card, 0, 0));
        assert_eq!(game.card_cost(card), 0);
    }
    #[test]
    fn hidden_deck_card_reveals_only_at_game_end() {
        let mut rng = PythonRandom::seed(8);
        let mut game = game(&mut rng);
        let card = game.players[0].deck[0];
        let source = game.create_card("Malekith", 0);
        assert!(game.move_deck_card_hidden_until_game_end(card, 0, source));
        assert!(!game.players[0].deck.contains(&card));
        game.reveal(card);
        game.finish_reveals();
        assert!(!game.cards[card].revealed);
        game.resolve_endgame_hidden_cards();
        assert!(game.cards[card].revealed);
    }

    #[test]
    fn remove_text_removes_default_attachments_and_recalculates_ongoing() {
        let mut rng = PythonRandom::seed(10);
        let mut game = game(&mut rng);
        let nico = game.create_card("NicoMinoru", 0);
        let other = game.create_card("Hawkeye", 0);
        game.cards[nico].attach_watch(other, 1, 0, true, 3);
        game.remove_text(nico);
        assert!(game.cards[nico].abilities_removed);
        assert_eq!(game.cards[nico].attached_abilities.len(), 1);
        assert_eq!(game.cards[nico].attached_abilities[0].origin, Some(other));

        let blue_marvel = game.create_card("BlueMarvel", 0);
        let hulk = game.create_card("Hulk", 0);
        game.add_card(blue_marvel, 0, 0, false);
        game.add_card(hulk, 0, 0, false);
        game.cards[blue_marvel].revealed = true;
        game.cards[hulk].revealed = true;
        game.ongoing();
        let before = game.cards[hulk].power();
        game.remove_text(blue_marvel);
        assert!(game.cards[blue_marvel].abilities_removed);
        assert_eq!(game.cards[hulk].power(), before - 1);
    }

    #[test]
    fn opening_hand_copy_uses_current_original_state() {
        let mut rng = PythonRandom::seed(9);
        let mut game = game(&mut rng);
        let original = game.players[0].hand[0];
        let initial_power = game.cards[original].permanent;
        game.add_power(original, 7, 0);
        let copy = game.copy_starting_hand_card(0, 0).unwrap();
        assert_eq!(game.cards[copy].permanent, initial_power + 7);
        assert_eq!(game.cards[copy].created_hand_seat, Some(0));
    }
}
