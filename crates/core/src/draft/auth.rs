use crate::collection::auth::{
    API_KEY_IN_HEADER, API_KEY_IN_QUERY, API_KEY_KIND, API_KEY_LOCATION_KEY, API_KEY_NAME_KEY,
    API_KEY_VALUE_KEY, BASIC_KIND, BEARER_KIND, PASSWORD_KEY, TOKEN_KEY, USERNAME_KEY,
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
        let text = |key: &str| auth.value_of(key).unwrap_or_default().to_string();
        AuthDraft {
            kind: AuthKind::of(auth),
            token: text(TOKEN_KEY),
            username: text(USERNAME_KEY),
            password: text(PASSWORD_KEY),
            api_key_name: text(API_KEY_NAME_KEY),
            api_key_value: text(API_KEY_VALUE_KEY),
            api_key_in_query: auth.value_of(API_KEY_LOCATION_KEY) == Some(API_KEY_IN_QUERY),
            source: Some(auth.clone()),
        }
    }

    pub fn to_auth(&self) -> Option<Auth> {
        if self.is_unchanged() {
            return self.source.clone();
        }
        match self.kind {
            AuthKind::Inherit => None,
            AuthKind::None => Some(Auth::none()),
            AuthKind::Bearer => Some(Auth::bearer(self.token.clone())),
            AuthKind::Basic => Some(Auth::basic(self.username.clone(), self.password.clone())),
            AuthKind::ApiKey => Some(Auth::api_key(
                &self.api_key_name,
                &self.api_key_value,
                self.api_key_location(),
            )),
            AuthKind::Unsupported => self.source.clone(),
        }
    }

    pub fn unsupported_kind(&self) -> Option<&str> {
        (self.kind == AuthKind::Unsupported)
            .then(|| self.source.as_ref().map(|auth| auth.kind.as_str()))
            .flatten()
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
mod tests {
    use super::{AuthDraft, AuthKind};
    use crate::collection::Auth;

    #[test]
    fn missing_auth_means_inherit_and_stays_missing() {
        let draft = AuthDraft::from_auth(None);
        assert_eq!(draft.kind, AuthKind::Inherit);
        assert!(draft.to_auth().is_none());
    }

    #[test]
    fn untouched_and_unsupported_auth_is_preserved_exactly() {
        let oauth: Auth = serde_json::from_str(
            r#"{"type":"oauth2","oauth2":[{"key":"accessToken","value":"t","type":"string"}]}"#,
        )
        .unwrap();
        let draft = AuthDraft::from_auth(Some(&oauth));
        assert_eq!(draft.kind, AuthKind::Unsupported);
        assert_eq!(draft.unsupported_kind(), Some("oauth2"));
        assert_eq!(draft.to_auth(), Some(oauth));
    }

    #[test]
    fn edits_produce_the_postman_shape_for_each_kind() {
        let mut draft = AuthDraft::from_auth(Some(&Auth::bearer("old")));
        draft.token = "new".to_string();
        assert_eq!(draft.to_auth().unwrap().value_of("token"), Some("new"));
        draft.kind = AuthKind::ApiKey;
        draft.api_key_name = "X-Key".to_string();
        draft.api_key_value = "v".to_string();
        draft.api_key_in_query = true;
        let api_key = draft.to_auth().unwrap();
        assert_eq!(api_key.kind, "apikey");
        assert_eq!(api_key.value_of("in"), Some("query"));
        draft.kind = AuthKind::None;
        assert!(draft.to_auth().unwrap().is_none());
    }
}
