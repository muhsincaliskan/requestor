use serde_json::{Map, Value};

use super::BodyDraft;
use crate::graphql::GraphQlOperation;

const OPERATION_GAP: &str = "\n";

impl BodyDraft {
    pub fn add_graphql_operation(&mut self, operation: &GraphQlOperation) {
        if self.graphql_query.trim().is_empty() {
            self.graphql_query = operation.query.clone();
        } else {
            if !self.graphql_query.ends_with('\n') {
                self.graphql_query.push('\n');
            }
            self.graphql_query.push_str(OPERATION_GAP);
            self.graphql_query.push_str(&operation.query);
        }
        self.merge_graphql_variables(&operation.variables);
    }

    fn merge_graphql_variables(&mut self, added: &Map<String, Value>) {
        if added.is_empty() {
            return;
        }
        let existing = if self.graphql_variables.trim().is_empty() {
            Map::new()
        } else {
            match serde_json::from_str::<Value>(&self.graphql_variables) {
                Ok(Value::Object(existing)) => existing,
                _ => return,
            }
        };
        let mut merged = existing;
        for (name, value) in added {
            merged.entry(name.clone()).or_insert_with(|| value.clone());
        }
        if let Ok(text) = serde_json::to_string_pretty(&Value::Object(merged)) {
            self.graphql_variables = text;
        }
    }
}
