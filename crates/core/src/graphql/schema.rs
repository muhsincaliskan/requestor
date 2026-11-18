use std::collections::BTreeMap;
use std::fmt;

use super::type_ref::TypeRef;
use super::wire::IntrospectionResponse;
use crate::response::HttpResponse;

const ARGUMENT_SEPARATOR: &str = ", ";
const NOT_JSON: &str = "The server did not answer with JSON";
const MISSING: &str = "The answer holds no schema. The server may have introspection turned off";
const REJECTED: &str = "The server refused the schema request";
const STATUS: &str = "The server answered the schema request with status";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperationKind {
    Query,
    Mutation,
    Subscription,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Scalar,
    Object,
    Interface,
    Union,
    Enum,
    InputObject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaArgument {
    pub name: String,
    pub description: Option<String>,
    pub type_ref: TypeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaField {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<SchemaArgument>,
    pub type_ref: TypeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaType {
    pub name: String,
    pub kind: TypeKind,
    pub description: Option<String>,
    pub fields: Vec<SchemaField>,
    pub input_fields: Vec<SchemaArgument>,
    pub enum_values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphQlSchema {
    roots: BTreeMap<OperationKind, String>,
    types: BTreeMap<String, SchemaType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    NotJson,
    Missing,
    Rejected(String),
    Status(u16),
}

impl OperationKind {
    pub const ALL: [OperationKind; 3] = [
        OperationKind::Query,
        OperationKind::Mutation,
        OperationKind::Subscription,
    ];

    pub fn label(self) -> &'static str {
        match self {
            OperationKind::Query => "Query",
            OperationKind::Mutation => "Mutation",
            OperationKind::Subscription => "Subscription",
        }
    }

    pub fn keyword(self) -> &'static str {
        match self {
            OperationKind::Query => "query",
            OperationKind::Mutation => "mutation",
            OperationKind::Subscription => "subscription",
        }
    }
}

impl TypeKind {
    pub(super) fn from_wire(kind: &str) -> Self {
        match kind {
            "OBJECT" => TypeKind::Object,
            "INTERFACE" => TypeKind::Interface,
            "UNION" => TypeKind::Union,
            "ENUM" => TypeKind::Enum,
            "INPUT_OBJECT" => TypeKind::InputObject,
            _ => TypeKind::Scalar,
        }
    }
}

impl SchemaField {
    pub fn signature(&self) -> String {
        if self.arguments.is_empty() {
            return format!("{}: {}", self.name, self.type_ref);
        }
        let arguments: Vec<String> = self
            .arguments
            .iter()
            .map(|argument| format!("{}: {}", argument.name, argument.type_ref))
            .collect();
        format!(
            "{}({}): {}",
            self.name,
            arguments.join(ARGUMENT_SEPARATOR),
            self.type_ref
        )
    }

    pub fn matches(&self, filter: &str) -> bool {
        let filter = filter.trim().to_lowercase();
        filter.is_empty() || self.signature().to_lowercase().contains(&filter)
    }

    pub(super) fn needs_arguments(&self) -> bool {
        self.arguments
            .iter()
            .any(|argument| argument.type_ref.is_required())
    }
}

impl SchemaType {
    pub fn is_leaf(&self) -> bool {
        matches!(self.kind, TypeKind::Scalar | TypeKind::Enum)
    }
}

impl GraphQlSchema {
    pub fn from_response(body: &[u8]) -> Result<Self, SchemaError> {
        serde_json::from_slice::<IntrospectionResponse>(body)
            .map_err(|_| SchemaError::NotJson)?
            .to_domain()
    }

    pub(super) fn new(roots: BTreeMap<OperationKind, String>, types: Vec<SchemaType>) -> Self {
        GraphQlSchema {
            roots,
            types: types
                .into_iter()
                .map(|found| (found.name.clone(), found))
                .collect(),
        }
    }

    pub fn type_named(&self, name: &str) -> Option<&SchemaType> {
        self.types.get(name)
    }

    pub fn type_count(&self) -> usize {
        self.types
            .keys()
            .filter(|name| !name.starts_with("__"))
            .count()
    }

    pub fn root_fields(&self, kind: OperationKind) -> &[SchemaField] {
        self.roots
            .get(&kind)
            .and_then(|name| self.types.get(name))
            .map_or(&[], |root| root.fields.as_slice())
    }

    pub fn root_field(&self, kind: OperationKind, name: &str) -> Option<&SchemaField> {
        self.root_fields(kind)
            .iter()
            .find(|field| field.name == name)
    }
}

impl HttpResponse {
    pub fn graphql_schema(&self) -> Result<GraphQlSchema, SchemaError> {
        match GraphQlSchema::from_response(&self.body) {
            Err(SchemaError::NotJson) if !self.is_success() => {
                Err(SchemaError::Status(self.status))
            }
            parsed => parsed,
        }
    }
}

impl fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaError::NotJson => formatter.write_str(NOT_JSON),
            SchemaError::Missing => formatter.write_str(MISSING),
            SchemaError::Rejected(reason) => write!(formatter, "{REJECTED}: {reason}"),
            SchemaError::Status(status) => write!(formatter, "{STATUS} {status}"),
        }
    }
}

impl std::error::Error for SchemaError {}
