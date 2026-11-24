use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::collection::Request;
use crate::script::Script;
use crate::settings::RequestOverrides;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsaved_before_script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsaved_after_script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_collection_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_item_id: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub tabs: Vec<SessionTab>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_tab: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_environment: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub collection_order: Vec<String>,
}

impl SessionTab {
    pub fn unsaved_after_text(&self) -> Option<String> {
        self.unsaved_after_script.clone().or_else(|| {
            let lines = self.unsaved_tests.as_ref()?;
            Script::from_javascript(lines)
                .ok()
                .map(|script| script.text())
        })
    }

    pub fn unsaved_overrides(&self) -> Option<RequestOverrides> {
        self.unsaved_request.as_ref()?;
        RequestOverrides::stored_in(&self.extra)
    }

    pub fn with_unsaved_overrides(mut self, overrides: Option<&RequestOverrides>) -> Self {
        if let Some(overrides) = overrides {
            overrides.store_in(&mut self.extra);
        }
        self
    }

    pub fn is_bound(&self) -> bool {
        self.collection_id.is_some() && self.item_id.is_some()
    }

    pub fn source(&self) -> Option<(&str, &str)> {
        self.source_collection_id
            .as_deref()
            .zip(self.source_item_id.as_deref())
    }
}
