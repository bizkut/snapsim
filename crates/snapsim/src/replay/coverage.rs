//! Evidence-backed per-definition replay coverage.
use super::{ResultRow, Status};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Serialize)]
struct Item {
    name: String,
    present_turns: usize,
    recovered_plays: usize,
    consistent_hook_turns: usize,
    inconsistent_closest_hook_turns: usize,
    recorded_stat_update_turns: usize,
    uncheckable_recorded_update_turns: usize,
    hook_kinds: BTreeMap<&'static str, usize>,
    uncheckable_presence_reasons: BTreeMap<String, usize>,
    uncheckable_update_reasons: BTreeMap<String, usize>,
}

/// A turn is counted once per definition, even if its hook runs repeatedly.
#[derive(Serialize)]
pub struct CoverageReport {
    description: &'static str,
    cards: BTreeMap<String, Item>,
    locations: BTreeMap<String, Item>,
}

impl Default for CoverageReport {
    fn default() -> Self {
        let cards = crate::data::cards()
            .into_iter()
            .filter(|c| c.status == "released")
            .map(|c| {
                (
                    c.carddefid,
                    Item {
                        name: c.name,
                        ..Item::default()
                    },
                )
            })
            .collect();
        let locations = crate::data::locations()
            .into_iter()
            .filter(|l| l.status == "released")
            .map(|l| {
                (
                    l.carddefid,
                    Item {
                        name: l.name,
                        ..Item::default()
                    },
                )
            })
            .collect();
        Self {
            description: "Counts are turns, except recovered_plays (card instances). Consistent hooks ran in a matching simulation; inconsistent hooks ran only in the closest trial. Uncheckable hook execution is unknown; recorded stat-update sources are separate evidence. Presence is not ability execution. Hook dispatch can be conditional and does not by itself prove a visible effect.",
            cards,
            locations,
        }
    }
}

impl CoverageReport {
    pub fn add(&mut self, rows: &[ResultRow]) {
        for row in rows {
            let Some(observed) = &row.observed else {
                continue;
            };
            let empty = BTreeSet::new();
            let (cards, locations) = row
                .hooks
                .as_ref()
                .map_or((&empty, &empty), |h| (&h.cards, &h.locations));
            let reason = row
                .detail
                .split_once(": ")
                .map_or(row.detail.as_str(), |(_, reason)| reason);
            for (items, present, updates, hooks) in [
                (
                    &mut self.cards,
                    &observed.present_cards,
                    &observed.updater_cards,
                    cards,
                ),
                (
                    &mut self.locations,
                    &observed.present_locations,
                    &observed.updater_locations,
                    locations,
                ),
            ] {
                for id in present {
                    let item = items.entry(id.clone()).or_default();
                    item.present_turns += 1;
                    if row.status == Status::Uncheckable {
                        *item
                            .uncheckable_presence_reasons
                            .entry(reason.to_owned())
                            .or_default() += 1;
                    }
                }
                for id in updates {
                    let item = items.entry(id.clone()).or_default();
                    item.recorded_stat_update_turns += 1;
                    if row.status == Status::Uncheckable {
                        item.uncheckable_recorded_update_turns += 1;
                        *item
                            .uncheckable_update_reasons
                            .entry(reason.to_owned())
                            .or_default() += 1;
                    }
                }
                let mut fired = BTreeSet::new();
                for &(id, kind) in hooks {
                    // Hidden hands and decks are sampled during replay; do not
                    // attribute their speculative definitions to the recording.
                    if !present.contains(id) && !updates.contains(id) {
                        continue;
                    }
                    let item = items.entry(id.to_owned()).or_default();
                    if row.status == Status::Consistent {
                        *item.hook_kinds.entry(kind).or_default() += 1;
                    }
                    fired.insert(id);
                }
                for id in fired {
                    let item = items.entry(id.to_owned()).or_default();
                    match row.status {
                        Status::Consistent => item.consistent_hook_turns += 1,
                        Status::Inconsistent => item.inconsistent_closest_hook_turns += 1,
                        Status::Uncheckable => {}
                    }
                }
            }
            for (id, count) in &observed.played_cards {
                self.cards.entry(id.clone()).or_default().recovered_plays += count;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ReplayHooks;
    use crate::replay::{ObservedEvidence, report};

    #[test]
    fn separates_matched_hooks_from_presence_and_recorded_updates() {
        let mut consistent = report::consistent(1, 1, 1, 2, 0, 1);
        let mut hooks = ReplayHooks::default();
        hooks.cards.insert(("Hawkeye", "on_reveal"));
        hooks.cards.insert(("Hawkeye", "on_event"));
        hooks.cards.insert(("IronMan", "on_event"));
        consistent.hooks = Some(hooks);
        let mut observed = ObservedEvidence::default();
        observed.present_cards.insert("Hawkeye".into());
        observed.played_cards.insert("Hawkeye".into(), 1);
        consistent.observed = Some(observed.clone());
        let mut uncheckable = report::uncheckable(1, 2, 3, 4, "missing history");
        observed.updater_cards.insert("Hawkeye".into());
        uncheckable.observed = Some(observed);
        let mut report = CoverageReport::default();
        report.add(&[consistent, uncheckable]);
        let card = &report.cards["Hawkeye"];
        assert_eq!(card.consistent_hook_turns, 1);
        assert_eq!(card.hook_kinds["on_reveal"], 1);
        assert_eq!(card.present_turns, 2);
        assert_eq!(card.recorded_stat_update_turns, 1);
        assert_eq!(card.uncheckable_recorded_update_turns, 1);
        assert_eq!(report.cards["IronMan"].consistent_hook_turns, 0);
        assert_eq!(card.uncheckable_update_reasons["missing history"], 1);
    }
}
