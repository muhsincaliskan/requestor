use crate::collection::Param;
use crate::secret::is_secret_name;
use crate::variables::VariableScope;

pub const CONTENT_TYPE: &str = "Content-Type";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

impl Header {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Header {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn is_named(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
    }

    pub fn is_sensitive(&self) -> bool {
        is_secret_name(&self.name)
    }
}

pub fn find_header<'a>(headers: &'a [Header], name: &str) -> Option<&'a Header> {
    headers.iter().find(|header| header.is_named(name))
}

impl Header {
    pub fn from_param(param: &Param, scope: &VariableScope) -> Option<Header> {
        let name = scope.resolve(param.key.as_deref()?).text;
        let value = scope
            .resolve(param.value.as_deref().unwrap_or_default())
            .text;
        Some(Header::new(name, value))
    }
}
