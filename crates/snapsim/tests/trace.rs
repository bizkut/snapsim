use serde_json::json;
use snapsim::trace::first_divergence;

#[test]
fn reports_first_nested_difference_and_missing_values() {
    let left = json!({"turns": [{"plays": [], "powers": [[0, 1], [2, 3]]}]});
    let right = json!({"turns": [{"plays": [], "powers": [[0, 1], [2, 4]]}]});
    let diff = first_divergence(&left, &right).unwrap();
    assert_eq!(diff.path, "$.turns[0].powers[1][1]");
    assert_eq!(diff.expected, Some(json!(3)));
    assert_eq!(diff.actual, Some(json!(4)));
    assert!(first_divergence(&left, &left).is_none());
    let missing = first_divergence(&json!({"x": null}), &json!({})).unwrap();
    assert_eq!(missing.expected, Some(json!(null)));
    assert_eq!(missing.actual, None);
}
