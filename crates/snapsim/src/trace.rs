//! Exact, first-divergence comparison for parity trace records.
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, PartialEq)]
pub struct Divergence {
    pub path: String,
    pub expected: Option<Value>,
    pub actual: Option<Value>,
}

pub fn first_divergence(expected: &Value, actual: &Value) -> Option<Divergence> {
    fn compare(expected: Option<&Value>, actual: Option<&Value>, path: &str) -> Option<Divergence> {
        match (expected, actual) {
            (Some(Value::Object(left)), Some(Value::Object(right))) => {
                let keys: BTreeSet<_> = left.keys().chain(right.keys()).collect();
                for key in keys {
                    if let Some(diff) =
                        compare(left.get(key), right.get(key), &format!("{path}.{key}"))
                    {
                        return Some(diff);
                    }
                }
                None
            }
            (Some(Value::Array(left)), Some(Value::Array(right))) => {
                for i in 0..left.len().max(right.len()) {
                    if let Some(diff) = compare(left.get(i), right.get(i), &format!("{path}[{i}]"))
                    {
                        return Some(diff);
                    }
                }
                None
            }
            (left, right) if left == right => None,
            (left, right) => Some(Divergence {
                path: path.to_owned(),
                expected: left.cloned(),
                actual: right.cloned(),
            }),
        }
    }
    compare(Some(expected), Some(actual), "$")
}

/// Whether any string in `value` names a card that has rules here but not in the frozen
/// reference (`cards::beyond_reference`); parity gates report such records as new content.
pub fn mentions_beyond_reference(value: &Value) -> bool {
    match value {
        Value::String(text) => crate::cards::beyond_reference(text),
        Value::Array(items) => items.iter().any(mentions_beyond_reference),
        Value::Object(fields) => fields.values().any(mentions_beyond_reference),
        _ => false,
    }
}
