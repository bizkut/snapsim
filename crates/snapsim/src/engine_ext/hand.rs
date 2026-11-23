//! Hand and deck mechanics shared by card abilities.
use crate::game::{
    AttachedAbility, AttachedAbilityKind, Game, QueuedReveal, StagedPlay, StagingContext,
};

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
        let (first, rest) = {
            let mut hooks = self.cards[selected]
                .abilities()
                .filter(|a| a.supports(super::abilities::AbilityKind::Reveal))
                .filter_map(|a| {
                    a.def
                        .reveal
                        .map(|hook| (a.def, hook, a.state.map(|state| state.clone_box())))
                });
            let Some(first) = hooks.next() else {
                return false;
            };
            (first, hooks.collect::<Vec<_>>())
        };
        for (def, hook, context) in std::iter::once(first).chain(rest) {
            let ability = super::abilities::AbilityInstance::new(
                def,
                super::abilities::AbilityKind::Reveal,
                Some(selected),
                false,
                true,
                context,
            );
            let instance = ability.id;
            self.cards[source].composed_abilities.push(ability);
            self.execute_ability(source, instance, |game, card| {
                game.trigger_reveal_hook(card, hook, def.id);
            });
            self.cards[source]
                .composed_abilities
                .retain(|ability| ability.id != instance);
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
        self.take_deck(owner, slot);
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
            self.put_deck(owner, slot, card);
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
        let slot = self.players[owner].hand.iter().position(|&id| id == card);
        if slot.is_none() && (!self.cards[card].creation_pending || self.cards[card].lane.is_some())
        {
            return false;
        }
        if lane >= self.lanes.len()
            || !self.has_space(lane, owner, 0)
            || self.lane_locked(lane)
            || (self.location_abilities_active(lane)
                && self.lanes[lane].def.allows_addition.is_some_and(|test| {
                    self.record_location_hook(lane, "allows_addition");
                    !test(self, lane, card, lane, owner)
                }))
        {
            return false;
        }
        self.finish_creation_zone(card, false, owner);
        self.emit(crate::game::Event::CardStaged { card, seat: owner });
        if let Some(slot) = slot {
            self.take_hand(owner, slot);
        }
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
        self.players[owner]
            .staged_lane_history
            .push((self.turn, lane));
        self.played[owner].push((card, lane));
        self.push_location(lane, card);
        self.post_resolve_plays.push(QueuedReveal {
            card,
            staging: Some(StagingContext { seat: owner, lane }),
            persistent: false,
        });
        self.notify_added(card, lane);
        true
    }

    /// ResurrectCard (0x7dac46c), targeting hand.
    pub fn resurrect_to_hand(&mut self, card: usize) -> bool {
        let owner = self.cards[card].owner;
        self.resurrect_to_hand_multiply_power(card, 1, owner)
    }

    /// SanctumInfinitumBehavior::ExecuteReturn (0x7d7cd18) addresses an
    /// entity, even if it has since moved out of the discarded zone.
    pub(crate) fn resurrect_entity_to_hand(&mut self, card: usize, seat: usize) -> bool {
        if self.players[seat].hand.len() >= 7 {
            return false;
        }
        if let Some(lane) = self.cards[card].lane.take() {
            self.detach_location(lane, card);
            self.cards[card].last_lane = Some(lane);
        }
        self.detach_player_zones(card);
        self.reset_resurrected_card(card);
        self.add_hand(seat, card, false)
    }

    /// ThePeakBehavior::SwapCards (0x7d8b7cc) exchanges each hand's top card.
    pub fn exchange_top_hand_cards(&mut self) {
        let first = if self.players[0].hand.is_empty() {
            None
        } else {
            Some(self.take_hand(0, 0))
        };
        let second = if self.players[1].hand.is_empty() {
            None
        } else {
            Some(self.take_hand(1, 0))
        };
        if let Some(card) = first {
            self.cards[card].owner = 1;
        }
        if let Some(card) = second {
            self.cards[card].owner = 0;
        }
        for (card, seat) in [(first, 1), (second, 0)] {
            if let Some(card) = card {
                self.push_hand(seat, card);
                self.notify_hand_added(seat);
                self.ongoing();
            }
        }
    }

    /// ToxieDoxieBehavior::BuffCardsAndLockOut (0x7ce066c) and
    /// ToxieDoxieThunderboltsBehavior::AddPower (0x7ce167c) attach their
    /// lock to the source player, affecting the target only in that hand.
    pub fn prevent_play_next_turn(&mut self, card: usize, source: usize) {
        let turn = self.turn.saturating_add(1);
        let seat = self.cards[source].owner;
        self.players[seat].play_locks.push((card, turn));
    }

    /// BeastBehavior::ExecuteTriggerAction (0x7e0f6e4).
    pub fn add_next_turn_cost(&mut self, card: usize, delta: i32) {
        let turn = self.turn.saturating_add(1);
        self.cards[card]
            .attach_cost_effect(crate::game::CardCostEffectKind::NextTurnDelta { turn, delta });
        if !crate::cards::reference_rules_enabled() {
            self.ongoing();
        }
    }

    /// BaronMordoBehavior::AddCostSetAbilityToPlayer (0x7e6eaac).
    pub fn set_player_card_cost_until(&mut self, card: usize, value: i32, until: u8) {
        let owner = self.cards[card].owner;
        self.players[owner].attach_cost_effect(crate::game::PlayerCostEffectKind::BaronMordo {
            card,
            until,
            value,
        });
        if !crate::cards::reference_rules_enabled() {
            self.ongoing();
        }
    }
    /// BlackSwanBehavior::SetCardCost (0x7e739f4) compares DefCost.
    pub fn set_hand_cost_for_printed_cost_until(
        &mut self,
        seat: usize,
        target: i32,
        value: i32,
        until: u8,
    ) {
        self.players[seat].attach_cost_effect(crate::game::PlayerCostEffectKind::BlackSwan {
            until,
            printed_cost: target,
            value,
        });
        if !crate::cards::reference_rules_enabled() {
            self.ongoing();
        }
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
        self.take_graveyard(owner, slot);
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
        self.take_graveyard(owner, slot);
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
                self.push_discard(owner, card);
            }
            return false;
        }
        if let Some(slot) = slot {
            self.take_discard(owner, slot);
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
                self.push_discard(owner, card);
            }
            return false;
        }
        if let Some(slot) = slot {
            self.take_discard(owner, slot);
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
                self.push_discard(owner, card);
            }
            return false;
        }
        if let Some(slot) = slot {
            self.take_discard(owner, slot);
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
            self.take_discard(owner, slot);
        }
        if let Some(slot) = destroyed {
            self.take_graveyard(owner, slot);
        }
        self.players[owner].graveyard_order.retain(|&id| id != card);
        self.reset_resurrected_card(card);
        self.add_power(card, amount, owner);
        self.add_hand(owner, card, false)
    }

    fn reset_resurrected_card(&mut self, card: usize) {
        let initial_state = self.cards[card]
            .abilities()
            .next()
            .unwrap()
            .def
            .initial_state;
        self.cards[card].state = initial_state.map(|init| init());
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
        game.ongoing();
        assert_eq!(game.card_cost(card), 4);
        game.turn += 1;
        game.ongoing();
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
        game.ongoing();
        assert_eq!(game.card_cost(card), 0);
        game.turn += 2;
        game.ongoing();
        assert_eq!(game.card_cost(card), 4);
    }

    #[test]
    fn player_cost_setters_follow_attachment_order() {
        for mordo_first in [true, false] {
            let mut rng = PythonRandom::seed(64);
            let mut game = game(&mut rng);
            let card = game.create_card("Hawkeye", 0);
            assert!(game.add_hand(0, card, false));
            if mordo_first {
                game.set_player_card_cost_until(card, 6, 6);
                game.set_hand_cost_for_printed_cost_until(0, 1, 0, 6);
            } else {
                game.set_hand_cost_for_printed_cost_until(0, 1, 0, 6);
                game.set_player_card_cost_until(card, 6, 6);
            }
            game.ongoing();
            assert_eq!(game.card_cost(card), if mordo_first { 0 } else { 6 });
        }
    }

    #[test]
    fn mordo_cost_setter_requires_target_in_affected_hand() {
        if crate::cards::reference_rules_enabled() {
            return;
        }
        let mut rng = PythonRandom::seed(64);
        let mut game = game(&mut rng);
        let card = game.create_card("Hawkeye", 0);
        assert!(game.add_hand(0, card, false));
        game.set_player_card_cost_until(card, 6, 6);
        game.ongoing();
        assert_eq!(game.card_cost(card), 6);
        game.players[0].hand.retain(|&id| id != card);
        game.cards[card].owner = 1;
        game.players[1].hand.push(card);
        game.ongoing();
        assert_eq!(game.card_cost(card), 1);
        game.players[1].hand.retain(|&id| id != card);
        game.cards[card].owner = 0;
        game.players[0].hand.push(card);
        game.ongoing();
        assert_eq!(game.card_cost(card), 6);
    }

    #[test]
    fn card_cost_setters_follow_attachment_order() {
        for beast_first in [true, false] {
            let mut rng = PythonRandom::seed(65);
            let mut game = game(&mut rng);
            let card = game.create_card("Hawkeye", 0);
            assert!(game.add_hand(0, card, false));
            if beast_first {
                game.add_next_turn_cost(card, -1);
                game.set_temporary_hand_cost(card, game.turn + 1, 0);
            } else {
                game.set_temporary_hand_cost(card, game.turn + 1, 0);
                game.add_next_turn_cost(card, -1);
            }
            game.turn += 1;
            game.ongoing();
            let updates: Vec<_> = game.cards[card]
                .stat_updates
                .iter()
                .filter(|update| update.stat == crate::game::Stat::Cost && update.ongoing)
                .map(|update| update.operation)
                .collect();
            let expected = if beast_first {
                [
                    crate::game::StatOperation::Add,
                    crate::game::StatOperation::Set,
                ]
            } else {
                [
                    crate::game::StatOperation::Set,
                    if crate::cards::reference_rules_enabled() {
                        crate::game::StatOperation::Add
                    } else {
                        crate::game::StatOperation::Set
                    },
                ]
            };
            assert_eq!(updates, expected);
        }
    }

    #[test]
    fn sandman_runs_before_location_player_and_card_cost_effects() {
        use crate::game::PlayerCostEffectKind;
        for case in [
            "elysium", "wave", "mordo", "swan", "beast", "queen", "superior",
        ] {
            let mut rng = PythonRandom::seed(66);
            let mut game = game(&mut rng);
            for lane in &mut game.lanes {
                lane.def = crate::locations::get("Ruins").unwrap();
                lane.revealed = true;
            }
            if case == "elysium" {
                game.lanes[0].def = crate::locations::get("Elysium").unwrap();
            }
            game.turn = 3;
            game.is_wait_for_staging = false;
            let id = match case {
                "swan" => "Hawkeye",
                "superior" => "SuperiorIronMan",
                _ => "Hulk",
            };
            let card = game.create_card(id, 0);
            assert!(game.add_hand(0, card, false));
            if case == "superior" {
                game.add_cost(card, -1, 0);
            }
            game.players[0].attach_cost_effect(PlayerCostEffectKind::Sandman { turn: 3 });
            match case {
                "wave" => {
                    game.players[0].attach_cost_effect(PlayerCostEffectKind::Wave { until: 3 })
                }
                "mordo" => game.set_player_card_cost_until(card, 6, 3),
                "swan" => game.set_hand_cost_for_printed_cost_until(0, 1, 0, 3),
                "beast" => {
                    game.turn = 2;
                    game.add_next_turn_cost(card, -1);
                    game.turn = 3;
                }
                "queen" => game.set_temporary_hand_cost(card, 3, 0),
                _ => {}
            }
            game.ongoing();
            let expected = match case {
                "elysium" | "beast" => 5,
                "wave" => 4,
                "mordo" => 6,
                "swan" | "queen" => 0,
                "superior" => game.printed_cost(card) - 1,
                _ => unreachable!(),
            };
            assert_eq!(game.card_cost(card), expected, "{case}");
        }
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
    fn resurrect_entity_to_hand_returns_discard_without_touching_destroyed_cards() {
        let mut rng = PythonRandom::seed(71);
        let mut game = game(&mut rng);
        let discarded = game.create_card("Hulk", 0);
        let destroyed = game.create_card("Wasp", 0);
        assert!(game.add_hand(0, discarded, false));
        assert!(game.discard_card(0, discarded));
        assert!(game.add_card(destroyed, 0, 0, false));
        assert!(game.destroy_card(destroyed));
        assert!(game.resurrect_entity_to_hand(discarded, 0));
        assert!(game.players[0].hand.contains(&discarded));
        assert!(!game.players[0].discard.contains(&discarded));
        assert!(!game.players[0].graveyard_order.contains(&discarded));
        assert!(
            game.players[0]
                .graveyard
                .iter()
                .any(|&(id, _)| id == destroyed)
        );
    }
    #[test]
    fn resurrect_entity_to_hand_moves_existing_entity_from_all_zones() {
        use crate::zones::CardZone;
        for zone in [
            CardZone::Hand(0),
            CardZone::Location(0),
            CardZone::Deck(0),
            CardZone::Banished(0),
        ] {
            let mut rng = PythonRandom::seed(72);
            let mut game = game(&mut rng);
            let card = game.create_card("Hulk", 0);
            match zone {
                CardZone::Hand(_) => {
                    assert!(game.add_hand(0, card, false));
                }
                CardZone::Location(_) => {
                    assert!(game.add_card(card, 0, 0, false));
                }
                CardZone::Deck(_) => game.push_deck(0, card),
                CardZone::Banished(_) => game.push_banished(0, card),
                _ => unreachable!(),
            }
            assert!(game.resurrect_entity_to_hand(card, 0));
            assert_eq!(
                game.players[0]
                    .hand
                    .iter()
                    .filter(|&&id| id == card)
                    .count(),
                1
            );
            assert!(!game.players[0].deck.contains(&card));
            assert!(!game.players[0].banished.contains(&card));
            assert!(!game.lanes[0].cards.contains(&card));
            assert_eq!(game.cards[card].zone, Some(CardZone::Hand(0)));
            if zone != CardZone::Hand(0) {
                assert_eq!(game.cards[card].previous_zone, Some(zone));
            }
        }
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
