use std::fmt;

use serde::{Deserialize, Serialize};

const MASK: &str = "********";

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
    use super::Secret;

    #[test]
    fn debug_output_never_contains_the_value() {
        let secret = Secret::new("hunter2");
        assert!(!format!("{secret:?}").contains("hunter2"));
        assert_eq!(secret.expose(), "hunter2");
    }
}
