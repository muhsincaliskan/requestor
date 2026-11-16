use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::param::Param;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Auth {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub parameters: BTreeMap<String, Vec<Param>>,
}

impl Auth {
    pub fn value_of(&self, key: &str) -> Option<&str> {
        self.parameters
            .get(&self.kind)?
            .iter()
            .find(|parameter| parameter.key.as_deref() == Some(key))
            .and_then(|parameter| parameter.value.as_deref())
    }

    pub fn is_none(&self) -> bool {
        self.kind == "noauth"
    }
}
