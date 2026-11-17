use super::{RulesReport, VariableTarget};
use crate::collection::Variable;
use crate::environment::Environment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Applied {
    pub environment: bool,
    pub collection: bool,
}

impl RulesReport {
    pub fn apply(
        &self,
        mut environment: Option<&mut Environment>,
        collection_variables: &mut Option<Vec<Variable>>,
    ) -> Applied {
        let mut applied = Applied::default();
        for extracted in &self.extracted {
            let Ok(value) = &extracted.value else {
                continue;
            };
            let variable = extracted.extraction.variable.as_str();
            match (extracted.extraction.target, environment.as_deref_mut()) {
                (VariableTarget::Environment, Some(active)) => {
                    active.set_value(variable, value);
                    applied.environment = true;
                }
                _ => {
                    Variable::set_in(
                        collection_variables.get_or_insert_with(Vec::new),
                        variable,
                        value,
                    );
                    applied.collection = true;
                }
            }
        }
        applied
    }
}

#[cfg(test)]
mod tests {
    use super::Applied;
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
        let mut variables = Some(vec![Variable::text("id", "1")]);
        let applied = report.apply(Some(&mut environment), &mut variables);
        assert_eq!(
            applied,
            Applied {
                environment: true,
                collection: true
            }
        );
        assert_eq!(environment.value_of("token").as_deref(), Some("t-1"));
        assert_eq!(environment.values.len(), 1);
        assert_eq!(variables.unwrap()[0].value_text(), "7");
        let mut without_environment = None;
        report.apply(None, &mut without_environment);
        assert_eq!(without_environment.map(|list| list.len()), Some(2));
    }

    #[test]
    fn nothing_extracted_leaves_variables_absent() {
        let report = RulesReport {
            checks: Vec::new(),
            extracted: vec![extracted("lost", VariableTarget::Collection, Err("x"))],
        };
        let mut variables = None;
        assert_eq!(report.apply(None, &mut variables), Applied::default());
        assert_eq!(variables, None);
    }
}
