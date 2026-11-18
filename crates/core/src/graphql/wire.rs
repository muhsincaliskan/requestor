use serde::Deserialize;

use super::schema::{
    GraphQlSchema, OperationKind, SchemaArgument, SchemaError, SchemaField, SchemaType, TypeKind,
};
use super::type_ref::TypeRef;

const LIST_KIND: &str = "LIST";
const NON_NULL_KIND: &str = "NON_NULL";

#[derive(Deserialize)]
pub(super) struct IntrospectionResponse {
    data: Option<IntrospectionData>,
    #[serde(default)]
    errors: Vec<WireError>,
}

#[derive(Deserialize)]
struct WireError {
    message: String,
}

#[derive(Deserialize)]
struct IntrospectionData {
    #[serde(rename = "__schema")]
    schema: Option<WireSchema>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireSchema {
    query_type: Option<WireName>,
    mutation_type: Option<WireName>,
    subscription_type: Option<WireName>,
    #[serde(default)]
    types: Vec<WireType>,
}

#[derive(Deserialize)]
struct WireName {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireType {
    kind: String,
    name: Option<String>,
    description: Option<String>,
    fields: Option<Vec<WireField>>,
    input_fields: Option<Vec<WireInputValue>>,
    enum_values: Option<Vec<WireName>>,
}

#[derive(Deserialize)]
struct WireField {
    name: String,
    description: Option<String>,
    #[serde(default)]
    args: Vec<WireInputValue>,
    #[serde(rename = "type")]
    type_ref: WireTypeRef,
}

#[derive(Deserialize)]
struct WireInputValue {
    name: String,
    description: Option<String>,
    #[serde(rename = "type")]
    type_ref: WireTypeRef,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireTypeRef {
    kind: String,
    name: Option<String>,
    of_type: Option<Box<WireTypeRef>>,
}

impl IntrospectionResponse {
    pub(super) fn to_domain(&self) -> Result<GraphQlSchema, SchemaError> {
        if let Some(error) = self.errors.first() {
            return Err(SchemaError::Rejected(error.message.clone()));
        }
        let schema = self
            .data
            .as_ref()
            .and_then(|data| data.schema.as_ref())
            .ok_or(SchemaError::Missing)?;
        let roots = [
            (OperationKind::Query, &schema.query_type),
            (OperationKind::Mutation, &schema.mutation_type),
            (OperationKind::Subscription, &schema.subscription_type),
        ]
        .into_iter()
        .filter_map(|(kind, root)| Some((kind, root.as_ref()?.name.clone())))
        .collect();
        let types = schema
            .types
            .iter()
            .filter_map(WireType::to_domain)
            .collect();
        Ok(GraphQlSchema::new(roots, types))
    }
}

impl WireType {
    fn to_domain(&self) -> Option<SchemaType> {
        Some(SchemaType {
            name: self.name.clone()?,
            kind: TypeKind::from_wire(&self.kind),
            description: self.description.clone(),
            fields: self
                .fields
                .iter()
                .flatten()
                .filter_map(WireField::to_domain)
                .collect(),
            input_fields: self
                .input_fields
                .iter()
                .flatten()
                .filter_map(WireInputValue::to_domain)
                .collect(),
            enum_values: self
                .enum_values
                .iter()
                .flatten()
                .map(|value| value.name.clone())
                .collect(),
        })
    }
}

impl WireField {
    fn to_domain(&self) -> Option<SchemaField> {
        Some(SchemaField {
            name: self.name.clone(),
            description: self.description.clone(),
            arguments: self
                .args
                .iter()
                .filter_map(WireInputValue::to_domain)
                .collect(),
            type_ref: self.type_ref.to_domain()?,
        })
    }
}

impl WireInputValue {
    fn to_domain(&self) -> Option<SchemaArgument> {
        Some(SchemaArgument {
            name: self.name.clone(),
            description: self.description.clone(),
            type_ref: self.type_ref.to_domain()?,
        })
    }
}

impl WireTypeRef {
    fn to_domain(&self) -> Option<TypeRef> {
        match self.kind.as_str() {
            LIST_KIND => Some(TypeRef::List(Box::new(self.of_type.as_ref()?.to_domain()?))),
            NON_NULL_KIND => Some(TypeRef::NonNull(Box::new(
                self.of_type.as_ref()?.to_domain()?,
            ))),
            _ => self.name.clone().map(TypeRef::Named),
        }
    }
}
