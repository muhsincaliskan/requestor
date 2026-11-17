use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::variables::VariableScope;

pub const NO_AUTH_KIND: &str = "noauth";
pub const BEARER_KIND: &str = "bearer";
pub const BASIC_KIND: &str = "basic";
pub const API_KEY_KIND: &str = "apikey";
pub const TOKEN_KEY: &str = "token";
pub const USERNAME_KEY: &str = "username";
pub const PASSWORD_KEY: &str = "password";
pub const API_KEY_NAME_KEY: &str = "key";
pub const API_KEY_VALUE_KEY: &str = "value";
pub const API_KEY_LOCATION_KEY: &str = "in";
pub const API_KEY_IN_QUERY: &str = "query";
pub const API_KEY_IN_HEADER: &str = "header";

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

    fn set_text(&mut self, text: &str) {
        if self.value.as_ref().and_then(Value::as_str) != Some(text) {
            self.value = Some(Value::String(text.to_string()));
        }
    }
}

impl Auth {
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Auth::with_attributes(
            BASIC_KIND,
            vec![
                AuthAttribute::string(USERNAME_KEY, username),
                AuthAttribute::string(PASSWORD_KEY, password),
            ],
        )
    }

    pub fn none() -> Self {
        Auth {
            kind: NO_AUTH_KIND.to_string(),
            parameters: BTreeMap::new(),
        }
    }

    pub fn api_key(name: &str, value: &str, location: &str) -> Self {
        Auth::with_attributes(
            API_KEY_KIND,
            vec![
                AuthAttribute::string(API_KEY_NAME_KEY, name),
                AuthAttribute::string(API_KEY_VALUE_KEY, value),
                AuthAttribute::string(API_KEY_LOCATION_KEY, location),
            ],
        )
    }

    pub fn from_credentials(credentials: &str) -> Self {
        let (username, password) = credentials.split_once(':').unwrap_or((credentials, ""));
        Auth::basic(username, password)
    }

    pub fn bearer(token: impl Into<String>) -> Self {
        Auth::with_attributes(BEARER_KIND, vec![AuthAttribute::string(TOKEN_KEY, token)])
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

    pub fn with_kind(mut self, kind: &str) -> Self {
        self.kind = kind.to_string();
        self
    }

    pub fn set_attribute(&mut self, section: &str, key: &str, value: &str) {
        let attributes = self
            .parameters
            .entry(section.to_string())
            .or_default()
            .get_or_insert_with(Vec::new);
        match attributes
            .iter_mut()
            .find(|attribute| attribute.key.as_deref() == Some(key))
        {
            Some(attribute) => attribute.set_text(value),
            None => attributes.push(AuthAttribute::string(key, value)),
        }
    }

    pub fn is_none(&self) -> bool {
        self.kind == NO_AUTH_KIND
    }
}

#[cfg(test)]
mod tests {
    use super::Auth;

    const MIXED_AUTH: &str = r#"{"type":"bearer","bearer":[{"key":"token","value":"old","type":"string","x-attr":1}],
        "oauth2":[{"key":"accessToken","value":"t"}]}"#;

    #[test]
    fn setting_an_attribute_keeps_other_sections_and_attribute_extras() {
        let mut auth: Auth = serde_json::from_str(MIXED_AUTH).unwrap();
        auth.set_attribute("bearer", "token", "new");
        auth.set_attribute("bearer", "added", "x");
        let json = serde_json::to_value(&auth).unwrap();
        assert_eq!(json["bearer"][0]["value"], "new");
        assert_eq!(json["bearer"][0]["x-attr"], 1);
        assert_eq!(json["bearer"][1]["key"], "added");
        assert_eq!(json["oauth2"][0]["key"], "accessToken");
    }

    #[test]
    fn setting_an_unchanged_attribute_leaves_the_document_equal() {
        let original: Auth = serde_json::from_str(MIXED_AUTH).unwrap();
        let mut auth = original.clone();
        auth.set_attribute("bearer", "token", "old");
        assert_eq!(auth, original);
    }

    #[test]
    fn switching_kind_only_changes_the_type() {
        let auth: Auth = serde_json::from_str(MIXED_AUTH).unwrap();
        assert_eq!(auth.with_kind("oauth2").kind, "oauth2");
    }

    #[test]
    fn credentials_split_on_the_first_colon() {
        let auth = Auth::from_credentials("ada:p:w");
        assert_eq!(auth.value_of("username"), Some("ada"));
        assert_eq!(auth.value_of("password"), Some("p:w"));
        assert_eq!(Auth::from_credentials("ada").value_of("password"), Some(""));
    }
}
