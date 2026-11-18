use std::fmt;

use serde::{Deserialize, Serialize};

mod redactor;
#[cfg(test)]
mod tests;

pub(crate) use redactor::mask_pairs;
pub use redactor::Redactor;

const MASK: &str = "********";
pub const REDACTED: &str = "<redacted>";
const VARIABLE_OPEN: &str = "{{";
const VARIABLE_CLOSE: &str = "}}";
const CREDENTIAL_SEPARATOR: char = ':';
const AUTH_SCHEMES: [&str; 4] = ["Bearer", "Basic", "Digest", "Token"];
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

pub(crate) fn is_literal(value: &str) -> bool {
    let mut remaining = value.to_string();
    while let Some(open) = remaining.find(VARIABLE_OPEN) {
        let Some(length) = remaining[open..].find(VARIABLE_CLOSE) else {
            break;
        };
        remaining.replace_range(open..open + length + VARIABLE_CLOSE.len(), " ");
    }
    remaining
        .split(|character: char| character.is_whitespace() || character == CREDENTIAL_SEPARATOR)
        .any(|word| {
            !word.is_empty()
                && !AUTH_SCHEMES
                    .iter()
                    .any(|scheme| word.eq_ignore_ascii_case(scheme))
        })
}

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
