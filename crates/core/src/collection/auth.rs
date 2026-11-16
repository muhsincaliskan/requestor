use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::variables::VariableScope;

const NO_AUTH_KIND: &str = "noauth";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthAttribute {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Auth {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub parameters: BTreeMap<String, Option<Vec<AuthAttribute>>>,
}

impl AuthAttribute {
    pub fn string(key: &str, value: impl Into<String>) -> Self {
        AuthAttribute {
            key: Some(key.to_string()),
            value: Some(Value::String(value.into())),
            kind: Some("string".to_string()),
            extra: Map::new(),
        }
    }
}

impl Auth {
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Auth::with_attributes(
            "basic",
            vec![
                AuthAttribute::string("username", username),
                AuthAttribute::string("password", password),
            ],
        )
    }

    pub fn from_credentials(credentials: &str) -> Self {
        let (username, password) = credentials.split_once(':').unwrap_or((credentials, ""));
        Auth::basic(username, password)
    }

    pub fn bearer(token: impl Into<String>) -> Self {
        Auth::with_attributes("bearer", vec![AuthAttribute::string("token", token)])
    }

    fn with_attributes(kind: &str, attributes: Vec<AuthAttribute>) -> Self {
        Auth {
            kind: kind.to_string(),
            parameters: BTreeMap::from([(kind.to_string(), Some(attributes))]),
        }
    }

    pub fn value_of(&self, key: &str) -> Option<&str> {
        self.parameters
            .get(&self.kind)?
            .as_ref()?
            .iter()
            .find(|attribute| attribute.key.as_deref() == Some(key))
            .and_then(|attribute| attribute.value.as_ref())
            .and_then(Value::as_str)
    }

    pub fn resolved_value_of(&self, key: &str, scope: &VariableScope) -> Option<String> {
        self.value_of(key)
            .map(|value| scope.resolve(value).text)
            .filter(|value| !value.is_empty())
    }

    pub fn is_none(&self) -> bool {
        self.kind == NO_AUTH_KIND
    }
}

#[cfg(test)]
mod tests {
    use super::Auth;

    #[test]
    fn credentials_split_on_the_first_colon() {
        let auth = Auth::from_credentials("ada:p:w");
        assert_eq!(auth.value_of("username"), Some("ada"));
        assert_eq!(auth.value_of("password"), Some("p:w"));
        assert_eq!(Auth::from_credentials("ada").value_of("password"), Some(""));
    }
}
