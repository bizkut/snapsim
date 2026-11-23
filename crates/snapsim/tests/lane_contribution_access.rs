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

// Heuristic: dispatch callbacks conventionally use these local names. Generic
// ability-frame execution calls an opaque closure; its caller records the hook.
fn unrecorded_hook_calls(source: &str, ability_plumbing: bool) -> Vec<usize> {
    let production = source.split("#[cfg(test)]").next().unwrap();
    let lines: Vec<_> = production.lines().collect();
    let mut missing = Vec::new();
    let mut generic_calls = 0;
    for (index, line) in lines.iter().enumerate() {
        let is_call = ["hook(", "test(", "guard(", "repetitions(", "condition("]
            .iter()
            .any(|name| {
                line.contains(&format!("{name}self,")) || line.contains(&format!("{name}game,"))
            })
            || line.contains("(effect.apply)(game,");
        if !is_call {
            continue;
        }
        let preceding = lines[index.saturating_sub(12)..index].join("\n");
        if preceding.contains("record_card_hook(") || preceding.contains("record_location_hook(") {
            continue;
        }
        if ability_plumbing && line.trim() == "hook(self, card);" {
            generic_calls += 1;
            continue;
        }
        missing.push(index + 1);
    }
    if ability_plumbing {
        assert_eq!(generic_calls, 2, "ability-frame plumbing changed");
    }
    missing
}

#[test]
fn hook_dispatches_record_their_source() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let paths = std::iter::once(root.join("game.rs")).chain(
        std::fs::read_dir(root.join("engine_ext"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "rs")),
    );
    for path in paths {
        let plumbing = path.file_name().is_some_and(|name| name == "abilities.rs");
        let missing = unrecorded_hook_calls(&std::fs::read_to_string(&path).unwrap(), plumbing);
        assert!(
            missing.is_empty(),
            "{}: unrecorded hook calls at {missing:?}",
            path.display()
        );
    }
}

#[test]
fn hook_guard_detects_unrecorded_dispatch() {
    assert_eq!(
        unrecorded_hook_calls("if active {\n hook(game, card);\n}", false),
        [2]
    );
    assert!(
        unrecorded_hook_calls(
            "game.record_card_hook(id, \"reveal\");\nhook(game, card);",
            false
        )
        .is_empty()
    );
}
