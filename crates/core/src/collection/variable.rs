use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::param::STRING_KIND;
use crate::environment::SECRET_KIND;
use crate::text::{scalar_text, Description};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Variable {
    pub fn text(key: &str, value: &str) -> Self {
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

    pub fn remove_from(variables: &mut Vec<Variable>, key: &str) {
        variables.retain(|variable| variable.key_text() != key);
    }

    pub fn set_in(variables: &mut Vec<Variable>, key: &str, text: &str) {
        match variables
            .iter_mut()
            .find(|variable| variable.key_text() == key)
        {
            Some(variable) => {
                variable.set_value_text(text);
                variable.set_enabled(true);
            }
            None => variables.push(Variable::text(key, text)),
        }
    }

    pub fn assign_in(variables: &mut Option<Vec<Variable>>, key: &str, text: &str) {
        let variables = variables.get_or_insert_with(Vec::new);
        match variables
            .iter_mut()
            .rev()
            .find(|variable| variable.key_text() == key && variable.provides_value())
        {
            Some(variable) => variable.set_value_text(text),
            None => variables.push(Variable::text(key, text)),
        }
    }

    pub fn provided_keys(variables: &[Variable]) -> impl Iterator<Item = &str> {
        variables
            .iter()
            .filter(|variable| variable.provides_value())
            .map(Variable::key_text)
    }

    pub fn provides_value(&self) -> bool {
        self.is_enabled() && self.key.is_some() && self.text_value().is_some()
    }

    pub fn is_secret(&self) -> bool {
        self.kind.as_deref() == Some(SECRET_KIND)
    }

    pub fn clear_secret_value(&mut self) {
        if self.is_secret() && self.value.is_some() {
            self.value = Some(Value::String(String::new()));
        }
    }

    pub fn is_enabled(&self) -> bool {
        !self.disabled.unwrap_or(false)
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.disabled = if enabled { None } else { Some(true) };
    }

    pub fn key_text(&self) -> &str {
        self.key.as_deref().unwrap_or_default()
    }

    pub fn set_key_text(&mut self, text: &str) {
        if self.key_text() != text {
            self.key = Some(text.to_string());
        }
    }

    pub fn value_text(&self) -> String {
        self.text_value().unwrap_or_default()
    }

    pub fn set_value_text(&mut self, text: &str) {
        if self.value_text() != text {
            self.value = Some(Value::String(text.to_string()));
        }
    }

    pub fn text_value(&self) -> Option<String> {
        self.value.as_ref().and_then(scalar_text)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Variable;

    #[test]
    fn enabled_flag_round_trips_through_disabled() {
        let mut variable = Variable::text("a", "1");
        variable.set_enabled(false);
        assert!(!variable.is_enabled());
        variable.set_enabled(true);
        assert!(variable.is_enabled());
        assert_eq!(variable.disabled, None);
    }

    #[test]
    fn untouched_non_string_values_are_preserved() {
        let mut variable = Variable::text("port", "");
        variable.value = Some(json!(8080));
        assert_eq!(variable.value_text(), "8080");
        variable.set_value_text("8080");
        assert_eq!(variable.value, Some(json!(8080)));
        variable.set_value_text("9090");
        assert_eq!(variable.value, Some(json!("9090")));
    }

    #[test]
    fn key_text_reads_empty_when_missing_and_is_created_on_write() {
        let mut variable = Variable::text("", "");
        variable.key = None;
        assert_eq!(variable.key_text(), "");
        variable.set_key_text("");
        assert_eq!(variable.key, None);
        variable.set_key_text("host");
        assert_eq!(variable.key.as_deref(), Some("host"));
    }
}
