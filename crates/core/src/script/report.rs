use super::VariableTarget;
use crate::collection::Variable;
use crate::environment::Environment;
use crate::secret::Redactor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    pub title: String,
    pub failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Written {
    Value(String),
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableWrite {
    pub variable: String,
    pub target: VariableTarget,
    pub value: Result<Written, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScriptReport {
    pub checks: Vec<CheckOutcome>,
    pub writes: Vec<VariableWrite>,
    pub failure: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Applied {
    pub environment: bool,
    pub collection: bool,
}

impl CheckOutcome {
    pub fn passed(&self) -> bool {
        self.failure.is_none()
    }

    pub fn redacted(self, redactor: &Redactor) -> Self {
        CheckOutcome {
            failure: self.failure.map(|failure| redactor.redact_text(&failure)),
            ..self
        }
    }
}

impl ScriptReport {
    pub fn passed_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|outcome| outcome.passed())
            .count()
    }

    pub fn all_passed(&self) -> bool {
        self.failure.is_none() && self.passed_count() == self.checks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.checks.is_empty() && self.writes.is_empty() && self.failure.is_none()
    }

    pub fn absorb(&mut self, other: ScriptReport) {
        self.checks.extend(other.checks);
        self.writes.extend(other.writes);
        self.failure = self.failure.take().or(other.failure);
    }

    pub fn request_variables(&self) -> Vec<(String, String)> {
        self.writes
            .iter()
            .filter(|write| write.target == VariableTarget::Request)
            .filter_map(|write| match &write.value {
                Ok(Written::Value(value)) => Some((write.variable.clone(), value.clone())),
                _ => None,
            })
            .collect()
    }

    pub fn apply(
        &self,
        mut environment: Option<&mut Environment>,
        collection_variables: &mut Option<Vec<Variable>>,
    ) -> Applied {
        let mut applied = Applied::default();
        for write in self.writes.iter().filter(|write| write.target.persists()) {
            let Ok(written) = &write.value else {
                continue;
            };
            match (write.target, environment.as_deref_mut()) {
                (VariableTarget::Environment, Some(active)) => {
                    written.apply_to_environment(active, &write.variable);
                    applied.environment = true;
                }
                _ => {
                    written.apply_to_variables(collection_variables, &write.variable);
                    applied.collection = true;
                }
            }
        }
        applied
    }
}

impl Written {
    fn apply_to_environment(&self, environment: &mut Environment, variable: &str) {
        match self {
            Written::Value(value) => environment.set_value(variable, value),
            Written::Removed => environment.remove_value(variable),
        }
    }

    fn apply_to_variables(&self, variables: &mut Option<Vec<Variable>>, variable: &str) {
        match self {
            Written::Value(value) => {
                Variable::set_in(variables.get_or_insert_with(Vec::new), variable, value)
            }
            Written::Removed => {
                if let Some(list) = variables {
                    Variable::remove_from(list, variable);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
