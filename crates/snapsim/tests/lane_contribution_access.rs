//! Lane score effects must retain their ability provenance.

fn direct_writes(source: &str) -> Vec<String> {
    let production = source.split("#[cfg(test)]").next().unwrap();
    let mut compact = production
        .lines()
        .map(|line| line.split("//").next().unwrap())
        .collect::<String>();
    compact.retain(|c| !c.is_whitespace());
    let mut writes = Vec::new();
    for field in ["].bonus", "].multiplier"] {
        for (index, _) in compact.match_indices(field) {
            let mut tail = &compact[index + field.len()..];
            if tail.starts_with('[')
                && let Some(end) = tail.find(']')
            {
                tail = &tail[end + 1..];
            }
            if tail.starts_with("+=")
                || tail.starts_with("*=")
                || tail.starts_with('=') && !tail.starts_with("==")
            {
                writes.push(format!("{field} at byte {index}"));
            }
        }
    }
    writes
}

#[test]
fn card_and_location_hooks_use_lane_contribution_methods() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for directory in ["cards", "locations"] {
        for entry in std::fs::read_dir(root.join(directory)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|extension| extension == "rs") {
                let writes = direct_writes(&std::fs::read_to_string(&path).unwrap());
                assert!(writes.is_empty(), "{}: {writes:?}", path.display());
            }
        }
    }
}

#[test]
fn guard_detects_bonus_and_multiplier_writes() {
    assert_eq!(direct_writes("game.lanes[lane].bonus[seat] += 3;").len(), 1);
    assert_eq!(
        direct_writes("g.lanes[lane].multiplier[seat] *= 2;").len(),
        1
    );
    assert!(direct_writes("game.lanes[lane].bonus[seat] == 3;").is_empty());
}
