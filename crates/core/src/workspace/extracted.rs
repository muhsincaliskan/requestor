use super::Workspace;
use crate::checks::RulesReport;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChangedStores {
    pub environment_id: Option<String>,
    pub collection_id: Option<String>,
}

impl Workspace {
    pub fn store_extracted(
        &mut self,
        report: &RulesReport,
        collection_id: Option<&str>,
    ) -> ChangedStores {
        let active = self.session.active_environment.clone();
        let environment = self
            .environments
            .iter_mut()
            .find(|environment| environment.id.is_some() && environment.id == active);
        let collection = collection_id.and_then(|id| {
            self.collections
                .iter_mut()
                .find(|collection| collection.id() == id)
        });
        let mut scratch = None;
        let variables = match collection {
            Some(collection) => &mut collection.variable,
            None => &mut scratch,
        };
        let applied = report.apply(environment, variables);
        ChangedStores {
            environment_id: active.filter(|_| applied.environment),
            collection_id: collection_id
                .filter(|_| applied.collection)
                .map(str::to_string),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChangedStores;
    use crate::checks::{ExtractSource, Extracted, Extraction, RulesReport, VariableTarget};
    use crate::collection::Collection;
    use crate::environment::Environment;
    use crate::workspace::Workspace;

    fn report(target: VariableTarget) -> RulesReport {
        RulesReport {
            checks: Vec::new(),
            extracted: vec![Extracted {
                extraction: Extraction {
                    variable: "token".to_string(),
                    source: ExtractSource::Header("X".to_string()),
                    target,
                },
                value: Ok("t-1".to_string()),
            }],
        }
    }

    #[test]
    fn values_land_in_the_active_environment_or_the_collection() {
        let mut workspace = Workspace::default();
        let collection_id = workspace.add_collection(Collection::new("C"));
        let changed =
            workspace.store_extracted(&report(VariableTarget::Environment), Some(&collection_id));
        assert_eq!(
            changed.collection_id.as_deref(),
            Some(collection_id.as_str())
        );
        let environment_id = workspace.add_environment(Environment::new("E"));
        workspace.session.active_environment = Some(environment_id.clone());
        let changed =
            workspace.store_extracted(&report(VariableTarget::Environment), Some(&collection_id));
        assert_eq!(
            changed,
            ChangedStores {
                environment_id: Some(environment_id.clone()),
                collection_id: None,
            }
        );
        let stored = workspace.environment(&environment_id).unwrap();
        assert_eq!(stored.value_of("token").as_deref(), Some("t-1"));
        let lost = workspace.store_extracted(&report(VariableTarget::Collection), None);
        assert_eq!(lost, ChangedStores::default());
    }
}
