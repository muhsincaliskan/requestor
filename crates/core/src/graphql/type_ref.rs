use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    Named(String),
    List(Box<TypeRef>),
    NonNull(Box<TypeRef>),
}

impl TypeRef {
    pub fn named(&self) -> &str {
        match self {
            TypeRef::Named(name) => name,
            TypeRef::List(inner) | TypeRef::NonNull(inner) => inner.named(),
        }
    }

    pub fn is_required(&self) -> bool {
        matches!(self, TypeRef::NonNull(_))
    }
}

impl fmt::Display for TypeRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeRef::Named(name) => formatter.write_str(name),
            TypeRef::List(inner) => write!(formatter, "[{inner}]"),
            TypeRef::NonNull(inner) => write!(formatter, "{inner}!"),
        }
    }
}
