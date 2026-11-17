mod rows;

pub use rows::{CheckKind, CheckRow, ExtractKind, ExtractRow};

use crate::checks::ResponseRules;
use crate::collection::Event;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RulesDraft {
    pub checks: Vec<CheckRow>,
    pub extractions: Vec<ExtractRow>,
    custom_script: bool,
}

impl RulesDraft {
    pub fn from_events(events: Option<&Vec<Event>>) -> Self {
        match ResponseRules::of_events(events) {
            Some(rules) => RulesDraft::from_rules(&rules),
            None => RulesDraft {
                custom_script: true,
                ..RulesDraft::default()
            },
        }
    }

    pub fn from_rules(rules: &ResponseRules) -> Self {
        RulesDraft {
            checks: rules.checks.iter().map(CheckRow::from_check).collect(),
            extractions: rules
                .extractions
                .iter()
                .map(ExtractRow::from_extraction)
                .collect(),
            custom_script: false,
        }
    }

    pub fn from_script(lines: &[String]) -> Self {
        let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
        ResponseRules::from_script(&borrowed)
            .map(|rules| RulesDraft::from_rules(&rules))
            .unwrap_or_default()
    }

    pub fn is_custom_script(&self) -> bool {
        self.custom_script
    }

    pub fn to_rules(&self) -> ResponseRules {
        ResponseRules {
            checks: self.checks.iter().filter_map(CheckRow::to_check).collect(),
            extractions: self
                .extractions
                .iter()
                .filter_map(ExtractRow::to_extraction)
                .collect(),
        }
    }

    pub fn active_count(&self) -> usize {
        let rules = self.to_rules();
        rules.checks.len() + rules.extractions.len()
    }

    pub fn script_lines(&self) -> Option<Vec<String>> {
        (!self.custom_script).then(|| self.to_rules().script_lines())
    }

    pub fn write_into(&self, events: &mut Option<Vec<Event>>) {
        if !self.custom_script {
            self.to_rules().write_into(events);
        }
    }
}

#[cfg(test)]
mod tests;
