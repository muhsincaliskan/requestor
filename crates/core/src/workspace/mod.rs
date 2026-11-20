mod accent;
mod data_folder;
mod extracted;
mod folder;
mod pending_write;
mod secrets;
mod secured_write;
mod session;
mod settings;
mod store;
mod store_write;
mod vault_write;

pub use accent::AccentName;
pub use data_folder::DataFolder;
pub use extracted::ChangedStores;
pub use folder::{read_folder, FolderLayout, FolderWrite};
pub use pending_write::{PendingDelete, PendingWrite, StoreError};
pub use secrets::{SecretRestore, UnsecuredSecrets};
pub use secured_write::SecuredWrite;
pub use session::{Session, SessionTab};
pub use settings::{AppSettings, InterfaceSettings, ThemeName};
pub use store::{LoadedWorkspace, WorkspaceStore};
pub use store_write::StoreWrite;
pub use vault_write::VaultWrite;

use crate::collection::{Collection, Request, Variable};
use crate::environment::Environment;
use crate::history::History;
use crate::identity::random_uuid;
use crate::variables::VariableScope;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Workspace {
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub settings: AppSettings,
    pub session: Session,
    pub history: History,
}

impl Workspace {
    pub fn collection(&self, id: &str) -> Option<&Collection> {
        self.collections
            .iter()
            .find(|collection| collection.id() == id)
    }

    pub fn collection_mut(&mut self, id: &str) -> Option<&mut Collection> {
        self.collections
            .iter_mut()
            .find(|collection| collection.id() == id)
    }

    pub fn add_collection(&mut self, mut collection: Collection) -> String {
        collection.ensure_ids();
        if self.collection(collection.id()).is_some() {
            collection.info.postman_id = Some(random_uuid());
        }
        let id = collection.id().to_string();
        self.collections.push(collection);
        id
    }

    pub fn remove_collection(&mut self, id: &str) -> Option<Collection> {
        let position = self
            .collections
            .iter()
            .position(|collection| collection.id() == id)?;
        Some(self.collections.remove(position))
    }

    pub fn environment(&self, id: &str) -> Option<&Environment> {
        self.environments
            .iter()
            .find(|environment| environment.id.as_deref() == Some(id))
    }

    pub fn environment_mut(&mut self, id: &str) -> Option<&mut Environment> {
        self.environments
            .iter_mut()
            .find(|environment| environment.id.as_deref() == Some(id))
    }

    pub fn add_environment(&mut self, mut environment: Environment) -> String {
        let taken = environment
            .id
            .as_deref()
            .is_none_or(|id| self.environment(id).is_some());
        if taken {
            environment.id = Some(random_uuid());
        }
        let id = environment.id.clone().unwrap_or_default();
        self.environments.push(environment);
        id
    }

    pub fn remove_environment(&mut self, id: &str) -> Option<Environment> {
        let position = self
            .environments
            .iter()
            .position(|environment| environment.id.as_deref() == Some(id))?;
        if self.session.active_environment.as_deref() == Some(id) {
            self.session.active_environment = None;
        }
        Some(self.environments.remove(position))
    }

    pub fn has_requests(&self) -> bool {
        !self.collections.is_empty() || !self.history.is_empty()
    }

    pub fn active_environment(&self) -> Option<&Environment> {
        self.environment(self.session.active_environment.as_deref()?)
    }

    pub fn scope_for(&self, collection_id: Option<&str>, item_id: Option<&str>) -> VariableScope {
        let inherited: Vec<Variable> = self
            .inherited_variables(collection_id, item_id)
            .into_iter()
            .cloned()
            .collect();
        VariableScope::layered(&inherited, &[], self.active_environment(), &[])
    }

    pub fn variable_names(
        &self,
        collection_id: Option<&str>,
        item_id: Option<&str>,
    ) -> Vec<String> {
        self.scope_for(collection_id, item_id).names()
    }

    pub fn sendable_request(
        &self,
        collection_id: Option<&str>,
        item_id: Option<&str>,
        mut request: Request,
    ) -> Request {
        let Some((collection, item)) = collection_id
            .and_then(|id| self.collection(id))
            .zip(item_id)
        else {
            return request;
        };
        if request.auth.is_none() {
            request.auth = collection.inherited_auth(item).cloned();
        }
        let inherited: Vec<_> = collection
            .inherited_headers(item)
            .into_iter()
            .map(|inherited| inherited.header)
            .collect();
        request.add_inherited_headers(&inherited);
        request
    }

    fn inherited_variables(
        &self,
        collection_id: Option<&str>,
        item_id: Option<&str>,
    ) -> Vec<&Variable> {
        let Some(collection) = collection_id.and_then(|id| self.collection(id)) else {
            return Vec::new();
        };
        match item_id {
            Some(item) => collection.inherited_variables(item),
            None => collection.variable.iter().flatten().collect(),
        }
    }

    pub(crate) fn normalize(&mut self) {
        let tab_count = self.session.tabs.len();
        self.session.active_tab = self.session.active_tab.filter(|index| *index < tab_count);
        let known_environment = self
            .session
            .active_environment
            .as_deref()
            .is_some_and(|id| self.environment(id).is_some());
        if !known_environment {
            self.session.active_environment = None;
        }
    }
}

#[cfg(test)]
mod tests;
