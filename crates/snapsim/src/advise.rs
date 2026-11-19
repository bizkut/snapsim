//! Successive-halving search over sampled hidden-information games.
use crate::{
    coverage::{coverage, sampling_priors},
    game::{Action, Game},
    policy::Policy,
    position::{Position, build_game},
    random::PythonRandom,
};
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    cmp::Ordering,
    time::{Duration, Instant},
};

type Play = (usize, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PlanStep {
    Play {
        index: usize,
        lane: usize,
    },
    Move {
        card: usize,
        from: usize,
        slot: usize,
        lane: usize,
    },
    Activate {
        card: usize,
        from: usize,
        slot: usize,
    },
}

impl PlanStep {
    fn action(self, game: &Game<'_>) -> Action {
        match self {
            Self::Play { index, lane } => Action::Play {
                card: game.players[0].hand[index],
                lane,
            },
            Self::Move { card, lane, .. } => Action::Move { card, lane },
            Self::Activate { card, .. } => Action::Activate { card },
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct AdviseOptions {
    pub rollouts: usize,
    pub seed: i128,
    pub max_plans: usize,
    pub time_limit: Option<f64>,
    #[serde(default)]
    pub approximate: bool,
    pub playout: String,
    /// Disable player-chosen moves and activations for frozen-reference comparisons.
    #[serde(default)]
    pub plays_only: bool,
    pub workers: usize,
}

#[derive(Clone)]
struct Candidate {
    plays: Vec<PlanStep>,
    samples: usize,
    wins: usize,
    draws: usize,
}
impl Candidate {
    fn new(plays: Vec<PlanStep>) -> Self {
        Self {
            plays,
            samples: 0,
            wins: 0,
            draws: 0,
        }
    }
    fn record(&mut self, winner: i32) {
        self.samples += 1;
        self.wins += usize::from(winner == 0);
        self.draws += usize::from(winner == 2);
    }
    fn cmp_rank(&self, other: &Self) -> Ordering {
        let fraction = |n: usize, d: usize| n as f64 / d as f64;
        fraction(other.wins, other.samples)
            .total_cmp(&fraction(self.wins, self.samples))
            .then_with(|| {
                fraction(other.draws, other.samples).total_cmp(&fraction(self.draws, self.samples))
            })
            .then_with(|| other.samples.cmp(&self.samples))
            .then_with(|| other.plays.len().cmp(&self.plays.len()))
            .then_with(|| self.plays.cmp(&other.plays))
    }
    fn result(&self, position: &Position, plays_only: bool) -> Value {
        let plays: Vec<_> = self.plays.iter().map(|&step| match step {
            PlanStep::Play { index, lane } if plays_only => json!({
                "hand_index": index, "card": position.hand[index].id, "location": lane,
            }),
            PlanStep::Play { index, lane } => json!({
                "kind":"play", "hand_index": index, "card": position.hand[index].id, "location": lane,
            }),
            PlanStep::Move { from, slot, lane, .. } => json!({
                "kind":"move", "from":from, "board_index":slot,
                "card":position.locations[from].mine[slot].id, "location":lane,
            }),
            PlanStep::Activate { from, slot, .. } => json!({
                "kind":"activate", "from":from, "board_index":slot,
                "card":position.locations[from].mine[slot].id, "location":from,
            }),
        }).collect();
        let samples = self.samples as f64;
        json!({"plays": plays, "win": self.wins as f64 / samples,
            "draw": self.draws as f64 / samples,
            "loss": (self.samples - self.wins - self.draws) as f64 / samples,
            "samples": self.samples})
    }
}

fn legal_plans(game: &Game<'_>, stopped: &(dyn Fn() -> bool + Sync)) -> Vec<Vec<Play>> {
    fn extend(
        game: &Game<'_>,
        stopped: &(dyn Fn() -> bool + Sync),
        start: usize,
        energy: i32,
        queued: &mut [usize; 3],
        plays: &mut Vec<Play>,
        result: &mut Vec<Vec<Play>>,
    ) {
        for index in start..game.players[0].hand.len() {
            if stopped() {
                return;
            }
            let card = game.players[0].hand[index];
            for lane in 0..3 {
                if !game.can_play(card, lane, 0, energy, queued[lane]) {
                    continue;
                }
                plays.push((index, lane));
                result.push(plays.clone());
                queued[lane] += 1;
                extend(
                    game,
                    stopped,
                    index + 1,
                    energy - game.card_cost(card),
                    queued,
                    plays,
                    result,
                );
                queued[lane] -= 1;
                plays.pop();
            }
        }
    }
    let mut result = vec![vec![]];
    extend(
        game,
        stopped,
        0,
        game.players[0].energy,
        &mut [0; 3],
        &mut vec![],
        &mut result,
    );
    result
}

/// Enumerate ordered plans on independent staging states so moves free slots and
/// activations consume energy before subsequent actions are considered.
fn legal_action_plans(
    game: &Game<'_>,
    authority: Option<(&Game<'_>, usize)>,
    stopped: &(dyn Fn() -> bool + Sync),
) -> Vec<Vec<PlanStep>> {
    let hand: Vec<_> = game.players[0]
        .hand
        .iter()
        .copied()
        .enumerate()
        .map(|(index, card)| {
            (
                card,
                authority.map_or(card, |(actual, seat)| actual.players[seat].hand[index]),
            )
        })
        .collect();
    let mut board = Vec::new();
    for lane in 0..3 {
        for (slot, card) in game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&card| game.cards[card].owner == 0)
            .enumerate()
        {
            let actual_card = authority.map_or(card, |(actual, seat)| {
                actual.lanes[lane]
                    .cards
                    .iter()
                    .copied()
                    .filter(|&id| actual.cards[id].owner == seat)
                    .nth(slot)
                    .unwrap()
            });
            board.push((card, actual_card, lane, slot));
        }
    }
    fn extend(
        game: &Game<'_>,
        hand: &[(usize, usize)],
        board: &[(usize, usize, usize, usize)],
        authority: Option<(&Game<'_>, usize)>,
        stopped: &(dyn Fn() -> bool + Sync),
        plan: &mut Vec<PlanStep>,
        output: &mut Vec<Vec<PlanStep>>,
    ) {
        if stopped() {
            return;
        }
        let mut options = Vec::new();
        for (index, &(card, actual_card)) in hand.iter().enumerate() {
            if game.players[0].hand.contains(&card) {
                for lane in 0..3 {
                    if game.can_play(card, lane, 0, game.players[0].energy, 0) {
                        options.push((
                            PlanStep::Play { index, lane },
                            Action::Play { card, lane },
                            Action::Play {
                                card: actual_card,
                                lane,
                            },
                        ));
                    }
                }
            }
        }
        for &(card, actual_card, from, slot) in board {
            for lane in 0..3 {
                if game.can_player_move(card, lane, 0) {
                    options.push((
                        PlanStep::Move {
                            card,
                            from,
                            slot,
                            lane,
                        },
                        Action::Move { card, lane },
                        Action::Move {
                            card: actual_card,
                            lane,
                        },
                    ));
                }
            }
            if game.can_activate(card, 0) {
                options.push((
                    PlanStep::Activate { card, from, slot },
                    Action::Activate { card },
                    Action::Activate { card: actual_card },
                ));
            }
        }
        for (step, action, actual_action) in options {
            if stopped() {
                return;
            }
            let mut rng = game.fork_rng();
            let mut next = game.fork(rng.as_mut());
            if next.stage_action(0, action).is_err() {
                continue;
            }
            if let Some((actual, seat)) = authority {
                let mut actual_rng = actual.fork_rng();
                let mut actual_next = actual.fork(actual_rng.as_mut());
                if actual_next.stage_action(seat, actual_action).is_err() {
                    continue;
                }
                plan.push(step);
                output.push(plan.clone());
                extend(
                    &next,
                    hand,
                    board,
                    Some((&actual_next, seat)),
                    stopped,
                    plan,
                    output,
                );
            } else {
                plan.push(step);
                output.push(plan.clone());
                extend(&next, hand, board, None, stopped, plan, output);
            }
            plan.pop();
        }
    }
    let mut output = vec![Vec::new()];
    extend(
        game,
        &hand,
        &board,
        authority,
        stopped,
        &mut Vec::new(),
        &mut output,
    );
    output
}

fn play_out(
    game: &mut Game<'_>,
    position: &Position,
    plays: &[PlanStep],
    policy: Policy,
    plays_only: bool,
) -> Result<i32, String> {
    let priority = match position.priority.as_deref() {
        Some("me") => Some(0),
        Some("opponent") => Some(1),
        _ => None,
    };
    if plays_only {
        let plan: Vec<_> = plays
            .iter()
            .map(|step| match *step {
                PlanStep::Play { index, lane } => (game.players[0].hand[index], lane),
                _ => unreachable!("plays-only plan contains another action"),
            })
            .collect();
        game.resolve_position_turn(&plan, policy, priority);
        while game.turn <= game.total_turns {
            game.play_turn_plays_only(policy);
        }
    } else {
        let actions: Vec<_> = plays.iter().map(|step| step.action(game)).collect();
        game.resolve_position_actions(&actions, policy, priority)?;
        while game.turn <= game.total_turns {
            game.play_turn_actions(policy)?;
        }
    }
    Ok(game.winner())
}

/// JSON interface for the self-contained advisor.
pub fn advise_json(position_json: &str, options_json: &str) -> Result<String, String> {
    advise_json_with_cancel(position_json, options_json, &|| false)
}

/// As `advise_json`, with cooperative cancellation between rollouts.
pub fn advise_json_with_cancel(
    position_json: &str,
    options_json: &str,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<String, String> {
    let position = Position::from_json(position_json)?;
    let options: AdviseOptions = serde_json::from_str(options_json).map_err(|e| e.to_string())?;
    serde_json::to_string(&advise(&position, &options, cancel)?).map_err(|e| e.to_string())
}

/// Search a position, reporting unsupported state without requiring a rollout.
pub fn advise(
    position: &Position,
    options: &AdviseOptions,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<Value, String> {
    advise_with_game(position, options, cancel, None)
}

pub(crate) fn advise_with_game(
    position: &Position,
    options: &AdviseOptions,
    cancel: &(dyn Fn() -> bool + Sync),
    authority: Option<(&Game<'_>, usize)>,
) -> Result<Value, String> {
    if options.rollouts == 0 || options.max_plans == 0 || options.workers == 0 {
        return Err("rollouts, max_plans and workers must be positive".into());
    }
    if options
        .time_limit
        .is_some_and(|v| !v.is_finite() || v < 0.0)
    {
        return Err("time_limit must be a finite nonnegative number".into());
    }
    let policy = match options.playout.as_str() {
        "spread" => Policy::Spread,
        "heuristic" => Policy::Heuristic,
        "margin" => Policy::Margin,
        _ => return Err("unknown playout policy".into()),
    };
    let unsupported = coverage(position);
    let priors = sampling_priors(position);
    let incomplete = unsupported
        .as_object()
        .expect("coverage is an object")
        .values()
        .any(|value| match value {
            Value::Null => false,
            Value::Array(values) => !values.is_empty(),
            Value::Bool(value) => *value,
            _ => true,
        });
    let status = if incomplete {
        if options.approximate {
            "approximate"
        } else {
            "unsupported"
        }
    } else {
        "ok"
    };
    if incomplete && !options.approximate {
        return Ok(
            json!({"engine":"simulator-rollout", "status":status, "rollouts":0, "plans":[], "unsupported":unsupported, "priors":priors}),
        );
    }
    let deadline = options
        .time_limit
        .and_then(|v| Instant::now().checked_add(Duration::from_secs_f64(v)));
    let stopped = || cancel() || deadline.is_some_and(|end| Instant::now() >= end);
    let mut rng = PythonRandom::seed(options.seed);
    let mut sample_seed = rng.getrandbits(64) as u64;
    let mut first_rng = PythonRandom::seed(sample_seed as i128);
    let mut first_game = build_game(position, &mut first_rng)?;
    let mut legal = if options.plays_only {
        legal_plans(&first_game, &stopped)
            .into_iter()
            .map(|plan| {
                plan.into_iter()
                    .map(|(index, lane)| PlanStep::Play { index, lane })
                    .collect()
            })
            .collect()
    } else {
        legal_action_plans(&first_game, authority, &stopped)
    };
    let limit = options
        .rollouts
        .min(options.max_plans.max(2).max(options.rollouts / 2));
    if legal.len() > limit {
        let selected = rng.sample_indices(legal.len() - 1, limit - 1);
        legal = std::iter::once(Vec::new())
            .chain(selected.into_iter().map(|i| legal[i + 1].clone()))
            .collect();
    }
    let mut active: Vec<_> = legal.into_iter().map(Candidate::new).collect();
    let keep = options.max_plans.min(active.len());
    let mut remaining = options.rollouts;
    let pool = if options.workers > 1 {
        Some(
            rayon::ThreadPoolBuilder::new()
                .num_threads(options.workers)
                .build()
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    while remaining > 0 {
        let count = active.len();
        let quotient = (count - 1) / keep;
        let stages = 1 + (usize::BITS - quotient.leading_zeros()) as usize;
        let rounds = (remaining / (count * stages)).max(1);
        let size = remaining.min(count * rounds);
        let mut cursor = 0;
        let mut round_seed = sample_seed;
        let mut interrupted = false;
        while cursor < size {
            let first = remaining == options.rollouts;
            if !first && stopped() {
                interrupted = true;
                break;
            }
            let batch_size = if first {
                1
            } else {
                options.workers.saturating_mul(8).min(size - cursor)
            };
            let mut jobs = Vec::with_capacity(batch_size);
            for index in cursor..cursor + batch_size {
                if index % count == 0 {
                    round_seed = sample_seed;
                    sample_seed = rng.getrandbits(64) as u64;
                }
                jobs.push((index % count, round_seed));
            }
            cursor += batch_size;
            if first {
                let winner = play_out(
                    &mut first_game,
                    position,
                    &active[0].plays,
                    policy,
                    options.plays_only,
                )?;
                active[0].record(winner);
                remaining -= 1;
                continue;
            }
            let run = |&(index, seed): &(usize, u64)| -> Result<Option<(usize, i32)>, String> {
                if stopped() {
                    return Ok(None);
                }
                let mut rng = PythonRandom::seed(seed as i128);
                let mut game = build_game(position, &mut rng)?;
                Ok(Some((
                    index,
                    play_out(
                        &mut game,
                        position,
                        &active[index].plays,
                        policy,
                        options.plays_only,
                    )?,
                )))
            };
            let completed: Vec<_> = if let Some(pool) = &pool {
                pool.install(|| jobs.par_iter().map(run).collect())
            } else {
                jobs.iter().map(run).collect()
            };
            for item in completed {
                if let Some((index, winner)) = item? {
                    active[index].record(winner);
                    remaining -= 1;
                } else {
                    interrupted = true;
                }
            }
            if interrupted {
                break;
            }
        }
        if interrupted {
            break;
        }
        if count > keep {
            active.sort_by(Candidate::cmp_rank);
            active.truncate(keep.max(count.div_ceil(2)));
        }
    }
    active.retain(|candidate| candidate.samples > 0);
    active.sort_by(Candidate::cmp_rank);
    let plans: Vec<_> = active
        .iter()
        .take(options.max_plans)
        .map(|c| c.result(position, options.plays_only))
        .collect();
    Ok(json!({"engine": "simulator-rollout", "status": status,
        "rollouts": options.rollouts - remaining, "plans": plans,
        "unsupported": unsupported, "priors": priors}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_use_actual_staged_state_not_only_a_reconstructed_position() {
        let position: Position = serde_json::from_value(json!({
            "turn": 3, "energy": 3,
            "hand": [{"id":"Wasp"}, {"id":"Medusa"}], "deck": [],
            "locations": [
                {"id":"Ruins", "mine":[{"id":"Nightcrawler"}], "theirs":[]},
                {"id":"Ruins", "mine":[], "theirs":[]},
                {"id":"Ruins", "mine":[], "theirs":[]}
            ],
            "opponent": {"hand":0, "deck":0}
        }))
        .unwrap();
        let mut actual_rng = PythonRandom::seed(1);
        let mut actual = build_game(&position, &mut actual_rng).unwrap();
        let crawler = actual.lanes[0].cards[0];
        actual.cards[crawler].abilities_removed = true;
        let mut sample_rng = PythonRandom::seed(1);
        let sample = build_game(&position, &mut sample_rng).unwrap();
        let unrestricted = legal_action_plans(&sample, None, &|| false);
        assert!(
            unrestricted
                .iter()
                .flatten()
                .any(|step| matches!(step, PlanStep::Move { .. }))
        );
        let plans = legal_action_plans(&sample, Some((&actual, 0)), &|| false);
        assert!(
            !plans
                .iter()
                .flatten()
                .any(|step| matches!(step, PlanStep::Move { .. }))
        );
        for plan in plans {
            let actions: Vec<_> = plan.iter().map(|step| step.action(&actual)).collect();
            let mut rng = actual.fork_rng();
            let mut staged = actual.fork(rng.as_mut());
            for action in actions {
                staged.stage_action(0, action).unwrap();
            }
        }
    }
}
