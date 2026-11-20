use serde::Deserialize;
use serde_json::{Map, Value};

use super::{CloudItem, CloudWorkspace, PostmanCloudError, WorkspaceContents};
use crate::collection::Collection;
use crate::environment::Environment;

const ENVIRONMENT_ACCOUNT_FIELDS: [&str; 4] = ["owner", "createdAt", "updatedAt", "isPublic"];
const COLLECTION_ACCOUNT_FIELDS: [&str; 4] = ["createdAt", "updatedAt", "lastUpdatedBy", "uid"];

#[derive(Debug, Deserialize)]
pub(super) struct WorkspacesResponse {
    workspaces: Vec<WorkspaceEntry>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceEntry {
    id: String,
    name: String,
    #[serde(rename = "type", default)]
    kind: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct WorkspaceResponse {
    workspace: WorkspaceBody,
}

#[derive(Debug, Deserialize)]
struct WorkspaceBody {
    #[serde(default)]
    collections: Vec<ItemEntry>,
    #[serde(default)]
    environments: Vec<ItemEntry>,
}

#[derive(Debug, Deserialize)]
struct ItemEntry {
    uid: String,
    name: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct CollectionResponse {
    collection: Collection,
}

#[derive(Debug, Deserialize)]
pub(super) struct EnvironmentResponse {
    environment: Map<String, Value>,
}

impl WorkspacesResponse {
    pub(super) fn into_domain(self) -> Vec<CloudWorkspace> {
        self.workspaces
            .into_iter()
            .map(|entry| CloudWorkspace {
                id: entry.id,
                name: entry.name,
                kind: entry.kind,
            })
            .collect()
    }
}

impl WorkspaceResponse {
    pub(super) fn into_domain(self) -> WorkspaceContents {
        WorkspaceContents {
            collections: ItemEntry::all_into_domain(self.workspace.collections),
            environments: ItemEntry::all_into_domain(self.workspace.environments),
        }
    }
}

impl ItemEntry {
    fn all_into_domain(entries: Vec<ItemEntry>) -> Vec<CloudItem> {
        entries
            .into_iter()
            .map(|entry| CloudItem {
                uid: entry.uid,
                name: entry.name,
            })
            .collect()
    }
}

impl CollectionResponse {
    pub(super) fn into_domain(mut self) -> Collection {
        for key in COLLECTION_ACCOUNT_FIELDS {
            self.collection.info.extra.remove(key);
        }
        self.collection
    }
}

impl EnvironmentResponse {
    pub(super) fn into_domain(mut self) -> Result<Environment, PostmanCloudError> {
        for key in ENVIRONMENT_ACCOUNT_FIELDS {
            self.environment.remove(key);
        }
        serde_json::from_value(Value::Object(self.environment))
            .map_err(|error| PostmanCloudError::InvalidResponse(error.to_string()))
    }
}
