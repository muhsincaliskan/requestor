mod expansion;
mod layer;
mod preview;
#[cfg(test)]
mod preview_tests;
mod segments;
#[cfg(test)]
mod tests;

use std::fmt;

use crate::collection::Variable;
use crate::environment::Environment;
use expansion::{sorted_unique, Expansion};

pub use expansion::placeholder_names;
pub(crate) use expansion::OPEN_MARK;
pub use layer::VariableLayer;
pub use preview::VariablePreview;
pub use segments::{TextSegment, TextSegments};

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

    pub fn layered(
        collection: &[Variable],
        folder: &[Variable],
        environment: Option<&Environment>,
        overrides: &[(String, String)],
    ) -> Self {
        let override_layer = overrides
            .iter()
            .fold(VariableLayer::default(), |layer, pair| {
                layer.with(pair.0.as_str(), pair.1.as_str())
            });
        let mut scope = VariableScope::new()
            .with_layer(VariableLayer::from_variables(collection))
            .with_layer(VariableLayer::from_variables(folder));
        if let Some(environment) = environment {
            scope = scope.with_layer(VariableLayer::from_environment(environment));
        }
        scope.with_layer(override_layer)
    }

    pub fn secret_values(&self) -> Vec<String> {
        let mut values = Vec::new();
        for layer in &self.layers {
            for key in &layer.secret_keys {
                let raw = &layer.values[key];
                values.push(self.resolve(raw).text);
                values.push(raw.clone());
            }
        }
        values.retain(|value| !value.is_empty());
        values.sort();
        values.dedup();
        values
    }

    pub fn names(&self) -> Vec<String> {
        let names = self
            .layers
            .iter()
            .flat_map(|layer| layer.values.keys().cloned())
            .collect();
        sorted_unique(names)
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
            budget: expansion::MAX_EXPANDED_BYTES,
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
