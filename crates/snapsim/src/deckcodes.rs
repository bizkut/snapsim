//! Marvel SNAP clipboard deck codes and deck-file inputs.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;
use std::sync::LazyLock;

const DECK_SIZE: usize = 12;

static CATALOG: LazyLock<HashSet<String>> = LazyLock::new(|| {
    crate::data::cards()
        .into_iter()
        .map(|c| c.carddefid)
        .collect()
});

static COMPACT_IDS: LazyLock<HashMap<String, Vec<&'static str>>> = LazyLock::new(|| {
    let mut matches: HashMap<String, Vec<&'static str>> = HashMap::new();
    for id in catalog() {
        matches.entry(compact(id)).or_default().push(id);
    }
    for ids in matches.values_mut() {
        ids.sort_unstable();
    }
    matches
});

fn catalog() -> &'static HashSet<String> {
    &CATALOG
}

fn python_space(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{1c}'..='\u{1f}')
}

// str.splitlines() recognizes more separators than Rust's lines(), and counts
// CRLF as one boundary. Preserve that behavior for comments and line labels.
fn python_lines(text: &str) -> impl Iterator<Item = &str> {
    let mut remaining = text;
    std::iter::from_fn(move || {
        if remaining.is_empty() {
            return None;
        }
        let end = remaining.char_indices().find(|(_, c)| {
            matches!(
                *c,
                '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{1c}'
                    ..='\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}'
            )
        });
        if let Some((index, separator)) = end {
            let line = &remaining[..index];
            let mut next = index + separator.len_utf8();
            if separator == '\r' && remaining[next..].starts_with('\n') {
                next += 1;
            }
            remaining = &remaining[next..];
            Some(line)
        } else {
            let line = remaining;
            remaining = "";
            Some(line)
        }
    })
}

fn compact(id: &str) -> String {
    format!(
        "{}{:X}",
        id.chars()
            .filter(|c| !"aeiouy".contains(*c))
            .collect::<String>(),
        id.chars().count()
    )
}

// Like Python's object_pairs_hook, reject duplicate fields in nested objects too.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON value")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(Value::Bool(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(v.into())))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(v.into())))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
                Ok(Unique(
                    Number::from_f64(v).map_or(Value::Null, Value::Number),
                ))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(Value::String(v.to_owned())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(Value::String(v)))
            }
            fn visit_none<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element::<Unique>()? {
                    v.push(x.0);
                }
                Ok(Unique(Value::Array(v)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut v = Map::new();
                while let Some((k, x)) = a.next_entry::<String, Unique>()? {
                    if v.insert(k, x.0).is_some() {
                        return Err(de::Error::custom("Duplicate field in JSON deck code"));
                    }
                }
                Ok(Unique(Value::Object(v)))
            }
        }
        d.deserialize_any(V)
    }
}

fn quoted(value: &str) -> String {
    use std::fmt::Write;
    let quote = if value.contains('\'') && !value.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut result = String::new();
    result.push(quote);
    for c in value.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c == quote => {
                result.push('\\');
                result.push(c);
            }
            c if c.is_control() && (c as u32) <= 0xff => {
                write!(&mut result, "\\x{:02x}", c as u32).expect("write to string");
            }
            c => result.push(c),
        }
    }
    result.push(quote);
    result
}

/// Decode and validate a twelve-card clipboard code, preserving card order.
pub fn parse_deck(text: &str) -> Result<Vec<String>, String> {
    let mut code: String = python_lines(text)
        .filter(|line| !line.trim_start_matches(python_space).starts_with('#'))
        .flat_map(|line| line.chars().filter(|&c| !python_space(c)))
        .collect();
    if code.is_empty() {
        return Err("No deck code found".into());
    }
    if !code.contains('=') {
        while !code.len().is_multiple_of(4) {
            code.push('=');
        }
    }
    let bytes = STANDARD
        .decode(&code)
        .map_err(|_| "Invalid Base64 deck code")?;
    if STANDARD.encode(&bytes) != code {
        return Err("Invalid Base64 deck code".into());
    }
    let payload = std::str::from_utf8(&bytes)
        .map_err(|_| "Invalid Base64 deck code")?
        .trim_matches(python_space);
    let legacy = payload.starts_with('{');
    let ids: Vec<String> = if legacy {
        let value: Unique = serde_json::from_str(payload).map_err(|e| {
            if e.to_string()
                .starts_with("Duplicate field in JSON deck code")
            {
                "Duplicate field in JSON deck code"
            } else {
                "Invalid JSON deck code"
            }
        })?;
        let cards = value
            .0
            .get("Cards")
            .and_then(Value::as_array)
            .ok_or("JSON deck code must contain a Cards list")?;
        cards
            .iter()
            .map(|v| {
                v.get("CardDefId")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| "Every JSON deck card must have a CardDefId string".to_string())
            })
            .collect::<Result<_, _>>()?
    } else {
        payload.split(',').map(str::to_owned).collect()
    };
    if ids.len() != DECK_SIZE {
        return Err(format!("Expected {DECK_SIZE} cards, got {}", ids.len()));
    }
    let mut expanded = Vec::with_capacity(DECK_SIZE);
    if legacy {
        for id in ids {
            if !catalog().contains(&id) {
                return Err(format!("Unknown card ID: {}", quoted(&id)));
            }
            expanded.push(id);
        }
    } else {
        for token in ids {
            match COMPACT_IDS.get(&token) {
                None => return Err(format!("Unknown compact card ID: {}", quoted(&token))),
                Some(m) if m.len() > 1 => {
                    // The cached catalog index is sorted for deterministic errors.
                    return Err(format!(
                        "Ambiguous compact card ID {}: {}",
                        quoted(&token),
                        m.join(", ")
                    ));
                }
                Some(m) => expanded.push(m[0].to_owned()),
            }
        }
    }
    let mut seen = HashSet::new();
    for id in &expanded {
        if !seen.insert(id) {
            return Err(format!("Duplicate card ID: {}", quoted(id)));
        }
    }
    Ok(expanded)
}

/// Encode a validated deck in the compact clipboard format.
pub fn encode_deck<S: AsRef<str>>(ids: &[S]) -> Result<String, String> {
    let distinct: HashSet<&str> = ids.iter().map(AsRef::as_ref).collect();
    if ids.len() != DECK_SIZE || distinct.len() != DECK_SIZE {
        return Err("A deck must contain 12 distinct card IDs".into());
    }
    let code = STANDARD.encode(
        ids.iter()
            .map(|id| compact(id.as_ref()))
            .collect::<Vec<_>>()
            .join(","),
    );
    if parse_deck(&code)?
        .iter()
        .map(String::as_str)
        .ne(ids.iter().map(AsRef::as_ref))
    {
        return Err("Deck does not round-trip through the card catalog".into());
    }
    Ok(code)
}
/// Validate that all cards have playable, ownable implementations.
pub fn resolve_deck(ids: &[String]) -> Result<(), String> {
    let mut unsupported = Vec::new();
    for id in ids {
        if (crate::cards::get(id).is_none() || crate::cards::is_token(id))
            && !unsupported.contains(&id.as_str())
        {
            unsupported.push(id.as_str());
        }
    }
    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Unsupported or non-ownable cards: {}",
            unsupported.join(", ")
        ))
    }
}

/// Parse an inline code and reject cards without playable implementations.
pub fn parse_playable_deck(code: &str) -> Result<Vec<String>, String> {
    let ids = parse_deck(code)?;
    resolve_deck(&ids)?;
    Ok(ids)
}

/// Valid decks with their source labels, and all per-source errors.
pub type DeckFile = (Vec<(String, Vec<String>)>, Vec<String>);

/// Parse one deck code per non-comment line, collecting all errors like deckinputs.py.
pub fn parse_deck_file(text: &str, label: &str) -> DeckFile {
    let mut decks = Vec::new();
    let mut errors = Vec::new();
    let mut found = false;
    for (index, line) in python_lines(text).enumerate() {
        let code = line
            .split('#')
            .next()
            .unwrap_or("")
            .trim_matches(python_space);
        if code.is_empty() {
            continue;
        }
        found = true;
        let name = format!("{label}:{}", index + 1);
        match parse_playable_deck(code) {
            Ok(ids) => decks.push((name, ids)),
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
    if !found {
        errors.push(format!("{label}: no deck codes found"));
    }
    (decks, errors)
}

/// Read a UTF-8 deck file, including an optional UTF-8 BOM.
pub fn read_deck_file(path: &Path) -> Result<DeckFile, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(parse_deck_file(
        text.strip_prefix('\u{feff}').unwrap_or(&text),
        &path.display().to_string(),
    ))
}
