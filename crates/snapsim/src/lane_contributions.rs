//! Provenance for the ongoing contributions to each location's score.
use crate::game::{Game, Lane, OngoingSource};

#[derive(Clone, Copy, Debug)]
enum LaneEffect {
    Bonus(i32),
    Multiplier(i32),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LaneContribution {
    source: Option<OngoingSource>,
    seat: usize,
    effect: LaneEffect,
}

impl Lane {
    fn recalculate_contributions(&mut self) {
        self.bonus = [0; 2];
        self.multiplier = [1; 2];
        for contribution in &self.contributions {
            match contribution.effect {
                LaneEffect::Bonus(value) => self.bonus[contribution.seat] += value,
                LaneEffect::Multiplier(value) => self.multiplier[contribution.seat] *= value,
            }
        }
    }
}

impl Game<'_> {
    pub(crate) fn add_lane_bonus(&mut self, lane: usize, seat: usize, value: i32) {
        self.lanes[lane].bonus[seat] += value;
        self.lanes[lane].contributions.push(LaneContribution {
            source: self.current_ongoing_source,
            seat,
            effect: LaneEffect::Bonus(value),
        });
    }

    pub(crate) fn multiply_lane_power(&mut self, lane: usize, seat: usize, value: i32) {
        self.lanes[lane].multiplier[seat] *= value;
        self.lanes[lane].contributions.push(LaneContribution {
            source: self.current_ongoing_source,
            seat,
            effect: LaneEffect::Multiplier(value),
        });
    }

    /// Remove one ability's contributions while remembering their original ordering.
    pub(crate) fn take_lane_contributions(
        &mut self,
        source: OngoingSource,
    ) -> Vec<(usize, Vec<usize>)> {
        let mut positions = Vec::new();
        for (index, lane) in self.lanes.iter_mut().enumerate() {
            let mut old_indices = Vec::new();
            let mut ordinal = 0;
            lane.contributions.retain(|entry| {
                let remove = entry.source == Some(source);
                if remove {
                    old_indices.push(ordinal);
                }
                ordinal += 1;
                !remove
            });
            if !old_indices.is_empty() {
                lane.recalculate_contributions();
                positions.push((index, old_indices));
            }
        }
        positions
    }

    /// Place newly evaluated entries back at their former relative positions.
    pub(crate) fn restore_lane_contributions(
        &mut self,
        source: OngoingSource,
        positions: Vec<(usize, Vec<usize>)>,
    ) {
        for (index, indices) in positions {
            let lane = &mut self.lanes[index];
            let mut replacements = Vec::new();
            lane.contributions.retain(|entry| {
                if entry.source == Some(source) {
                    replacements.push(*entry);
                    false
                } else {
                    true
                }
            });
            for (ordinal, entry) in replacements.into_iter().enumerate() {
                let position = indices
                    .get(ordinal)
                    .copied()
                    .unwrap_or_else(|| indices[indices.len() - 1] + ordinal + 1 - indices.len());
                lane.contributions
                    .insert(position.min(lane.contributions.len()), entry);
            }
            lane.recalculate_contributions();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn replacing_one_source_preserves_other_entries_and_order() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Hawkeye",
            "Medusa",
            "Wasp",
            "Hulk",
            "IronMan",
            "Cyclops",
            "AntMan",
            "Shocker",
            "Sentinel",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned)
        .to_vec();
        let locations = ["Ruins", "Xandar", "KamarTaj"].map(str::to_owned).to_vec();
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let a = OngoingSource::Card {
            card: 0,
            instance: Some(1),
            hand: false,
        };
        let b = OngoingSource::Card {
            card: 1,
            instance: None,
            hand: false,
        };
        game.add_lane_bonus(0, 0, 2);
        game.current_ongoing_source = Some(a);
        game.add_lane_bonus(0, 0, 3);
        game.current_ongoing_source = Some(b);
        game.add_lane_bonus(0, 0, 5);
        game.current_ongoing_source = Some(a);
        game.multiply_lane_power(0, 0, 2);
        let positions = game.take_lane_contributions(a);
        assert_eq!(game.lanes[0].bonus[0], 7);
        assert_eq!(game.lanes[0].multiplier[0], 1);
        game.add_lane_bonus(0, 0, 4);
        game.multiply_lane_power(0, 0, 3);
        game.restore_lane_contributions(a, positions);
        assert_eq!(game.lanes[0].bonus[0], 11);
        assert_eq!(game.lanes[0].multiplier[0], 3);
        assert_eq!(
            game.lanes[0]
                .contributions
                .iter()
                .map(|entry| entry.source)
                .collect::<Vec<_>>(),
            [None, Some(a), Some(b), Some(a)]
        );
    }
}
