use std::fmt;

use super::{is_secret_name, REDACTED};
use crate::collection::{Auth, Url, SCHEME_SEPARATOR};
use crate::encoding::percent_encode;
use crate::variables::VariableScope;

const MIN_MASKED_VALUE_BYTES: usize = 4;
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
        values.retain(|value| value.len() >= MIN_MASKED_VALUE_BYTES);
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
            text.push_str(&mask_pairs(query));
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

pub(crate) fn mask_pairs(query: &str) -> String {
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
