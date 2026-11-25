//! Reference heuristic and advisor playout policies.
use crate::{
    game::{Action, Game},
    random::GameRng,
};

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
    OptionalHeuristic,
    OptionalSpread,
    OptionalMargin,
    /// Mix spread with a lane-margin opponent in the last two turns.
    SmartOpponent,
    OptionalSmartOpponent,
}
impl Policy {
    pub(crate) fn optional_moves(self) -> bool {
        matches!(
            self,
            Self::OptionalHeuristic
                | Self::OptionalSpread
                | Self::OptionalMargin
                | Self::OptionalSmartOpponent
        )
    }

    pub(crate) fn with_optional_moves(self) -> Self {
        match self {
            Self::Heuristic | Self::OptionalHeuristic => Self::OptionalHeuristic,
            Self::Spread | Self::OptionalSpread => Self::OptionalSpread,
            Self::Margin | Self::OptionalMargin => Self::OptionalMargin,
            Self::SmartOpponent | Self::OptionalSmartOpponent => Self::OptionalSmartOpponent,
        }
    }

    fn base(self) -> Self {
        match self {
            Self::OptionalHeuristic => Self::Heuristic,
            Self::OptionalSpread => Self::Spread,
            Self::OptionalMargin => Self::Margin,
            Self::OptionalSmartOpponent => Self::SmartOpponent,
            other => other,
        }
    }

    pub fn plan<R: GameRng + ?Sized>(
        self,
        game: &Game,
        seat: usize,
        rng: &mut R,
    ) -> Vec<(usize, usize)> {
        if matches!(self.base(), Self::SmartOpponent) {
            let policy = if seat == 1 && game.total_turns - game.turn < 2 && rng.randbelow(2) == 0 {
                Self::Margin
            } else {
                Self::Spread
            };
            return policy.plan(game, seat, rng);
        }
        let this = self.base();
        let player = &game.players[seat];
        let mut hand = player.hand.clone();
        match this {
            Self::Heuristic => hand.sort_by_key(|&id| game.card_cost(id)),
            _ => hand.sort_by_key(|&id| std::cmp::Reverse(game.card_cost(id))),
        }
        let mut lanes = [0, 1, 2];
        if matches!(this, Self::Margin) {
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
        if matches!(this, Self::Margin) {
            let sees_staged = game.daredevil_viewer() == Some(seat);
            for lane in 0..3 {
                let location = &game.lanes[lane];
                let mut power = [0; 2];
                let mut occupied = [0; 2];
                for &id in &location.cards {
                    let card = &game.cards[id];
                    occupied[card.owner] += 1;
                    let visible_staged = sees_staged
                        && card.owner != seat
                        && game.players[card.owner].staged_history.iter().any(|play| {
                            play.card == id && play.turn == game.turn && !play.resolved
                        });
                    power[card.owner] += if card.revealed || visible_staged {
                        card.power()
                    } else {
                        3
                    };
                }
                if location.revealed {
                    for (s, value) in power.iter_mut().enumerate() {
                        *value = (*value + location.bonus[s]) * location.multiplier[s];
                    }
                    multiplier[lane] = location.multiplier[seat];
                    if game.location_abilities_active(lane) {
                        lane_power[lane] = location.def.power;
                    }
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
            if matches!(this, Self::Spread) {
                shuffle_lanes(rng, &mut lanes);
            }
            if matches!(this, Self::Margin) {
                if game.card_cost(id) > energy {
                    continue;
                }
                let mut best = None;
                for &lane in &lanes {
                    if !game.can_play_with_count(id, lane, seat, energy, queued[lane], plays.len())
                    {
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
                    if game.can_play_with_count(id, lane, seat, energy, queued[lane], plays.len()) {
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

    /// Plan player actions on an isolated staging state, then choose normal plays.
    pub fn plan_actions(self, game: &Game<'_>, seat: usize, rng: &mut dyn GameRng) -> Vec<Action> {
        if matches!(self.base(), Self::SmartOpponent) {
            let policy = if seat == 1 && game.total_turns - game.turn < 2 && rng.randbelow(2) == 0 {
                Self::OptionalMargin
            } else if self.optional_moves() {
                Self::OptionalSpread
            } else {
                Self::Spread
            };
            return policy.plan_actions(game, seat, rng);
        }
        if let Some(controller) = game.autoplay_controller(seat) {
            return game.autoplay_plan_with_rng(seat, controller, rng);
        }
        if self.optional_moves() {
            return self.plan_actions_optional(game, seat, rng);
        }
        let mut fork_rng = rng.clone_rng();
        let mut staging = game.fork(fork_rng.as_mut());
        let mut actions = Vec::new();
        let board: Vec<_> = staging
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| staging.cards[id].owner == seat)
            .collect();
        for &card in &board {
            if let Some(lane) = (0..3).find(|&lane| staging.can_player_move(card, lane, seat)) {
                let action = Action::Move { card, lane };
                if staging.stage_action(seat, action).is_ok() {
                    actions.push(action);
                }
            }
        }
        for &card in &board {
            let action = Action::Activate { card };
            if staging.can_activate(card, seat) && staging.stage_action(seat, action).is_ok() {
                actions.push(action);
            }
        }
        for (card, lane) in self.plan(&staging, seat, rng) {
            let action = Action::Play { card, lane };
            if staging.stage_action(seat, action).is_ok() {
                actions.push(action);
            }
        }
        actions
    }

    /// Commit plays first so a voluntary move cannot consume their slots. Then
    /// move only when it improves the same lane-margin objective used by the
    /// margin policy; ties keep the card in place.
    fn plan_actions_optional(
        self,
        game: &Game<'_>,
        seat: usize,
        rng: &mut dyn GameRng,
    ) -> Vec<Action> {
        let mut fork_rng = rng.clone_rng();
        let mut staging = game.fork(fork_rng.as_mut());
        let board: Vec<_> = staging
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| staging.cards[id].owner == seat)
            .collect();
        let mut actions = Vec::new();
        for &card in &board {
            let action = Action::Activate { card };
            if staging.can_activate(card, seat) && staging.stage_action(seat, action).is_ok() {
                actions.push(action);
            }
        }
        for (card, lane) in self.base().plan(&staging, seat, rng) {
            let action = Action::Play { card, lane };
            if staging.stage_action(seat, action).is_ok() {
                actions.push(action);
            }
        }
        let score = |game: &Game<'_>| -> f64 {
            game.lanes
                .iter()
                .map(|lane| {
                    let mut power = [0; 2];
                    for &id in &lane.cards {
                        let card = &game.cards[id];
                        power[card.owner] += if card.revealed || card.owner == seat {
                            card.power()
                        } else {
                            3
                        };
                    }
                    if lane.revealed {
                        for (s, value) in power.iter_mut().enumerate() {
                            *value = (*value + lane.bonus[s]) * lane.multiplier[s];
                        }
                    }
                    let margin = power[seat] - power[1 - seat];
                    f64::from(margin) / f64::from(3 + margin.abs())
                })
                .sum()
        };
        for card in board {
            let mut best = score(&staging);
            let mut best_lane = None;
            for lane in 0..3 {
                if !staging.can_player_move(card, lane, seat) {
                    continue;
                }
                let mut branch_rng = rng.clone_rng();
                let mut branch = staging.fork(branch_rng.as_mut());
                if branch
                    .stage_action(seat, Action::Move { card, lane })
                    .is_ok()
                {
                    let value = score(&branch);
                    if value > best {
                        best = value;
                        best_lane = Some(lane);
                    }
                }
            }
            if let Some(lane) = best_lane {
                let action = Action::Move { card, lane };
                if staging.stage_action(seat, action).is_ok() {
                    actions.push(action);
                }
            }
        }
        actions
    }
}
