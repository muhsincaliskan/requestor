use serde_json::{Map, Value};

use crate::collection::Variable;
use crate::text::{scalar_text, Description};

const LIST_SEPARATOR: &str = ",";
const STRING_KIND: &str = "string";

pub fn text_variable(key: &str, value: &str) -> Variable {
    Variable {
        id: None,
        key: Some(key.to_string()),
        value: Some(Value::String(value.to_string())),
        kind: Some(STRING_KIND.to_string()),
        name: None,
        description: None,
        system: None,
        disabled: None,
        extra: Map::new(),
    }
}

pub fn description_of(holder: &Value) -> Option<Description> {
    non_empty_text(holder, "description").map(|text| Description::Text(text.to_string()))
}

pub fn non_empty_text<'a>(holder: &'a Value, key: &str) -> Option<&'a str> {
    holder
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

pub fn value_text(value: &Value) -> String {
    match value {
        Value::Array(items) => items
            .iter()
            .map(value_text)
            .collect::<Vec<_>>()
            .join(LIST_SEPARATOR),
        Value::Object(_) => value.to_string(),
        scalar => scalar_text(scalar).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::value_text;

    #[test]
    fn values_render_as_plain_text() {
        assert_eq!(value_text(&json!("a")), "a");
        assert_eq!(value_text(&json!([1, "b"])), "1,b");
        assert_eq!(value_text(&json!(null)), "");
        assert_eq!(value_text(&json!({"k": 1})), "{\"k\":1}");
    }
}
