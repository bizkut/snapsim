use crate::game::Game;

impl Game<'_> {
    /// SpaceThroneBehavior::SetNumCardSlotsPerSide (0x7d81910).
    pub fn lane_slot_capacity(&self, lane: usize) -> usize {
        if self.lanes[lane].destroyed {
            0
        } else if self.lanes[lane].revealed && self.lanes[lane].def.id == "SpaceThrone" {
            1
        } else {
            4
        }
    }

    pub(crate) fn swap_location_objects(&mut self, a: usize, b: usize) {
        self.lanes.swap(a, b);
        for lane in [a, b] {
            for &card in &self.lanes[lane].cards {
                self.cards[card].lane = Some(lane);
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
