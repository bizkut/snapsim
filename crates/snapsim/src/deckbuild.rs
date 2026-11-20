//! Owned-deck search with common random numbers for paired comparisons.
use crate::{cards, compare, deckcodes, random::PythonRandom};
use rayon::prelude::*;
use serde_json::{Value, json};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::Instant;

type Deck = Vec<String>;
#[derive(Default)]
struct Candidate {
    source: Cow<'static, str>,
    original: Option<Deck>,
}
fn ids(v: &Value) -> Deck {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}
fn entry_ids(v: &Value) -> &Value {
    if v.is_object() { &v["card_ids"] } else { v }
}
fn entry(v: &Value) -> Deck {
    ids(entry_ids(v))
}

fn saved_frequencies(saved: &[Value]) -> HashMap<&str, usize> {
    let mut frequencies = HashMap::new();
    let mut seen = HashSet::new();
    for deck in saved {
        seen.clear();
        for id in entry_ids(deck)
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if seen.insert(id) {
                *frequencies.entry(id).or_default() += 1;
            }
        }
    }
    frequencies
}

fn fill_from_saved(d: &mut Deck, pool: &BTreeSet<String>, frequencies: &HashMap<&str, usize>) {
    let missing = 12 - d.len();
    if missing == 0 {
        return;
    }
    let mut rest: Vec<_> = pool.iter().filter(|c| !d.contains(c)).collect();
    rest.sort_unstable_by_key(|c| {
        (
            std::cmp::Reverse(frequencies.get(c.as_str()).copied().unwrap_or(0)),
            *c,
        )
    });
    d.extend(rest.into_iter().take(missing).cloned());
}
fn name(v: &Value, default: &str) -> String {
    v.get("name")
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_owned()
}
fn normalized(mut d: Deck) -> Deck {
    d.sort();
    d
}
fn unique(d: &[String]) -> bool {
    d.iter().collect::<HashSet<_>>().len() == d.len()
}
fn subset(d: &[String], s: &BTreeSet<String>) -> bool {
    d.iter().all(|x| s.contains(x))
}
fn score(v: &[f64], difference: bool) -> (f64, Vec<f64>) {
    if v.is_empty() {
        return if difference {
            (0., vec![-1., 1.])
        } else {
            (0.5, vec![0., 1.])
        };
    }
    if !difference {
        let result = compare::paired_95(v);
        return (
            result["score"].as_f64().unwrap(),
            result["ci95"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect(),
        );
    }
    let mean = compare::python_float_sum(v.iter().copied()) / v.len() as f64;
    if v.len() < 2 {
        return (
            mean,
            if difference {
                vec![-1., 1.]
            } else {
                vec![0., 1.]
            },
        );
    }
    let radius = 1.959963984540054
        * (compare::python_float_sum(v.iter().map(|x| (x - mean).powi(2)))
            / (v.len() - 1) as f64
            / v.len() as f64)
            .sqrt();
    (
        mean,
        vec![
            (mean - radius).max(if difference { -1. } else { 0. }),
            (mean + radius).min(1.),
        ],
    )
}
fn ranked(mut keys: Vec<Deck>, samples: &HashMap<Deck, impl AsRef<[f64]>>) -> Vec<Deck> {
    let mean = |key: &Deck| {
        let values = samples.get(key).map(AsRef::as_ref).unwrap_or(&[]);
        if values.is_empty() {
            0.5
        } else {
            values.iter().sum::<f64>() / values.len() as f64
        }
    };
    keys.sort_by(|a, b| mean(b).total_cmp(&mean(a)).then_with(|| a.cmp(b)));
    keys
}
fn diverse(keys: Vec<Deck>, count: usize) -> Vec<Deck> {
    let mut out: Vec<Deck> = Vec::new();
    for k in keys {
        if out
            .iter()
            .all(|o| k.iter().filter(|c| !o.contains(c)).count() >= 3)
        {
            out.push(k);
            if out.len() == count {
                break;
            }
        }
    }
    out
}
fn swaps(deck: &Deck, eligible: &BTreeSet<String>, locked: &[String]) -> Vec<Deck> {
    let mut out = Vec::new();
    for old in deck.iter().filter(|c| !locked.contains(c)) {
        for new in eligible {
            if !deck.contains(new) {
                let mut d = deck.clone();
                d.retain(|c| c != old);
                d.push(new.clone());
                out.push(normalized(d));
            }
        }
    }
    out
}
fn append_unique(out: &mut Vec<Deck>, keys: impl IntoIterator<Item = Deck>) {
    for k in keys {
        if !out.contains(&k) {
            out.push(k);
        }
    }
}
fn changed(a: &Deck, b: &Deck) -> Value {
    let mut removed = b.iter().filter(|x| !a.contains(x)).collect::<Vec<_>>();
    removed.sort();
    json!({"in":a.iter().filter(|x| !b.contains(x)).collect::<Vec<_>>(),"out":removed})
}
/// Completions in result order: confirmed decks by their confirmation games, then the
/// rest by screening. The two use different rollouts, so their scores are not comparable.
fn completion_order(
    confirm: &HashMap<Deck, Vec<f64>>,
    screen: &HashMap<Deck, Vec<f64>>,
    count: usize,
) -> Vec<Deck> {
    let scored = |samples: &HashMap<Deck, Vec<f64>>| -> Vec<Deck> {
        samples
            .iter()
            .filter(|(_, scores)| !scores.is_empty())
            .map(|(key, _)| key.clone())
            .collect()
    };
    let mut ordered = ranked(scored(confirm), confirm);
    let screened: Vec<_> = ranked(scored(screen), screen)
        .into_iter()
        .filter(|k| !ordered.contains(k))
        .collect();
    ordered.extend(screened);
    ordered.truncate(count);
    ordered
}

#[derive(Clone, Copy)]
enum SearchMode {
    Build,
    Complete,
    TryCards,
}

struct Round<'a> {
    stage: &'a str,
    index: usize,
    rollouts: usize,
    fraction: f64,
    mode: SearchMode,
    count: usize,
}
impl Round<'_> {
    fn screen(index: usize, mode: SearchMode, count: usize) -> Self {
        Self {
            stage: "screen",
            index,
            rollouts: 32,
            fraction: 0.55,
            mode,
            count,
        }
    }
    fn confirm(index: usize, rollouts: usize, mode: SearchMode, count: usize) -> Self {
        Self {
            stage: "confirm",
            index,
            rollouts,
            fraction: 1.,
            mode,
            count,
        }
    }
}
struct Search<'a> {
    started: Instant,
    budget: Option<f64>,
    workers: usize,
    pool: rayon::ThreadPool,
    seed: i128,
    plays_only: bool,
    games: usize,
    progress: &'a mut dyn FnMut(Value),
    cancel: &'a dyn Fn() -> bool,
}
impl Search<'_> {
    fn expired(&self, fraction: f64) -> bool {
        (self.cancel)()
            || self
                .budget
                .is_some_and(|b| self.started.elapsed().as_secs_f64() >= b * fraction)
    }
    fn play(
        &mut self,
        keys: &[Deck],
        samples: &mut HashMap<Deck, Vec<f64>>,
        round: Round<'_>,
    ) -> Result<bool, String> {
        let Round {
            stage,
            index,
            rollouts,
            fraction,
            mode,
            count,
        } = round;
        let try_cards = matches!(mode, SearchMode::TryCards);
        if self.expired(fraction) {
            return Ok(false);
        }
        let deal = compare::episode_seed(
            self.seed + if stage == "confirm" { 1000003 } else { 0 },
            index,
            true,
        );
        let side = json!({"rollouts":rollouts,"playout":"margin","prior":"default"});
        // Bounded batches preserve input order and deterministic progress/cancellation.
        for (batch, chunk) in keys.chunks(self.workers).enumerate() {
            if self.expired(fraction) {
                return Ok(false);
            }
            let results = self.pool.install(|| {
                chunk.par_iter().map(|deck| {
                    let opts = json!({"decks":[deck,Value::Null],"mode":"recruit","deck_prior":"weighted","a":side,"b":side,"time_limit":null,"plays_only":self.plays_only});
                    compare::compare_pair(deal,&opts).map(|pair| pair.iter().map(|c| match c {'W'=>1.,'D'=>0.5,_=>0.}).sum::<f64>()/2.)
                }).collect::<Result<Vec<_>, _>>()
            })?;
            for (offset, (key, value)) in chunk.iter().zip(results).enumerate() {
                if self.expired(fraction) {
                    return Ok(false);
                }
                samples.entry(key.clone()).or_default().push(value);
                self.games += 2;
                let done = batch * self.workers + offset + 1;
                if try_cards || done == 1 || done == keys.len() || done.is_multiple_of(16) {
                    let candidates = if !try_cards {
                        samples.keys().cloned().collect()
                    } else {
                        keys.iter()
                            .filter(|k| samples.get(*k).is_some_and(|v| !v.is_empty()))
                            .cloned()
                            .collect()
                    };
                    let best = ranked(candidates, samples);
                    let best = match mode {
                        SearchMode::Build => diverse(best, count),
                        SearchMode::Complete | SearchMode::TryCards => {
                            let mut best = best;
                            best.truncate(if try_cards { 3 } else { count });
                            best
                        }
                    };
                    (self.progress)(
                        json!({"type":"progress","stage":stage,"done":done,"total":keys.len(),"elapsed":self.started.elapsed().as_secs_f64(),"best":best.iter().map(|k| json!({"card_ids":k,"score":score(&samples[k],false).0})).collect::<Vec<_>>() }),
                    );
                }
            }
        }
        Ok(true)
    }
}

/// Search for owned decks, try new cards, or complete a deck with fixed cards.
pub fn deckbuild(
    args: &Value,
    progress: &mut dyn FnMut(Value),
    cancel: &dyn Fn() -> bool,
) -> Result<Value, String> {
    let started = Instant::now();
    let mode = args["mode"].as_str().unwrap_or("recruit");
    if mode != "recruit" {
        return Err("Only recruit mode is supported".into());
    }
    let workers = args["workers"]
        .as_u64()
        .map(|n| n as usize)
        .unwrap_or_else(|| {
            let cores = std::process::Command::new("sysctl")
                .args(["-n", "hw.perflevel0.physicalcpu"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse::<usize>().ok())
                .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from));
            (cores / 2).clamp(1, 3)
        });
    let rollouts = args["confirm_rollouts"].as_u64().unwrap_or(200) as usize;
    let count = args["count"].as_u64().unwrap_or(3) as usize;
    if !args["complete"].is_null()
        && ["cards", "base", "baseline"]
            .into_iter()
            .any(|key| !args[key].is_null())
    {
        return Err(
            "--complete cannot be combined with --try-cards, --base-deck or --selected".into(),
        );
    }
    if workers == 0 || rollouts == 0 || (args["cards"].is_null() && count == 0) {
        return Err(if args["cards"].is_null() {
            "count, workers and confirm_rollouts must be positive"
        } else {
            "workers and confirm_rollouts must be positive"
        }
        .into());
    }
    let budget = args["time_budget"].as_f64();
    if budget.is_some_and(|x| x <= 0.) {
        return Err("time_budget must be positive".into());
    }
    let seed = args["seed"].as_i64().unwrap_or(0) as i128;
    let owned = if args["owned"].is_object() {
        &args["owned"]["cards"]
    } else {
        &args["owned"]
    };
    let all: BTreeSet<String> = owned
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| {
            if v.is_object() {
                v["card_id"].as_str()
            } else {
                v.as_str()
            }
        })
        .map(str::to_owned)
        .collect();
    let eligible_all: BTreeSet<String> = cards::deck_eligible_ids()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut eligible: BTreeSet<String> = all.intersection(&eligible_all).cloned().collect();
    let complete = if args["complete"].is_null() {
        None
    } else {
        let input = &args["complete"];
        let deck = ids(input);
        if !(1..=11).contains(&deck.len())
            || input.as_array().is_none_or(|a| a.len() != deck.len())
            || !unique(&deck)
        {
            return Err("A deck to complete must have 1 to 11 different cards".into());
        }
        let mut unsupported: Vec<_> = deck
            .iter()
            .filter(|id| !eligible_all.contains(*id))
            .map(String::as_str)
            .collect();
        if !unsupported.is_empty() {
            unsupported.sort_unstable();
            return Err(format!(
                "Cards without simulator rules: {}",
                unsupported.join(", ")
            ));
        }
        eligible.retain(|id| !deck.contains(id));
        if eligible.len() < 12 - deck.len() {
            return Err("Not enough owned cards with simulator rules to complete the deck".into());
        }
        Some(deck)
    };
    let mut search = Search {
        started,
        budget,
        workers,
        pool: rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .map_err(|e| e.to_string())?,
        seed,
        plays_only: args["plays_only"].as_bool().unwrap_or(false),
        games: 0,
        progress,
        cancel,
    };
    if !args["cards"].is_null() {
        return try_cards(args, &all, &eligible, &mut search, rollouts);
    }
    if complete.is_none() && eligible.len() < 12 {
        return Err("At least 12 distinct supported owned cards are required".into());
    }
    let saved = if args["saved"].is_object() {
        &args["saved"]["decks"]
    } else {
        &args["saved"]
    };
    let saved = saved.as_array().map(Vec::as_slice).unwrap_or_default();
    let frequencies = saved_frequencies(saved);
    let baseline = if args["baseline"].get("decks").is_some() {
        args["baseline"]["decks"]
            .as_array()
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or(Value::Null)
    } else {
        args["baseline"].clone()
    };
    let base = entry(&baseline);
    if !base.is_empty() && (base.len() != 12 || !unique(&base) || !subset(&base, &eligible_all)) {
        return Err("Baseline must contain 12 distinct supported cards".into());
    }
    let mut candidates: BTreeMap<Deck, Candidate> = BTreeMap::new();
    let mut order = Vec::new();
    fn add(
        map: &mut BTreeMap<Deck, Candidate>,
        order: &mut Vec<Deck>,
        d: Deck,
        source: Cow<'static, str>,
        original: Option<Deck>,
    ) {
        let d = normalized(d);
        if d.len() == 12 && unique(&d) && !map.contains_key(&d) {
            order.push(d.clone());
            map.insert(d, Candidate { source, original });
        }
    }
    if let Some(deck) = &complete {
        let mut d = deck.clone();
        fill_from_saved(&mut d, &eligible, &frequencies);
        add(&mut candidates, &mut order, d, "completion".into(), None);
    } else {
        for e in saved {
            let original = entry(e);
            let mut d = Vec::new();
            for c in &original {
                if eligible.contains(c) && !d.contains(c) && d.len() < 12 {
                    d.push(c.clone());
                }
            }
            fill_from_saved(&mut d, &eligible, &frequencies);
            add(
                &mut candidates,
                &mut order,
                d,
                name(e, "saved").into(),
                Some(original),
            );
        }
        if !base.is_empty() && subset(&base, &eligible) {
            add(
                &mut candidates,
                &mut order,
                base.clone(),
                name(&baseline, "baseline").into(),
                Some(base.clone()),
            );
        }
    }
    let locked = complete.as_deref().unwrap_or_default();
    let search_mode = if complete.is_some() {
        SearchMode::Complete
    } else {
        SearchMode::Build
    };
    let list: Vec<_> = eligible.iter().collect();
    let mut rng = PythonRandom::seed(seed);
    for i in 0..4 {
        let mut d = Vec::with_capacity(12);
        d.extend_from_slice(locked);
        d.extend(
            rng.sample_indices(list.len(), 12 - locked.len())
                .into_iter()
                .map(|i| list[i].clone()),
        );
        add(
            &mut candidates,
            &mut order,
            d,
            if complete.is_some() {
                "completion".into()
            } else {
                format!("random {}", i + 1).into()
            },
            None,
        );
    }
    let starts = order.clone();
    let mut neighbours = BTreeMap::new();
    for d in &starts {
        let variants = swaps(d, &eligible, locked);
        for v in &variants {
            let c = &candidates[d];
            let source = c.source.clone();
            let original = if complete.is_some() {
                None
            } else {
                c.original.clone().or_else(|| Some(d.clone()))
            };
            add(&mut candidates, &mut order, v.clone(), source, original);
        }
        neighbours.insert(d.clone(), variants);
    }
    let mut screen_keys = order;
    rng.shuffle(&mut screen_keys);
    let mut screen = HashMap::new();
    let mut confirm = HashMap::new();
    let base_key = normalized(base.clone());
    let mut finalists;
    if !search.expired(1.) {
        for i in 0..2 {
            if !search.play(
                &screen_keys,
                &mut screen,
                Round::screen(i, search_mode, count),
            )? {
                break;
            }
        }
        let mut survivors = BTreeSet::new();
        for d in &starts {
            let observed = neighbours[d]
                .iter()
                .filter(|k| screen.contains_key(*k))
                .cloned()
                .collect();
            survivors.extend(ranked(observed, &screen).into_iter().take(24));
            if screen.contains_key(d) {
                survivors.insert(d.clone());
            }
        }
        let observed: Vec<_> = screen_keys
            .iter()
            .filter(|k| screen.contains_key(*k))
            .cloned()
            .collect();
        survivors.extend(ranked(observed.clone(), &screen).into_iter().take(20));
        let active: Vec<_> = survivors.iter().cloned().collect();
        for i in 2..16 {
            let ready: Vec<_> = active
                .iter()
                .filter(|k| screen.get(*k).map_or(0, Vec::len) == i)
                .cloned()
                .collect();
            if !search.play(&ready, &mut screen, Round::screen(i, search_mode, count))? {
                break;
            }
        }
        let mut promoted = BTreeSet::new();
        for d in &starts {
            let tested = neighbours[d]
                .iter()
                .filter(|k| survivors.contains(*k) && screen.contains_key(*k))
                .cloned()
                .collect();
            if let Some(k) = ranked(tested, &screen).first() {
                promoted.insert(k.clone());
            }
            if survivors.contains(d) && screen.contains_key(d) {
                promoted.insert(d.clone());
            }
        }
        promoted.extend(
            ranked(
                active
                    .into_iter()
                    .filter(|k| screen.contains_key(k))
                    .collect(),
                &screen,
            )
            .into_iter()
            .take(10),
        );
        let active: Vec<_> = promoted.into_iter().collect();
        for i in 16..128 {
            let ready: Vec<_> = active
                .iter()
                .filter(|k| screen.get(*k).map_or(0, Vec::len) == i)
                .cloned()
                .collect();
            if !search.play(&ready, &mut screen, Round::screen(i, search_mode, count))? {
                break;
            }
        }
        finalists = ranked(
            active
                .into_iter()
                .filter(|k| screen.contains_key(k))
                .collect(),
            &screen,
        );
        if finalists.is_empty() {
            finalists = ranked(observed, &screen);
        }
        if finalists.is_empty() {
            finalists = screen_keys.clone();
        }
        if !base.is_empty() && !candidates.contains_key(&base_key) {
            candidates.insert(
                base_key.clone(),
                Candidate {
                    source: "baseline".into(),
                    original: Some(base.clone()),
                },
            );
        }
        let mut keys = finalists.clone();
        if !base.is_empty() {
            append_unique(&mut keys, [base_key.clone()]);
        }
        for i in 0..300 {
            if !search.play(
                &keys,
                &mut confirm,
                Round::confirm(i, rollouts, search_mode, count),
            )? {
                break;
            }
        }
    } else {
        finalists = if complete.is_some() {
            Vec::new()
        } else {
            diverse(screen_keys.clone(), count)
        };
    }
    let stopped = search.expired(1.);
    if complete.is_none() {
        finalists.retain(|k| subset(k, &eligible));
    }
    let mut combined = HashMap::new();
    for k in candidates.keys() {
        combined.insert(
            k.clone(),
            confirm
                .get(k)
                .filter(|v| !v.is_empty())
                .or_else(|| screen.get(k))
                .map(Vec::as_slice)
                .unwrap_or_default(),
        );
    }
    finalists = if complete.is_some() {
        if screen.is_empty() && confirm.is_empty() {
            // As in build mode, keep valid candidates if interrupted before the first score.
            screen_keys.into_iter().take(count).collect()
        } else {
            completion_order(&confirm, &screen, count)
        }
    } else {
        let mut ordered = ranked(finalists, &combined);
        let fallback: Vec<_> = screen_keys
            .into_iter()
            .filter(|k| !ordered.contains(k))
            .collect();
        ordered.extend(fallback);
        diverse(ordered, count)
    };
    let bs = confirm.get(&base_key).map(Vec::as_slice).unwrap_or(&[]);
    let mut decks = Vec::new();
    for k in finalists {
        let c = &candidates[&k];
        let cs = confirm.get(&k).map(Vec::as_slice).unwrap_or(&[]);
        let scores = if cs.is_empty() {
            screen.get(&k).map(Vec::as_slice).unwrap_or(&[])
        } else {
            cs
        };
        let (s, ci) = score(scores, false);
        let original = complete.as_ref().or(c.original.as_ref());
        let changes = original.map(|o| changed(&k, o));
        let mut deck = json!({"card_ids":k,"deck_code":deckcodes::encode_deck(&k)?,"score":s,"ci95":ci,"games":2*scores.len(),"stage":if cs.is_empty(){"screened"}else{"confirmed"},"started_from":c.source,"changes":changes});
        if complete.is_none() {
            let n = cs.len().min(bs.len());
            deck["versus_baseline"] = if n == 0 {
                Value::Null
            } else {
                let d: Vec<_> = cs.iter().zip(bs).take(n).map(|(a, b)| a - b).collect();
                let (v, ci) = score(&d, true);
                json!({"difference":v,"ci95":ci})
            };
        }
        decks.push(deck);
    }
    let mut result = json!({"status":if stopped {"cancelled"}else{"ok"},"config":{"mode":mode,"count":count,"time_budget":budget,"workers":workers,"seed":seed,"screen_rollouts":32,"confirm_rollouts":rollouts,"engine":"rust"},"decks":decks,"evaluated_decks":combined.values().filter(|v|!v.is_empty()).count(),"games_played":search.games,"elapsed_seconds":started.elapsed().as_secs_f64()});
    if let Some(deck) = complete {
        result["mode"] = json!("complete");
        result["deck"] = json!({"card_ids":deck});
    } else {
        result["owned"] = json!({"supported":eligible,"unsupported":all.difference(&eligible).collect::<Vec<_>>()});
        result["baseline"] = if base.is_empty() {
            Value::Null
        } else {
            let (bscore, bci) = score(bs, false);
            json!({"name":name(&baseline,"baseline"),"card_ids":base,"score":bscore,"ci95":bci})
        };
    }
    Ok(result)
}

fn try_cards(
    args: &Value,
    owned: &BTreeSet<String>,
    eligible: &BTreeSet<String>,
    search: &mut Search<'_>,
    rollouts: usize,
) -> Result<Value, String> {
    let base = if args["base"].get("decks").is_some() {
        args["base"]["decks"]
            .as_array()
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or(Value::Null)
    } else {
        args["base"].clone()
    };
    let base_ids = entry(&base);
    if base_ids.len() != 12 || !unique(&base_ids) || !subset(&base_ids, eligible) {
        return Err("Base deck must contain 12 distinct owned, supported cards".into());
    }
    let base_key = normalized(base_ids.clone());
    let input = if let Some(s) = args["cards"].as_str() {
        s.split(',').map(|s| s.trim().to_owned()).collect()
    } else {
        ids(&args["cards"])
    };
    let mut cards = Vec::new();
    for c in input {
        if !cards.contains(&c) {
            cards.push(c);
        }
    }
    if cards.is_empty() || cards.iter().any(String::is_empty) {
        return Err("Provide at least one card ID".into());
    }
    let unowned: Vec<_> = cards
        .iter()
        .filter(|c| !owned.contains(*c))
        .cloned()
        .collect();
    if !unowned.is_empty() {
        let mut u = unowned;
        u.sort();
        return Err(format!("Cards not owned: {}", u.join(", ")));
    }
    if cards.iter().any(|c| base_ids.contains(c)) {
        return Err("Cards to try must not already be in the base deck".into());
    }
    let supported: Vec<_> = cards
        .iter()
        .filter(|c| eligible.contains(*c))
        .cloned()
        .collect();
    let mut placements = BTreeMap::new();
    let mut samples = HashMap::new();
    let mut chosen = BTreeMap::new();
    for card in &supported {
        let decks: Vec<_> = base_key
            .iter()
            .map(|old| {
                let mut d = base_key.clone();
                d.retain(|x| x != old);
                d.push(card.clone());
                normalized(d)
            })
            .collect();
        for d in &decks {
            samples.insert(d.clone(), Vec::new());
        }
        chosen.insert(card.clone(), decks[0].clone());
        placements.insert(card.clone(), decks);
    }
    let mut confirm = HashMap::new();
    if !supported.is_empty() && !search.expired(1.) {
        let mut all = Vec::new();
        for c in &supported {
            append_unique(&mut all, placements[c].clone());
        }
        for i in 0..16 {
            if !search.play(
                &all,
                &mut samples,
                Round::screen(i, SearchMode::TryCards, 3),
            )? {
                break;
            }
        }
        let mut partners = BTreeMap::new();
        for c in &supported {
            let parents: Vec<_> = ranked(placements[c].clone(), &samples)
                .into_iter()
                .take(2)
                .collect();
            let mut variants: BTreeSet<_> = parents.iter().cloned().collect();
            for p in &parents {
                for old in p {
                    if old != c {
                        for new in eligible {
                            if !p.contains(new) {
                                let mut d = p.clone();
                                d.retain(|x| x != old);
                                d.push(new.clone());
                                variants.insert(normalized(d));
                            }
                        }
                    }
                }
            }
            let variants: Vec<_> = variants.into_iter().collect();
            for d in &variants {
                samples.entry(d.clone()).or_default();
            }
            partners.insert(c.clone(), variants);
        }
        let mut all = Vec::new();
        for c in &supported {
            append_unique(&mut all, partners[c].clone());
        }
        for i in 0..4 {
            let missing: Vec<_> = all
                .iter()
                .filter(|k| samples[*k].len() == i)
                .cloned()
                .collect();
            if !search.play(
                &missing,
                &mut samples,
                Round::screen(i, SearchMode::TryCards, 3),
            )? {
                break;
            }
        }
        let mut survivors = BTreeMap::new();
        for c in &supported {
            let screened: Vec<_> = partners[c]
                .iter()
                .filter(|k| samples[*k].len() >= 4)
                .cloned()
                .collect();
            let s = if screened.is_empty() {
                placements[c].clone()
            } else {
                ranked(screened, &samples).into_iter().take(24).collect()
            };
            survivors.insert(c.clone(), s);
        }
        let mut all = Vec::new();
        for c in &supported {
            append_unique(&mut all, survivors[c].clone());
        }
        for i in 4..16 {
            let missing: Vec<_> = all
                .iter()
                .filter(|k| samples[*k].len() == i)
                .cloned()
                .collect();
            if !search.play(
                &missing,
                &mut samples,
                Round::screen(i, SearchMode::TryCards, 3),
            )? {
                break;
            }
        }
        for c in &supported {
            let mut scored: Vec<_> = survivors[c]
                .iter()
                .filter(|k| !samples[*k].is_empty())
                .cloned()
                .collect();
            if !scored.is_empty() {
                let common = scored.iter().map(|k| samples[k].len()).min().unwrap();
                scored.sort_by(|a, b| {
                    score(&samples[b][..common], false)
                        .0
                        .total_cmp(&score(&samples[a][..common], false).0)
                        .then_with(|| a.cmp(b))
                });
                chosen.insert(c.clone(), scored[0].clone());
            }
        }
        let mut keys = vec![base_key.clone()];
        for c in &supported {
            append_unique(&mut keys, [chosen[c].clone()]);
        }
        for i in 0..300 {
            if !search.play(
                &keys,
                &mut confirm,
                Round::confirm(i, rollouts, SearchMode::TryCards, 3),
            )? {
                break;
            }
        }
    }
    let stopped = search.expired(1.);
    let bs = confirm.get(&base_key).map(Vec::as_slice).unwrap_or(&[]);
    let (base_score, base_ci) = score(bs, false);
    let mut results = Vec::new();
    for c in cards {
        if !eligible.contains(&c) {
            results
                .push(json!({"card_id":c,"supported":false,"verdict":"unsupported","best":null}));
            continue;
        }
        let key = &chosen[&c];
        let cs = confirm.get(key).map(Vec::as_slice).unwrap_or(&[]);
        let values = if cs.is_empty() {
            samples[key].as_slice()
        } else {
            cs
        };
        let (s, ci) = score(values, false);
        let n = cs.len().min(bs.len());
        let delta: Vec<_> = cs.iter().zip(bs).take(n).map(|(a, b)| a - b).collect();
        let (d, dci) = score(&delta, true);
        let changes = changed(key, &base_key);
        results.push(json!({"card_id":c,"supported":true,"verdict":if dci[0]>0. {"better"}else{"not better"},"best":{"card_ids":key,"in":changes["in"],"out":changes["out"],"deck_code":deckcodes::encode_deck(key)?,"score":s,"ci95":ci,"versus_base":{"difference":d,"ci95":dci}}}));
    }
    Ok(
        json!({"status":if stopped{"cancelled"}else{"ok"},"mode":"try-cards","config":{"mode":"recruit","time_budget":search.budget,"workers":search.workers,"seed":search.seed,"screen_rollouts":32,"confirm_rollouts":rollouts,"engine":"rust"},"base":{"card_ids":base_ids,"deck_code":deckcodes::encode_deck(&base_ids)?,"score":base_score,"ci95":base_ci},"cards":results,"games_played":search.games,"elapsed_seconds":search.started.elapsed().as_secs_f64()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    const FIXED: [&str; 11] = [
        "Ultron",
        "BlueMarvel",
        "IronMan",
        "JessicaJones",
        "KaZar",
        "Bishop",
        "Wolfsbane",
        "AntMan",
        "Hawkeye",
        "SquirrelGirl",
        "Nightcrawler",
    ];
    const POOL: [&str; 6] = [
        "Abomination",
        "Angela",
        "CaptainAmerica",
        "Hulk",
        "MistyKnight",
        "Shocker",
    ];

    #[test]
    fn completions_keep_fixed_cards_and_cover_every_card_for_one_slot() {
        for (fixed, pool) in [(&FIXED[..], &POOL[..]), (&FIXED[..10], &POOL[..3])] {
            // Most of the fixed cards are not owned, and must still be kept.
            let owned: Vec<_> = pool
                .iter()
                .copied()
                .chain([fixed[0], "Unsupported"])
                .collect();
            let args = json!({
                "complete":fixed, "owned":owned, "count":pool.len(),
                "workers":1, "confirm_rollouts":1, "seed":42,
            });
            let done = Cell::new(false);
            let result = deckbuild(
                &args,
                &mut |event| {
                    if event["stage"] == "screen" && event["done"] == event["total"] {
                        assert_eq!(event["total"], pool.len());
                        assert_eq!(event["best"].as_array().unwrap().len(), pool.len());
                        done.set(true);
                    }
                },
                &|| done.get(),
            )
            .unwrap();
            assert_eq!(result["mode"], "complete");
            assert_eq!(result["status"], "cancelled");
            assert_eq!(result["deck"]["card_ids"], json!(fixed));
            assert_eq!(result["evaluated_decks"], pool.len());
            assert_eq!(result["games_played"], pool.len() * 2);
            assert!(result.get("baseline").is_none());

            let mut seen = BTreeSet::new();
            let decks = result["decks"].as_array().unwrap();
            assert_eq!(decks.len(), pool.len());
            let mut previous_score = 1.;
            for deck in decks {
                let ids = ids(&deck["card_ids"]);
                assert_eq!(ids.len(), 12);
                assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
                for card in fixed {
                    assert!(ids.iter().any(|id| id == card));
                }
                let added: Vec<_> = ids
                    .iter()
                    .map(String::as_str)
                    .filter(|id| !fixed.contains(id))
                    .collect();
                assert!(added.iter().all(|id| pool.contains(id)));
                assert_eq!(deck["changes"], json!({"in":added, "out":[]}));
                assert_eq!(deck["started_from"], "completion");
                assert_eq!(deck["stage"], "screened");
                assert_eq!(deck["games"], 2);
                assert_eq!(deck["deck_code"], deckcodes::encode_deck(&ids).unwrap());
                assert!(deck.get("versus_baseline").is_none());
                let score = deck["score"].as_f64().unwrap();
                assert!(score <= previous_score);
                previous_score = score;
                seen.extend(added.into_iter().map(str::to_owned));
            }
            assert_eq!(seen, pool.iter().map(ToString::to_string).collect());
        }
    }

    #[test]
    fn completion_fill_counts_saved_decks_not_duplicate_cards() {
        let mut deck = FIXED[..10].iter().map(ToString::to_string).collect();
        let pool = POOL.iter().map(ToString::to_string).collect();
        let saved = [
            json!({"card_ids":["Angela", "Hulk", "Hulk"]}),
            json!(["Angela", "Abomination"]),
        ];
        fill_from_saved(&mut deck, &pool, &saved_frequencies(&saved));
        assert_eq!(&deck[10..], ["Angela", "Abomination"]);
    }

    #[test]
    fn completion_rejects_invalid_cards_or_pool() {
        let too_many: Vec<_> = FIXED.into_iter().chain(["Angela"]).collect();
        let invalid_deck = "A deck to complete must have 1 to 11 different cards";
        for (deck, expected) in [
            (json!([]), invalid_deck),
            (json!(too_many), invalid_deck),
            (json!(["Ultron", "Ultron"]), invalid_deck),
            (json!(["Ultron", 7]), invalid_deck),
            (
                json!(["UnsupportedZ", "UnsupportedA"]),
                "Cards without simulator rules: UnsupportedA, UnsupportedZ",
            ),
            (
                json!(FIXED),
                "Not enough owned cards with simulator rules to complete the deck",
            ),
        ] {
            let args = json!({"complete":deck, "owned":[FIXED[0], "Unsupported"], "workers":1});
            assert_eq!(
                deckbuild(&args, &mut |_| {}, &|| false).unwrap_err(),
                expected
            );
        }
    }

    #[test]
    fn completion_rejects_incompatible_inputs() {
        for key in ["cards", "base", "baseline"] {
            let mut args = json!({"complete":FIXED, "workers":1});
            args[key] = json!([]);
            assert_eq!(
                deckbuild(&args, &mut |_| {}, &|| false).unwrap_err(),
                "--complete cannot be combined with --try-cards, --base-deck or --selected"
            );
        }
    }

    #[test]
    fn confirmed_completions_rank_ahead_of_lucky_screens() {
        let deck = |card: &str| vec![card.to_owned()];
        let confirm = HashMap::from([(deck("Confirmed"), vec![0.6, 0.6])]);
        let screen = HashMap::from([
            (deck("Confirmed"), vec![0.5; 16]),
            (deck("Lucky"), vec![1.0, 1.0]),
            (deck("Steady"), vec![0.7; 16]),
        ]);
        assert_eq!(
            completion_order(&confirm, &screen, 3),
            [deck("Confirmed"), deck("Lucky"), deck("Steady")]
        );
        assert_eq!(completion_order(&confirm, &screen, 1), [deck("Confirmed")]);
    }
}
