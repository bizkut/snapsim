//! Reference heuristic and advisor playout policies.
use crate::{game::Game, random::GameRng};

fn shuffle_lanes<R: GameRng + ?Sized>(rng: &mut R, lanes: &mut [usize; 3]) {
    for i in (1..lanes.len()).rev() {
        lanes.swap(i, rng.randbelow(i + 1));
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Policy {
    Heuristic,
    Spread,
    Margin,
}
impl Policy {
    pub fn plan<R: GameRng + ?Sized>(
        self,
        game: &Game,
        seat: usize,
        rng: &mut R,
    ) -> Vec<(usize, usize)> {
        let player = &game.players[seat];
        let mut hand = player.hand.clone();
        match self {
            Self::Heuristic => hand.sort_by_key(|&id| game.card_cost(id)),
            _ => hand.sort_by_key(|&id| std::cmp::Reverse(game.card_cost(id))),
        }
        let mut lanes = [0, 1, 2];
        if matches!(self, Self::Margin) {
            shuffle_lanes(rng, &mut lanes);
        }
        let mut queued = [0usize; 3];
        let mut energy = player.energy;
        let mut plays = Vec::new();
        let mut margins = [0; 3];
        let mut width = [0; 3];
        let mut own_power = [0; 3];
        let mut own_slots = [0; 3];
        let mut lane_power = [0; 3];
        let mut multiplier = [1; 3];
        let mut scores = [0.0; 3];
        if matches!(self, Self::Margin) {
            for lane in 0..3 {
                let location = &game.lanes[lane];
                let mut power = [0; 2];
                let mut occupied = [0; 2];
                for &id in &location.cards {
                    let card = &game.cards[id];
                    occupied[card.owner] += 1;
                    power[card.owner] += if card.revealed { card.power() } else { 3 };
                }
                if location.revealed {
                    for (s, value) in power.iter_mut().enumerate() {
                        *value = (*value + location.bonus[s]) * location.multiplier[s];
                    }
                    multiplier[lane] = location.multiplier[seat];
                    lane_power[lane] = location.def.power;
                }
                margins[lane] = power[seat] - power[1 - seat];
                width[lane] = if occupied[1 - seat] == 4 {
                    1
                } else {
                    3 + i32::from(game.total_turns - game.turn)
                };
                own_power[lane] = power[seat];
                own_slots[lane] = occupied[seat];
                scores[lane] =
                    f64::from(margins[lane]) / f64::from(width[lane] + margins[lane].abs());
            }
        }
        for id in hand {
            let card = &game.cards[id];
            if matches!(self, Self::Spread) {
                shuffle_lanes(rng, &mut lanes);
            }
            if matches!(self, Self::Margin) {
                if game.card_cost(id) > energy {
                    continue;
                }
                let mut best = None;
                for &lane in &lanes {
                    if !game.can_play(id, lane, seat, energy, queued[lane]) {
                        continue;
                    }
                    let mut gain = card.power() + lane_power[lane];
                    if card.def.id == "Medusa" && lane == 1 {
                        gain += 3;
                    } else if card.def.id == "Wolfsbane" {
                        gain += 2 * (own_slots[lane] + queued[lane] as i32);
                    }
                    gain *= multiplier[lane];
                    if card.def.id == "IronMan" {
                        gain = gain * 2 + own_power[lane];
                    }
                    let after = margins[lane] + gain;
                    let score = f64::from(after) / f64::from(width[lane] + after.abs());
                    let split = 1.0 - scores[(lane + 2) % 3] * scores[(lane + 1) % 3];
                    let value = split * (score - scores[lane]) + 0.0005 * f64::from(gain);
                    if best.is_none_or(|(_, previous, _, _)| value > previous) {
                        best = Some((lane, value, gain, score));
                    }
                }
                if let Some((lane, _, gain, score)) = best {
                    plays.push((id, lane));
                    queued[lane] += 1;
                    energy -= game.card_cost(id);
                    margins[lane] += gain;
                    own_power[lane] += gain;
                    scores[lane] = score;
                    if card.def.id == "IronMan" {
                        multiplier[lane] *= 2;
                    }
                }
            } else {
                for &lane in &lanes {
                    if game.can_play(id, lane, seat, energy, queued[lane]) {
                        plays.push((id, lane));
                        queued[lane] += 1;
                        energy -= game.card_cost(id);
                        break;
                    }
                }
            }
        }
        plays
    }
}
