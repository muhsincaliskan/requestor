use crate::collection::Param;
use crate::secret::{is_secret_name, REDACTED};
use crate::variables::VariableScope;

pub const CONTENT_TYPE: &str = "Content-Type";
pub const ACCEPT: &str = "Accept";

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

    pub fn shown_value(&self) -> &str {
        if self.is_sensitive() {
            REDACTED
        } else {
            &self.value
        }
    }
}

pub fn find_header<'a>(headers: &'a [Header], name: &str) -> Option<&'a Header> {
    headers.iter().find(|header| header.is_named(name))
}

impl Header {
    pub fn from_param(param: &Param, scope: &VariableScope) -> Option<Header> {
        let name = scope.resolve(param.key.as_deref()?).text;
        if name.trim().is_empty() {
            return None;
        }
        let value = scope
            .resolve(param.value.as_deref().unwrap_or_default())
            .text;
        Some(Header::new(name, value))
    }
}

#[cfg(test)]
mod tests {
    use super::Header;
    use crate::collection::Param;
    use crate::variables::VariableScope;

    #[test]
    fn rows_without_a_name_are_not_sent() {
        let scope = VariableScope::new();
        let mut described = Param::text("", "");
        described.description = Some(serde_json::from_str(r#""only a note""#).unwrap());
        assert_eq!(Header::from_param(&described, &scope), None);
        let named = Header::from_param(&Param::text("Accept", "*/*"), &scope);
        assert_eq!(named, Some(Header::new("Accept", "*/*")));
    }
}
