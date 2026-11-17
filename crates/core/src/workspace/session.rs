use serde::{Deserialize, Serialize};

use crate::collection::Request;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionTab {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsaved_request: Option<Request>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsaved_tests: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub tabs: Vec<SessionTab>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_tab: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_environment: Option<String>,
}

impl SessionTab {
    pub fn is_bound(&self) -> bool {
        self.collection_id.is_some() && self.item_id.is_some()
    }
}
