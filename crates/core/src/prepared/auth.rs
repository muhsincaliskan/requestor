use super::PreparedRequest;
use crate::collection::Auth;
use crate::encoding::{base64_encode, percent_encode};
use crate::variables::VariableScope;

const AUTHORIZATION: &str = "Authorization";
const BEARER_KIND: &str = "bearer";
const BASIC_KIND: &str = "basic";
const API_KEY_KIND: &str = "apikey";
const API_KEY_IN_QUERY: &str = "query";
const FRAGMENT_MARK: char = '#';

impl PreparedRequest {
    pub(super) fn apply_auth(&mut self, authentication: &Auth, scope: &VariableScope) {
        let value_of = |key: &str| {
            authentication
                .value_of(key)
                .map(|value| scope.resolve(value).text)
        };
        match authentication.kind.as_str() {
            BEARER_KIND => self.apply_bearer(value_of("token")),
            BASIC_KIND => self.apply_basic(value_of("username"), value_of("password")),
            API_KEY_KIND => self.apply_api_key(value_of("key"), value_of("value"), value_of("in")),
            _ => {}
        }
    }

    fn apply_bearer(&mut self, token: Option<String>) {
        if let Some(token) = token {
            self.add_header_if_absent(AUTHORIZATION, format!("Bearer {token}"));
        }
    }

    fn apply_basic(&mut self, username: Option<String>, password: Option<String>) {
        let Some(username) = username else { return };
        let credentials = format!("{username}:{}", password.unwrap_or_default());
        self.add_header_if_absent(
            AUTHORIZATION,
            format!("Basic {}", base64_encode(credentials.as_bytes())),
        );
    }

    fn apply_api_key(
        &mut self,
        key: Option<String>,
        value: Option<String>,
        location: Option<String>,
    ) {
        let (Some(key), Some(value)) = (key, value) else {
            return;
        };
        if location.as_deref() == Some(API_KEY_IN_QUERY) {
            self.append_query_pair(&key, &value);
        } else {
            self.add_header_if_absent(&key, value);
        }
    }

    fn append_query_pair(&mut self, key: &str, value: &str) {
        let (address, fragment) = match self.url.split_once(FRAGMENT_MARK) {
            Some((address, fragment)) => {
                (address.to_string(), format!("{FRAGMENT_MARK}{fragment}"))
            }
            None => (self.url.clone(), String::new()),
        };
        let separator = if address.contains('?') { '&' } else { '?' };
        self.url = format!(
            "{address}{separator}{}={}{fragment}",
            percent_encode(key),
            percent_encode(value)
        );
    }
}
