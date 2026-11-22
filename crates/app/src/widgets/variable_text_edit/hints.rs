use requestor_core::variables::{VariablePreview, VariableScope};
use requestor_core::workspace::{VariableHome, VariableHomes};

pub struct VariableHints {
    scope: VariableScope,
    names: Vec<String>,
    suggest: bool,
    homes: Option<VariableHomes>,
}

impl VariableHints {
    pub fn new(scope: VariableScope, suggest: bool) -> Self {
        let names = scope.names();
        VariableHints {
            scope,
            names,
            suggest,
            homes: None,
        }
    }

    pub fn editable(mut self, homes: VariableHomes) -> Self {
        self.homes = Some(homes);
        self
    }

    pub fn home_of(&self, name: &str) -> Option<&VariableHome> {
        self.homes.as_ref()?.home_of(name)
    }

    pub fn stored_value(&self, name: &str) -> Option<&str> {
        self.scope.lookup(name)
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
