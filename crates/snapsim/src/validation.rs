//! Validation of the Python position input format.
use crate::{data, position::Position};
use serde_json::{Map, Value};
use std::{collections::HashSet, sync::LazyLock};

static CARD_IDS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    data::cards()
        .into_iter()
        .map(|card| card.carddefid)
        .collect()
});

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{path} must be an object"))
}
fn array<'a>(value: &'a Value, path: &str, limit: Option<usize>) -> Result<&'a Vec<Value>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{path} must be an array"))?;
    if limit.is_some_and(|limit| values.len() > limit) {
        return Err(format!("{path} has too many entries"));
    }
    Ok(values)
}
fn integer(value: &Value, path: &str, min: Option<i64>, max: Option<i64>) -> Result<(), String> {
    let n = value
        .as_i64()
        .map(i128::from)
        .or_else(|| value.as_u64().map(i128::from))
        .ok_or_else(|| format!("{path} must be an integer in the supported range"))?;
    let field = path.rsplit('.').next().unwrap_or(path);
    let fits = match field {
        "turn" | "total_turns" | "played_turn" | "revealed_turn" => u8::try_from(n).is_ok(),
        "energy" | "cost" | "power" | "pending_power" => i32::try_from(n).is_ok(),
        _ => usize::try_from(n).is_ok(),
    };
    if !fits
        || min.is_some_and(|min| n < i128::from(min))
        || max.is_some_and(|max| n > i128::from(max))
    {
        return Err(format!("{path} is out of range"));
    }
    Ok(())
}
fn optional_integer(value: &Value, path: &str, min: Option<i64>) -> Result<(), String> {
    if value.is_null() {
        Ok(())
    } else {
        integer(value, path, min, None)
    }
}
fn card_id(value: &Value, path: &str) -> Result<(), String> {
    if value.as_str().is_some_and(|id| CARD_IDS.contains(id)) {
        Ok(())
    } else {
        Err(format!("{path}: unknown card ID"))
    }
}
fn card_ids(value: &Value, path: &str) -> Result<(), String> {
    for (i, id) in array(value, path, None)?.iter().enumerate() {
        card_id(id, &format!("{path}[{i}]"))?;
    }
    Ok(())
}
fn card_refs(value: &Value, path: &str, limit: usize) -> Result<(), String> {
    for (i, card) in array(value, path, Some(limit))?.iter().enumerate() {
        let path = format!("{path}[{i}]");
        let card = object(card, &path)?;
        if !card
            .get("revealed")
            .unwrap_or(&Value::Bool(true))
            .is_boolean()
        {
            return Err(format!("{path}.revealed must be a boolean"));
        }
        card_id(
            card.get("id").unwrap_or(&Value::Null),
            &format!("{path}.id"),
        )?;
        for (name, min) in [("cost", Some(0)), ("power", None), ("played_turn", Some(0))] {
            optional_integer(
                card.get(name).unwrap_or(&Value::Null),
                &format!("{path}.{name}"),
                min,
            )?;
        }
    }
    Ok(())
}
impl Position {
    /// Parse and validate a Python position dictionary encoded as JSON.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let mut value: Value = serde_json::from_str(text).map_err(|e| format!("position: {e}"))?;
        let root = object(&value, "position")?;
        let null = Value::Null;
        let six = Value::from(6);
        let zero = Value::from(0);
        let empty = Value::Array(vec![]);
        let total = root.get("total_turns").unwrap_or(&six);
        integer(total, "total_turns", Some(1), None)?;
        integer(
            root.get("turn").unwrap_or(&null),
            "turn",
            Some(1),
            total.as_i64(),
        )?;
        let lanes = array(root.get("locations").unwrap_or(&null), "locations", None)?;
        if lanes.len() != 3 {
            return Err("locations must contain exactly three entries".into());
        }
        for (field, allowed) in [
            ("priority", &["me", "opponent"][..]),
            ("mode", &["recruit"][..]),
        ] {
            let v = root.get(field).unwrap_or(&null);
            if !v.is_null() && !v.as_str().is_some_and(|v| allowed.contains(&v)) {
                return Err(format!("{field} has an invalid value"));
            }
        }
        integer(root.get("energy").unwrap_or(&null), "energy", Some(0), None)?;
        card_refs(root.get("hand").unwrap_or(&null), "hand", 7)?;
        card_ids(root.get("deck").unwrap_or(&null), "deck")?;
        for (i, lane) in lanes.iter().enumerate() {
            let path = format!("locations[{i}]");
            let lane = object(lane, &path)?;
            let id = lane.get("id").unwrap_or(&null);
            if !id.is_null() && !id.as_str().is_some_and(|s| !s.is_empty()) {
                return Err(format!("{path}.id must be a location ID or null"));
            }
            let theirs = lane.get("theirs").unwrap_or(&null);
            card_refs(theirs, &format!("{path}.theirs"), 4)?;
            if let Some(spent) = lane.get("ottos_lab_spent").filter(|v| !v.is_null())
                && (id.as_str() != Some("OttosLab")
                    || !spent
                        .as_array()
                        .is_some_and(|a| a.len() == 2 && a.iter().all(Value::is_boolean)))
            {
                return Err(format!(
                    "{path}.ottos_lab_spent must be two booleans for Otto's Lab"
                ));
            }
            card_refs(
                lane.get("mine").unwrap_or(&null),
                &format!("{path}.mine"),
                4,
            )?;
            integer(
                lane.get("theirs_face_down").unwrap_or(&zero),
                &format!("{path}.theirs_face_down"),
                Some(0),
                Some((4 - theirs.as_array().unwrap().len()) as i64),
            )?;
            optional_integer(
                lane.get("revealed_turn").unwrap_or(&null),
                &format!("{path}.revealed_turn"),
                Some(0),
            )?;
        }
        let opponent = object(root.get("opponent").unwrap_or(&null), "opponent")?;
        if let Some(prior) = opponent.get("prior")
            && !matches!(prior.as_str(), Some("uniform" | "weighted"))
        {
            return Err("opponent.prior is invalid".into());
        }
        integer(
            opponent.get("hand").unwrap_or(&null),
            "opponent.hand",
            Some(0),
            Some(7),
        )?;
        integer(
            opponent.get("deck").unwrap_or(&null),
            "opponent.deck",
            Some(0),
            None,
        )?;
        optional_integer(
            opponent.get("energy").unwrap_or(&null),
            "opponent.energy",
            Some(0),
        )?;
        card_ids(opponent.get("seen").unwrap_or(&empty), "opponent.seen")?;
        for field in ["deck_list", "pool"] {
            let v = opponent.get(field).unwrap_or(&null);
            if !v.is_null() {
                card_ids(v, &format!("opponent.{field}"))?;
            }
        }
        integer(
            opponent.get("pending_power").unwrap_or(&zero),
            "opponent.pending_power",
            None,
            None,
        )?;
        integer(
            root.get("pending_power").unwrap_or(&zero),
            "pending_power",
            None,
            None,
        )?;
        optional_integer(
            root.get("deck_count").unwrap_or(&null),
            "deck_count",
            Some(0),
        )?;
        // Python treats a missing location id as null; serde requires the key.
        for lane in value["locations"].as_array_mut().unwrap() {
            lane.as_object_mut()
                .unwrap()
                .entry("id")
                .or_insert(Value::Null);
        }
        serde_json::from_value(value).map_err(|e| format!("position: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::Position;
    use serde_json::{Value, json};

    fn position() -> Value {
        json!({
            "turn": 3, "energy": 3, "hand": [{"id": "Hulk"}], "deck": [],
            "locations": [{"mine": [], "theirs": []}, {"mine": [], "theirs": []},
                          {"mine": [], "theirs": []}],
            "opponent": {"hand": 0, "deck": 0}
        })
    }

    #[test]
    fn validates_defaults_and_nested_rules() {
        let mut input = position();
        assert!(Position::from_json(&input.to_string()).is_ok());
        input["hand"][0]["cost"] = json!(true);
        assert!(
            Position::from_json(&input.to_string())
                .unwrap_err()
                .contains("hand[0].cost")
        );
        input = position();
        input["locations"][0]["theirs_face_down"] = json!(5);
        assert!(
            Position::from_json(&input.to_string())
                .unwrap_err()
                .contains("locations[0].theirs_face_down")
        );
        input = position();
        input["opponent"]["prior"] = Value::Null;
        assert!(
            Position::from_json(&input.to_string())
                .unwrap_err()
                .contains("opponent.prior")
        );
    }

    #[test]
    fn rejects_values_outside_rust_storage_range() {
        let mut input = position();
        input["energy"] = json!(i64::from(i32::MAX) + 1);
        assert!(
            Position::from_json(&input.to_string())
                .unwrap_err()
                .contains("energy")
        );
        input = position();
        input["hand"][0]["played_turn"] = json!(256);
        assert!(
            Position::from_json(&input.to_string())
                .unwrap_err()
                .contains("hand[0].played_turn")
        );
    }
}
