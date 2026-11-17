use super::VariableScope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariablePreview {
    Value(String),
    Secret,
    Undefined,
}

impl VariableScope {
    pub fn preview(&self, name: &str) -> VariablePreview {
        let Some(raw) = self.lookup(name) else {
            return VariablePreview::Undefined;
        };
        let value = self.resolve(raw).text;
        if self.is_secret(name) || self.reveals_secret(&value) {
            return VariablePreview::Secret;
        }
        VariablePreview::Value(value)
    }

    fn is_secret(&self, name: &str) -> bool {
        self.layers
            .iter()
            .rev()
            .find(|layer| layer.values.contains_key(name))
            .is_some_and(|layer| layer.secret_keys.contains(name))
    }

    fn reveals_secret(&self, value: &str) -> bool {
        self.secret_values()
            .iter()
            .any(|secret| value.contains(secret.as_str()))
    }
}
