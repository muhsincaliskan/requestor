use super::{RulesReport, VariableTarget};
use crate::collection::Variable;
use crate::environment::Environment;

impl RulesReport {
    pub fn apply(
        &self,
        mut environment: Option<&mut Environment>,
        collection_variables: &mut Vec<Variable>,
    ) -> usize {
        let mut applied = 0;
        for extracted in &self.extracted {
            let Ok(value) = &extracted.value else {
                continue;
            };
            let variable = extracted.extraction.variable.as_str();
            match (extracted.extraction.target, environment.as_deref_mut()) {
                (VariableTarget::Environment, Some(active)) => active.set_value(variable, value),
                _ => Variable::set_in(collection_variables, variable, value),
            }
            applied += 1;
        }
        applied
    }
}

#[cfg(test)]
mod tests {
    use crate::checks::{ExtractSource, Extracted, Extraction, RulesReport, VariableTarget};
    use crate::collection::Variable;
    use crate::environment::Environment;

    fn extracted(variable: &str, target: VariableTarget, value: Result<&str, &str>) -> Extracted {
        Extracted {
            extraction: Extraction {
                variable: variable.to_string(),
                source: ExtractSource::Header("X".to_string()),
                target,
            },
            value: value.map(str::to_string).map_err(str::to_string),
        }
    }

    #[test]
    fn values_go_to_their_target_and_failures_change_nothing() {
        let report = RulesReport {
            checks: Vec::new(),
            extracted: vec![
                extracted("token", VariableTarget::Environment, Ok("t-1")),
                extracted("id", VariableTarget::Collection, Ok("7")),
                extracted("lost", VariableTarget::Environment, Err("missing")),
            ],
        };
        let mut environment = Environment::new("Dev");
        environment.set_value("token", "old");
        let mut variables = vec![Variable::text("id", "1")];
        assert_eq!(report.apply(Some(&mut environment), &mut variables), 2);
        assert_eq!(environment.value_of("token").as_deref(), Some("t-1"));
        assert_eq!(environment.values.len(), 1);
        assert_eq!(variables[0].value_text(), "7");
        let mut without_environment = Vec::new();
        report.apply(None, &mut without_environment);
        assert_eq!(without_environment.len(), 2);
    }
}
