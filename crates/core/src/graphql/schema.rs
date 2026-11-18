use std::collections::BTreeMap;
use std::fmt;

use super::type_ref::TypeRef;

const INTROSPECTION_PREFIX: &str = "__";
const ARGUMENT_SEPARATOR: &str = ", ";
const NOT_JSON: &str = "The server did not answer with JSON";
const MISSING: &str = "The answer holds no schema. The server may have introspection turned off";
const REJECTED: &str = "The server refused the schema request";
const STATUS: &str = "The server answered the schema request with status";
const TOO_LARGE: &str = "The schema is larger than the response size limit in Settings";
const TOO_DEEP: &str = "The schema nests a type deeper than Requestor can read";

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
    pub type_ref: TypeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaField {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<SchemaArgument>,
    pub type_ref: TypeRef,
    signature: String,
    search_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldFilter(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaType {
    pub name: String,
    pub kind: TypeKind,
    pub fields: Vec<SchemaField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphQlSchema {
    roots: BTreeMap<OperationKind, String>,
    types: BTreeMap<String, SchemaType>,
    type_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    NotJson,
    Missing,
    Rejected(String),
    Status(u16),
    TooLarge,
    TooDeep,
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

impl SchemaArgument {
    pub fn declaration(&self) -> String {
        format!("{}: {}", self.name, self.type_ref)
    }
}

impl SchemaField {
    pub fn new(
        name: String,
        description: Option<String>,
        arguments: Vec<SchemaArgument>,
        type_ref: TypeRef,
    ) -> Self {
        let signature = if arguments.is_empty() {
            format!("{name}: {type_ref}")
        } else {
            let declared: Vec<String> = arguments.iter().map(SchemaArgument::declaration).collect();
            format!("{name}({}): {type_ref}", declared.join(ARGUMENT_SEPARATOR))
        };
        SchemaField {
            search_key: signature.to_lowercase(),
            signature,
            name,
            description,
            arguments,
            type_ref,
        }
    }

    pub fn signature(&self) -> &str {
        &self.signature
    }

    pub fn matches(&self, filter: &FieldFilter) -> bool {
        self.search_key.contains(&filter.0)
    }

    pub(super) fn needs_arguments(&self) -> bool {
        self.arguments
            .iter()
            .any(|argument| argument.type_ref.is_required())
    }
}

impl FieldFilter {
    pub fn new(text: &str) -> Self {
        FieldFilter(text.trim().to_lowercase())
    }
}

impl SchemaType {
    pub fn is_leaf(&self) -> bool {
        matches!(self.kind, TypeKind::Scalar | TypeKind::Enum)
    }
}

impl GraphQlSchema {
    pub(super) fn new(roots: BTreeMap<OperationKind, String>, types: Vec<SchemaType>) -> Self {
        let type_count = types
            .iter()
            .filter(|found| !found.name.starts_with(INTROSPECTION_PREFIX))
            .count();
        GraphQlSchema {
            roots,
            types: types
                .into_iter()
                .map(|found| (found.name.clone(), found))
                .collect(),
            type_count,
        }
    }

    pub fn type_named(&self, name: &str) -> Option<&SchemaType> {
        self.types.get(name)
    }

    pub fn type_count(&self) -> usize {
        self.type_count
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

impl fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaError::NotJson => formatter.write_str(NOT_JSON),
            SchemaError::Missing => formatter.write_str(MISSING),
            SchemaError::Rejected(reason) if reason.is_empty() => formatter.write_str(REJECTED),
            SchemaError::Rejected(reason) => write!(formatter, "{REJECTED}: {reason}"),
            SchemaError::Status(status) => write!(formatter, "{STATUS} {status}"),
            SchemaError::TooLarge => formatter.write_str(TOO_LARGE),
            SchemaError::TooDeep => formatter.write_str(TOO_DEEP),
        }
    }
}

impl std::error::Error for SchemaError {}
