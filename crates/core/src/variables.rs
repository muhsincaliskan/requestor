use std::collections::BTreeMap;

use crate::collection::Variable;
use crate::environment::Environment;
use crate::text::scalar_text;

const OPEN_MARK: &str = "{{";
const CLOSE_MARK: &str = "}}";
const MAX_NESTING_DEPTH: usize = 8;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VariableLayer {
    values: BTreeMap<String, String>,
}

impl VariableLayer {
    pub fn from_environment(environment: &Environment) -> Self {
        let values = environment
            .values
            .iter()
            .filter(|entry| entry.is_enabled())
            .filter_map(|entry| Some((entry.key.clone(), scalar_text(entry.value.as_ref()?)?)))
            .collect();
        VariableLayer { values }
    }

    pub fn from_variables(variables: &[Variable]) -> Self {
        let values = variables
            .iter()
            .filter(|variable| variable.is_enabled())
            .filter_map(|variable| Some((variable.key.clone()?, variable.text_value()?)))
            .collect();
        VariableLayer { values }
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub text: String,
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct VariableScope {
    layers: Vec<VariableLayer>,
}

impl VariableScope {
    pub fn new() -> Self {
        VariableScope::default()
    }

    pub fn with_layer(mut self, layer: VariableLayer) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn lookup(&self, key: &str) -> Option<&str> {
        self.layers
            .iter()
            .rev()
            .find_map(|layer| layer.values.get(key))
            .map(String::as_str)
    }

    pub fn resolve(&self, text: &str) -> Resolved {
        let mut unresolved = Vec::new();
        let text = self.expand(text, 0, &mut unresolved);
        unresolved.sort();
        unresolved.dedup();
        Resolved { text, unresolved }
    }

    fn expand(&self, text: &str, depth: usize, unresolved: &mut Vec<String>) -> String {
        let mut output = String::with_capacity(text.len());
        let mut rest = text;
        while let Some((before, name, after)) = split_placeholder(rest) {
            output.push_str(before);
            output.push_str(&self.expand_name(name, depth, unresolved));
            rest = after;
        }
        output.push_str(rest);
        output
    }

    fn expand_name(&self, name: &str, depth: usize, unresolved: &mut Vec<String>) -> String {
        match self.lookup(name) {
            Some(value) if depth < MAX_NESTING_DEPTH => self.expand(value, depth + 1, unresolved),
            Some(value) => value.to_string(),
            None => {
                unresolved.push(name.to_string());
                format!("{OPEN_MARK}{name}{CLOSE_MARK}")
            }
        }
    }
}

fn split_placeholder(text: &str) -> Option<(&str, &str, &str)> {
    let start = text.find(OPEN_MARK)?;
    let name_start = start + OPEN_MARK.len();
    let length = text[name_start..].find(CLOSE_MARK)?;
    let name = text[name_start..name_start + length].trim();
    let after = &text[name_start + length + CLOSE_MARK.len()..];
    Some((&text[..start], name, after))
}

#[cfg(test)]
mod tests {
    use super::{VariableLayer, VariableScope};

    fn scope() -> VariableScope {
        VariableScope::new()
            .with_layer(
                VariableLayer::default()
                    .with("host", "global.example")
                    .with("a", "1"),
            )
            .with_layer(VariableLayer::default().with("host", "env.example"))
    }

    #[test]
    fn later_layers_override_earlier_ones() {
        assert_eq!(
            scope().resolve("https://{{host}}/x").text,
            "https://env.example/x"
        );
        assert_eq!(scope().resolve("{{a}}").text, "1");
    }

    #[test]
    fn unresolved_names_are_kept_and_reported_once() {
        let resolved = scope().resolve("{{missing}}/{{ missing }}/{{host}}");
        assert_eq!(resolved.text, "{{missing}}/{{missing}}/env.example");
        assert_eq!(resolved.unresolved, vec!["missing".to_string()]);
    }

    #[test]
    fn nested_variables_expand_and_cycles_terminate() {
        let nested = VariableScope::new().with_layer(
            VariableLayer::default()
                .with("base", "https://{{host}}")
                .with("host", "h"),
        );
        assert_eq!(nested.resolve("{{base}}/p").text, "https://h/p");
        let cyclic = VariableScope::new().with_layer(VariableLayer::default().with("a", "{{a}}x"));
        assert!(cyclic.resolve("{{a}}").text.starts_with("{{a}}x"));
    }

    #[test]
    fn unterminated_placeholder_is_left_alone() {
        assert_eq!(scope().resolve("a {{host").text, "a {{host");
    }
}
