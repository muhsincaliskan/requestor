use super::PreparedRequest;
use crate::collection::auth::{
    API_KEY_IN_QUERY, API_KEY_KIND, API_KEY_LOCATION_KEY, API_KEY_NAME_KEY, API_KEY_VALUE_KEY,
    BASIC_KIND, BEARER_KIND, PASSWORD_KEY, TOKEN_KEY, USERNAME_KEY,
};
use crate::collection::Auth;
use crate::encoding::{base64_encode, percent_encode};
use crate::variables::VariableScope;

const AUTHORIZATION: &str = "Authorization";
const FRAGMENT_MARK: char = '#';
const QUERY_MARK: char = '?';
const QUERY_SEPARATOR: char = '&';

impl PreparedRequest {
    pub(super) fn apply_auth(&mut self, authentication: &Auth, scope: &VariableScope) {
        let value_of = |key: &str| authentication.resolved_value_of(key, scope);
        match authentication.kind.as_str() {
            BEARER_KIND => self.apply_bearer(value_of(TOKEN_KEY)),
            BASIC_KIND => self.apply_basic(value_of(USERNAME_KEY), value_of(PASSWORD_KEY)),
            API_KEY_KIND => self.apply_api_key(
                value_of(API_KEY_NAME_KEY),
                value_of(API_KEY_VALUE_KEY),
                value_of(API_KEY_LOCATION_KEY),
            ),
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
        let separator = match address.chars().last() {
            Some(QUERY_MARK | QUERY_SEPARATOR) => String::new(),
            _ if address.contains(QUERY_MARK) => QUERY_SEPARATOR.to_string(),
            _ => QUERY_MARK.to_string(),
        };
        self.url = format!(
            "{address}{separator}{}={}{fragment}",
            percent_encode(key),
            percent_encode(value)
        );
    }
}
