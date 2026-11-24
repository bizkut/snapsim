//! Best-reply opponent for the final turns of a planning search.
//!
//! The default playout has the opponent stage whatever the playout policy
//! picks, which is rarely the plan a strong player would choose on the final
//! turn. With `best_reply`, the opponent of each sampled hidden state instead
//! tries a bounded set of its own legal plans against the plan already chosen
//! for our seat and keeps the one that is best for it (win, then draw, then
//! loss; the first winning reply ends the search). Results are still averaged
//! over samples, so this is not a worst case over hidden states: in every
//! sample the opponent has its sampled hand and our plan, and nothing else.
//!
//! Candidates are the playout policy's own plan, which always stays in the set
//! and wins ties, plus up to `plans - 1` maximal play plans (plans that cannot
//! take another card from the hand) of the sampled opponent hand, each with the
//! policy's moves and activations in front. When the maximal plans do not fit,
//! a uniform subset is evaluated; enumeration itself stops after
//! [`NODE_BUDGET`] search nodes. Both truncations are counted in
//! [`BestReplyStats`] and reported by the advisor.
use crate::{
    game::{Action, Game},
    policy::Policy,
};
use serde::{Deserialize, Deserializer};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Search nodes the maximal-plan enumeration visits per sampled state.
pub const NODE_BUDGET: usize = 20_000;
/// Evaluated plans per sampled state unless the options say otherwise.
pub const DEFAULT_PLANS: usize = 32;

/// Opt-in best-reply opponent. `true` selects the defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BestReplyConfig {
    /// Evaluated opponent plans per sampled state, including the policy's own.
    pub plans: usize,
    /// Final turns covered: 1 is the last turn, 2 also the one before it
    /// (the turn after is then played by the playout policy).
    pub turns: u8,
}

impl Default for BestReplyConfig {
    fn default() -> Self {
        Self {
            plans: DEFAULT_PLANS,
            turns: 1,
        }
    }
}

/// Parse `best_reply`: `false`/`null` disable it, `true` or an object with
/// optional `plans` and `turns` enable it.
pub fn deserialize_option<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<BestReplyConfig>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Flag(bool),
        Config {
            plans: Option<usize>,
            turns: Option<u8>,
        },
    }
    let config = match Option::<Raw>::deserialize(deserializer)? {
        None | Some(Raw::Flag(false)) => return Ok(None),
        Some(Raw::Flag(true)) => BestReplyConfig::default(),
        Some(Raw::Config { plans, turns }) => BestReplyConfig {
            plans: plans.unwrap_or(DEFAULT_PLANS),
            turns: turns.unwrap_or(1),
        },
    };
    if config.plans == 0 || !(1..=2).contains(&config.turns) {
        return Err(serde::de::Error::custom(
            "best_reply needs plans >= 1 and turns of 1 or 2",
        ));
    }
    Ok(Some(config))
}

/// What the best-reply search did, summed over all samples of one advice call.
#[derive(Default)]
pub struct BestReplyStats {
    samples: AtomicUsize,
    evaluated: AtomicUsize,
    sampled_down: AtomicUsize,
    enumeration_cut: AtomicUsize,
    changed: AtomicUsize,
    rejected: AtomicUsize,
}

impl BestReplyStats {
    pub fn report(&self, config: BestReplyConfig) -> serde_json::Value {
        let get = |counter: &AtomicUsize| counter.load(Ordering::Relaxed);
        serde_json::json!({
            "plans": config.plans, "turns": config.turns,
            "samples": get(&self.samples), "evaluated_plans": get(&self.evaluated),
            // Samples whose maximal plans exceeded the cap and were sampled down.
            "sampled_down_samples": get(&self.sampled_down),
            // Samples whose enumeration stopped at the node budget.
            "enumeration_cut_samples": get(&self.enumeration_cut),
            // Samples where a candidate gave the opponent a better result (win,
            // draw or loss) than the playout policy's own plan.
            "changed_result_samples": get(&self.changed),
            "rejected_plans": get(&self.rejected),
        })
    }
}

/// Maximal play plans of `seat`'s hand as `(card, lane)` lists, and whether the
/// enumeration finished within the node budget.
fn maximal_plans(game: &Game<'_>, seat: usize) -> (Vec<Vec<(usize, usize)>>, bool) {
    struct Search<'g, 'a> {
        game: &'g Game<'a>,
        seat: usize,
        hand: Vec<usize>,
        nodes: usize,
        complete: bool,
        found: Vec<Vec<(usize, usize)>>,
    }
    impl Search<'_, '_> {
        fn visit(
            &mut self,
            index: usize,
            energy: i32,
            queued: &mut [usize; 3],
            plan: &mut Vec<(usize, usize)>,
            skipped: &mut Vec<usize>,
        ) {
            if self.nodes >= NODE_BUDGET {
                self.complete = false;
                return;
            }
            self.nodes += 1;
            if index == self.hand.len() {
                let extendable = skipped.iter().any(|&card| {
                    (0..3).any(|lane| {
                        self.game.can_play_with_count(
                            card,
                            lane,
                            self.seat,
                            energy,
                            queued[lane],
                            plan.len(),
                        )
                    })
                });
                if !extendable {
                    self.found.push(plan.clone());
                }
                return;
            }
            let card = self.hand[index];
            for lane in 0..3 {
                if !self.game.can_play_with_count(
                    card,
                    lane,
                    self.seat,
                    energy,
                    queued[lane],
                    plan.len(),
                ) {
                    continue;
                }
                plan.push((card, lane));
                queued[lane] += 1;
                let left = energy - self.game.card_cost(card);
                self.visit(index + 1, left, queued, plan, skipped);
                queued[lane] -= 1;
                plan.pop();
            }
            skipped.push(card);
            self.visit(index + 1, energy, queued, plan, skipped);
            skipped.pop();
        }
    }
    let mut search = Search {
        game,
        seat,
        hand: game.players[seat].hand.clone(),
        nodes: 0,
        complete: true,
        found: Vec::new(),
    };
    let energy = game.players[seat].energy;
    search.visit(0, energy, &mut [0; 3], &mut Vec::new(), &mut Vec::new());
    (search.found, search.complete)
}

fn plays_of(actions: &[Action]) -> Vec<(usize, usize)> {
    let mut plays: Vec<_> = actions
        .iter()
        .filter_map(|&action| match action {
            Action::Play { card, lane } => Some((card, lane)),
            _ => None,
        })
        .collect();
    plays.sort_unstable();
    plays
}

/// Enumerate no move and each legal destination for each original board card.
/// Both the move-prefix count and the accumulated play plans are bounded.
fn optional_candidates(
    game: &Game<'_>,
    default: Vec<Action>,
    config: BestReplyConfig,
    stats: &BestReplyStats,
) -> Vec<Vec<Action>> {
    fn visit(
        game: &Game<'_>,
        board: &[usize],
        index: usize,
        prefix: &mut Vec<Action>,
        plans: &mut Vec<Vec<Action>>,
        leaves: &mut usize,
        complete: &mut bool,
    ) {
        if plans.len() >= NODE_BUDGET || *leaves >= 64 {
            *complete = false;
            return;
        }
        if index == board.len() {
            *leaves += 1;
            let (plays, finished) = maximal_plans(game, 1);
            *complete &= finished;
            for play in plays {
                if plans.len() >= NODE_BUDGET {
                    *complete = false;
                    break;
                }
                let mut actions = prefix.clone();
                actions.extend(
                    play.into_iter()
                        .map(|(card, lane)| Action::Play { card, lane }),
                );
                plans.push(actions);
            }
            return;
        }
        visit(game, board, index + 1, prefix, plans, leaves, complete);
        let card = board[index];
        for lane in 0..3 {
            if !game.can_player_move(card, lane, 1) || plans.len() >= NODE_BUDGET {
                continue;
            }
            let mut rng = game.fork_rng();
            let mut branch = game.fork(rng.as_mut());
            let action = Action::Move { card, lane };
            if branch.stage_action(1, action).is_ok() {
                prefix.push(action);
                visit(&branch, board, index + 1, prefix, plans, leaves, complete);
                prefix.pop();
            }
        }
    }
    let wanted = config.plans.saturating_sub(1);
    if wanted == 0 {
        return vec![default];
    }
    let mut rng = game.fork_rng();
    let mut staging = game.fork(rng.as_mut());
    let board: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&card| game.cards[card].owner == 1)
        .collect();
    let mut prefix = Vec::new();
    for &action in &default {
        if matches!(action, Action::Activate { .. }) && staging.stage_action(1, action).is_ok() {
            prefix.push(action);
        }
    }
    let mut plans = Vec::new();
    let mut complete = true;
    let mut leaves = 0;
    visit(
        &staging,
        &board,
        0,
        &mut prefix,
        &mut plans,
        &mut leaves,
        &mut complete,
    );
    if !complete {
        stats.enumeration_cut.fetch_add(1, Ordering::Relaxed);
    }
    plans.retain(|plan| *plan != default);
    if plans.len() > wanted {
        stats.sampled_down.fetch_add(1, Ordering::Relaxed);
        let mut rng = game.fork_rng();
        for slot in 0..wanted {
            let pick = slot + rng.randbelow(plans.len() - slot);
            plans.swap(slot, pick);
        }
        plans.truncate(wanted);
    }
    let mut all = vec![default];
    all.extend(plans);
    all
}

/// The opponent's candidate plans: the policy's own plan first, then maximal
/// play plans (after the policy's moves and activations), at most `config.plans`.
fn candidates(
    game: &Game<'_>,
    default: Vec<Action>,
    plays_only: bool,
    config: BestReplyConfig,
    stats: &BestReplyStats,
) -> Vec<Vec<Action>> {
    let wanted = config.plans.saturating_sub(1);
    if wanted == 0 {
        return vec![default];
    }
    let (mut plans, complete) = maximal_plans(game, 1);
    if !complete {
        stats.enumeration_cut.fetch_add(1, Ordering::Relaxed);
    }
    let policy_plays = plays_of(&default);
    plans.retain(|plan| {
        let mut sorted = plan.clone();
        sorted.sort_unstable();
        sorted != policy_plays
    });
    if plans.len() > wanted {
        stats.sampled_down.fetch_add(1, Ordering::Relaxed);
        // Partial Fisher-Yates on a clone of the game's stream: the real stream
        // is untouched, and the subset still differs between sampled states.
        let mut rng = game.fork_rng();
        for slot in 0..wanted {
            let pick = slot + rng.randbelow(plans.len() - slot);
            plans.swap(slot, pick);
        }
        plans.truncate(wanted);
    }
    let prefix: Vec<Action> = if plays_only {
        Vec::new()
    } else {
        default
            .iter()
            .copied()
            .filter(|action| !matches!(action, Action::Play { .. }))
            .collect()
    };
    let mut all = vec![default];
    all.extend(plans.into_iter().map(|plan| {
        let mut actions = prefix.clone();
        actions.extend(
            plan.into_iter()
                .map(|(card, lane)| Action::Play { card, lane }),
        );
        actions
    }));
    all
}

/// Resolve the position's turn with the opponent's best reply and play on to
/// the end with `policy`. Returns the winner (0 us, 1 opponent, 2 draw), or
/// `None` when the turn is outside the configured window or either seat is
/// scripted (autoplay, Daredevil's ordered staging), in which case the game is
/// untouched and the caller plays the ordinary playout.
///
/// `own` is our seat's staged actions; `None` lets `policy` plan them, which is
/// what the policy-vs-policy position estimate does.
pub(crate) fn resolve_position(
    game: &mut Game<'_>,
    policy: Policy,
    priority: Option<usize>,
    plays_only: bool,
    own: Option<&[Action]>,
    config: BestReplyConfig,
    stats: &BestReplyStats,
) -> Result<Option<i32>, String> {
    let remaining = i32::from(game.total_turns) - i32::from(game.turn) + 1;
    if remaining < 1
        || remaining > i32::from(config.turns)
        || (0..2).any(|seat| game.autoplay_controller(seat).is_some())
        || game.daredevil_viewer().is_some()
    {
        return Ok(None);
    }
    game.pending_plays = std::array::from_fn(|seat| {
        game.delayed_reveals[seat]
            .iter()
            .filter_map(|queued| game.cards[queued.card].lane.map(|lane| (queued.card, lane)))
            .collect()
    });
    let priority = game.action_priority(priority);
    let plan = |game: &Game<'_>, seat: usize| -> Vec<Action> {
        let mut rng = game.rng.borrow_mut();
        if plays_only {
            policy
                .plan(game, seat, &mut **rng)
                .into_iter()
                .map(|(card, lane)| Action::Play { card, lane })
                .collect()
        } else {
            policy.plan_actions(game, seat, &mut **rng)
        }
    };
    let own = own.map_or_else(|| plan(game, 0), <[Action]>::to_vec);
    let default = plan(game, 1);
    let candidates = if policy.optional_moves() && !plays_only {
        optional_candidates(game, default, config, stats)
    } else {
        candidates(game, default, plays_only, config, stats)
    };
    stats.samples.fetch_add(1, Ordering::Relaxed);
    // Only the sample's result is kept, so the opponent ranks replies win, draw,
    // loss and the first reply that wins ends the search.
    let rank = |winner: i32| match winner {
        1 => 2,
        2 => 1,
        _ => 0,
    };
    let mut best: Option<i32> = None;
    let mut policy_winner = None;
    for (index, reply) in candidates.into_iter().enumerate() {
        let mut rng = game.fork_rng();
        let mut branch = game.fork(rng.as_mut());
        match branch.resolve_action_plans([own.clone(), reply], priority) {
            Ok(_) => {}
            // The policy's own plan is what the ordinary playout would run.
            Err(error) if index == 0 => return Err(error),
            Err(_) => {
                stats.rejected.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        }
        while branch.turn <= branch.total_turns {
            if plays_only {
                branch.play_turn_plays_only(policy);
            } else {
                branch.play_turn_actions(policy)?;
            }
        }
        stats.evaluated.fetch_add(1, Ordering::Relaxed);
        let winner = branch.winner();
        policy_winner.get_or_insert(winner);
        if best.is_none_or(|top| rank(winner) > rank(top)) {
            best = Some(winner);
        }
        if best == Some(1) {
            break;
        }
    }
    let winner = best.ok_or("no opponent plan could be resolved")?;
    if policy_winner != Some(winner) {
        stats.changed.fetch_add(1, Ordering::Relaxed);
    }
    Ok(Some(winner))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        advise::advise_json,
        position::{Position, build_game},
        random::PythonRandom,
    };
    use serde_json::{Value, json};

    /// Turn 6, plain lanes. We lead lane 1 by 8 and lane 2 by 3; the opponent
    /// leads lane 0 by 5. It holds Hulk, Abomination and Misty Knight, with six
    /// energy to spend.
    fn flip_position() -> Value {
        json!({
            "turn": 6, "energy": 6,
            "hand": [{"id":"TheThing"}, {"id":"Abomination"}, {"id":"Hulk"}],
            "deck": [],
            "locations": [
                {"id":"Ruins","mine":[],"theirs":[{"id":"Cyclops"}]},
                {"id":"Ruins","mine":[{"id":"MistyKnight"},{"id":"Hulk"}],
                 "theirs":[{"id":"Shocker"},{"id":"Cyclops"}]},
                {"id":"Ruins","mine":[{"id":"MistyKnight"}],"theirs":[]}
            ],
            "previous_turn_locations": {"complete":true,"ids":[["Ruins"],["Ruins"],["Ruins"]]},
            "opponent": {"hand":3,"deck":0,"energy":6,"known":["Hulk","MistyKnight","Abomination"]}
        })
    }

    fn options(best_reply: Value) -> Value {
        json!({
            "rollouts": 1500, "position_rollouts": 200, "seed": 1, "max_plans": 40,
            "time_limit": null, "approximate": true, "playout": "spread", "workers": 1,
            "best_reply": best_reply
        })
    }

    fn run(position: &Value, options: &Value) -> Value {
        serde_json::from_str(&advise_json(&position.to_string(), &options.to_string()).unwrap())
            .unwrap()
    }

    fn plan_key(plan: &Value) -> Vec<(String, u64)> {
        let mut key: Vec<_> = plan["plays"]
            .as_array()
            .unwrap()
            .iter()
            .map(|step| {
                (
                    step["card"].as_str().unwrap().to_owned(),
                    step["location"].as_u64().unwrap(),
                )
            })
            .collect();
        key.sort();
        key
    }

    fn win_of(result: &Value, wanted: &[(&str, u64)]) -> f64 {
        let mut wanted: Vec<_> = wanted.iter().map(|&(c, l)| (c.to_owned(), l)).collect();
        wanted.sort();
        result["plans"]
            .as_array()
            .unwrap()
            .iter()
            .find(|plan| plan_key(plan) == wanted)
            .unwrap_or_else(|| panic!("plan {wanted:?} was not searched"))["win"]
            .as_f64()
            .unwrap()
    }

    // Synthetic turn-six board reproducing the slot competition: moving
    // Nightcrawler into Murderworld prevents Hulk from being played there.
    fn hulk_moves_position() -> Value {
        json!({
            "turn": 6, "total_turns": 6, "energy": 6,
            "hand": [{"id":"BlueMarvel", "entity_id":1, "power":3}],
            "deck": [],
            "locations": [
                {"id":"FrontierOutpost", "revealed_turn":1,
                 "mine":[{"id":"Squirrel","entity_id":2,"power":1},
                         {"id":"Bishop","entity_id":3,"power":6},
                         {"id":"Hawkeye","entity_id":4,"power":5},
                         {"id":"Klaw","entity_id":5,"power":3}],
                 "theirs":[{"id":"Quicksilver","entity_id":6,"power":2},
                           {"id":"Groot","entity_id":7,"power":9},
                           {"id":"Sentinel","entity_id":8,"power":3},
                           {"id":"Starlord","entity_id":9,"power":6}]},
                {"id":"Murderworld", "revealed_turn":4,
                 "mine":[{"id":"SquirrelGirl","entity_id":10,"power":2},
                         {"id":"Sentinel","entity_id":11,"power":3},
                         {"id":"AntMan","entity_id":12,"power":1}],
                 "theirs":[{"id":"Sentinel","entity_id":13,"power":3},
                           {"id":"Wolfsbane","entity_id":14,"power":4},
                           {"id":"Hawkeye","entity_id":15,"power":1}]},
                {"id":"Kyln", "revealed_turn":3,
                 "mine":[{"id":"Squirrel","entity_id":16,"power":1}],
                 "theirs":[{"id":"Nightcrawler","entity_id":17,"power":3}]}
            ],
            "previous_turn_locations":{"complete":true,
                "ids":[["FrontierOutpost"],["Murderworld"],["Kyln"]]},
            "opponent":{"hand":1,"deck":0,"energy":6,"known":["Hulk"]}
        })
    }

    #[test]
    fn optional_moves_preserve_hulk_slot_in_synthetic_position() {
        let position = hulk_moves_position();
        let mut opts = options(json!(true));
        opts["rollouts"] = json!(6000);
        opts["position_rollouts"] = json!(1000);
        opts["max_plans"] = json!(100);
        opts["best_reply_estimate"] = json!(false);
        let baseline = run(&position, &opts);
        opts["optional_moves"] = json!(true);
        let result = run(&position, &opts);
        assert!(win_of(&result, &[("BlueMarvel", 1)]) < 0.5);
        assert!(win_of(&baseline, &[("BlueMarvel", 1)]) > 0.9);
        assert!(baseline["position"]["win"].as_f64().unwrap() > 0.9);
        assert!(result["position"]["win"].as_f64().unwrap() < 0.5);
    }

    #[test]
    fn optional_reply_enumerates_keep_and_all_legal_destinations() {
        let position = hulk_moves_position();
        let position = Position::from_json(&position.to_string()).unwrap();
        let mut rng = PythonRandom::seed(1);
        let game = build_game(&position, &mut rng).unwrap();
        let nightcrawler = *game.lanes[2]
            .cards
            .iter()
            .find(|&&id| game.cards[id].owner == 1 && game.cards[id].def.id == "Nightcrawler")
            .unwrap();
        let hulk = *game.players[1]
            .hand
            .iter()
            .find(|&&id| game.cards[id].def.id == "Hulk")
            .unwrap();
        let stats = BestReplyStats::default();
        let candidates = optional_candidates(
            &game,
            vec![],
            BestReplyConfig {
                plans: 20000,
                turns: 1,
            },
            &stats,
        );
        assert!(candidates.iter().any(|actions| {
            !actions
                .iter()
                .any(|action| matches!(action, Action::Move { card, .. } if *card == nightcrawler))
                && actions.contains(&Action::Play {
                    card: hulk,
                    lane: 1,
                })
        }));
        for lane in 0..3 {
            if game.can_player_move(nightcrawler, lane, 1) {
                assert!(
                    candidates
                        .iter()
                        .any(|actions| actions.contains(&Action::Move {
                            card: nightcrawler,
                            lane
                        }))
                );
            }
        }
    }

    #[test]
    fn option_parsing() {
        let parse = |value: Value| deserialize_option(value);
        assert_eq!(parse(json!(null)).unwrap(), None);
        assert_eq!(parse(json!(false)).unwrap(), None);
        assert_eq!(
            parse(json!(true)).unwrap(),
            Some(BestReplyConfig::default())
        );
        assert_eq!(
            parse(json!({"plans": 8})).unwrap(),
            Some(BestReplyConfig { plans: 8, turns: 1 })
        );
        assert_eq!(
            parse(json!({"turns": 2})).unwrap(),
            Some(BestReplyConfig {
                plans: DEFAULT_PLANS,
                turns: 2
            })
        );
        assert!(parse(json!({"plans": 0})).is_err());
        assert!(parse(json!({"turns": 3})).is_err());
        assert!(parse(json!("yes")).is_err());
    }

    #[test]
    fn maximal_plans_cover_every_way_to_spend_the_hand() {
        let mut position = flip_position();
        position["opponent"]["known"] = json!(["MistyKnight", "Shocker"]);
        position["opponent"]["hand"] = json!(2);
        let mut rng = PythonRandom::seed(3);
        let count = |position: &Value, rng: &mut PythonRandom| {
            let position = Position::from_json(&position.to_string()).unwrap();
            let game = build_game(&position, rng).unwrap();
            let (plans, complete) = maximal_plans(&game, 1);
            assert!(complete);
            plans
        };
        // Two energy buys either card but not both: 2 cards x 3 lanes.
        position["opponent"]["energy"] = json!(2);
        let plans = count(&position, &mut rng);
        assert_eq!(plans.len(), 6);
        assert!(plans.iter().all(|plan| plan.len() == 1));
        // Three energy buys both; nothing smaller is maximal: 3 x 3 lane choices.
        position["opponent"]["energy"] = json!(3);
        let plans = count(&position, &mut rng);
        assert_eq!(plans.len(), 9);
        assert!(plans.iter().all(|plan| plan.len() == 2));
    }

    #[test]
    fn disabled_best_reply_is_the_ordinary_advice() {
        let position = flip_position();
        let default = run(&position, &options(json!(null)));
        assert_eq!(default, run(&position, &options(json!(false))));
        let mut explicitly_disabled = options(json!(null));
        explicitly_disabled["optional_moves"] = json!(false);
        assert_eq!(default, run(&position, &explicitly_disabled));
        assert!(default.get("best_reply").is_none());
        // Policy plan only: the same random draws as the ordinary playout.
        let single = run(&position, &options(json!({"plans": 1})));
        assert_eq!(single["plans"], default["plans"]);
        assert_eq!(single["position"], default["position"]);
        assert_eq!(
            single["best_reply"]["evaluated_plans"],
            single["best_reply"]["samples"]
        );
    }

    #[test]
    fn best_reply_flips_a_lane_and_moves_the_plan() {
        let position = flip_position();
        let ordinary = run(&position, &options(json!(false)));
        let reply = run(&position, &options(json!(true)));
        // Thing in lane 0 wins it by two, which the opponent's one-cost Misty Knight
        // overturns while its Abomination takes lane 2: an ordinary playout rarely
        // finds both, the best reply always does. Abomination there wins by four.
        let risky = [("TheThing", 0)];
        let safe = [("Abomination", 0)];
        assert!(win_of(&ordinary, &risky) >= 0.9);
        assert!(win_of(&reply, &risky) <= 0.1);
        assert!(win_of(&ordinary, &safe) >= 0.9);
        assert!(win_of(&reply, &safe) >= 0.9);
        // The suggestion under best reply is a plan that no reply beats.
        let top = &reply["plans"][0];
        assert!(top["win"].as_f64().unwrap() >= 0.9);
        assert_ne!(plan_key(top), vec![("TheThing".to_owned(), 0)]);
        // The position estimate drops with it (both play the policy's own plan).
        let stats = &reply["best_reply"];
        assert!(stats["changed_result_samples"].as_u64().unwrap() > 0);
        assert!(stats["evaluated_plans"].as_u64() > stats["samples"].as_u64());
        assert_eq!(stats["enumeration_cut_samples"], 0);
        assert!(
            reply["position"]["win"].as_f64().unwrap()
                < ordinary["position"]["win"].as_f64().unwrap()
        );
    }

    #[test]
    fn best_reply_can_rank_plans_while_keeping_the_ordinary_estimate() {
        let position = flip_position();
        let ordinary = run(&position, &options(json!(false)));
        let mut plans_only = options(json!(true));
        plans_only["best_reply_estimate"] = json!(false);
        let reply = run(&position, &plans_only);
        // Plans are ranked against the best reply; the board estimate is the ordinary one.
        assert!(win_of(&reply, &[("TheThing", 0)]) <= 0.1);
        assert_eq!(reply["position"], ordinary["position"]);
    }

    #[test]
    fn best_reply_only_applies_inside_its_window() {
        let mut position = flip_position();
        position["turn"] = json!(5);
        let ordinary = run(&position, &options(json!(false)));
        let outside = run(&position, &options(json!({"turns": 1})));
        assert_eq!(outside["plans"], ordinary["plans"]);
        assert_eq!(outside["position"], ordinary["position"]);
        assert_eq!(outside["best_reply"]["samples"], 0);
        let inside = run(&position, &options(json!({"turns": 2})));
        assert!(inside["best_reply"]["samples"].as_u64().unwrap() > 0);
    }

    #[test]
    fn plans_cap_bounds_the_evaluated_set() {
        let position = flip_position();
        let result = run(&position, &options(json!({"plans": 2})));
        let stats = &result["best_reply"];
        let samples = stats["samples"].as_u64().unwrap();
        assert!(stats["evaluated_plans"].as_u64().unwrap() <= 2 * samples);
        assert!(stats["sampled_down_samples"].as_u64().unwrap() > 0);
    }

    #[test]
    fn plays_only_best_reply_stays_inside_plays() {
        let position = flip_position();
        let mut plays_only = options(json!(true));
        plays_only["plays_only"] = json!(true);
        let result = run(&position, &plays_only);
        assert!(result["best_reply"]["samples"].as_u64().unwrap() > 0);
        assert!(
            result["plans"][0]["plays"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s.get("kind").is_none())
        );
    }
}
