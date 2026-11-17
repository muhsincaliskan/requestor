use crate::collection::auth::{
    API_KEY_IN_HEADER, API_KEY_IN_QUERY, API_KEY_KIND, API_KEY_LOCATION_KEY, API_KEY_NAME_KEY,
    API_KEY_VALUE_KEY, BASIC_KIND, BEARER_KIND, NO_AUTH_KIND, PASSWORD_KEY, TOKEN_KEY,
    USERNAME_KEY,
};
use crate::collection::Auth;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthKind {
    #[default]
    Inherit,
    None,
    Bearer,
    Basic,
    ApiKey,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AuthDraft {
    pub kind: AuthKind,
    pub token: String,
    pub username: String,
    pub password: String,
    pub api_key_name: String,
    pub api_key_value: String,
    pub api_key_in_query: bool,
    source: Option<Auth>,
}

impl AuthKind {
    pub const SELECTABLE: [AuthKind; 5] = [
        AuthKind::Inherit,
        AuthKind::None,
        AuthKind::Bearer,
        AuthKind::Basic,
        AuthKind::ApiKey,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AuthKind::Inherit => "Inherit from parent",
            AuthKind::None => "No auth",
            AuthKind::Bearer => "Bearer token",
            AuthKind::Basic => "Basic auth",
            AuthKind::ApiKey => "API key",
            AuthKind::Unsupported => "Unsupported type",
        }
    }

    fn of(auth: &Auth) -> Self {
        if auth.is_none() {
            return AuthKind::None;
        }
        match auth.kind.as_str() {
            BEARER_KIND => AuthKind::Bearer,
            BASIC_KIND => AuthKind::Basic,
            API_KEY_KIND => AuthKind::ApiKey,
            _ => AuthKind::Unsupported,
        }
    }
}

impl AuthDraft {
    pub fn from_auth(auth: Option<&Auth>) -> Self {
        let Some(auth) = auth else {
            return AuthDraft::default();
        };
        let text = |section: &str, key: &str| {
            auth.section_value(section, key)
                .unwrap_or_default()
                .to_string()
        };
        AuthDraft {
            kind: AuthKind::of(auth),
            token: text(BEARER_KIND, TOKEN_KEY),
            username: text(BASIC_KIND, USERNAME_KEY),
            password: text(BASIC_KIND, PASSWORD_KEY),
            api_key_name: text(API_KEY_KIND, API_KEY_NAME_KEY),
            api_key_value: text(API_KEY_KIND, API_KEY_VALUE_KEY),
            api_key_in_query: auth.section_value(API_KEY_KIND, API_KEY_LOCATION_KEY)
                == Some(API_KEY_IN_QUERY),
            source: Some(auth.clone()),
        }
    }

    pub fn to_auth(&self) -> Option<Auth> {
        if self.is_unchanged() {
            return self.source.clone();
        }
        match self.kind {
            AuthKind::Inherit => None,
            AuthKind::None => Some(self.based_on(NO_AUTH_KIND)),
            AuthKind::Bearer => Some(self.edited(BEARER_KIND, &[(TOKEN_KEY, &self.token)])),
            AuthKind::Basic => Some(self.edited(
                BASIC_KIND,
                &[
                    (USERNAME_KEY, &self.username),
                    (PASSWORD_KEY, &self.password),
                ],
            )),
            AuthKind::ApiKey => Some(self.edited(
                API_KEY_KIND,
                &[
                    (API_KEY_NAME_KEY, &self.api_key_name),
                    (API_KEY_VALUE_KEY, &self.api_key_value),
                    (API_KEY_LOCATION_KEY, self.api_key_location()),
                ],
            )),
            AuthKind::Unsupported => self.source.clone(),
        }
    }

    pub fn unsupported_kind(&self) -> Option<&str> {
        (self.kind == AuthKind::Unsupported)
            .then(|| self.source.as_ref().map(|auth| auth.kind.as_str()))
            .flatten()
    }

    fn edited(&self, kind: &str, attributes: &[(&str, &str)]) -> Auth {
        let mut auth = self.based_on(kind);
        for (key, value) in attributes {
            auth.set_attribute(kind, key, value);
        }
        auth
    }

    fn based_on(&self, kind: &str) -> Auth {
        self.source
            .clone()
            .unwrap_or_else(Auth::none)
            .with_kind(kind)
    }

    fn api_key_location(&self) -> &'static str {
        if self.api_key_in_query {
            API_KEY_IN_QUERY
        } else {
            API_KEY_IN_HEADER
        }
    }

    fn is_unchanged(&self) -> bool {
        self.source
            .as_ref()
            .is_some_and(|auth| AuthDraft::from_auth(Some(auth)) == *self)
    }
}

#[cfg(test)]
mod tests;
