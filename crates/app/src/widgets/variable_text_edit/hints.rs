use requestor_core::variables::{VariablePreview, VariableScope};

pub struct VariableHints {
    scope: VariableScope,
    names: Vec<String>,
    suggest: bool,
}

impl VariableHints {
    pub fn new(scope: VariableScope, suggest: bool) -> Self {
        let names = scope.names();
        VariableHints {
            scope,
            names,
            suggest,
        }
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn suggestions(&self) -> &[String] {
        match self.suggest {
            true => &self.names,
            false => &[],
        }
    }

    pub fn preview(&self, name: &str) -> VariablePreview {
        self.scope.preview(name)
    }
}
