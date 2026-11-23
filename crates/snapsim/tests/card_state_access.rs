//! Keep card hooks on the active ability's state rather than the default text's state.

fn direct_state_accesses(source: &str) -> Vec<String> {
    let production = source.split("#[cfg(test)]").next().unwrap();
    let mut compact = production
        .lines()
        .map(|line| line.split("//").next().unwrap())
        .collect::<String>();
    compact.retain(|c| !c.is_whitespace());
    // PendingEffect owns its own state. Nico preparation replaces the Default text,
    // not the state of the ability that happened to request preparation.
    compact = compact.replace("effect.state", "");
    compact = compact.replace(
        "self.cards[card].state=def.initial_state.map(|init|init());",
        "",
    );
    compact
        .match_indices(".state")
        .map(|(index, _)| format!("byte {index}"))
        .collect()
}

#[test]
fn card_hooks_use_ability_state_accessors() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let source = std::fs::read_to_string(&path).unwrap();
            let accesses = direct_state_accesses(&source);
            assert!(accesses.is_empty(), "{}: {accesses:?}", path.display());
        }
    }
}

#[test]
fn state_access_guard_detects_reads_and_writes() {
    assert_eq!(
        direct_state_accesses("fn hook() { game.cards[id].state.as_ref(); }").len(),
        1
    );
    assert_eq!(
        direct_state_accesses("fn hook() { card\n .state = None; }").len(),
        1
    );
    assert!(
        direct_state_accesses("fn hook() { effect.state.as_any(); card.ability_state(); }")
            .is_empty()
    );
}
