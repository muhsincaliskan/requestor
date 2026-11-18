use requestor_core::graphql::{FieldFilter, GraphQlSchema, OperationKind, SchemaField};

pub(super) enum ExplorerRow<'a> {
    Heading(OperationKind),
    Field(OperationKind, &'a SchemaField),
}

impl<'a> ExplorerRow<'a> {
    pub(super) fn listed(schema: &'a GraphQlSchema, filter: &FieldFilter) -> Vec<Self> {
        let mut rows = Vec::new();
        for kind in OperationKind::ALL {
            let start = rows.len();
            rows.extend(
                schema
                    .root_fields(kind)
                    .iter()
                    .filter(|field| field.matches(filter))
                    .map(|field| ExplorerRow::Field(kind, field)),
            );
            if rows.len() > start {
                rows.insert(start, ExplorerRow::Heading(kind));
            }
        }
        rows
    }
}
