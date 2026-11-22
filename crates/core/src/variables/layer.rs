use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::collection::Variable;
use crate::environment::Environment;

#[derive(Clone, Default, PartialEq)]
pub struct VariableLayer {
    pub(super) values: BTreeMap<String, String>,
    pub(super) secret_keys: BTreeSet<String>,
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
        environment
            .values
            .iter()
            .filter(|entry| entry.provides_value())
            .map(|entry| (entry.key.clone(), entry.value_text(), entry.is_secret()))
            .collect()
    }

    pub fn from_variables(variables: &[Variable]) -> Self {
        variables
            .iter()
            .filter(|variable| variable.provides_value())
            .map(|variable| {
                let key = variable.key_text().to_string();
                (key, variable.value_text(), variable.is_secret())
            })
            .collect()
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }
}

impl FromIterator<(String, String, bool)> for VariableLayer {
    fn from_iter<Entries: IntoIterator<Item = (String, String, bool)>>(entries: Entries) -> Self {
        let mut layer = VariableLayer::default();
        for (key, value, secret) in entries {
            if secret {
                layer.secret_keys.insert(key.clone());
            }
            layer.values.insert(key, value);
        }
        layer
    }
}
