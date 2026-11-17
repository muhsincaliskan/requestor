use std::fmt;

use serde::{Deserialize, Serialize};

use crate::collection::{Auth, Url, SCHEME_SEPARATOR};
use crate::encoding::percent_encode;
use crate::variables::VariableScope;

const MASK: &str = "********";
pub const REDACTED: &str = "<redacted>";
const SECRET_NAME_MARKERS: [&str; 9] = [
    "auth",
    "cookie",
    "key",
    "token",
    "secret",
    "password",
    "passwd",
    "signature",
    "credential",
];

const PUBLIC_AUTH_KEYS: [&str; 6] = [
    "in",
    "key",
    "username",
    "addTokenTo",
    "tokenType",
    "headerPrefix",
];
const FRAGMENT_MARK: char = '#';
const QUERY_MARK: char = '?';
const PAIR_SEPARATOR: &str = "&";
const VALUE_SEPARATOR: char = '=';
const PATH_START: char = '/';
const USERINFO_END: char = '@';

pub fn is_secret_name(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    SECRET_NAME_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker))
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Secret(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(MASK)
    }
}

#[derive(Clone, Default)]
pub struct Redactor {
    values: Vec<String>,
}

impl fmt::Debug for Redactor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Redactor")
            .field("values", &self.values.len())
            .finish()
    }
}

impl Redactor {
    pub fn for_request(scope: &VariableScope, auth: Option<&Auth>) -> Self {
        let mut values = scope.secret_values();
        if let Some(auth) = auth {
            values.extend(auth_secret_values(auth, scope));
        }
        let encoded: Vec<String> = values.iter().map(|value| percent_encode(value)).collect();
        values.extend(encoded);
        values.retain(|value| !value.is_empty());
        values.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
        values.dedup();
        Redactor { values }
    }

    pub fn redact_text(&self, text: &str) -> String {
        self.values.iter().fold(text.to_string(), |masked, value| {
            masked.replace(value.as_str(), REDACTED)
        })
    }

    pub fn redact_url(&self, url: &str) -> String {
        let masked = self.redact_text(url);
        let (address, fragment) = split_keeping(&masked, FRAGMENT_MARK);
        let (base, query) = split_keeping(address, QUERY_MARK);
        let mut text = mask_userinfo(base);
        if let Some(query) = query {
            text.push(QUERY_MARK);
            text.push_str(&mask_query(query));
        }
        if let Some(fragment) = fragment {
            text.push(FRAGMENT_MARK);
            text.push_str(fragment);
        }
        text
    }
}

fn auth_secret_values(auth: &Auth, scope: &VariableScope) -> Vec<String> {
    auth.parameters
        .get(&auth.kind)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|attribute| {
            attribute
                .key
                .as_deref()
                .is_some_and(|key| !PUBLIC_AUTH_KEYS.contains(&key))
        })
        .filter_map(|attribute| attribute.value.as_ref()?.as_str())
        .map(|value| scope.resolve(value).text)
        .collect()
}

fn split_keeping(text: &str, separator: char) -> (&str, Option<&str>) {
    match text.split_once(separator) {
        Some((head, tail)) => (head, Some(tail)),
        None => (text, None),
    }
}

fn mask_userinfo(base: &str) -> String {
    let (prefix, remainder) = match Url::split_scheme(base) {
        Some((scheme, rest)) => (format!("{scheme}{SCHEME_SEPARATOR}"), rest),
        None => (String::new(), base),
    };
    let (authority, path) =
        remainder.split_at(remainder.find(PATH_START).unwrap_or(remainder.len()));
    match authority.rsplit_once(USERINFO_END) {
        Some((_, host)) => format!("{prefix}{REDACTED}{USERINFO_END}{host}{path}"),
        None => base.to_string(),
    }
}

fn mask_query(query: &str) -> String {
    query
        .split(PAIR_SEPARATOR)
        .map(|pair| match pair.split_once(VALUE_SEPARATOR) {
            Some((name, _)) if is_secret_name(name) => {
                format!("{name}{VALUE_SEPARATOR}{REDACTED}")
            }
            _ => pair.to_string(),
        })
        .collect::<Vec<_>>()
        .join(PAIR_SEPARATOR)
}

#[cfg(test)]
mod tests {
    use super::{is_secret_name, Redactor, Secret, REDACTED};
    use crate::collection::Auth;
    use crate::environment::Environment;
    use crate::variables::VariableScope;

    fn scope_with_secrets() -> VariableScope {
        let environment = Environment::from_json(
            r#"{"name":"E","values":[
                {"key":"apiToken","value":"tok-123","type":"secret"},
                {"key":"base","value":"http://h","type":"default"}]}"#,
        )
        .unwrap();
        VariableScope::layered(&[], &[], Some(&environment), &[])
    }

    #[test]
    fn secret_values_are_masked_in_any_query_parameter() {
        let redactor = Redactor::for_request(&scope_with_secrets(), None);
        assert_eq!(
            redactor.redact_url("http://h/p?q=tok-123&page=2"),
            format!("http://h/p?q={REDACTED}&page=2")
        );
    }

    #[test]
    fn secret_values_are_masked_in_the_path_and_fragment() {
        let redactor = Redactor::for_request(&scope_with_secrets(), None);
        assert_eq!(
            redactor.redact_url("http://h/v1/tok-123/x#tok-123"),
            format!("http://h/v1/{REDACTED}/x#{REDACTED}")
        );
    }

    #[test]
    fn auth_values_are_masked_even_when_literal_and_percent_encoded() {
        let auth = Auth::api_key("k", "a b&c", "query");
        let redactor = Redactor::for_request(&VariableScope::new(), Some(&auth));
        assert_eq!(
            redactor.redact_url("http://h/p?id=a%20b%26c"),
            format!("http://h/p?id={REDACTED}")
        );
        assert_eq!(redactor.redact_text("x a b&c y"), format!("x {REDACTED} y"));
    }

    #[test]
    fn userinfo_is_masked_with_and_without_a_scheme() {
        let redactor = Redactor::default();
        assert_eq!(
            redactor.redact_url("https://user:pw@h:8080/a"),
            format!("https://{REDACTED}@h:8080/a")
        );
        assert_eq!(
            redactor.redact_url("user:pw@host/p"),
            format!("{REDACTED}@host/p")
        );
        assert_eq!(redactor.redact_url("host/p@x"), "host/p@x");
    }

    #[test]
    fn secret_named_query_values_are_masked_and_others_kept() {
        let redactor = Redactor::default();
        assert_eq!(
            redactor.redact_url("http://h/p?page=2&api_key=abc&Token=x#top"),
            format!("http://h/p?page=2&api_key={REDACTED}&Token={REDACTED}#top")
        );
    }

    #[test]
    fn debug_output_hides_values() {
        let redactor = Redactor::for_request(&scope_with_secrets(), None);
        assert!(!format!("{redactor:?}").contains("tok-123"));
    }

    #[test]
    fn recognizes_secret_bearing_names() {
        for name in [
            "Authorization",
            "Cookie",
            "X-API-Key",
            "access_token",
            "client_secret",
        ] {
            assert!(is_secret_name(name), "{name}");
        }
        for name in ["Accept", "Content-Type", "page"] {
            assert!(!is_secret_name(name), "{name}");
        }
    }

    #[test]
    fn debug_output_never_contains_the_value() {
        let secret = Secret::new("hunter2");
        assert!(!format!("{secret:?}").contains("hunter2"));
        assert_eq!(secret.expose(), "hunter2");
    }
}
