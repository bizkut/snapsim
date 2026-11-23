use crate::game::Game;

impl Game<'_> {
    /// Whether this revealed, surviving location can execute its abilities.
    /// SnowguardHawkBehavior::SilenceLocation (0x7f41cd0) silences all
    /// revealed locations until EndGame or the end of the following turn.
    pub fn location_abilities_active(&self, lane: usize) -> bool {
        self.lanes.get(lane).is_some_and(|location| {
            location.revealed
                && !location.destroyed
                && (self.ending_reveals
                    || !self.players.iter().any(|player| {
                        player
                            .location_silence_until
                            .is_some_and(|until| self.turn <= until)
                    }))
        })
    }

    pub(crate) fn silence_locations_through_next_turn(&mut self, seat: usize) {
        let until = self.turn.saturating_add(1);
        self.players[seat].location_silence_until = Some(
            self.players[seat]
                .location_silence_until
                .unwrap_or(0)
                .max(until),
        );
        self.delays_reveals = false;
        self.refresh_location_turn_limit();
        self.ongoing();
    }

    /// Remove expired player-owned silence before the next turn or EndGame.
    pub(crate) fn expire_location_silence(&mut self, end_game: bool) {
        let mut changed = false;
        for player in &mut self.players {
            if player
                .location_silence_until
                .is_some_and(|until| end_game || until < self.turn)
            {
                player.location_silence_until = None;
                changed = true;
            }
        }
        if changed {
            if !end_game {
                self.refresh_location_turn_limit();
            }
            self.ongoing();
        }
    }

    fn refresh_location_turn_limit(&mut self) {
        if self
            .lanes
            .iter()
            .any(|lane| lane.revealed && lane.def.minimum_turns > 6)
        {
            let minimum = self
                .lanes
                .iter()
                .enumerate()
                .filter(|(lane, _)| self.location_abilities_active(*lane))
                .map(|(_, location)| location.def.minimum_turns)
                .max()
                .unwrap_or(0);
            self.set_total_turns(6.max(minimum));
        }
    }

    /// SpaceThroneBehavior::SetNumCardSlotsPerSide (0x7d81910).
    pub fn lane_slot_capacity(&self, lane: usize) -> usize {
        if self.lanes[lane].destroyed {
            0
        } else if self.location_abilities_active(lane) && self.lanes[lane].def.id == "SpaceThrone" {
            1
        } else {
            4
        }
    }

    /// Whether players may retreat in the specified lifecycle phase.
    pub fn can_retreat(&self, end_turn: bool) -> bool {
        !self.lanes.iter().enumerate().any(|(lane, location)| {
            self.location_abilities_active(lane)
                && location.def.prevents_retreat.is_some_and(|test| {
                    self.record_location_hook(lane, "prevents_retreat");
                    test(self, end_turn)
                })
        })
    }

    /// Reveal one scheduled location, preserving the regular reveal lifecycle.
    pub(crate) fn reveal_scheduled_location(&mut self, lane: usize) {
        if self.lanes[lane].revealed || self.lanes[lane].destroyed {
            return;
        }
        let was_copied = self.lanes[lane].copied_reveal_turn.take().is_some();
        if was_copied {
            self.lanes[lane].state = self.lanes[lane].def.initial_state.map(|init| init());
        }
        self.lanes[lane].revealed = true;
        self.lanes[lane].revealed_turn = Some(self.turn);
        self.delays_reveals |=
            self.location_abilities_active(lane) && self.lanes[lane].def.delays_reveals;
        if was_copied {
            self.emit(crate::game::Event::LocationMorphed { lane });
        }
        self.emit(crate::game::Event::LocationRevealed { lane });
        if self.location_abilities_active(lane) {
            if let Some(hook) = self.lanes[lane].def.on_reveal {
                let _ = self.counted_trigger(|game| {
                    game.record_location_hook(lane, "on_reveal");
                    hook(game, lane)
                });
            }
            self.set_total_turns(self.total_turns.max(self.lanes[lane].def.minimum_turns));
            for seat in 0..2 {
                let energy = self.players[seat].energy + self.lanes[lane].def.reveal_energy;
                self.set_energy(seat, energy);
            }
        }
        // Location::RevealLocation (0x7de100c) refreshes ongoing state after
        // ThisLocationRevealed before returning to the next trigger observer.
        if !crate::cards::reference_rules_enabled() {
            self.ongoing();
        }
    }

    /// Legion's hidden-source path morphs a target into a fresh reveal
    /// placeholder carrying the source's LocationToReveal (0x7ee7c68).
    pub(crate) fn copy_hidden_location(&mut self, source: usize, target: usize, reveal_turn: u8) {
        let def = self.lanes[source].def;
        let observed_id = self.lanes[source].observed_id.clone();
        self.lanes[target].def = def;
        self.lanes[target].observed_id = observed_id;
        self.lanes[target].state = None;
        self.lanes[target].revealed = false;
        self.lanes[target].revealed_turn = None;
        self.lanes[target].copied_reveal_turn = Some(reveal_turn);
        let minimum = self
            .lanes
            .iter()
            .enumerate()
            .filter(|(lane, _)| self.location_abilities_active(*lane))
            .map(|(_, location)| location.def.minimum_turns)
            .max()
            .unwrap_or(0);
        self.set_total_turns(6.max(minimum));
        self.ongoing();
        self.emit(crate::game::Event::LocationMorphed { lane: target });
    }

    pub(crate) fn swap_location_objects(&mut self, a: usize, b: usize) {
        self.lanes.swap(a, b);
        crate::cards::swap_punisher_marked_locations(self, a, b);
        for lane in [a, b] {
            for &card in &self.lanes[lane].cards {
                self.cards[card].lane = Some(lane);
            }
        }
        // Proximity abilities retain a location entity, not a board slot.
        // Update every retained watch, including those on cards outside play.
        for card in &mut self.cards {
            for zone in [&mut card.zone, &mut card.previous_zone] {
                if let Some(crate::zones::CardZone::Location(lane)) = zone {
                    if *lane == a {
                        *lane = b;
                    } else if *lane == b {
                        *lane = a;
                    }
                }
            }
            for ability in &mut card.attached_abilities {
                if let crate::game::AttachedAbilityKind::Watch { lane, .. } = &mut ability.kind {
                    if *lane == a {
                        *lane = b;
                    } else if *lane == b {
                        *lane = a;
                    }
                }
            }
            for effect in &mut card.cost_effects {
                if let crate::game::CardCostEffectKind::WhiteQueenWinning { lane, .. } =
                    &mut effect.kind
                {
                    if *lane == a {
                        *lane = b;
                    } else if *lane == b {
                        *lane = a;
                    }
                }
            }
        }
        for player in &mut self.players {
            for grant in &mut player.move_grants {
                for lane in [&mut grant.lane, &mut grant.destination] {
                    if *lane == Some(a) {
                        *lane = Some(b);
                    } else if *lane == Some(b) {
                        *lane = Some(a);
                    }
                }
            }
        }
        if let Some(order) = &mut self.end_turn_lane_order {
            for lane in order {
                if *lane == a {
                    *lane = b;
                } else if *lane == b {
                    *lane = a;
                }
            }
        }
    }

    /// Game::ShuffleLocations (0x7daf300) uses one random bit for the two
    /// three-location derangements, moving each location with its cards.
    pub fn shuffle_locations(&mut self) {
        let odd = self.rng.borrow_mut().randbelow(10) & 1 != 0;
        self.swap_location_objects(0, if odd { 1 } else { 2 });
        self.swap_location_objects(1, 2);
        self.ongoing();
    }

    /// Game::DestroyLocation (0x7daeaf4) removes the location before force
    /// destroying its snapshot of cards in order.
    pub fn destroy_location(&mut self, lane: usize) {
        if self.lanes[lane].destroyed {
            return;
        }
        self.lanes[lane].destroyed = true;
        self.lanes[lane].revealed = false;
        let cards = self.lanes[lane].cards.clone();
        self.ongoing();
        for card in cards {
            self.force_destroy_card(card);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::random::PythonRandom;

    #[test]
    fn hawk_suppresses_location_end_of_staging_callback() {
        for silenced in [false, true] {
            let mut rng = PythonRandom::seed(53);
            let mut game = super::super::test_game(&mut rng);
            game.lanes[0].def = crate::locations::get("JosiesBar").unwrap();
            game.lanes[0].revealed = true;
            game.lanes[0].revealed_turn = Some(1);
            game.turn = 2;
            game.players[0].hand.clear();
            game.players[0].energy = 10;
            let first = game.create_card("Wasp", 0);
            let second = game.create_card("Wasp", 0);
            game.add_hand(0, first, false);
            game.add_hand(0, second, false);
            if silenced {
                game.silence_locations_through_next_turn(1);
            }
            game.stage_action(
                0,
                crate::game::Action::Play {
                    card: first,
                    lane: 0,
                },
            )
            .unwrap();
            game.stage_action(
                0,
                crate::game::Action::Play {
                    card: second,
                    lane: 0,
                },
            )
            .unwrap();
            game.resolve_staged_actions(0);
            let bonus = if silenced { 0 } else { 2 };
            assert_eq!(game.cards[first].permanent, 1 + bonus);
            assert_eq!(game.cards[second].permanent, 1 + bonus);
        }
    }

    #[test]
    fn hawk_suppresses_location_energy_and_ego_autoplay() {
        let mut rng = PythonRandom::seed(54);
        let mut game = super::super::test_game(&mut rng);
        game.lanes[0].def = crate::locations::get("Ego").unwrap();
        game.lanes[1].def = crate::locations::get("EmpireStateUniversity").unwrap();
        for lane in &mut game.lanes {
            lane.revealed = true;
        }
        game.current_location = 3;
        game.turn = 5;
        assert!(game.autoplay_controller(0).is_some());
        game.silence_locations_through_next_turn(0);
        assert!(game.autoplay_controller(0).is_none());
        game.turn = 6;
        game.start_turn();
        assert_eq!(game.players[0].energy, 6);
        game.finish_reveals();
        assert!(game.autoplay_controller(0).is_some());
    }

    #[test]
    fn hawk_suppresses_play_move_draw_and_morph_reapplication() {
        let mut rng = PythonRandom::seed(55);
        let mut game = super::super::test_game(&mut rng);
        for (lane, id) in ["SanctumSanctorum", "IslandPrison", "TimeTheater"]
            .iter()
            .enumerate()
        {
            let def = crate::locations::get(id).unwrap();
            game.lanes[lane].def = def;
            game.lanes[lane].state = def.initial_state.map(|init| init());
            game.lanes[lane].revealed = true;
        }
        game.turn = 5;
        let moving = game.create_card("Wasp", 0);
        game.add_card(moving, 1, 0, true);
        game.players[0].draws.push((1, moving));
        let hand = game.create_card("Hulk", 0);
        game.add_hand(0, hand, false);
        assert!(!game.can_play(hand, 0, 0, 10, 0));
        assert!(!game.can_move_card(moving, 0));
        let top = game.players[0].deck[0];
        game.silence_locations_through_next_turn(0);
        assert!(game.can_play(hand, 0, 0, 10, 0));
        assert!(game.can_move_card(moving, 0));
        game.draw(0);
        assert_eq!(game.players[0].hand.last(), Some(&top));
        game.replace_location(0, "SanctumSanctorum");
        assert!(!game.location_abilities_active(0));
        assert!(game.can_play(hand, 0, 0, 10, 0));
    }

    #[test]
    fn hawk_suppresses_turn_end_and_reconciles_limbo() {
        let mut rng = PythonRandom::seed(56);
        let mut game = super::super::test_game(&mut rng);
        for (lane, id) in ["Jotunheim", "Limbo", "Ruins"].iter().enumerate() {
            game.lanes[lane].def = crate::locations::get(id).unwrap();
            game.lanes[lane].revealed = true;
        }
        game.set_total_turns(7);
        let card = game.create_card("Wasp", 0);
        game.add_card(card, 0, 0, true);
        let initial = game.cards[card].permanent;
        game.silence_locations_through_next_turn(0);
        assert_eq!(game.total_turns, 6);
        game.end_turn_effects();
        assert_eq!(game.cards[card].permanent, initial);
        game.turn += 2;
        game.expire_location_silence(false);
        assert_eq!(game.total_turns, 7);
        game.end_turn_effects();
        assert_eq!(game.cards[card].permanent, initial - 1);
    }

    #[test]
    fn hawk_skips_tva_end_turn_without_retroactive_end() {
        let mut rng = PythonRandom::seed(57);
        let mut game = super::super::test_game(&mut rng);
        game.lanes[0].def = crate::locations::get("Tva").unwrap();
        game.lanes[0].revealed = true;
        game.turn = 3;
        game.silence_locations_through_next_turn(0);
        game.turn = 4;
        game.end_turn_effects();
        assert_eq!(game.total_turns, 6);
        game.turn = 5;
        game.expire_location_silence(false);
        game.end_turn_effects();
        assert_eq!(game.total_turns, 6);
    }

    #[test]
    fn hawk_silences_location_flags_but_not_card_provided_cosmo() {
        let mut rng = PythonRandom::seed(51);
        let mut game = super::super::test_game(&mut rng);
        for (lane, id) in ["SpaceThrone", "DeepSpace", "Elysium"].iter().enumerate() {
            game.lanes[lane].def = crate::locations::get(id).unwrap();
            game.lanes[lane].revealed = true;
        }
        let cosmo = game.create_card("Cosmo", 0);
        game.add_card(cosmo, 1, 0, true);
        let hulk = game.create_card("Hulk", 0);
        game.add_hand(0, hulk, false);
        game.ongoing();
        assert_eq!(game.lane_slot_capacity(0), 1);
        assert!(!game.lanes[1].ability_flags.trigger);
        assert_eq!(game.card_cost(hulk), 5);
        game.silence_locations_through_next_turn(0);
        assert_eq!(game.lane_slot_capacity(0), 4);
        assert!(game.lanes[1].ability_flags.trigger);
        assert!(!game.lanes[1].ability_flags.on_reveal);
        assert_eq!(game.card_cost(hulk), 6);
        game.turn += 1;
        assert!(!game.location_abilities_active(0));
        game.turn += 1;
        game.expire_location_silence(false);
        assert_eq!(game.lane_slot_capacity(0), 1);
        assert!(!game.lanes[1].ability_flags.trigger);
        assert_eq!(game.card_cost(hulk), 5);
    }

    #[test]
    fn hawk_suppresses_location_scoring_and_citadel_repetition() {
        let mut rng = PythonRandom::seed(52);
        let mut game = super::super::test_game(&mut rng);
        for (lane, id) in ["OnslaughtsCitadel", "TheNexus", "Cancun"]
            .iter()
            .enumerate()
        {
            game.lanes[lane].def = crate::locations::get(id).unwrap();
            game.lanes[lane].revealed = true;
        }
        let wasp = game.create_card("Wasp", 0);
        let iron_man = game.create_card("IronMan", 0);
        let other = game.create_card("Wasp", 0);
        game.add_card(wasp, 0, 0, true);
        game.add_card(iron_man, 0, 0, true);
        game.add_card(other, 2, 0, true);
        game.ongoing();
        assert_eq!(game.powers()[0][0], 4);
        assert_eq!(game.lane_winner(2), None);
        game.silence_locations_through_next_turn(0);
        assert_eq!(game.powers()[0][0], 2);
        assert_eq!(game.lane_winner(2), Some(0));
        game.finish_reveals();
        assert!(game.location_abilities_active(0));
        assert_eq!(game.powers()[0][0], 4);
        assert_eq!(game.lane_winner(2), None);
    }

    #[test]
    fn simultaneous_copied_reveals_run_each_location_start_in_order() {
        fn reveal(game: &mut crate::game::Game<'_>, _: usize) {
            game.players[0].energy = game.players[0].energy * 10 + 1;
        }
        fn start(game: &mut crate::game::Game<'_>, _: usize) {
            game.players[0].energy = game.players[0].energy * 10 + 2;
        }
        static PROBE: crate::locations::LocationDef = crate::locations::LocationDef {
            on_reveal: Some(reveal),
            on_turn_start: Some(start),
            ..crate::locations::location("probe", 0, 0, 1, false)
        };
        let mut rng = PythonRandom::seed(40);
        let mut game = super::super::test_game(&mut rng);
        game.turn = 2;
        game.current_location = 1;
        game.location_reveal_order = [0, 1, 2];
        game.lanes[0].def = &PROBE;
        game.lanes[1].def = &PROBE;
        game.lanes[0].copied_reveal_turn = Some(2);
        game.lanes[0].revealed = false;
        game.lanes[1].revealed = false;
        game.start_turn();
        assert_eq!(game.players[0].energy, 21212);
    }

    #[test]
    fn shuffle_moves_all_locations_and_tracks_cards_and_hooks() {
        let mut rng = PythonRandom::seed(41);
        let mut game = super::super::test_game(&mut rng);
        let original = std::array::from_fn::<_, 3, _>(|i| game.lanes[i].def.id);
        let cards = std::array::from_fn::<_, 3, _>(|i| {
            let card = game.create_card("Wasp", 0);
            game.add_card(card, i, 0, true);
            card
        });
        game.end_turn_lane_order = Some([0, 1, 2]);
        game.shuffle_locations();
        let order = game.end_turn_lane_order.unwrap();
        for i in 0..3 {
            assert_ne!(order[i], i);
            assert_eq!(game.lanes[order[i]].def.id, original[i]);
            assert_eq!(game.cards[cards[i]].lane, Some(order[i]));
        }
    }

    #[test]
    fn proximity_watches_follow_swapped_location_in_all_zones() {
        for (id, play_lane, bonus) in [
            ("BasicArrow", 2, 4),
            ("Hawkeye", 2, 4),
            ("JessicaJones", 0, 5),
        ] {
            let mut rng = PythonRandom::seed(43);
            let mut game = super::super::test_game(&mut rng);
            let watcher = game.create_card(id, 0);
            game.add_card(watcher, 0, 0, true);
            let original = game.cards[watcher].permanent;
            let off_board = game.create_card("Wasp", 1);
            game.cards[off_board].attach_watch(off_board, game.turn + 1, 0, true, 4);
            game.cards[off_board].previous_zone = Some(crate::zones::CardZone::Location(0));
            game.players[1].hand.push(off_board);
            game.swap_location_objects(0, 2);
            assert_eq!(
                game.cards[off_board].watch_contexts(),
                vec![(game.turn + 1, 2)]
            );
            assert_eq!(
                game.cards[off_board].previous_zone,
                Some(crate::zones::CardZone::Location(2))
            );
            assert_eq!(
                game.cards[watcher].zone,
                Some(crate::zones::CardZone::Location(2))
            );
            game.turn += 1;
            let played = game.create_card("Wasp", 0);
            game.players[0].hand.push(played);
            game.players[0].energy = 10;
            game.stage_action(
                0,
                crate::game::Action::Play {
                    card: played,
                    lane: play_lane,
                },
            )
            .unwrap();
            game.end_turn_effects();
            assert_eq!(game.cards[watcher].permanent, original + bonus, "{id}");
        }
    }

    #[test]
    fn slot_capacity_tracks_space_throne_reveal() {
        let mut rng = PythonRandom::seed(42);
        let mut game = super::super::test_game(&mut rng);
        game.lanes[0].def = crate::locations::get("SpaceThrone").unwrap();
        game.lanes[0].revealed = false;
        assert_eq!(game.lane_slot_capacity(0), 4);
        game.lanes[0].revealed = true;
        assert_eq!(game.lane_slot_capacity(0), 1);
    }

    #[test]
    fn destroy_location_force_destroys_protected_and_hidden_cards() {
        let mut rng = PythonRandom::seed(44);
        let mut game = super::super::test_game(&mut rng);
        game.lanes[0].def = crate::locations::get("Wakanda").unwrap();
        game.lanes[0].revealed = true;
        let card = game.create_card("Wasp", 0);
        game.add_card(card, 0, 0, false);
        assert!(!game.destroy_card(card));
        game.destroy_location(0);
        assert!(game.lanes[0].destroyed);
        assert!(game.lanes[0].cards.is_empty());
        assert!(game.cards[card].revealed);
        assert_eq!(game.players[0].graveyard.last(), Some(&(card, Some(0))));
    }
}
