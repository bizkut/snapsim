//! Duplicate-seat match evaluation and SearchAI's public-information projection.
use crate::{
    advise::{self, AdviseOptions},
    game::{Action, Game, RetreatCost, Stakes},
    position::{self, CardRef, LocationRef, Opponent, Position},
    random::PythonRandom,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// Collision-free pairing of a signed root seed and an episode index.
pub fn episode_seed(seed: i128, index: usize, held_out: bool) -> i128 {
    let root = if seed >= 0 { 2 * seed } else { -2 * seed - 1 };
    let index = index as i128;
    let diagonal = root + index;
    diagonal * (diagonal + 1) + 2 * index + i128::from(held_out)
}

/// Wilson interval for the marginal win rate (draws are not wins).
pub fn wilson_95(rate: f64, games: usize) -> [f64; 2] {
    let z = 1.959963984540054;
    let adjustment = z * z / games as f64;
    let center = rate + adjustment / 2.0;
    let radius = z * ((rate * (1.0 - rate) + adjustment / 4.0) / games as f64).sqrt();
    [
        if rate == 0.0 {
            0.0
        } else {
            (center - radius) / (1.0 + adjustment)
        },
        if rate == 1.0 {
            1.0
        } else {
            (center + radius) / (1.0 + adjustment)
        },
    ]
}

// CPython 3.12's sum() uses a compensated summation for float inputs.
pub(crate) fn python_float_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut hi = 0.0_f64;
    let mut lo = 0.0_f64;
    for value in values {
        let next = hi + value;
        lo += if hi.abs() >= value.abs() {
            (hi - next) + value
        } else {
            (value - next) + hi
        };
        hi = next;
    }
    if lo != 0.0 && lo.is_finite() {
        hi + lo
    } else {
        hi
    }
}

/// Pair-level normal confidence interval; sort before summing as the reference does.
pub fn paired_95(scores: &[f64]) -> Value {
    let mut scores = scores.to_vec();
    scores.sort_by(f64::total_cmp);
    let count = scores.len();
    let mean = python_float_sum(scores.iter().copied()) / count as f64;
    let ci = if count < 2 {
        [0.0, 1.0]
    } else {
        let variance = python_float_sum(scores.iter().map(|score| (score - mean).powi(2)))
            / (count - 1) as f64;
        let radius = 1.959963984540054 * (variance / count as f64).sqrt();
        [(mean - radius).max(0.0), (mean + radius).min(1.0)]
    };
    json!({"score": mean, "ci95": ci, "pairs": count})
}

/// Deal one Recruit deck using the exact uniform or weighted reference algorithm.
pub fn recruit_deck(rng: &mut PythonRandom, weighted: bool) -> Vec<String> {
    let pool = position::recruit_cards();
    if !weighted {
        return rng
            .sample_indices(pool.len(), 12)
            .into_iter()
            .map(|i| pool[i].clone())
            .collect();
    }
    let weights = position::recruit_weights();
    let mut clocks: Vec<_> = pool
        .iter()
        .map(|id| ((-rng.random()).ln_1p() / weights[id], id))
        .collect();
    // heapq.nlargest compares (clock, card ID) tuples descending.
    clocks.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| b.1.cmp(a.1)));
    clocks
        .into_iter()
        .take(12)
        .map(|(_, id)| id.clone())
        .collect()
}

fn card_ref(game: &Game<'_>, id: usize, board: bool) -> CardRef {
    let card = &game.cards[id];
    CardRef {
        id: card.def.id.to_owned(),
        cost: Some(game.card_cost(id)),
        power: Some(card.power()),
        revealed: card.revealed,
        played_turn: board.then_some(card.turn_played),
        activated: (board && card.activated).then_some(true),
        player_moves_used: (board && card.player_moves_used != 0).then_some(card.player_moves_used),
        can_activate: board.then_some(card.observed_can_activate).flatten(),
        move_targets: if board {
            card.observed_move_targets
                .map(|mask| (0..3).filter(|lane| mask & (1 << lane) != 0).collect())
        } else {
            None
        },
        ..Default::default()
    }
}

/// Project only information visible to a controller in `seat`.
pub fn search_position(
    game: &Game<'_>,
    seat: usize,
    priority: Option<usize>,
    mode: Option<&str>,
    prior: Option<&str>,
) -> Position {
    let opponent = 1 - seat;
    let mut seen = BTreeSet::new();
    let locations = std::array::from_fn(|index| {
        let lane = &game.lanes[index];
        let mut mine = Vec::new();
        let mut theirs = Vec::new();
        let mut theirs_face_down = 0;
        for &id in &lane.cards {
            let card = &game.cards[id];
            if card.owner == seat {
                mine.push(card_ref(game, id, true));
            } else if card.revealed {
                theirs.push(card_ref(game, id, true));
                seen.insert(card.def.id.to_owned());
            } else {
                theirs_face_down += 1;
            }
        }
        let ottos_lab_spent = if lane.revealed && lane.def.id == "OttosLab" {
            lane.state
                .as_deref()
                .and_then(|state| state.as_any().downcast_ref::<[bool; 2]>())
                .map(|spent| [spent[seat], spent[opponent]])
        } else {
            None
        };
        LocationRef {
            id: lane.revealed.then(|| lane.def.id.to_owned()),
            mine,
            theirs,
            theirs_face_down,
            revealed_turn: if lane.revealed {
                lane.revealed_turn
            } else {
                None
            },
            ottos_lab_spent,
            history_state: None,
        }
    });
    let player = &game.players[seat];
    let other = &game.players[opponent];
    let mut deck: Vec<_> = player
        .deck
        .iter()
        .map(|&id| game.cards[id].def.id.to_owned())
        .collect();
    deck.sort();
    Position {
        turn: game.turn,
        total_turns: game.total_turns,
        energy: player.energy,
        hand: player
            .hand
            .iter()
            .map(|&id| card_ref(game, id, false))
            .collect(),
        deck,
        locations,
        opponent: Opponent {
            hand: other.hand.len(),
            deck: other.deck.len(),
            energy: None,
            seen: seen.into_iter().collect(),
            deck_list: None,
            pool: None,
            pending_power: other.next_play_power,
            prior: prior.map(str::to_owned),
            ..Default::default()
        },
        priority: priority.map(|p| if p == seat { "me" } else { "opponent" }.to_owned()),
        mode: mode.map(str::to_owned),
        pending_power: player.next_play_power,
        deck_count: None,
        snapped_this_turn: (player.snapped_turns.contains(&game.turn)
            || other.snapped_turns.contains(&game.turn))
        .then_some([
            player.snapped_turns.contains(&game.turn),
            other.snapped_turns.contains(&game.turn),
        ]),
        ..Default::default()
    }
}

/// Select a SearchAI plan, mapping advisor hand slots back to game card indices.
pub fn search_plan(
    game: &Game<'_>,
    seat: usize,
    priority: Option<usize>,
    mode: Option<&str>,
    options: &Value,
    seed: i128,
    time_limit: Option<f64>,
) -> Result<Vec<Action>, String> {
    let prior = options
        .get("prior")
        .and_then(Value::as_str)
        .filter(|s| *s != "default");
    let position = search_position(game, seat, priority, mode, prior);
    let options = AdviseOptions {
        rollouts: options
            .get("rollouts")
            .and_then(Value::as_u64)
            .unwrap_or(32) as usize,
        seed,
        max_plans: 1,
        time_limit,
        approximate: true,
        playout: options
            .get("playout")
            .and_then(Value::as_str)
            .unwrap_or("spread")
            .to_owned(),
        workers: 1,
        plays_only: options
            .get("plays_only")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    };
    let result = advise::advise_with_game(&position, &options, &|| false, Some((game, seat)))?;
    let plan = result["plans"]
        .as_array()
        .and_then(|p| p.first())
        .ok_or_else(|| {
            format!(
                "SearchAI cannot plan an unsupported position: {}",
                result["unsupported"]
            )
        })?;
    plan["plays"]
        .as_array()
        .ok_or("invalid advisor plan")?
        .iter()
        .map(|play| {
            let lane = play["location"].as_u64().ok_or("invalid advisor lane")? as usize;
            match play["kind"].as_str().unwrap_or("play") {
                "play" => {
                    let hand = play["hand_index"]
                        .as_u64()
                        .ok_or("invalid advisor hand index")?
                        as usize;
                    let card = *game.players[seat]
                        .hand
                        .get(hand)
                        .ok_or("advisor hand index out of range")?;
                    Ok(Action::Play { card, lane })
                }
                kind @ ("move" | "activate") => {
                    let from = play["from"].as_u64().ok_or("invalid board lane")? as usize;
                    let slot = play["board_index"].as_u64().ok_or("invalid board index")? as usize;
                    let card = game
                        .lanes
                        .get(from)
                        .ok_or("board lane out of range")?
                        .cards
                        .iter()
                        .copied()
                        .filter(|&id| game.cards[id].owner == seat)
                        .nth(slot)
                        .ok_or("board index out of range")?;
                    if kind == "move" {
                        Ok(Action::Move { card, lane })
                    } else {
                        Ok(Action::Activate { card })
                    }
                }
                _ => Err("invalid advisor action kind".into()),
            }
        })
        .collect()
}

fn read_pair(path: &str) -> Result<[Vec<String>; 2], String> {
    let (decks, errors) = if path == "-" {
        use std::io::Read;
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| format!("could not import decks:\n  stdin: {error}"))?;
        crate::deckcodes::parse_deck_file(&text, "stdin")
    } else {
        crate::deckcodes::read_deck_file(std::path::Path::new(path))
            .map_err(|error| format!("could not import decks:\n  {error}"))?
    };
    if !errors.is_empty() {
        return Err(format!(
            "could not import decks:\n  {}",
            errors.join("\n  ")
        ));
    }
    if decks.len() != 2 {
        return Err(format!(
            "{path}: expected exactly two deck codes, found {}",
            decks.len()
        ));
    }
    let mut decks = decks.into_iter().map(|(_, ids)| ids);
    Ok([
        decks.next().expect("two decks"),
        decks.next().expect("two decks"),
    ])
}

fn hero_deck(value: &str) -> Result<Vec<String>, String> {
    let ids: Vec<String> = if value.ends_with(".json") {
        let text = std::fs::read_to_string(value).map_err(|e| e.to_string())?;
        let data: Value =
            serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
        let entries: Vec<_> = data
            .get("decks")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                entry
                    .get("card_ids")
                    .and_then(Value::as_array)
                    .filter(|a| !a.is_empty())
            })
            .collect();
        if entries.len() != 1 {
            return Err(format!(
                "{value}: expected exactly one selected deck, found {}",
                entries.len()
            ));
        }
        entries[0]
            .iter()
            .map(|id| id.as_str().unwrap_or("").to_owned())
            .collect()
    } else {
        value.split(',').map(|id| id.trim().to_owned()).collect()
    };
    if ids.len() != 12
        || ids.iter().any(String::is_empty)
        || ids.iter().collect::<BTreeSet<_>>().len() != 12
    {
        return Err("--hero-deck must contain exactly 12 distinct card IDs".into());
    }
    crate::deckcodes::resolve_deck(&ids)?;
    Ok(ids)
}

fn input_decks(args: &Value) -> Result<[Option<Vec<String>>; 2], String> {
    if let Some(path) = args.get("deck_file").and_then(Value::as_str) {
        let [a, b] = read_pair(path)?;
        return Ok([Some(a), Some(b)]);
    }
    let hero = args
        .get("hero_deck")
        .and_then(Value::as_str)
        .map(hero_deck)
        .transpose()?;
    Ok([hero, None])
}

fn supplied_decks(args: &Value) -> Result<[Option<Vec<String>>; 2], String> {
    if let Some(values) = args.get("decks").and_then(Value::as_array) {
        if values.len() != 2 {
            return Err("expected two deck slots".into());
        }
        let mut decks = [None, None];
        for (slot, value) in values.iter().enumerate() {
            if !value.is_null() {
                let ids: Vec<String> =
                    serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
                crate::deckcodes::resolve_deck(&ids)?;
                decks[slot] = Some(ids);
            }
        }
        Ok(decks)
    } else {
        input_decks(args)
    }
}

fn run_game(
    decks: &[Vec<String>; 2],
    seed: i128,
    seat: usize,
    controllers: [&Value; 2],
    mode: Option<&str>,
    time_limit: Option<f64>,
    plays_only: bool,
) -> Result<char, String> {
    let locations = position::all_locations(mode == Some("recruit"));
    let mut rng = PythonRandom::seed(seed);
    let mut game = Game::new([&decks[0], &decks[1]], &locations, &mut rng)?;
    while game.turn <= game.total_turns {
        let mut planner =
            |game: &Game<'_>, player: usize, priority: usize| -> Result<Vec<Action>, String> {
                let side = if player == seat { 0 } else { 1 };
                let controller = controllers[side];
                let policy = match controller.get("controller").and_then(Value::as_str) {
                    Some("heuristic") => Some(crate::policy::Policy::Heuristic),
                    Some("spread") => Some(crate::policy::Policy::Spread),
                    _ => None,
                };
                if let Some(policy) = policy {
                    let mut rng = game.rng.borrow_mut();
                    if plays_only {
                        Ok(policy
                            .plan(game, player, &mut **rng)
                            .into_iter()
                            .map(|(card, lane)| Action::Play { card, lane })
                            .collect())
                    } else {
                        Ok(policy.plan_actions(game, player, &mut **rng))
                    }
                } else {
                    let mut options = controller.clone();
                    options["plays_only"] = Value::Bool(plays_only);
                    search_plan(
                        game,
                        player,
                        Some(priority),
                        mode,
                        &options,
                        seed,
                        time_limit,
                    )
                }
            };
        if plays_only {
            game.play_turn_with(|game, player, priority| {
                planner(game, player, priority)?
                    .into_iter()
                    .map(|action| match action {
                        Action::Play { card, lane } => Ok((card, lane)),
                        _ => Err("non-play action in plays-only plan".into()),
                    })
                    .collect()
            })?;
        } else {
            game.play_turn_with_actions(&mut planner)?;
        }
    }
    let winner = game.winner();
    Ok(if winner == 2 {
        'D'
    } else if winner as usize == seat {
        'W'
    } else {
        'L'
    })
}

/// Play both seats from an identical deal; a fixed Recruit hero deck follows A.
pub fn compare_pair(seed: i128, options: &Value) -> Result<[char; 2], String> {
    let mode = options.get("mode").and_then(Value::as_str);
    let mut decks = supplied_decks(options)?;
    let hero = mode == Some("recruit") && decks[0].is_some();
    if mode == Some("recruit") {
        let mut rng = PythonRandom::seed(seed);
        let weighted = options.get("deck_prior").and_then(Value::as_str) == Some("weighted");
        for deck in &mut decks {
            if deck.is_none() {
                *deck = Some(recruit_deck(&mut rng, weighted));
            }
        }
    }
    let [a, b] = decks;
    let decks = [
        a.ok_or("missing first deck")?,
        b.ok_or("missing second deck")?,
    ];
    let default = json!({});
    let controllers = [
        options.get("a").unwrap_or(&default),
        options.get("b").unwrap_or(&default),
    ];
    let time_limit = options.get("time_limit").and_then(Value::as_f64);
    let plays_only = options
        .get("plays_only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let first = run_game(&decks, seed, 0, controllers, mode, time_limit, plays_only)?;
    let second = if hero {
        run_game(
            &[decks[1].clone(), decks[0].clone()],
            seed,
            1,
            controllers,
            mode,
            time_limit,
            plays_only,
        )?
    } else {
        run_game(&decks, seed, 1, controllers, mode, time_limit, plays_only)?
    };
    Ok([first, second])
}

fn side(args: &Value, name: &str) -> Value {
    let rollouts = args
        .get(format!("{name}_rollouts"))
        .and_then(Value::as_u64)
        .or_else(|| args.get("rollouts").and_then(Value::as_u64))
        .unwrap_or(32);
    json!({"rollouts": rollouts, "playout": args.get(format!("{name}_playout")).and_then(Value::as_str).unwrap_or("spread"),
        "prior": args.get(format!("{name}_prior")).and_then(Value::as_str).unwrap_or("default"), "engine": "rust"})
}

fn counts(outcomes: [usize; 3]) -> Value {
    json!({"wins": outcomes[0], "losses": outcomes[1], "draws": outcomes[2]})
}

fn default_workers() -> usize {
    #[cfg(target_os = "macos")]
    let performance_cores = std::process::Command::new("sysctl")
        .args(["-n", "hw.perflevel0.physicalcpu"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| text.trim().parse::<usize>().ok())
        .filter(|&cores| cores != 0);
    #[cfg(not(target_os = "macos"))]
    let performance_cores: Option<usize> = None;
    (performance_cores
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
        / 2)
    .max(1)
}

/// A stake decision is made from an estimate derived from the player's visible position.
pub trait StakePolicy {
    fn decide(&self, estimate: f64, snapped: bool) -> StakeDecision;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StakeDecision {
    Stay,
    Snap,
    Retreat,
}

#[derive(Clone, Copy)]
struct ThresholdStakePolicy {
    snap: Option<f64>,
    retreat: Option<f64>,
}

impl StakePolicy for ThresholdStakePolicy {
    fn decide(&self, estimate: f64, snapped: bool) -> StakeDecision {
        if self.retreat.is_some_and(|threshold| estimate < threshold) {
            StakeDecision::Retreat
        } else if !snapped && self.snap.is_some_and(|threshold| estimate >= threshold) {
            StakeDecision::Snap
        } else {
            StakeDecision::Stay
        }
    }
}

fn stake_estimate(
    game: &Game<'_>,
    player: usize,
    mode: Option<&str>,
    rollouts: usize,
    seed: i128,
) -> Result<f64, String> {
    let mut position = search_position(game, player, None, mode, None);
    // The opponent's pending power may not be visible to this player.
    position.opponent.pending_power = 0;
    let options = AdviseOptions {
        rollouts,
        seed,
        max_plans: 1,
        time_limit: None,
        approximate: true,
        playout: "spread".into(),
        workers: 1,
        plays_only: false,
    };
    let result = advise::advise(&position, &options, &|| false)?;
    result["plans"]
        .as_array()
        .and_then(|plans| plans.first())
        .and_then(|plan| plan["win"].as_f64())
        .ok_or_else(|| "stake estimate unavailable".to_owned())
}

/// Whether snaps may raise the stake. In the owner's recordings RankedRecruit matches stayed at
/// stake 1 with no projected raise (7 matches, 702 frames, 2026-11-22), so Recruit mode forbids
/// raising unless `stake_prevent_raise` is given explicitly.
fn stake_can_raise(mode: Option<&str>, args: &Value) -> bool {
    !args
        .get("stake_prevent_raise")
        .and_then(Value::as_bool)
        .unwrap_or(mode == Some("recruit"))
}

fn stake_game(
    decks: &[Vec<String>; 2],
    seed: i128,
    a_seat: usize,
    mode: Option<&str>,
    args: &Value,
) -> Result<i64, String> {
    let mut rng = PythonRandom::seed(seed);
    let mut game = Game::new(
        [&decks[0], &decks[1]],
        &position::all_locations(mode == Some("recruit")),
        &mut rng,
    )?;
    let initial = args
        .get("stake_initial")
        .and_then(Value::as_u64)
        .unwrap_or(1) as u32;
    let maximum = args
        .get("stake_maximum")
        .and_then(Value::as_u64)
        .unwrap_or(8) as u32;
    let final_multiplier = args
        .get("stake_final_multiplier")
        .and_then(Value::as_u64)
        .unwrap_or(2) as u32;
    let rollouts = args.get("rollouts").and_then(Value::as_u64).unwrap_or(32) as usize;
    if initial == 0 || initial > maximum || rollouts == 0 {
        return Err("invalid stake configuration".into());
    }
    let initial_next = args
        .get("stake_initial_next")
        .and_then(Value::as_u64)
        .unwrap_or(u64::from(initial)) as u32;
    if initial_next > maximum {
        return Err("initial projected stake exceeds maximum".into());
    }
    game.stakes = Some(Stakes {
        current: initial,
        next: initial_next,
        maximum,
        can_raise: stake_can_raise(mode, args),
        player_multiplier: [1; 2],
        end_game_multiplier: 1,
        game_multiplier: 1,
        snapped: [false; 2],
    });
    let policies = [
        ThresholdStakePolicy {
            snap: args.get("a_snap_threshold").and_then(Value::as_f64),
            retreat: args.get("a_retreat_threshold").and_then(Value::as_f64),
        },
        ThresholdStakePolicy {
            snap: args.get("b_snap_threshold").and_then(Value::as_f64),
            retreat: args.get("b_retreat_threshold").and_then(Value::as_f64),
        },
    ];
    let mut policy_rng = PythonRandom::seed(seed.wrapping_add(0x534e4150));
    let mut retreat_result = None;
    while game.turn <= game.total_turns {
        let turn = game.turn;
        let result = game.play_turn_with_stakes(
            |game| {
                let estimates = [
                    stake_estimate(
                        game,
                        0,
                        mode,
                        rollouts,
                        seed.wrapping_add(i128::from(turn) * 2),
                    )?,
                    stake_estimate(
                        game,
                        1,
                        mode,
                        rollouts,
                        seed.wrapping_add(i128::from(turn) * 2 + 1),
                    )?,
                ];
                let stakes = game.stakes.as_ref().ok_or("missing stakes")?;
                let mut decisions = [StakeDecision::Stay; 2];
                for player in 0..2 {
                    let side = usize::from(player != a_seat);
                    decisions[player] =
                        policies[side].decide(estimates[player], stakes.snapped[player]);
                    if decisions[player] == StakeDecision::Snap
                        && (!stakes.can_raise || stakes.next >= stakes.maximum)
                    {
                        decisions[player] = StakeDecision::Stay;
                    }
                    let probability = match decisions[player] {
                        StakeDecision::Snap => args
                            .get(format!(
                                "{}_snap_probability",
                                if side == 0 { "a" } else { "b" }
                            ))
                            .and_then(Value::as_f64)
                            .unwrap_or(1.0),
                        StakeDecision::Retreat => args
                            .get(format!(
                                "{}_retreat_probability",
                                if side == 0 { "a" } else { "b" }
                            ))
                            .and_then(Value::as_f64)
                            .unwrap_or(1.0),
                        StakeDecision::Stay => 1.0,
                    };
                    if !(0.0..=1.0).contains(&probability) {
                        return Err("stake probability must be in [0,1]".into());
                    }
                    if probability < 1.0 && policy_rng.random() >= probability {
                        decisions[player] = StakeDecision::Stay;
                    }
                }
                if decisions == [StakeDecision::Retreat; 2] || decisions == [StakeDecision::Snap; 2]
                {
                    return Err("simultaneous retreat or snap is not established".into());
                }
                if let Some(player) = decisions.iter().position(|d| *d == StakeDecision::Retreat) {
                    retreat_result = Some(
                        game.stakes
                            .as_ref()
                            .unwrap()
                            .retreat_now(player, RetreatCost::DisplayedCurrent)?[a_seat],
                    );
                    return Ok(false);
                }
                if let Some(player) = decisions.iter().position(|d| *d == StakeDecision::Snap) {
                    let stakes = game.stakes.as_mut().unwrap();
                    if stakes.can_raise && stakes.next < stakes.maximum {
                        stakes.snap(player)?;
                        game.players[player].snapped_turns.push(turn);
                    }
                }
                Ok(true)
            },
            |game, player, priority| {
                search_plan(
                    game,
                    player,
                    Some(priority),
                    mode,
                    &json!({"rollouts": rollouts, "playout": "spread"}),
                    seed,
                    None,
                )
            },
        )?;
        if result.is_none() {
            return retreat_result.ok_or_else(|| "retreat result unavailable".into());
        }
    }
    let winner = game.winner();
    let stakes = game.stakes.as_mut().ok_or("missing stakes")?;
    stakes.set_end_game_multiplier(final_multiplier);
    stakes.finish_game();
    Ok(stakes.showdown((winner < 2).then_some(winner as usize))[a_seat])
}

fn compare_stakes(args: &Value) -> Result<Value, String> {
    use rayon::prelude::*;
    let mode = args.get("mode").and_then(Value::as_str);
    if mode.is_some() == args.get("deck_file").and_then(Value::as_str).is_some() {
        return Err("pass exactly one of --deck-file and --mode".into());
    }
    for key in [
        "a_snap_threshold",
        "b_snap_threshold",
        "a_retreat_threshold",
        "b_retreat_threshold",
        "a_snap_probability",
        "b_snap_probability",
        "a_retreat_probability",
        "b_retreat_probability",
    ] {
        if args
            .get(key)
            .and_then(Value::as_f64)
            .is_some_and(|v| !(0.0..=1.0).contains(&v))
        {
            return Err(format!("{key} must be in [0,1]"));
        }
    }
    if args.get("time_limit").is_some_and(|v| !v.is_null())
        || args.get("a_rollouts").is_some_and(|v| !v.is_null())
        || args.get("b_rollouts").is_some_and(|v| !v.is_null())
        || args
            .get("plays_only")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || ["a_playout", "b_playout"].iter().any(|key| {
            args.get(key)
                .and_then(Value::as_str)
                .is_some_and(|v| v != "spread")
        })
        || ["a_prior", "b_prior"].iter().any(|key| {
            args.get(key)
                .and_then(Value::as_str)
                .is_some_and(|v| v != "default")
        })
    {
        return Err(
            "stakes comparison supports equal --rollouts, spread and default prior only".into(),
        );
    }
    let decks = input_decks(args)?;
    let games = args.get("games").and_then(Value::as_u64).unwrap_or(10) as usize;
    let workers = args
        .get("workers")
        .and_then(Value::as_u64)
        .map_or_else(default_workers, |n| n as usize);
    if games == 0 || workers == 0 {
        return Err("games and workers must be positive".into());
    }
    let seed = i128::from(args.get("seed").and_then(Value::as_i64).unwrap_or(0));
    let weighted = args.get("deck_prior").and_then(Value::as_str) == Some("weighted");
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .map_err(|e| e.to_string())?;
    let pairs: Vec<f64> = pool.install(|| {
        (0..games)
            .into_par_iter()
            .map(|index| {
                let s = episode_seed(seed, index, true);
                let mut deal_rng = PythonRandom::seed(s);
                let mut deal = decks.clone();
                if mode == Some("recruit") {
                    for deck in &mut deal {
                        if deck.is_none() {
                            *deck = Some(recruit_deck(&mut deal_rng, weighted));
                        }
                    }
                }
                let [a, b] = deal;
                let a = a.ok_or("missing first deck")?;
                let b = b.ok_or("missing second deck")?;
                let first = stake_game(&[a.clone(), b.clone()], s, 0, mode, args)?;
                let second_decks = if decks[0].is_some() && mode == Some("recruit") {
                    [b, a]
                } else {
                    [a, b]
                };
                let second = stake_game(&second_decks, s, 1, mode, args)?;
                Ok((first + second) as f64 / 2.0)
            })
            .collect::<Result<_, String>>()
    })?;
    let mean = pairs.iter().sum::<f64>() / games as f64;
    let ci = if games > 1 {
        let variance = pairs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (games - 1) as f64;
        let radius = 1.959963984540054 * (variance / games as f64).sqrt();
        Some([mean - radius, mean + radius])
    } else {
        None
    };
    Ok(
        json!({"net_cubes_per_game": mean, "paired": {"score": mean, "ci95": ci, "pairs": games},
        "pair_scores": pairs, "retreat_semantics": if args.get("a_retreat_threshold").and_then(Value::as_f64).is_some() || args.get("b_retreat_threshold").and_then(Value::as_f64).is_some() { "provisional" } else { "not_used" },
        "draw_semantics": "provisional", "snap_acceptance": "provisional", "config": args}),
    )
}

/// Run the duplicate-seat compare harness with seed-ordered pair results.
pub fn compare(args: &Value) -> Result<Value, String> {
    if args.get("stakes").and_then(Value::as_bool).unwrap_or(false) {
        return compare_stakes(args);
    }
    use rayon::prelude::*;
    let mode = args.get("mode").and_then(Value::as_str);
    let deck_file = args.get("deck_file").and_then(Value::as_str);
    if mode.is_some() == deck_file.is_some() {
        return Err("pass exactly one of --deck-file and --mode".into());
    }
    if mode.is_none() && args.get("hero_deck").is_some_and(|v| !v.is_null()) {
        return Err("--hero-deck requires --mode recruit".into());
    }
    if mode.is_none() && args.get("deck_prior").is_some_and(|v| !v.is_null()) {
        return Err("--deck-prior requires --mode recruit".into());
    }
    let decks = input_decks(args)?;
    let games = args.get("games").and_then(Value::as_u64).unwrap_or(10) as usize;
    if games == 0 {
        return Err("games must be positive".into());
    }
    let seed = i128::from(args.get("seed").and_then(Value::as_i64).unwrap_or(0));
    let workers = args
        .get("workers")
        .and_then(Value::as_u64)
        .map_or_else(default_workers, |value| value as usize)
        .min(games);
    if workers == 0 {
        return Err("workers must be positive".into());
    }
    let a = side(args, "a");
    let b = side(args, "b");
    let prior = mode.map(|_| {
        args.get("deck_prior")
            .and_then(Value::as_str)
            .unwrap_or("uniform")
    });
    let time_limit = args.get("time_limit").and_then(Value::as_f64);
    let options = json!({"decks": decks, "mode": mode, "deck_prior": prior, "a": a, "b": b, "time_limit": time_limit, "plays_only": args.get("plays_only").and_then(Value::as_bool).unwrap_or(false)});
    let started = std::time::Instant::now();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .map_err(|e| e.to_string())?;
    let pairs: Vec<[char; 2]> = pool.install(|| {
        (0..games)
            .into_par_iter()
            .map(|index| compare_pair(episode_seed(seed, index, true), &options))
            .collect::<Result<_, _>>()
    })?;
    let mut seats = [[0usize; 3]; 2];
    for pair in &pairs {
        for (seat, outcome) in pair.iter().enumerate() {
            seats[seat][match outcome {
                'W' => 0,
                'L' => 1,
                _ => 2,
            }] += 1;
        }
    }
    let overall = std::array::from_fn(|i| seats[0][i] + seats[1][i]);
    let scores: Vec<_> = pairs
        .iter()
        .map(|pair| {
            pair.iter()
                .map(|c| match c {
                    'W' => 1.0,
                    'D' => 0.5,
                    _ => 0.0,
                })
                .sum::<f64>()
                / 2.0
        })
        .collect();
    let win_rate = overall[0] as f64 / (games * 2) as f64;
    Ok(json!({
        "config": {"a": a, "b": b, "games": games, "seed": seed, "workers": workers, "time_limit": time_limit,
            "mode": mode, "deck_prior": prior, "hero_deck": args.get("hero_deck"),
            "pairing": if args.get("hero_deck").is_some_and(|v| !v.is_null()) { "hero" } else { "duplicate" }, "deck_file": deck_file},
        "overall": counts(overall), "by_seat": {"1": counts(seats[0]), "2": counts(seats[1])}, "win_rate": win_rate,
        "wilson_95": wilson_95(win_rate, games * 2), "paired": paired_95(&scores), "pair_scores": scores,
        "elapsed_seconds": started.elapsed().as_secs_f64()
    }))
}

/// Run SearchAI, Spread and Heuristic against the specified benchmark opponent.
pub fn benchmark(args: &Value) -> Result<Value, String> {
    let mode = args.get("mode").and_then(Value::as_str);
    let file = args.get("deck_file").and_then(Value::as_str);
    if mode.is_some() == file.is_some() {
        return Err("pass exactly one of --deck-file and --mode".into());
    }
    let fixed = file.map(read_pair).transpose()?;
    let games = args.get("games").and_then(Value::as_u64).unwrap_or(10) as usize;
    let root = i128::from(args.get("seed").and_then(Value::as_i64).unwrap_or(0));
    let rollouts = args.get("rollouts").and_then(Value::as_u64).unwrap_or(32);
    let opponent =
        json!({"controller": args.get("opponent").and_then(Value::as_str).unwrap_or("heuristic")});
    let controllers = [
        json!({"rollouts": rollouts}),
        json!({"controller":"spread"}),
        json!({"controller":"heuristic"}),
    ];
    let mut outcomes: [Vec<char>; 3] = Default::default();
    for index in 0..games {
        let seed = episode_seed(root, index, true);
        let decks = if let Some(fixed) = &fixed {
            fixed.clone()
        } else {
            let mut rng = PythonRandom::seed(seed);
            [recruit_deck(&mut rng, false), recruit_deck(&mut rng, false)]
        };
        for seat in 0..2 {
            let seats = if seat == 0 {
                decks.clone()
            } else {
                [decks[1].clone(), decks[0].clone()]
            };
            for (outcome, controller) in outcomes.iter_mut().zip(&controllers) {
                outcome.push(run_game(
                    &seats,
                    seed,
                    seat,
                    [controller, &opponent],
                    mode,
                    None,
                    args.get("plays_only")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                )?);
            }
        }
    }
    let tally = |results: &[char]| {
        json!({"W":results.iter().filter(|&&x| x == 'W').count(),
        "L":results.iter().filter(|&&x| x == 'L').count(), "D":results.iter().filter(|&&x| x == 'D').count()})
    };
    Ok(
        json!({"SearchAI":tally(&outcomes[0]), "Spread":tally(&outcomes[1]), "Heuristic":tally(&outcomes[2])}),
    )
}

#[cfg(test)]
mod tests {
    use super::{compare_pair, episode_seed, paired_95, wilson_95};
    use serde_json::json;

    #[test]
    fn stake_comparison_is_seed_deterministic() {
        let options = json!({"stakes": true, "mode": "recruit", "games": 2,
            "workers": 1, "rollouts": 2, "seed": 19, "a_snap_threshold": 0.8});
        let first = super::compare(&options).unwrap();
        let second = super::compare(&options).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn recruit_stakes_cannot_be_raised_by_default() {
        // Recorded RankedRecruit matches never raise the stake, so a snapping policy must play
        // exactly like a never-snapping one there unless raising is explicitly allowed.
        let base = json!({"stakes": true, "mode": "recruit", "games": 2, "workers": 1,
            "rollouts": 2, "seed": 19});
        let mut snapping = base.clone();
        snapping["a_snap_threshold"] = json!(0.0);
        let never = super::compare(&base).unwrap();
        let fixed = super::compare(&snapping).unwrap();
        assert_eq!(fixed["paired"], never["paired"]);
        snapping["stake_prevent_raise"] = json!(true);
        assert_eq!(
            super::compare(&snapping).unwrap()["paired"],
            never["paired"]
        );
    }

    #[test]
    fn stake_raise_setting_defaults_by_mode_and_honours_overrides() {
        use super::stake_can_raise;
        assert!(!stake_can_raise(Some("recruit"), &json!({})));
        assert!(!stake_can_raise(
            Some("recruit"),
            &json!({"stake_prevent_raise": null})
        ));
        assert!(stake_can_raise(
            Some("recruit"),
            &json!({"stake_prevent_raise": false})
        ));
        assert!(stake_can_raise(None, &json!({})));
        assert!(!stake_can_raise(
            None,
            &json!({"stake_prevent_raise": true})
        ));
    }

    #[test]
    fn episode_seed_reserves_held_out_bit() {
        assert_eq!(episode_seed(0, 0, false), 0);
        assert_eq!(episode_seed(0, 0, true), 1);
        assert_eq!(episode_seed(-1, 0, true), 3);
        assert_eq!(episode_seed(1, 2, true), 25);
    }

    #[test]
    fn intervals_use_seed_pairs_not_individual_games() {
        assert_eq!(
            paired_95(&[0.5]),
            json!({"score": 0.5, "ci95": [0.0, 1.0], "pairs": 1})
        );
        assert_eq!(
            paired_95(&[1.0, 1.0]),
            json!({"score": 1.0, "ci95": [1.0, 1.0], "pairs": 2})
        );
        assert_eq!(wilson_95(0.0, 10)[0], 0.0);
        assert_eq!(wilson_95(1.0, 10)[1], 1.0);
    }

    #[test]
    fn interval_matches_python_compensated_sum() {
        let scores = [
            0.0, 0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 1.0, 0.5, 1.0, 0.0, 0.5, 0.75, 1.0, 0.5, 0.5, 1.0,
            1.0, 0.5, 0.5,
        ];
        assert_eq!(
            paired_95(&scores),
            json!({"score": 0.5875,
            "ci95": [0.42353691404462696, 0.751463085955373], "pairs": 20})
        );
    }

    #[test]
    fn search_stages_fuzz_pair_actions_legally() {
        let options = json!({
            "decks": [
                ["ArthurPendragon", "NicholasScratch", "MajesticWingbeat", "CaptainMarvel",
                 "ZombieGalacti", "WebSling", "Bishop", "Korg", "WerewolfByNight",
                 "Nightcrawler", "Magik", "ZombieScarletWitch"],
                ["WebSling", "DeafeningChord", "Zabu", "Surge", "Kluh", "TheHunger",
                 "Punisher", "JeffTheBabyDolphin", "KarimaShapandar", "TechnoOrganicVirus",
                 "Echo", "Medusa"]
            ],
            "a": {"rollouts": 4}, "b": {"rollouts": 4}
        });
        for seed in 0..3 {
            compare_pair(seed, &options).unwrap();
        }
    }
}
