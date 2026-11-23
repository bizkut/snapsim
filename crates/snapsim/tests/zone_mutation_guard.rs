//! Prevent new production zone-vector mutations outside zones.rs.
use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn production_zone_mutations_use_helpers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = vec![
        root.join("game.rs"),
        root.join("position.rs"),
        root.join("replay/simulate.rs"),
    ];
    for dir in ["cards", "locations", "engine_ext"] {
        files.extend(
            fs::read_dir(root.join(dir))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "rs")),
        );
    }
    files.sort();
    let mut offenders = Vec::<(PathBuf, usize, String)>::new();
    for path in files {
        let text = fs::read_to_string(&path).unwrap();
        let mut lines = Vec::new();
        let mut pending_test = false;
        let mut depth = 0isize;
        for line in text.lines() {
            let trimmed = line.trim();
            if depth > 0 {
                depth += line.chars().filter(|&c| c == '{').count() as isize;
                depth -= line.chars().filter(|&c| c == '}').count() as isize;
                lines.push("");
                continue;
            }
            if pending_test && trimmed.starts_with("mod ") && trimmed.contains('{') {
                depth = line.chars().filter(|&c| c == '{').count() as isize
                    - line.chars().filter(|&c| c == '}').count() as isize;
                pending_test = false;
                lines.push("");
                continue;
            }
            pending_test = trimmed == "#[cfg(test)]";
            lines.push(line);
        }
        for (i, line) in lines.iter().enumerate() {
            let compact: String = lines[i..lines.len().min(i + 3)]
                .iter()
                .flat_map(|s| s.chars())
                .filter(|c| !c.is_whitespace())
                .collect();
            let write = [
                "hand",
                "deck",
                "discard",
                "graveyard",
                "banished",
                "set_aside",
            ]
            .iter()
            .any(|zone| {
                [
                    "push", "insert", "remove", "retain", "clear", "pop", "swap", "extend",
                    "truncate",
                ]
                .iter()
                .any(|method| compact.contains(&format!(".{zone}.{method}(")))
            });
            let lane_write = compact.find(".lanes[").is_some_and(|start| {
                let rest = &compact[start..];
                rest.find("].cards").is_some_and(|end| {
                    let operation = &rest[end + "].cards".len()..];
                    operation.starts_with('=') && !operation.starts_with("==")
                        || [
                            ".push(",
                            ".insert(",
                            ".remove(",
                            ".retain(",
                            ".clear(",
                            ".pop(",
                            ".swap(",
                            ".swap_remove(",
                            ".extend(",
                            ".truncate(",
                            ".drain(",
                            ".append(",
                        ]
                        .iter()
                        .any(|method| operation.starts_with(method))
                })
            });
            let bulk = (compact.contains("mem::take(") || compact.contains("mem::replace("))
                && ([
                    ".hand",
                    ".deck",
                    ".discard",
                    ".graveyard",
                    ".banished",
                    ".set_aside",
                ]
                .iter()
                .any(|zone| compact.contains(zone))
                    || (compact.contains(".lanes[") && compact.contains("].cards")));
            if write || lane_write || bulk {
                offenders.push((path.clone(), i + 1, line.trim().to_owned()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "direct zone-vector mutations outside zones.rs: {offenders:#?}"
    );
}
