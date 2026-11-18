use serde_json::{Map, Value};

use super::schema::{GraphQlSchema, OperationKind, SchemaField, TypeKind};

const INDENT: &str = "  ";
const TYPENAME_FIELD: &str = "__typename";
const VARIABLE_SEPARATOR: &str = ", ";

#[derive(Debug, Clone, PartialEq)]
pub struct GraphQlOperation {
    pub query: String,
    pub variables: Map<String, Value>,
}

impl GraphQlSchema {
    pub fn operation(&self, kind: OperationKind, field_name: &str) -> Option<GraphQlOperation> {
        let field = self.root_field(kind, field_name)?;
        let declarations = field.variable_declarations();
        let mut query = format!(
            "{} {}{declarations} {{\n",
            kind.keyword(),
            field.operation_name()
        );
        query.push_str(&format!("{INDENT}{}{}", field.name, field.argument_list()));
        let selection = self.selection_for(field);
        if selection.is_empty() {
            query.push('\n');
        } else {
            query.push_str(" {\n");
            for name in selection {
                query.push_str(&format!("{INDENT}{INDENT}{name}\n"));
            }
            query.push_str(&format!("{INDENT}}}\n"));
        }
        query.push_str("}\n");
        let variables = field
            .arguments
            .iter()
            .map(|argument| (argument.name.clone(), Value::Null))
            .collect();
        Some(GraphQlOperation { query, variables })
    }

    fn selection_for(&self, field: &SchemaField) -> Vec<String> {
        let Some(returned) = self.type_named(field.type_ref.named()) else {
            return Vec::new();
        };
        if returned.is_leaf() {
            return Vec::new();
        }
        let leaves: Vec<String> = match returned.kind {
            TypeKind::Object | TypeKind::Interface => returned
                .fields
                .iter()
                .filter(|child| !child.needs_arguments())
                .filter(|child| {
                    self.type_named(child.type_ref.named())
                        .is_some_and(|found| found.is_leaf())
                })
                .map(|child| child.name.clone())
                .collect(),
            _ => Vec::new(),
        };
        if leaves.is_empty() {
            vec![TYPENAME_FIELD.to_string()]
        } else {
            leaves
        }
    }
}

impl SchemaField {
    fn operation_name(&self) -> String {
        let mut characters = self.name.chars();
        characters
            .next()
            .map(|first| first.to_uppercase().chain(characters).collect())
            .unwrap_or_default()
    }

    fn variable_declarations(&self) -> String {
        if self.arguments.is_empty() {
            return String::new();
        }
        let declared: Vec<String> = self
            .arguments
            .iter()
            .map(|argument| format!("${}: {}", argument.name, argument.type_ref))
            .collect();
        format!("({})", declared.join(VARIABLE_SEPARATOR))
    }

    fn argument_list(&self) -> String {
        if self.arguments.is_empty() {
            return String::new();
        }
        let passed: Vec<String> = self
            .arguments
            .iter()
            .map(|argument| format!("{0}: ${0}", argument.name))
            .collect();
        format!("({})", passed.join(VARIABLE_SEPARATOR))
    }
}
