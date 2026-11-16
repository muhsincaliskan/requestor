use std::collections::BTreeMap;
use std::fmt;

use crate::collection::Variable;
use crate::environment::Environment;
use crate::text::scalar_text;

const OPEN_MARK: &str = "{{";
const CLOSE_MARK: &str = "}}";
const MAX_EXPANDED_BYTES: usize = 1024 * 1024;

#[derive(Clone, Default, PartialEq)]
pub struct VariableLayer {
    values: BTreeMap<String, String>,
}

impl fmt::Debug for VariableLayer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VariableLayer")
            .field("keys", &self.values.keys().collect::<Vec<_>>())
            .finish()
    }
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

#[derive(Clone, PartialEq, Eq)]
pub struct Resolved {
    pub text: String,
    pub unresolved: Vec<String>,
    pub circular: Vec<String>,
    pub limit_exceeded: bool,
}

impl fmt::Debug for Resolved {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Resolved")
            .field("text_bytes", &self.text.len())
            .field("unresolved", &self.unresolved)
            .field("circular", &self.circular)
            .field("limit_exceeded", &self.limit_exceeded)
            .finish()
    }
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
        let mut expansion = Expansion {
            scope: self,
            budget: MAX_EXPANDED_BYTES,
            active: Vec::new(),
            unresolved: Vec::new(),
            circular: Vec::new(),
            limit_exceeded: false,
        };
        let text = expansion.expand(text);
        Resolved {
            text,
            unresolved: sorted_unique(expansion.unresolved),
            circular: sorted_unique(expansion.circular),
            limit_exceeded: expansion.limit_exceeded,
        }
    }
}

pub fn placeholder_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(found) = Placeholder::find(rest) {
        names.push(found.name.to_string());
        rest = found.after;
    }
    sorted_unique(names)
}

fn sorted_unique(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names.dedup();
    names
}

struct Expansion<'a> {
    scope: &'a VariableScope,
    budget: usize,
    active: Vec<String>,
    unresolved: Vec<String>,
    circular: Vec<String>,
    limit_exceeded: bool,
}

impl Expansion<'_> {
    fn expand(&mut self, text: &str) -> String {
        let mut output = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(found) = Placeholder::find(rest) {
            output.push_str(found.before);
            output.push_str(&self.expand_placeholder(&found));
            rest = found.after;
        }
        output.push_str(rest);
        output
    }

    fn expand_placeholder(&mut self, found: &Placeholder<'_>) -> String {
        if self.active.iter().any(|name| name == found.name) {
            self.circular.push(found.name.to_string());
            return found.literal.to_string();
        }
        let Some(value) = self.scope.lookup(found.name) else {
            self.unresolved.push(found.name.to_string());
            return found.literal.to_string();
        };
        if value.len() > self.budget {
            self.limit_exceeded = true;
            return found.literal.to_string();
        }
        self.budget -= value.len();
        self.active.push(found.name.to_string());
        let expanded = self.expand(value);
        self.active.pop();
        expanded
    }
}

struct Placeholder<'a> {
    before: &'a str,
    literal: &'a str,
    name: &'a str,
    after: &'a str,
}

impl<'a> Placeholder<'a> {
    fn find(text: &'a str) -> Option<Self> {
        let mut from = 0;
        while let Some(relative) = text[from..].find(CLOSE_MARK) {
            let close = from + relative;
            if let Some(found) = Self::ending_at(text, from, close) {
                return Some(found);
            }
            from = close + CLOSE_MARK.len();
        }
        None
    }

    fn ending_at(text: &'a str, from: usize, close: usize) -> Option<Self> {
        let open = from + text[from..close].rfind(OPEN_MARK)?;
        let name = text[open + OPEN_MARK.len()..close].trim();
        if name.is_empty() {
            return None;
        }
        let end = close + CLOSE_MARK.len();
        Some(Placeholder {
            before: &text[..open],
            literal: &text[open..end],
            name,
            after: &text[end..],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{placeholder_names, VariableLayer, VariableScope};

    fn scope() -> VariableScope {
        VariableScope::new()
            .with_layer(
                VariableLayer::default()
                    .with("host", "global.example")
                    .with("a", "1"),
            )
            .with_layer(VariableLayer::default().with("host", "env.example"))
    }

    fn scope_with(pairs: &[(&str, &str)]) -> VariableScope {
        let layer = pairs
            .iter()
            .fold(VariableLayer::default(), |layer, (key, value)| {
                layer.with(*key, *value)
            });
        VariableScope::new().with_layer(layer)
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
    fn unresolved_names_keep_their_original_spelling_and_are_reported_once() {
        let resolved = scope().resolve("{{missing}}/{{ missing }}/{{host}}");
        assert_eq!(resolved.text, "{{missing}}/{{ missing }}/env.example");
        assert_eq!(resolved.unresolved, vec!["missing".to_string()]);
    }

    #[test]
    fn nested_variables_expand() {
        let nested = scope_with(&[("base", "https://{{host}}"), ("host", "h")]);
        assert_eq!(nested.resolve("{{base}}/p").text, "https://h/p");
    }

    #[test]
    fn cycles_are_reported_and_left_unexpanded() {
        let resolved = scope_with(&[("a", "{{b}}"), ("b", "{{a}}")]).resolve("{{a}}");
        assert_eq!(resolved.text, "{{a}}");
        assert_eq!(resolved.circular, vec!["a".to_string()]);
        let selfish = scope_with(&[("a", "x{{a}}")]).resolve("{{a}}");
        assert_eq!(selfish.text, "x{{a}}");
        assert_eq!(selfish.circular, vec!["a".to_string()]);
    }

    #[test]
    fn exponential_expansion_is_bounded() {
        let fanout = "{{c}}".repeat(8);
        let mut pairs: Vec<(String, String)> = (0..12)
            .map(|level| {
                (
                    format!("v{level}"),
                    format!("{{{{v{}}}}}", level + 1).repeat(8),
                )
            })
            .collect();
        pairs.push(("v12".to_string(), "x".to_string()));
        pairs.push(("c".to_string(), fanout));
        let layer = pairs
            .iter()
            .fold(VariableLayer::default(), |layer, (key, value)| {
                layer.with(key.as_str(), value.as_str())
            });
        let resolved = VariableScope::new().with_layer(layer).resolve("{{v0}}");
        assert!(resolved.limit_exceeded);
        assert!(resolved.text.len() < 16 * 1024 * 1024);
    }

    #[test]
    fn placeholder_parsing_matches_postman_for_edge_cases() {
        let a = scope();
        assert_eq!(a.resolve("{{{a}}}").text, "{1}");
        assert_eq!(a.resolve("{{x {{a}}").text, "{{x 1");
        assert_eq!(a.resolve("{{}}").text, "{{}}");
        assert!(a.resolve("{{}}").unresolved.is_empty());
        assert_eq!(a.resolve("}} {{a}}").text, "}} 1");
        assert_eq!(a.resolve("a {{host").text, "a {{host");
    }

    #[test]
    fn placeholder_names_lists_distinct_names() {
        assert_eq!(
            placeholder_names("{{b}} {{a}} {{b}} {{ }}"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn debug_output_hides_values() {
        let debug = format!("{:?}", scope_with(&[("token", "s3cret")]));
        assert!(debug.contains("token"));
        assert!(!debug.contains("s3cret"));
    }
}
