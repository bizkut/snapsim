//! Successive-halving search over sampled hidden-information games.
#[cfg(test)]
use crate::position::build_game;
use crate::{
    best_reply::{self, BestReplyConfig, BestReplyStats},
    coverage::{coverage, sampling_priors},
    game::{Action, Game},
    policy::Policy,
    position::{FrozenGeneratedAbilities, Position, build_game_with_generated},
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
    /// Independent policy-vs-policy samples of the current position (zero disables it).
    #[serde(default)]
    pub position_rollouts: u32,
    /// Independent cube-policy samples per scenario; zero preserves legacy advice.
    #[serde(default)]
    pub stake_rollouts: u32,
    /// Minimum samples for the chosen plan behind each stake-policy estimate.
    #[serde(default = "default_stake_policy_rollouts")]
    pub stake_policy_rollouts: usize,
    pub max_plans: usize,
    pub time_limit: Option<f64>,
    #[serde(default)]
    pub approximate: bool,
    pub playout: String,
    /// Disable player-chosen moves and activations for frozen-reference comparisons.
    #[serde(default)]
    pub plays_only: bool,
    /// Consider optional player moves in opponent replies and playouts.
    #[serde(default)]
    pub optional_moves: bool,
    /// Let the opponent of every sampled hidden state pick its best reply to our
    /// plan on the final turn (`true`, or `{"plans": N, "turns": 1|2}`); see
    /// `best_reply`. Off by default: the opponent plays the playout policy.
    #[serde(default, deserialize_with = "best_reply::deserialize_option")]
    pub best_reply: Option<BestReplyConfig>,
    /// Also use `best_reply` for the policy-vs-policy position estimate. Set
    /// false to rank plans with it while keeping the ordinary board estimate.
    #[serde(default = "default_true")]
    pub best_reply_estimate: bool,
    pub workers: usize,
}

fn default_true() -> bool {
    true
}

fn default_stake_policy_rollouts() -> usize {
    32
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
    if game.autoplay_controller(0).is_some() {
        return vec![Vec::new()];
    }
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
                if !game.can_play_with_count(card, lane, 0, energy, queued[lane], plays.len()) {
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
    if game.autoplay_controller(0).is_some() {
        return vec![Vec::new()];
    }
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
    reply: Option<(BestReplyConfig, &BestReplyStats)>,
) -> Result<i32, String> {
    let priority = match position.priority.as_deref() {
        Some("me") => Some(0),
        Some("opponent") => Some(1),
        _ => None,
    };
    if let Some((config, stats)) = reply {
        let actions: Vec<_> = plays.iter().map(|step| step.action(game)).collect();
        if let Some(winner) = best_reply::resolve_position(
            game,
            policy,
            priority,
            plays_only,
            Some(&actions),
            config,
            stats,
        )? {
            return Ok(winner);
        }
    }
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
    let mut position = Position::from_json(position_json)?;
    let raw_options: Value = serde_json::from_str(options_json).map_err(|e| e.to_string())?;
    if let Some(previous) = raw_options.get("previous_turn_locations") {
        position.previous_turn_locations = serde_json::from_value(previous.clone()).ok();
    }
    let options: AdviseOptions = serde_json::from_value(raw_options).map_err(|e| e.to_string())?;
    serde_json::to_string(&advise(&position, &options, cancel)?).map_err(|e| e.to_string())
}

/// Search a position, reporting unsupported state without requiring a rollout.
pub fn advise(
    position: &Position,
    options: &AdviseOptions,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<Value, String> {
    advise_with_game(position, options, cancel, None, 0)
}

pub(crate) fn advise_with_game(
    position: &Position,
    options: &AdviseOptions,
    cancel: &(dyn Fn() -> bool + Sync),
    authority: Option<(&Game<'_>, usize)>,
    min_plan_samples: usize,
) -> Result<Value, String> {
    if options.rollouts == 0 || options.max_plans == 0 || options.workers == 0 {
        return Err("rollouts, max_plans and workers must be positive".into());
    }
    if options.stake_rollouts > 0 && options.stake_policy_rollouts == 0 {
        return Err("stake_policy_rollouts must be positive when stake_rollouts is enabled".into());
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
        "smart_opponent" => Policy::SmartOpponent,
        _ => return Err("unknown playout policy".into()),
    };
    let policy = if options.optional_moves {
        policy.with_optional_moves()
    } else {
        policy
    };
    let mut unsupported = coverage(position);
    let priors = sampling_priors(position);
    let attached_red = authority.is_some_and(|(game, seat)| {
        game.lanes.iter().flat_map(|lane| &lane.cards).any(|&id| {
            let card = &game.cards[id];
            (card.owner == seat || card.revealed)
                && card
                    .composed_abilities
                    .iter()
                    .any(|ability| ability.def.id == "RedOnslaught")
        })
    }) || position
        .locations
        .iter()
        .flat_map(|lane| lane.mine.iter().chain(&lane.theirs))
        .any(|card| {
            card.id != "RedOnslaught"
                && card.ability_state.as_ref().is_some_and(|state| {
                    state
                        .abilities
                        .iter()
                        .any(|ability| ability.ability_def_id.starts_with("RedOnslaught"))
                })
        })
        || position.provenance.as_ref().is_some_and(|provenance| {
            provenance.entities.iter().any(|entity| {
                entity.kind == "card"
                    && entity.is_active_holder()
                    && entity.card_id.as_deref() != Some("RedOnslaught")
                    && entity
                        .abilities
                        .items
                        .iter()
                        .any(|ability| ability.owner_def_id == "RedOnslaught")
            })
        });
    if attached_red {
        unsupported["state"]
            .as_array_mut()
            .expect("coverage state")
            .push(json!(
                "attached Red Onslaught generated abilities cannot be reconstructed"
            ));
        return Ok(
            json!({"engine":"simulator-rollout", "status":"unsupported", "rollouts":0,
            "plans":[], "unsupported":unsupported, "priors":priors}),
        );
    }
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
    let start = Instant::now();
    let deadline = options
        .time_limit
        .and_then(|v| start.checked_add(Duration::from_secs_f64(v)));
    let plan_deadline = options.time_limit.and_then(|v| {
        let fraction = if options.stake_rollouts > 0 {
            0.5
        } else if options.position_rollouts > 0 {
            0.7
        } else {
            1.0
        };
        start.checked_add(Duration::from_secs_f64(v * fraction))
    });
    let stopped = || cancel() || plan_deadline.is_some_and(|end| Instant::now() >= end);
    let position_deadline = if options.stake_rollouts > 0 {
        options
            .time_limit
            .and_then(|v| start.checked_add(Duration::from_secs_f64(v * 0.7)))
    } else {
        deadline
    };
    let position_stopped =
        || cancel() || position_deadline.is_some_and(|end| Instant::now() >= end);
    let stake_stopped = || cancel() || deadline.is_some_and(|end| Instant::now() >= end);
    let best_reply_stats = BestReplyStats::default();
    let reply = options.best_reply.map(|config| (config, &best_reply_stats));
    let mut rng = PythonRandom::seed(options.seed);
    let mut sample_seed = rng.getrandbits(64) as u64;
    let mut first_rng = PythonRandom::seed(sample_seed as i128);
    let authoritative_generated =
        authority.map(|(game, seat)| FrozenGeneratedAbilities::capture(game, seat));
    let mut first_game =
        build_game_with_generated(position, &mut first_rng, authoritative_generated.as_ref())?;
    let frozen_generated = std::sync::Mutex::new(FrozenGeneratedAbilities::capture(&first_game, 0));
    let retreat_available = first_game.can_retreat(false)
        && !(position.stakes.as_ref().and_then(|s| s.last_raiser) == Some(0)
            && position.snapped_this_turn.is_some_and(|snapped| snapped[0]));
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
    let candidate_plans = legal.len();
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
            // An enclosing cancellation must also stop the mandatory first
            // rollout; the local time budget still permits that first sample.
            if stopped() && (!first || cancel()) {
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
                    reply,
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
                let frozen = frozen_generated.lock().map_err(|e| e.to_string())?;
                let mut game = build_game_with_generated(position, &mut rng, Some(&frozen))?;
                drop(frozen);
                Ok(Some((
                    index,
                    play_out(
                        &mut game,
                        position,
                        &active[index].plays,
                        policy,
                        options.plays_only,
                        reply,
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
    let mut extra_samples = 0;
    if let Some(best) = active.first_mut() {
        while best.samples < min_plan_samples && !stopped() {
            let seed = rng.getrandbits(64) as u64;
            let mut sample_rng = PythonRandom::seed(seed as i128);
            let mut game = build_game_with_generated(
                position,
                &mut sample_rng,
                Some(&*frozen_generated.lock().map_err(|e| e.to_string())?),
            )?;
            let winner = play_out(
                &mut game,
                position,
                &best.plays,
                policy,
                options.plays_only,
                reply,
            )?;
            best.record(winner);
            extra_samples += 1;
        }
    }
    let plans: Vec<_> = active
        .iter()
        .take(options.max_plans)
        .map(|c| c.result(position, options.plays_only))
        .collect();
    let mut result = json!({"engine": "simulator-rollout", "status": status,
        "rollouts": options.rollouts - remaining + extra_samples, "plans": plans,
        "unsupported": unsupported, "priors": priors});
    if min_plan_samples > 0 {
        result["policy_estimate_diagnostics"] = json!({"candidate_plans": candidate_plans});
    }
    if !crate::cards::reference_rules_enabled() {
        result["retreat_available"] = json!(retreat_available);
    }
    if options.position_rollouts > 0 {
        // This stream is independent of plan sampling and never changes plan results.
        let mut position_rng = PythonRandom::seed(options.seed ^ 0x504f_5349_5449_4f4e);
        let mut estimate = Candidate::new(Vec::new());
        for _ in 0..options.position_rollouts {
            if position_stopped() {
                break;
            }
            let seed = position_rng.getrandbits(64) as u64;
            let mut rng = PythonRandom::seed(seed as i128);
            let mut game = build_game_with_generated(
                position,
                &mut rng,
                Some(&*frozen_generated.lock().map_err(|e| e.to_string())?),
            )?;
            let priority = match position.priority.as_deref() {
                Some("me") => Some(0),
                Some("opponent") => Some(1),
                _ => None,
            };
            let reply = match options.best_reply.filter(|_| options.best_reply_estimate) {
                Some(config) => best_reply::resolve_position(
                    &mut game,
                    policy,
                    priority,
                    options.plays_only,
                    None,
                    config,
                    &best_reply_stats,
                )?,
                None => None,
            };
            let winner = if let Some(winner) = reply {
                winner
            } else {
                game.play_position_policy_turn(policy, priority, options.plays_only)?;
                while game.turn <= game.total_turns {
                    if options.plays_only {
                        game.play_turn_plays_only(policy);
                    } else {
                        game.play_turn_actions(policy)?;
                    }
                }
                game.winner()
            };
            estimate.record(winner);
        }
        if estimate.samples > 0 {
            let samples = estimate.samples as f64;
            result["position"] = json!({
                "win": estimate.wins as f64 / samples,
                "draw": estimate.draws as f64 / samples,
                "loss": (estimate.samples - estimate.wins - estimate.draws) as f64 / samples,
                "samples": estimate.samples,
                "approximate": incomplete,
            });
        }
    }
    if let Some(config) = options.best_reply {
        result["best_reply"] = best_reply_stats.report(config);
    }
    if options.stake_rollouts > 0 && position.stakes.is_some() && result.get("position").is_some() {
        result["stake_rollouts"] = crate::compare::stake_position_sweep(
            position,
            &*frozen_generated.lock().map_err(|e| e.to_string())?,
            options.stake_rollouts,
            options.stake_policy_rollouts,
            options.seed,
            &stake_stopped,
        )?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stake_advice_respects_outer_time_limit() {
        let position: Position =
            serde_json::from_str(include_str!("../tests/fixtures/stakes-kyln.json")).unwrap();
        let options: AdviseOptions = serde_json::from_value(json!({
            "rollouts": 8, "position_rollouts": 1, "stake_rollouts": 8,
            "stake_policy_rollouts": 64, "seed": 20261123, "max_plans": 1,
            "time_limit": 0.1, "approximate": true, "playout": "spread", "workers": 1
        }))
        .unwrap();
        let start = Instant::now();
        let result = advise_with_game(&position, &options, &|| false, None, 0).unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(
            result["stake_rollouts"]["scenarios"]
                .as_array()
                .unwrap()
                .iter()
                .any(|scenario| {
                    scenario["values"]
                        .as_object()
                        .unwrap()
                        .values()
                        .any(|value| value["reason"] == "not_enough_time")
                })
        );
    }

    #[test]
    fn unregistered_cards_in_discarded_zones_are_declined() {
        let options = json!({"rollouts":1,"seed":42,"max_plans":1,"time_limit":null,
            "approximate":false,"playout":"spread","workers":1});
        for (opponent, zone) in [
            (false, "graveyard"),
            (false, "banished"),
            (true, "graveyard"),
            (true, "banished"),
        ] {
            let mut position = json!({
                "turn":1,"energy":1,"hand":[],"deck":[],
                "locations":[{"id":"Ruins","mine":[],"theirs":[]},
                    {"id":"Ruins","mine":[],"theirs":[]},
                    {"id":"Ruins","mine":[],"theirs":[]}],
                "opponent":{"hand":0,"deck":0}
            });
            let owner = if opponent {
                &mut position["opponent"]
            } else {
                &mut position
            };
            owner[zone] = json!([{"id":"UnregisteredCard"}]);
            let result: Value = serde_json::from_str(
                &advise_json(&position.to_string(), &options.to_string()).unwrap(),
            )
            .unwrap();
            assert_eq!(result["status"], "unsupported", "{opponent} {zone}");
            assert_eq!(result["unsupported"]["cards"], json!(["UnregisteredCard"]));
            assert_eq!(result["rollouts"], 0);
        }
    }

    #[test]
    fn position_estimate_is_opt_in_and_independent() {
        let position = json!({
            "turn": 3, "energy": 3,
            "hand": [{"id":"Wasp"}, {"id":"Medusa"}], "deck": [],
            "locations": [
                {"id":"Ruins", "mine":[], "theirs":[]},
                {"id":"Ruins", "mine":[], "theirs":[]},
                {"id":"Ruins", "mine":[], "theirs":[]}
            ],
            "previous_turn_locations":{"complete":true,"ids":[["Ruins"],["Ruins"],["Ruins"]]},
            "opponent": {"hand":0, "deck":0}
        })
        .to_string();
        let mut options = json!({
            "rollouts": 20, "seed": 42, "max_plans": 3, "time_limit": null,
            "approximate": true, "playout": "spread", "workers": 1
        });
        let default = advise_json(&position, &options.to_string()).unwrap();
        options["stake_policy_rollouts"] = json!(73);
        assert_eq!(
            default,
            advise_json(&position, &options.to_string()).unwrap()
        );
        options["position_rollouts"] = json!(0);
        assert_eq!(
            default,
            advise_json(&position, &options.to_string()).unwrap()
        );
        options["position_rollouts"] = json!(25);
        let enabled = advise_json(&position, &options.to_string()).unwrap();
        assert_eq!(
            enabled,
            advise_json(&position, &options.to_string()).unwrap()
        );
        let mut enabled: Value = serde_json::from_str(&enabled).unwrap();
        let estimate = enabled.as_object_mut().unwrap().remove("position").unwrap();
        assert_eq!(estimate["samples"], 25);
        assert_eq!(estimate["approximate"], false);
        let total = ["win", "draw", "loss"]
            .iter()
            .map(|key| estimate[key].as_f64().unwrap())
            .sum::<f64>();
        assert!((total - 1.0).abs() < 1e-12);
        assert_eq!(enabled, serde_json::from_str::<Value>(&default).unwrap());
    }

    #[test]
    fn policy_estimate_refines_the_selected_plan_to_requested_samples() {
        let position = Position::from_json(
            &json!({
                "turn": 3, "energy": 3,
                "hand": [{"id":"Wasp"}, {"id":"Medusa"}], "deck": [],
                "locations": [
                    {"id":"Ruins", "mine":[], "theirs":[]},
                    {"id":"Ruins", "mine":[], "theirs":[]},
                    {"id":"Ruins", "mine":[], "theirs":[]}
                ],
                "opponent": {"hand":0, "deck":0}
            })
            .to_string(),
        )
        .unwrap();
        let options: AdviseOptions = serde_json::from_value(json!({
            "rollouts":32,"seed":42,"max_plans":1,"time_limit":null,
            "approximate":true,"playout":"spread","workers":1
        }))
        .unwrap();
        assert_eq!(options.stake_policy_rollouts, 32);
        let result = advise_with_game(&position, &options, &|| false, None, 32).unwrap();
        assert_eq!(result["plans"][0]["samples"], 32);
        assert!(
            result["policy_estimate_diagnostics"]["candidate_plans"]
                .as_u64()
                .unwrap()
                > 1
        );
    }

    #[test]
    fn advice_reports_vesuvius_retreat_legality() {
        let mut position = json!({
            "turn": 6, "energy": 6, "hand": [], "deck": [],
            "locations": [
                {"id":"Ruins","mine":[],"theirs":[]},
                {"id":"MountVesuvius","mine":[],"theirs":[]},
                {"id":"Wakanda","mine":[],"theirs":[]}
            ],
            "opponent":{"hand":0,"deck":0}
        });
        let options = json!({
            "rollouts":1,"seed":1,"max_plans":1,"time_limit":null,
            "approximate":true,"playout":"spread","workers":1
        });
        let result: Value = serde_json::from_str(
            &advise_json(&position.to_string(), &options.to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(result["retreat_available"], false);
        position["turn"] = json!(5);
        let result: Value = serde_json::from_str(
            &advise_json(&position.to_string(), &options.to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(result["retreat_available"], true);
    }

    #[test]
    fn missing_previous_locations_are_approximate_or_declined() {
        let position = json!({
            "turn":2,"energy":2,"hand":[],"deck":[],
            "locations":[{"id":"Ruins","mine":[],"theirs":[]},
                {"id":"Ruins","mine":[],"theirs":[]},
                {"id":"Ruins","mine":[],"theirs":[]}],
            "opponent":{"hand":0,"deck":0}
        })
        .to_string();
        let mut options = json!({"rollouts":1,"seed":42,"max_plans":1,"time_limit":null,
            "approximate":false,"playout":"spread","workers":1});
        let exact: Value =
            serde_json::from_str(&advise_json(&position, &options.to_string()).unwrap()).unwrap();
        assert_eq!(exact["status"], "unsupported");
        assert!(
            exact["unsupported"]["state"]
                .to_string()
                .contains("Feast/AltarOfDeath")
        );
        options["approximate"] = json!(true);
        let approximate: Value =
            serde_json::from_str(&advise_json(&position, &options.to_string()).unwrap()).unwrap();
        assert_eq!(approximate["status"], "approximate");
        options["approximate"] = json!(false);
        options["previous_turn_locations"] =
            json!({"complete":true,"ids":[["Ruins"],["Ruins"],["Ruins"]]});
        let complete: Value =
            serde_json::from_str(&advise_json(&position, &options.to_string()).unwrap()).unwrap();
        assert_eq!(complete["status"], "ok");
    }
    #[test]
    fn generated_ongoing_is_fixed_across_stake_style_samples() {
        let position: Position = serde_json::from_value(json!({
            "turn": 5, "energy": 0, "hand": [], "deck": [],
            "locations": [
                {"id":"Hala","mine":[{"id":"MrSinister"},{"id":"SinisterClone"}],"theirs":[]},
                {"id":"TheHub","mine":[{"id":"Mirage"}],"theirs":[{"id":"Nightcrawler"},{"id":"Squirrel"},{"id":"SuperSkrull"}]},
                {"id":"WakandanEmbassy","mine":[{"id":"RedOnslaught"},{"id":"Makkari"}],"theirs":[{"id":"AntMan"},{"id":"Squirrel"}]}
            ],
            "opponent":{"hand":4,"deck":3}
        })).unwrap();
        let mut chosen = None;
        for seed in 0..500 {
            let mut rng = PythonRandom::seed(seed);
            let game = build_game(&position, &mut rng).unwrap();
            let red = game.lanes[2]
                .cards
                .iter()
                .copied()
                .find(|&id| game.cards[id].def.id == "RedOnslaught")
                .unwrap();
            if game.cards[red]
                .composed_abilities
                .iter()
                .any(|a| a.def.id == "MadameWeb")
            {
                chosen = Some(game.into_owned());
                break;
            }
        }
        let first = chosen.expect("a seeded Red Onslaught grants Madame Web");
        let plans = legal_action_plans(&first, None, &|| false);
        assert!(
            plans
                .iter()
                .flatten()
                .any(|step| matches!(step, PlanStep::Move { .. }))
        );
        let children = |game: &Game<'_>| {
            let skrull = game.lanes[1]
                .cards
                .iter()
                .copied()
                .find(|&id| game.cards[id].def.id == "SuperSkrull")
                .unwrap();
            let red = game.cards[skrull]
                .virtual_ongoing_ids
                .iter()
                .find(|entry| entry.4.def.id == "RedOnslaught")
                .unwrap()
                .4
                .id;
            game.cards[skrull]
                .composed_abilities
                .iter()
                .filter(|a| a.parent.is_some_and(|parent| parent.0 == red))
                .map(|a| a.def.id)
                .collect::<Vec<_>>()
        };
        let expected_children = children(&first);
        assert_eq!(expected_children.len(), 3);
        let frozen = FrozenGeneratedAbilities::capture(&first, 0);
        for seed in 0..64 {
            let mut rng = PythonRandom::seed(seed);
            let game = build_game_with_generated(&position, &mut rng, Some(&frozen)).unwrap();
            assert_eq!(children(&game), expected_children, "seed {seed}");
            for plan in &plans {
                let actions: Vec<_> = plan.iter().map(|step| step.action(&game)).collect();
                let mut staged = game.fork_owned();
                for action in actions {
                    staged.stage_action(0, action).unwrap_or_else(|error| {
                        panic!("seed {seed}, action {action:?}, plan {plan:?}: {error}")
                    });
                }
            }
        }
    }

    #[test]
    fn visible_staged_play_changes_advice_without_changing_absent_default() {
        let mut position = json!({
            "turn":6,"total_turns":6,"energy":6,"hand":[{"id":"Hulk"}],"deck":[],
            "locations":[
                {"id":"Ruins","mine":[],"theirs":[],"theirs_face_down":1},
                {"id":"Ruins","mine":[],"theirs":[]},
                {"id":"Ruins","mine":[],"theirs":[]}
            ],
            "opponent":{"hand":0,"deck":0,"pool":["Wasp"]}
        });
        let options = json!({"rollouts":40,"seed":42,"max_plans":3,"time_limit":null,
                             "approximate":true,"playout":"spread","workers":1});
        let hidden = advise_json(&position.to_string(), &options.to_string()).unwrap();
        position["locations"][0]["known_opponent_plays"] = json!([]);
        assert_eq!(
            hidden,
            advise_json(&position.to_string(), &options.to_string()).unwrap()
        );
        position["locations"][0]["theirs_face_down"] = json!(0);
        position["locations"][0]["known_opponent_plays"] = json!(["Hulk"]);
        let shown = advise_json(&position.to_string(), &options.to_string()).unwrap();
        let hidden: Value = serde_json::from_str(&hidden).unwrap();
        let shown: Value = serde_json::from_str(&shown).unwrap();
        assert_ne!(hidden["plans"], shown["plans"]);
    }

    #[test]
    fn attached_red_onslaught_declines_even_approximate_advice() {
        let position: Position = serde_json::from_value(json!({
            "turn":3,"energy":3,"hand":[],"deck":[],
            "locations":[
                {"id":"Ruins","mine":[{"id":"MistyKnight"}],"theirs":[]},
                {"id":"Ruins","mine":[],"theirs":[]},
                {"id":"Ruins","mine":[],"theirs":[]}
            ],
            "opponent":{"hand":0,"deck":0}
        }))
        .unwrap();
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&position, &mut rng).unwrap();
        let card = game.lanes[0].cards[0];
        game.cards[card].composed_abilities.push(
            crate::engine_ext::abilities::AbilityInstance::new(
                crate::cards::get("RedOnslaught").unwrap(),
                crate::engine_ext::abilities::AbilityKind::Ongoing,
                Some(card),
                false,
                true,
                None,
            ),
        );
        let options: AdviseOptions = serde_json::from_value(json!({
            "rollouts":1,"seed":1,"max_plans":1,"time_limit":null,
            "approximate":true,"playout":"spread","workers":1
        }))
        .unwrap();
        let result = advise_with_game(&position, &options, &|| false, Some((&game, 0)), 0).unwrap();
        assert_eq!(result["status"], "unsupported");
        assert!(
            result["unsupported"]["state"]
                .to_string()
                .contains("attached Red Onslaught")
        );
    }

    #[test]
    fn inactive_red_onslaught_abilities_do_not_block_advice() {
        let mut position: Position = serde_json::from_value(json!({
            "turn":3,"energy":3,"hand":[],"deck":[],
            "locations":[
                {"id":"Ruins","mine":[],"theirs":[]},
                {"id":"Ruins","mine":[],"theirs":[]},
                {"id":"Ruins","mine":[],"theirs":[]}
            ],
            "opponent":{"hand":0,"deck":0},
            "provenance":{"schema_version":2,"entities_complete":true,
                "entities":[{"kind":"card","entity_id":42,"card_id":"MistyKnight",
                    "lane_index":null,"zone":"graveyard",
                    "abilities":{"complete":false,"reasons":[],"items":[{
                        "ability_def_id":"RedOnslaughtOngoing","owner_def_type":"Card",
                        "owner_def_id":"RedOnslaught","index":0}]},
                    "movable_components":{"complete":false,"reasons":[],"items":[]}}],
                "ledger":{"complete":true,"complete_for_legality":true,
                    "match_start_seen":true,"reasons":[],"items":[]}}
        }))
        .unwrap();
        let options: AdviseOptions = serde_json::from_value(json!({
            "rollouts":1,"seed":1,"max_plans":1,"time_limit":null,
            "approximate":true,"playout":"spread","workers":1
        }))
        .unwrap();
        let result = advise_with_game(&position, &options, &|| false, None, 0).unwrap();
        assert!(
            !result["unsupported"]["state"]
                .to_string()
                .contains("attached Red Onslaught")
        );
        position.provenance.as_mut().unwrap().entities[0].zone = Some("board".into());
        let result = advise_with_game(&position, &options, &|| false, None, 0).unwrap();
        assert!(
            result["unsupported"]["state"]
                .to_string()
                .contains("attached Red Onslaught")
        );
    }

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

    fn merge_plan_game(rng: &mut PythonRandom) -> Game<'_> {
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
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, rng).unwrap();
        game.players[0].hand.clear();
        game.players[0].energy = 0;
        game
    }

    fn plan_contains_move(game: &Game<'_>, card: usize) -> bool {
        legal_action_plans(game, None, &|| false)
            .iter()
            .flatten()
            .any(|step| matches!(step, PlanStep::Move { card: id, .. } if *id == card))
    }

    #[test]
    fn phoenix_movement_survives_merge_in_real_plans() {
        let mut rng = PythonRandom::seed(91);
        let mut game = merge_plan_game(&mut rng);
        let dead = game.create_card("Wasp", 0);
        game.cards[dead].observed_move_targets = Some(0);
        game.players[0].graveyard.push((dead, Some(1)));
        let phoenix = game.create_card("ThePhoenixForce", 0);
        assert!(game.add_card(phoenix, 0, 0, true));
        assert_eq!(game.cards[phoenix].merged_to, Some(dead));
        assert!(plan_contains_move(&game, dead));
        let receiver = game.create_card("Hulk", 0);
        assert!(game.add_card(receiver, 0, 0, false));
        game.cards[receiver].observed_move_targets = Some(0);
        assert!(game.merge_card(receiver, dead));
        assert!(plan_contains_move(&game, receiver));
        game.turn += 1;
        game.start_turn();
        game.players[0].hand.clear();
        game.players[0].energy = 0;
        assert!(plan_contains_move(&game, receiver));
    }

    #[test]
    fn peni_suit_movement_survives_merge_and_expires_in_real_plans() {
        let mut rng = PythonRandom::seed(92);
        let mut game = merge_plan_game(&mut rng);
        let peni = game.create_card("PeniParker", 0);
        assert!(game.add_card(peni, 0, 0, true));
        game.cards[peni].observed_move_targets = Some(0);
        let suit = game.players[0].hand.pop().unwrap();
        assert_eq!(game.cards[suit].def.id, "SPdr");
        assert!(game.add_card(suit, 0, 0, true));
        assert_eq!(game.cards[suit].merged_to, Some(peni));
        let receiver = game.create_card("Wasp", 0);
        assert!(game.add_card(receiver, 0, 0, false));
        game.cards[receiver].observed_move_targets = Some(0);
        assert!(game.merge_card(receiver, peni));
        assert!(!plan_contains_move(&game, receiver));
        game.turn += 1;
        game.start_turn();
        game.players[0].hand.clear();
        game.players[0].energy = 0;
        assert!(plan_contains_move(&game, receiver));
        game.turn += 1;
        game.start_turn();
        game.players[0].hand.clear();
        game.players[0].energy = 0;
        assert!(!game.cards[receiver].has_ongoing(true));
        assert!(!plan_contains_move(&game, receiver));
    }
    #[test]
    fn zero_cost_staging_keeps_all_ordered_plans() {
        let position: Position = serde_json::from_value(json!({
            "turn": 1, "energy": 0,
            "hand": [{"id":"Wasp"}, {"id":"Wasp"}, {"id":"Wasp"}],
            "deck": [], "opponent": {"hand":0, "deck":0},
            "locations": [
                {"id":"Ruins", "mine":[], "theirs":[]},
                {"id":"Xandar", "mine":[], "theirs":[]},
                {"id":"KamarTaj", "mine":[], "theirs":[]}
            ]
        }))
        .unwrap();
        let mut rng = PythonRandom::seed(123);
        let game = build_game(&position, &mut rng).unwrap();
        let plans = legal_action_plans(&game, None, &|| false);
        // Empty plan plus P(3,k) * 3^k for k=1..3.
        assert_eq!(plans.len(), 226);
    }
}
