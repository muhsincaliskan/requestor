use serde_json::{Map, Value};

use super::BodyDraft;
use crate::graphql::GraphQlOperation;

impl BodyDraft {
    pub fn use_graphql_operation(&mut self, operation: GraphQlOperation) {
        let existing = match serde_json::from_str::<Value>(&self.graphql_variables) {
            Ok(Value::Object(existing)) => existing,
            _ => Map::new(),
        };
        let operation = operation.keeping_values_of(&existing);
        self.graphql_query = operation.query;
        self.graphql_variables = if operation.variables.is_empty() {
            String::new()
        } else {
            serde_json::to_string_pretty(&Value::Object(operation.variables)).unwrap_or_default()
        };
    }
}
