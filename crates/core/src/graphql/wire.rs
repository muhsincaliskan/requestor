use serde::Deserialize;

use super::schema::{
    GraphQlSchema, OperationKind, SchemaArgument, SchemaError, SchemaField, SchemaType, TypeKind,
};
use super::type_ref::TypeRef;
use crate::response::HttpResponse;

const LIST_KIND: &str = "LIST";
const NON_NULL_KIND: &str = "NON_NULL";
const OBJECT_KIND: &str = "OBJECT";
const INTERFACE_KIND: &str = "INTERFACE";
const UNION_KIND: &str = "UNION";
const ENUM_KIND: &str = "ENUM";
const INPUT_OBJECT_KIND: &str = "INPUT_OBJECT";

#[derive(Deserialize)]
struct IntrospectionResponse {
    data: Option<IntrospectionData>,
    errors: Option<Vec<WireError>>,
}

#[derive(Deserialize)]
struct WireError {
    message: Option<String>,
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
    types: Option<Vec<WireType>>,
}

#[derive(Deserialize)]
struct WireName {
    name: String,
}

#[derive(Deserialize)]
struct WireType {
    kind: String,
    name: Option<String>,
    fields: Option<Vec<WireField>>,
}

#[derive(Deserialize)]
struct WireField {
    name: String,
    description: Option<String>,
    args: Option<Vec<WireInputValue>>,
    #[serde(rename = "type")]
    type_ref: WireTypeRef,
}

#[derive(Deserialize)]
struct WireInputValue {
    name: String,
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

impl HttpResponse {
    pub fn graphql_schema(&self) -> Result<GraphQlSchema, SchemaError> {
        if self.truncated {
            return Err(SchemaError::TooLarge);
        }
        match GraphQlSchema::from_response(&self.body) {
            Err(SchemaError::NotJson) if !self.is_success() => {
                Err(SchemaError::Status(self.status))
            }
            parsed => parsed,
        }
    }
}

impl GraphQlSchema {
    pub fn from_response(body: &[u8]) -> Result<Self, SchemaError> {
        serde_json::from_slice::<IntrospectionResponse>(body)
            .map_err(|_| SchemaError::NotJson)?
            .into_domain()
    }
}

impl IntrospectionResponse {
    fn into_domain(self) -> Result<GraphQlSchema, SchemaError> {
        let Some(schema) = self.data.and_then(|data| data.schema) else {
            let first = self.errors.into_iter().flatten().next();
            return Err(match first {
                Some(error) => SchemaError::Rejected(error.message.unwrap_or_default()),
                None => SchemaError::Missing,
            });
        };
        let roots = [
            (OperationKind::Query, schema.query_type),
            (OperationKind::Mutation, schema.mutation_type),
            (OperationKind::Subscription, schema.subscription_type),
        ]
        .into_iter()
        .filter_map(|(kind, root)| Some((kind, root?.name)))
        .collect();
        let types = schema
            .types
            .into_iter()
            .flatten()
            .filter_map(|found| found.into_domain().transpose())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(GraphQlSchema::new(roots, types))
    }
}

impl WireType {
    fn into_domain(self) -> Result<Option<SchemaType>, SchemaError> {
        let kind = self.kind();
        let Some(name) = self.name else {
            return Ok(None);
        };
        let fields = self
            .fields
            .into_iter()
            .flatten()
            .map(WireField::into_domain)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(SchemaType { name, kind, fields }))
    }

    fn kind(&self) -> TypeKind {
        match self.kind.as_str() {
            OBJECT_KIND => TypeKind::Object,
            INTERFACE_KIND => TypeKind::Interface,
            UNION_KIND => TypeKind::Union,
            ENUM_KIND => TypeKind::Enum,
            INPUT_OBJECT_KIND => TypeKind::InputObject,
            _ => TypeKind::Scalar,
        }
    }
}

impl WireField {
    fn into_domain(self) -> Result<SchemaField, SchemaError> {
        let arguments = self
            .args
            .into_iter()
            .flatten()
            .map(WireInputValue::into_domain)
            .collect::<Result<Vec<_>, _>>()?;
        let type_ref = self.type_ref.into_domain()?;
        Ok(SchemaField::new(
            self.name,
            self.description,
            arguments,
            type_ref,
        ))
    }
}

impl WireInputValue {
    fn into_domain(self) -> Result<SchemaArgument, SchemaError> {
        Ok(SchemaArgument {
            name: self.name,
            type_ref: self.type_ref.into_domain()?,
        })
    }
}

impl WireTypeRef {
    fn into_domain(self) -> Result<TypeRef, SchemaError> {
        let inner = |of_type: Option<Box<WireTypeRef>>| {
            of_type
                .ok_or(SchemaError::TooDeep)
                .and_then(|inner| inner.into_domain())
                .map(Box::new)
        };
        match self.kind.as_str() {
            LIST_KIND => inner(self.of_type).map(TypeRef::List),
            NON_NULL_KIND => inner(self.of_type).map(TypeRef::NonNull),
            _ => self.name.map(TypeRef::Named).ok_or(SchemaError::TooDeep),
        }
    }
}
