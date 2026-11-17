use std::fmt;

use serde::{Deserialize, Serialize};

mod redactor;
#[cfg(test)]
mod tests;

pub(crate) use redactor::mask_pairs;
pub use redactor::Redactor;

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
