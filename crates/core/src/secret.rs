use std::fmt;

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::{is_secret_name, Secret};

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
