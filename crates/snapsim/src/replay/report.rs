//! Human-readable replaycheck verdicts and cross-file summaries.
use super::{Difference, ResultRow, Status};
use std::collections::BTreeMap;
use std::io::{self, Write};

/// A seed reproduced every visible outcome on this turn.
pub fn consistent(
    match_number: usize,
    turn: i64,
    before_line: usize,
    target_line: usize,
    seed: usize,
    seeds: usize,
) -> ResultRow {
    ResultRow {
        match_number,
        turn,
        status: Status::Consistent,
        detail: format!(
            "lines {before_line}→{target_line}; seed {seed} (matched after {}/{seeds} attempts)",
            seed + 1
        ),
        differences: Vec::new(),
        cards: Vec::new(),
        locations: Vec::new(),
    }
}

/// No tested seed reproduced all visible outcomes.
pub fn inconsistent(
    match_number: usize,
    turn: i64,
    lines: (usize, usize),
    seed_search: (usize, usize),
    differences: Vec<Difference>,
    cards: Vec<String>,
    locations: Vec<String>,
) -> ResultRow {
    let (before_line, target_line) = lines;
    let (seeds, closest_seed) = seed_search;
    ResultRow {
        match_number,
        turn,
        status: Status::Inconsistent,
        detail: format!(
            "lines {before_line}→{target_line}; no match in {seeds} seeds (closest seed {closest_seed})"
        ),
        differences,
        cards,
        locations,
    }
}

/// The recording lacks enough public information to check this turn.
pub fn uncheckable(
    match_number: usize,
    turn: i64,
    first_line: usize,
    last_line: usize,
    reason: &str,
) -> ResultRow {
    ResultRow {
        match_number,
        turn,
        status: Status::Uncheckable,
        detail: format!("lines {first_line}–{last_line}: {reason}"),
        differences: Vec::new(),
        cards: Vec::new(),
        locations: Vec::new(),
    }
}

fn label(status: &Status) -> &'static str {
    match status {
        Status::Consistent => "consistent",
        Status::Inconsistent => "inconsistent",
        Status::Uncheckable => "not checkable",
    }
}

#[derive(Default)]
struct Counts {
    consistent: usize,
    inconsistent: usize,
    uncheckable: usize,
}
impl Counts {
    fn add(&mut self, status: &Status) {
        match status {
            Status::Consistent => self.consistent += 1,
            Status::Inconsistent => self.inconsistent += 1,
            Status::Uncheckable => self.uncheckable += 1,
        }
    }
}

impl std::fmt::Display for Counts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} consistent, {} inconsistent, {} not checkable",
            self.consistent, self.inconsistent, self.uncheckable
        )
    }
}

/// Accumulate per-file verdicts and write the exact replaycheck text format.
#[derive(Default)]
pub struct Report {
    counts: Counts,
    cards: BTreeMap<String, usize>,
    locations: BTreeMap<String, usize>,
}
impl Report {
    pub fn file(
        &mut self,
        output: &mut impl Write,
        path: &str,
        rows: &[ResultRow],
    ) -> io::Result<()> {
        writeln!(output, "{path}")?;
        let mut file_counts = Counts::default();
        for row in rows {
            file_counts.add(&row.status);
            self.counts.add(&row.status);
            writeln!(
                output,
                "  match {} turn {}: {} — {}",
                row.match_number,
                row.turn,
                label(&row.status),
                row.detail
            )?;
            for difference in &row.differences {
                writeln!(output, "    {}", difference.item)?;
            }
            for (values, names) in [
                (&mut self.cards, &row.cards),
                (&mut self.locations, &row.locations),
            ] {
                for name in names {
                    if let Some(count) = values.get_mut(name) {
                        *count += 1;
                    } else {
                        values.insert(name.clone(), 1);
                    }
                }
            }
        }
        writeln!(output, "  {file_counts}")
    }

    /// Print overall counts and descending-frequency, then alphabetic, suspects.
    pub fn finish(&self, output: &mut impl Write) -> io::Result<()> {
        writeln!(output, "Overall: {}", self.counts)?;
        for (name, values) in [("cards", &self.cards), ("locations", &self.locations)] {
            let mut ranked: Vec<_> = values.iter().collect();
            ranked.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
            write!(output, "Suspect {name}: ")?;
            if ranked.is_empty() {
                write!(output, "none")?;
            }
            for (index, (id, count)) in ranked.iter().enumerate() {
                if index != 0 {
                    write!(output, ", ")?;
                }
                write!(output, "{id} ({count})")?;
            }
            writeln!(output)?;
        }
        Ok(())
    }

    pub fn has_inconsistent(&self) -> bool {
        self.counts.inconsistent != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdicts_and_suspects_match_reference_format() {
        let rows = vec![
            consistent(1, 1, 2, 4, 3, 128),
            uncheckable(1, 2, 5, 8, "missing frame"),
            inconsistent(
                1,
                3,
                (9, 15),
                (128, 42),
                vec![Difference {
                    item: "lane 1 differs".into(),
                    cards: vec![],
                    lanes: vec![],
                }],
                vec!["Zola".into(), "Angel".into()],
                vec!["Xandar".into()],
            ),
        ];
        let mut report = Report::default();
        let mut output = Vec::new();
        report.file(&mut output, "example.jsonl", &rows).unwrap();
        report.file(&mut output, "empty.jsonl", &[]).unwrap();
        report.finish(&mut output).unwrap();
        assert!(report.has_inconsistent());
        assert_eq!(
            String::from_utf8(output).unwrap(),
            concat!(
                "example.jsonl\n",
                "  match 1 turn 1: consistent — lines 2→4; seed 3 (matched after 4/128 attempts)\n",
                "  match 1 turn 2: not checkable — lines 5–8: missing frame\n",
                "  match 1 turn 3: inconsistent — lines 9→15; no match in 128 seeds (closest seed 42)\n",
                "    lane 1 differs\n",
                "  1 consistent, 1 inconsistent, 1 not checkable\n",
                "empty.jsonl\n",
                "  0 consistent, 0 inconsistent, 0 not checkable\n",
                "Overall: 1 consistent, 1 inconsistent, 1 not checkable\n",
                "Suspect cards: Angel (1), Zola (1)\n",
                "Suspect locations: Xandar (1)\n",
            )
        );
    }
}
