use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::collection::Variable;
use crate::environment::Environment;
use crate::text::scalar_text;

const SECRET_KIND: &str = "secret";

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
            .filter(|entry| entry.is_enabled())
            .filter_map(|entry| {
                let value = scalar_text(entry.value.as_ref()?)?;
                let secret = entry.kind.as_deref() == Some(SECRET_KIND);
                Some((entry.key.clone(), value, secret))
            })
            .collect()
    }

    pub fn from_variables(variables: &[Variable]) -> Self {
        variables
            .iter()
            .filter(|variable| variable.is_enabled())
            .filter_map(|variable| {
                let secret = variable.kind.as_deref() == Some(SECRET_KIND);
                Some((variable.key.clone()?, variable.text_value()?, secret))
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
