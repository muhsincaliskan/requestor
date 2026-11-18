use serde_json::{Map, Value};

use super::schema::{GraphQlSchema, OperationKind, SchemaArgument, SchemaField, TypeKind};

const INDENT: &str = "  ";
const OPEN_BLOCK: &str = " {";
const CLOSE_BLOCK: &str = "}";
const LINE_END: char = '\n';
const TYPENAME_FIELD: &str = "__typename";
const VARIABLE_PREFIX: &str = "$";
const LIST_SEPARATOR: &str = ", ";

#[derive(Debug, Clone, PartialEq)]
pub struct GraphQlOperation {
    pub query: String,
    pub variables: Map<String, Value>,
}

impl GraphQlSchema {
    pub fn operation(&self, kind: OperationKind, field_name: &str) -> Option<GraphQlOperation> {
        let field = self.root_field(kind, field_name)?;
        let mut lines = vec![format!(
            "{} {}{}{OPEN_BLOCK}",
            kind.keyword(),
            field.operation_name(),
            field.variable_declarations()
        )];
        let call = format!("{INDENT}{}{}", field.name, field.argument_list());
        let selection = self.selection_for(field);
        if selection.is_empty() {
            lines.push(call);
        } else {
            lines.push(format!("{call}{OPEN_BLOCK}"));
            lines.extend(
                selection
                    .iter()
                    .map(|name| format!("{INDENT}{INDENT}{name}")),
            );
            lines.push(format!("{INDENT}{CLOSE_BLOCK}"));
        }
        lines.push(CLOSE_BLOCK.to_string());
        let mut query = lines.join(&LINE_END.to_string());
        query.push(LINE_END);
        let variables = field
            .arguments
            .iter()
            .map(|argument| (argument.name.clone(), Value::Null))
            .collect();
        Some(GraphQlOperation { query, variables })
    }

    fn selection_for(&self, field: &SchemaField) -> Vec<String> {
        let typename = || vec![TYPENAME_FIELD.to_string()];
        let Some(returned) = self.type_named(field.type_ref.named()) else {
            return typename();
        };
        if returned.is_leaf() {
            return Vec::new();
        }
        if !matches!(returned.kind, TypeKind::Object | TypeKind::Interface) {
            return typename();
        }
        let leaves: Vec<String> = returned
            .fields
            .iter()
            .filter(|child| !child.needs_arguments())
            .filter(|child| {
                self.type_named(child.type_ref.named())
                    .is_some_and(|found| found.is_leaf())
            })
            .map(|child| child.name.clone())
            .collect();
        if leaves.is_empty() {
            typename()
        } else {
            leaves
        }
    }
}

impl GraphQlOperation {
    pub fn keeping_values_of(mut self, existing: &Map<String, Value>) -> Self {
        for (name, value) in self.variables.iter_mut() {
            if let Some(kept) = existing.get(name) {
                value.clone_from(kept);
            }
        }
        self
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
        self.listed(|argument| format!("{VARIABLE_PREFIX}{}", argument.declaration()))
    }

    fn argument_list(&self) -> String {
        self.listed(|argument| format!("{0}: {VARIABLE_PREFIX}{0}", argument.name))
    }

    fn listed(&self, show: impl Fn(&SchemaArgument) -> String) -> String {
        if self.arguments.is_empty() {
            return String::new();
        }
        let shown: Vec<String> = self.arguments.iter().map(show).collect();
        format!("({})", shown.join(LIST_SEPARATOR))
    }
}
