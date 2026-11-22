use clap::{Args, Parser, Subcommand};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Read},
    sync::atomic::{AtomicBool, Ordering},
};

static CANCELLED: AtomicBool = AtomicBool::new(false);
fn cli_workers() -> usize {
    let cores = std::process::Command::new("sysctl")
        .args(["-n", "hw.perflevel0.physicalcpu"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from));
    cores.saturating_sub(1).max(1)
}
#[derive(Parser)]
#[command(name = "snapsim")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Deckbuild(Deckbuild),
    Position(Position),
    Coverage,
    Compare(Compare),
    Benchmark(Benchmark),
}
#[derive(Args)]
struct Deckbuild {
    #[arg(long)]
    collection: String,
    #[arg(long)]
    decks: Option<String>,
    #[arg(long)]
    selected: Option<String>,
    #[arg(long)]
    base_deck: Option<String>,
    #[arg(long)]
    try_cards: Option<String>,
    /// Comma-separated card IDs to keep when completing an incomplete deck.
    #[arg(long, conflicts_with_all = ["try_cards", "base_deck", "selected"])]
    complete: Option<String>,
    #[arg(long, default_value_t = 3)]
    count: usize,
    #[arg(long)]
    time_budget: Option<f64>,
    #[arg(long)]
    workers: Option<usize>,
    #[arg(long, default_value_t = 0)]
    seed: i64,
    #[arg(long, default_value_t = 200)]
    confirm_rollouts: usize,
    #[arg(long)]
    output: Option<String>,
    #[arg(long)]
    progress: bool,
    #[arg(long)]
    plays_only: bool,
}
#[derive(Args)]
struct Position {
    #[arg(long, default_value = "-")]
    file: String,
    #[arg(long)]
    snap: bool,
    #[arg(long)]
    deck_code: Option<String>,
    #[arg(long)]
    snap_deck: Option<String>,
    #[arg(long)]
    mode: Option<String>,
    #[arg(long, default_value_t = 200)]
    rollouts: usize,
    #[arg(long, default_value_t = 0)]
    seed: i64,
    #[arg(long)]
    time_limit: Option<f64>,
    #[arg(long, default_value_t = 5)]
    max_plans: usize,
    #[arg(long)]
    approximate: bool,
    #[arg(long)]
    plays_only: bool,
}
#[derive(Args)]
struct Compare {
    #[arg(long)]
    deck_file: Option<String>,
    #[arg(long)]
    mode: Option<String>,
    #[arg(long)]
    hero_deck: Option<String>,
    #[arg(long)]
    deck_prior: Option<String>,
    #[arg(long, default_value_t = 10)]
    games: usize,
    #[arg(long, default_value_t = 0)]
    seed: i64,
    #[arg(long)]
    workers: Option<usize>,
    #[arg(long, default_value_t = 32)]
    rollouts: usize,
    #[arg(long)]
    time_limit: Option<f64>,
    #[arg(long)]
    a_rollouts: Option<usize>,
    #[arg(long, default_value = "spread")]
    a_playout: String,
    #[arg(long, default_value = "default")]
    a_prior: String,
    #[arg(long)]
    b_rollouts: Option<usize>,
    #[arg(long, default_value = "spread")]
    b_playout: String,
    #[arg(long, default_value = "default")]
    b_prior: String,
    #[arg(long)]
    stakes: bool,
    #[arg(long)]
    a_snap_threshold: Option<f64>,
    #[arg(long)]
    b_snap_threshold: Option<f64>,
    #[arg(long)]
    a_retreat_threshold: Option<f64>,
    #[arg(long)]
    b_retreat_threshold: Option<f64>,
    #[arg(long, default_value_t = 1.0)]
    a_snap_probability: f64,
    #[arg(long, default_value_t = 1.0)]
    b_snap_probability: f64,
    #[arg(long, default_value_t = 1.0)]
    a_retreat_probability: f64,
    #[arg(long, default_value_t = 1.0)]
    b_retreat_probability: f64,
    #[arg(long, default_value_t = 1)]
    stake_initial: u32,
    #[arg(long)]
    stake_initial_next: Option<u32>,
    /// `true` forbids snapping, `false` allows it; default: forbidden in Recruit mode, allowed otherwise.
    #[arg(long)]
    stake_prevent_raise: Option<bool>,
    #[arg(long, default_value_t = 8)]
    stake_maximum: u32,
    #[arg(long, default_value_t = 2)]
    stake_final_multiplier: u32,
    #[arg(long)]
    plays_only: bool,
}
#[derive(Args)]
struct Benchmark {
    #[arg(long)]
    deck_file: Option<String>,
    #[arg(long)]
    mode: Option<String>,
    #[arg(long, default_value_t = 10)]
    games: usize,
    #[arg(long)]
    seed: Option<i64>,
    #[arg(long, default_value_t = 32)]
    rollouts: usize,
    #[arg(long, default_value = "heuristic")]
    opponent: String,
    #[arg(long)]
    plays_only: bool,
}
fn read(path: &str) -> Result<Value, String> {
    let s = if path == "-" {
        let mut s = String::new();
        io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| e.to_string())?;
        s
    } else {
        fs::read_to_string(path).map_err(|e| e.to_string())?
    };
    serde_json::from_str(s.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())
}
fn selected(path: &str) -> Result<Value, String> {
    let v = read(path)?;
    let entries: Vec<_> = v["decks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| e["card_ids"].as_array().filter(|a| !a.is_empty()))
        .collect();
    if entries.len() != 1 {
        return Err(format!(
            "{path}: expected exactly one selected deck, found {}",
            entries.len()
        ));
    }
    Ok(json!(entries[0]))
}
fn run(cli: Cli) -> Result<(), String> {
    let result = match cli.command {
        Command::Coverage => snapsim::coverage::supported(),
        Command::Deckbuild(a) => {
            if a.base_deck.is_some() != a.try_cards.is_some() {
                return Err("--base-deck and --try-cards must be supplied together".into());
            }
            ctrlc::set_handler(|| CANCELLED.store(true, Ordering::Relaxed))
                .map_err(|e| e.to_string())?;
            let base = match a.base_deck {
                Some(s) if s.contains(',') => {
                    json!(s.split(',').map(str::trim).collect::<Vec<_>>())
                }
                Some(s) => read(&s)?,
                None => Value::Null,
            };
            let complete = a.complete.as_deref().map(|ids| {
                if ids.trim().is_empty() {
                    Vec::new()
                } else {
                    ids.split(',').map(str::trim).collect::<Vec<_>>()
                }
            });
            let args = json!({"owned":read(&a.collection)?,"saved":match a.decks {Some(s)=>read(&s)?,None=>Value::Null},"baseline":match a.selected {Some(s)=>read(&s)?,None=>Value::Null},"base":base,"cards":a.try_cards,"complete":complete,"count":a.count,"time_budget":a.time_budget,"workers":a.workers.unwrap_or_else(cli_workers),"seed":a.seed,"confirm_rollouts":a.confirm_rollouts,"plays_only":a.plays_only});
            let mut progress = |v: Value| {
                if a.progress {
                    println!("{v}");
                }
            };
            let mut result = snapsim::deckbuild::deckbuild(&args, &mut progress, &|| {
                CANCELLED.load(Ordering::Relaxed)
            })?;
            if let Some(path) = a.output {
                fs::write(
                    path,
                    format!("{}\n", serde_json::to_string_pretty(&result).unwrap()),
                )
                .map_err(|e| e.to_string())?;
            }
            if a.progress {
                result["type"] = json!("result");
            }
            result
        }
        Command::Position(a) => {
            if !a.snap && (a.deck_code.is_some() || a.snap_deck.is_some()) {
                return Err("--deck-code and --snap-deck require --snap".into());
            }
            if a.deck_code.is_some() && a.snap_deck.is_some() {
                return Err("pass only one of --deck-code and --snap-deck".into());
            }
            let mut data = read(&a.file)?;
            if a.snap {
                let deck = if let Some(code) = a.deck_code {
                    json!(snapsim::deckcodes::parse_deck(&code)?)
                } else if let Some(path) = a.snap_deck {
                    selected(&path)?
                } else {
                    Value::Null
                };
                data = snapsim::snap::position_from_snap(
                    &data,
                    &json!({"deck":deck,"mode":a.mode,"plays_only":a.plays_only}),
                )
                .map_err(|e| format!("{e:?}"))?;
            } else if let Some(mode) = a.mode {
                data["mode"] = json!(mode);
            }
            let p = snapsim::position::Position::from_json(&data.to_string())?;
            let options = snapsim::advise::AdviseOptions {
                rollouts: a.rollouts,
                seed: a.seed as i128,
                max_plans: a.max_plans,
                time_limit: a.time_limit,
                approximate: a.approximate,
                playout: "spread".into(),
                workers: 1,
                plays_only: a.plays_only,
            };
            snapsim::advise::advise(&p, &options, &|| false)?
        }
        Command::Compare(a) => snapsim::compare::compare(
            &json!({"deck_file":a.deck_file,"mode":a.mode,"hero_deck":a.hero_deck,"deck_prior":a.deck_prior,"games":a.games,"seed":a.seed,"workers":a.workers,"rollouts":a.rollouts,"time_limit":a.time_limit,"a_rollouts":a.a_rollouts,"a_playout":a.a_playout,"a_prior":a.a_prior,"b_rollouts":a.b_rollouts,"b_playout":a.b_playout,"b_prior":a.b_prior,"plays_only":a.plays_only,"stakes":a.stakes,"a_snap_threshold":a.a_snap_threshold,"b_snap_threshold":a.b_snap_threshold,"a_retreat_threshold":a.a_retreat_threshold,"b_retreat_threshold":a.b_retreat_threshold,"a_snap_probability":a.a_snap_probability,"b_snap_probability":a.b_snap_probability,"a_retreat_probability":a.a_retreat_probability,"b_retreat_probability":a.b_retreat_probability,"stake_initial":a.stake_initial,"stake_initial_next":a.stake_initial_next,"stake_prevent_raise":a.stake_prevent_raise,"stake_maximum":a.stake_maximum,"stake_final_multiplier":a.stake_final_multiplier}),
        )?,
        Command::Benchmark(a) => snapsim::compare::benchmark(
            &json!({"deck_file":a.deck_file,"mode":a.mode,"games":a.games,"seed":a.seed,"rollouts":a.rollouts,"opponent":a.opponent,"plays_only":a.plays_only}),
        )?,
    };
    println!("{result}");
    Ok(())
}
fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
}
