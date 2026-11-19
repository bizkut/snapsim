//! Duplicate-seat match evaluation and SearchAI's public-information projection.
use crate::{
    advise::{self, AdviseOptions},
    game::Game,
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
) -> Result<Vec<(usize, usize)>, String> {
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
    };
    let result = advise::advise(&position, &options, &|| false)?;
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
            let hand = play["hand_index"]
                .as_u64()
                .ok_or("invalid advisor hand index")? as usize;
            let lane = play["location"].as_u64().ok_or("invalid advisor lane")? as usize;
            let card = *game.players[seat]
                .hand
                .get(hand)
                .ok_or("advisor hand index out of range")?;
            Ok((card, lane))
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
) -> Result<char, String> {
    let locations = position::all_locations(mode == Some("recruit"));
    let mut rng = PythonRandom::seed(seed);
    let mut game = Game::new([&decks[0], &decks[1]], &locations, &mut rng)?;
    while game.turn <= game.total_turns {
        game.play_turn_with(|game, player, priority| {
            let side = if player == seat { 0 } else { 1 };
            let controller = controllers[side];
            match controller.get("controller").and_then(Value::as_str) {
                Some("heuristic") => Ok(crate::policy::Policy::Heuristic.plan(
                    game,
                    player,
                    &mut **game.rng.borrow_mut(),
                )),
                Some("spread") => Ok(crate::policy::Policy::Spread.plan(
                    game,
                    player,
                    &mut **game.rng.borrow_mut(),
                )),
                _ => search_plan(
                    game,
                    player,
                    Some(priority),
                    mode,
                    controller,
                    seed,
                    time_limit,
                ),
            }
        })?;
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
    let first = run_game(&decks, seed, 0, controllers, mode, time_limit)?;
    let second = if hero {
        run_game(
            &[decks[1].clone(), decks[0].clone()],
            seed,
            1,
            controllers,
            mode,
            time_limit,
        )?
    } else {
        run_game(&decks, seed, 1, controllers, mode, time_limit)?
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

/// Run the duplicate-seat compare harness with seed-ordered pair results.
pub fn compare(args: &Value) -> Result<Value, String> {
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
    let options = json!({"decks": decks, "mode": mode, "deck_prior": prior, "a": a, "b": b, "time_limit": time_limit});
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
    use super::{episode_seed, paired_95, wilson_95};
    use serde_json::json;

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
}
