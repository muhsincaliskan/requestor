use serde_json::Value;

use crate::script::Comparison;

pub(super) fn holds(
    comparison: Comparison,
    observed: Option<&Value>,
    operand: Option<&Value>,
) -> bool {
    match comparison {
        Comparison::Exists => observed.is_some(),
        Comparison::NotExists => observed.is_none(),
        Comparison::IsEmpty => observed.is_none_or(is_empty),
        Comparison::IsNotEmpty => !observed.is_none_or(is_empty),
        Comparison::Is => both(observed, operand, loosely_equal),
        Comparison::IsNot => !both(observed, operand, loosely_equal),
        Comparison::Contains => both(observed, operand, contains),
        Comparison::NotContains => !both(observed, operand, contains),
        Comparison::Above => both(observed, operand, |left, right| {
            ordered(left, right, f64::gt)
        }),
        Comparison::Below => both(observed, operand, |left, right| {
            ordered(left, right, f64::lt)
        }),
    }
}

pub(crate) fn text_of(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn both(
    observed: Option<&Value>,
    operand: Option<&Value>,
    test: fn(&Value, &Value) -> bool,
) -> bool {
    match (observed, operand) {
        (Some(left), Some(right)) => test(left, right),
        _ => false,
    }
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Object(fields) => fields.is_empty(),
        Value::Bool(_) | Value::Number(_) => false,
    }
}

fn loosely_equal(left: &Value, right: &Value) -> bool {
    if left == right {
        return true;
    }
    match (number_of(left), number_of(right)) {
        (Some(left), Some(right)) => left == right,
        _ => text_of(left) == text_of(right),
    }
}

fn contains(haystack: &Value, needle: &Value) -> bool {
    match haystack {
        Value::Array(items) => items.iter().any(|item| loosely_equal(item, needle)),
        Value::Object(fields) => fields.contains_key(&text_of(needle)),
        other => text_of(other).contains(&text_of(needle)),
    }
}

fn ordered(left: &Value, right: &Value, order: fn(&f64, &f64) -> bool) -> bool {
    match (number_of(left), number_of(right)) {
        (Some(left), Some(right)) => order(&left, &right),
        _ => false,
    }
}

fn number_of(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::holds;
    use crate::script::Comparison;

    fn check(comparison: Comparison, observed: Option<Value>, operand: Value) -> bool {
        holds(comparison, observed.as_ref(), Some(&operand))
    }

    #[test]
    fn equality_is_loose_between_text_and_numbers() {
        assert!(check(Comparison::Is, Some(json!(200)), json!(200)));
        assert!(check(Comparison::Is, Some(json!("200")), json!(200)));
        assert!(check(Comparison::Is, Some(json!("prod")), json!("prod")));
        assert!(!check(Comparison::Is, None, json!("x")));
        assert!(check(Comparison::IsNot, None, json!("x")));
        assert!(check(Comparison::Is, Some(json!(true)), json!(true)));
    }

    #[test]
    fn contains_searches_text_lists_and_keys() {
        assert!(check(
            Comparison::Contains,
            Some(json!("application/json")),
            json!("json")
        ));
        assert!(check(Comparison::Contains, Some(json!([1, 2])), json!(2)));
        assert!(check(
            Comparison::Contains,
            Some(json!({"id": 1})),
            json!("id")
        ));
        assert!(check(Comparison::NotContains, None, json!("x")));
    }

    #[test]
    fn ordering_needs_numbers_on_both_sides() {
        assert!(check(Comparison::Above, Some(json!(3)), json!(2)));
        assert!(check(Comparison::Below, Some(json!("120")), json!(500)));
        assert!(!check(Comparison::Above, Some(json!("abc")), json!(1)));
    }

    #[test]
    fn existence_and_emptiness_treat_missing_values_as_empty() {
        assert!(holds(Comparison::IsEmpty, None, None));
        assert!(holds(Comparison::IsEmpty, Some(&json!([])), None));
        assert!(holds(Comparison::IsNotEmpty, Some(&json!("x")), None));
        assert!(holds(Comparison::Exists, Some(&Value::Null), None));
        assert!(holds(Comparison::NotExists, None, None));
    }
}
