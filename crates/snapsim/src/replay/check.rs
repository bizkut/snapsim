use super::{History, ResultRow, entity, label, matches, report, staging, turns};
use crate::{cards, locations};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

fn io_error(path: &Path, error: std::io::Error) -> String {
    if let Some(code) = error.raw_os_error() {
        let message = error.to_string();
        let message = message.split(" (os error ").next().unwrap_or(&message);
        let name = path
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('\'', "\\'");
        format!("[Errno {code}] {message}: '{name}'")
    } else {
        error.to_string()
    }
}

pub fn check_file(path: &Path, seeds: usize) -> Result<Vec<ResultRow>, String> {
    if seeds == 0 {
        return Err("--seeds must be positive".into());
    }
    let deck_path = path.with_extension("deck.json");
    let deck: Vec<String> = if deck_path.exists() {
        let text = fs::read_to_string(&deck_path).map_err(|e| io_error(&deck_path, e))?;
        let data: Value =
            serde_json::from_str(&text).map_err(|e| super::json_error::decode_error(&text, &e))?;
        let decks = data["decks"].as_array().map_or(&[][..], Vec::as_slice);
        if decks.len() == 1 {
            decks[0]["card_ids"].as_array().map_or_else(Vec::new, |v| {
                v.iter()
                    .filter_map(|s| s.as_str().map(str::to_owned))
                    .collect()
            })
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let text = fs::read_to_string(path).map_err(|e| io_error(path, e))?;
    let mut observations = Vec::new();
    for (i, line) in text.split_inclusive('\n').enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let row: Value =
            serde_json::from_str(line).map_err(|e| super::json_error::decode_error(line, &e))?;
        let observation = if row.get("envelope").is_some() {
            row["envelope"]["observation"].clone()
        } else {
            row
        };
        observations.push((i + 1, observation));
    }
    check_observations(observations, &deck, seeds)
}

/// Check line-numbered SNAP observations without reading a recording file.
pub fn check_observations(
    observations: impl IntoIterator<Item = (usize, Value)>,
    deck: &[String],
    seeds: usize,
) -> Result<Vec<ResultRow>, String> {
    if seeds == 0 {
        return Err("--seeds must be positive".into());
    }
    let mut output = Vec::new();
    for (index, frames) in matches(observations)?.iter().enumerate() {
        let mut history = History::default();
        let groups = turns(frames);
        let mut seen_locations = vec![Value::Null; 3];
        let mut noted = Vec::new();
        for (group_index, samples) in groups.iter().enumerate() {
            let turn = samples[0].turn;
            let start = samples.iter().find(|f| f.ready);
            if let Some(start) = start {
                for (lane, location) in start.locations.iter().enumerate() {
                    if *location != seen_locations[lane] && *location == "WakandanEmbassy" {
                        history.embassy_reveals += 1;
                    }
                }
                seen_locations.clone_from(&start.locations);
            }
            let next = groups.get(group_index + 1).copied().unwrap_or(&[]);
            let mut target = if !next.is_empty() && next[0].turn == turn + 1 {
                next.iter().find(|f| f.ready)
            } else {
                None
            };
            if next.is_empty()
                && samples.last().is_some_and(|f| {
                    f.terminal && !f.conceded && f.total_turns.as_i64() == Some(turn)
                })
            {
                target = samples.last();
            }
            let mut checked = None;
            let mut used_samples: Vec<_> = samples.iter().collect();
            let result = (|| -> Result<ResultRow, String> {
                let start = start.ok_or("no settled planning frame before either side played")?;
                let target = target.ok_or_else(|| {
                    if next
                        .last()
                        .or_else(|| samples.last())
                        .is_some_and(|f| f.conceded)
                    {
                        "match conceded before resolution"
                    } else {
                        "no settled next-turn or completed final frame"
                    }
                })?;
                used_samples.retain(|f| f.line >= start.line);
                let mut missing_cards = BTreeSet::new();
                let mut missing_locations = BTreeSet::new();
                for f in used_samples.iter().copied().chain(std::iter::once(target)) {
                    for card in &f.board {
                        if card.revealed
                            && card.id.as_str().is_none_or(|id| cards::get(id).is_none())
                        {
                            missing_cards.insert(label(&card.id));
                        }
                    }
                    for location in &f.locations {
                        if let Some(id) = location.as_str()
                            && locations::get(id).is_none()
                        {
                            missing_locations.insert(id.to_owned());
                        }
                    }
                }
                let mut missing = Vec::new();
                if !missing_cards.is_empty() {
                    missing.push(format!(
                        "no card rules for {}",
                        missing_cards.into_iter().collect::<Vec<_>>().join(", ")
                    ));
                }
                if !missing_locations.is_empty() {
                    missing.push(format!(
                        "no location rules for {}",
                        missing_locations.into_iter().collect::<Vec<_>>().join(", ")
                    ));
                }
                if !missing.is_empty() {
                    return Err(missing.join("; "));
                }
                if target
                    .board
                    .iter()
                    .any(|card| card.revealed && card.power.is_null())
                {
                    return Err("target has revealed cards with no recorded power".into());
                }
                for card in &start.board {
                    if !card.revealed {
                        let first = frames
                            .iter()
                            .find(|f| entity(f, &card.entity).is_some_and(|c| !c.revealed));
                        if first.is_none_or(|f| f.turn != turn - 1)
                            || !start.locations.iter().zip(&start.location_turns).any(
                                |(id, reveal)| id == "Lemuria" && reveal.as_i64() == Some(turn - 1),
                            )
                        {
                            return Err(
                                "first planning frame has face-down cards of uncertain timing"
                                    .into(),
                            );
                        }
                    } else if card.id == "Forge" {
                        return Err(
                            "pending Forge boost not observable at the start of the turn".into(),
                        );
                    }
                }
                if target.turn != turn
                    && target.board.iter().any(|card| {
                        !card.revealed
                            && (card.staged.as_i64() == Some(target.turn)
                                || !used_samples
                                    .iter()
                                    .any(|f| entity(f, &card.entity).is_some()))
                    })
                {
                    return Err("first settled next-turn frame already contains new plays".into());
                }
                let delays = start
                    .locations
                    .iter()
                    .zip(&start.location_turns)
                    .any(|(id, reveal)| id == "Lemuria" && reveal.as_i64() == Some(turn));
                let staged = staging(start, &used_samples, target, frames, delays)?;
                checked = Some((staged.plays.clone(), delays));
                let mut best = None;
                for seed in 0..seeds {
                    let diffs = super::simulate::simulate(
                        start, target, &staged, frames, &history, deck, seed,
                    )?;
                    if diffs.is_empty() {
                        return Ok(report::consistent(
                            index + 1,
                            turn,
                            staged.before.line,
                            target.line,
                            seed,
                            seeds,
                        ));
                    }
                    if best.as_ref().is_none_or(
                        |(_, previous): &(usize, Vec<super::Difference>)| {
                            diffs.len() < previous.len()
                        },
                    ) {
                        best = Some((seed, diffs));
                    }
                }
                let (seed, differences) = best.expect("positive seeds");
                let mut cards: BTreeSet<String> = differences
                    .iter()
                    .flat_map(|d| d.cards.iter().cloned())
                    .collect();
                cards.extend(
                    staged
                        .plays
                        .iter()
                        .filter_map(|p| p.id.as_str().map(str::to_owned)),
                );
                let mut lanes: BTreeSet<usize> = differences
                    .iter()
                    .flat_map(|d| d.lanes.iter().copied())
                    .collect();
                lanes.extend(staged.plays.iter().map(|p| p.card.lane));
                let locations: BTreeSet<String> = [start, target]
                    .into_iter()
                    .flat_map(|f| {
                        lanes
                            .iter()
                            .filter_map(|&lane| f.locations[lane].as_str().map(str::to_owned))
                    })
                    .collect();
                Ok(report::inconsistent(
                    index + 1,
                    turn,
                    (staged.before.line, target.line),
                    (seeds, seed),
                    differences,
                    cards.into_iter().collect(),
                    locations.into_iter().collect(),
                ))
            })();
            output.push(result.unwrap_or_else(|error| {
                report::uncheckable(
                    index + 1,
                    turn,
                    used_samples[0].line,
                    used_samples[used_samples.len() - 1].line,
                    &error,
                )
            }));
            if let Some(start) = start {
                if let Some((plays, delays)) = &checked {
                    if !delays {
                        for play in plays {
                            let lane = play.card.lane;
                            if start.locations[lane] == "OttosLab" {
                                history.lab_spent.push((
                                    lane,
                                    start.location_turns[lane].clone(),
                                    play.card.owner,
                                ));
                            }
                        }
                    }
                } else {
                    for (lane, id) in start.locations.iter().enumerate() {
                        if *id == "OttosLab" {
                            history
                                .lab_unknown
                                .push((lane, start.location_turns[lane].clone()));
                        }
                    }
                }
            }
            for frame in used_samples {
                for card in &frame.board {
                    if !card.revealed || card.id.as_str().is_none() {
                        continue;
                    }
                    let id = card.id.as_str().unwrap().to_owned();
                    if !history.seen[card.owner].contains(&id) {
                        history.seen[card.owner].push(id.clone());
                    }
                    if !noted.contains(&card.entity) && card.owner == 1 && id == "AmericaChavez" {
                        history.chavez_reveals += 1;
                    }
                    if !noted.contains(&card.entity) {
                        noted.push(card.entity.clone());
                    }
                }
            }
        }
    }
    Ok(output)
}
