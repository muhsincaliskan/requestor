use std::fmt;

use requestor_core::postman_cloud::{CloudWorkspace, MigrationProgress};
use requestor_core::secret::Secret;

#[derive(Clone, PartialEq)]
pub struct PostmanAccountForm {
    pub api_key: String,
    pub remember_key: bool,
    pub workspaces: Vec<CloudWorkspace>,
    pub selected: Option<String>,
    pub error: Option<String>,
    pub progress: Option<MigrationProgress>,
    listing: Option<u64>,
    key_looked_up: bool,
}

impl Default for PostmanAccountForm {
    fn default() -> Self {
        PostmanAccountForm {
            api_key: String::new(),
            remember_key: true,
            workspaces: Vec::new(),
            selected: None,
            error: None,
            progress: None,
            listing: None,
            key_looked_up: false,
        }
    }
}

impl fmt::Debug for PostmanAccountForm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PostmanAccountForm")
            .field("remember_key", &self.remember_key)
            .field("workspaces", &self.workspaces.len())
            .field("selected", &self.selected)
            .field("listing", &self.listing)
            .finish_non_exhaustive()
    }
}

impl PostmanAccountForm {
    pub fn key(&self) -> Option<Secret> {
        let key = self.api_key.trim();
        (!key.is_empty()).then(|| Secret::new(key))
    }

    pub fn kept_key(&self) -> Option<Secret> {
        self.key().filter(|_| self.remember_key)
    }

    pub fn is_listing(&self) -> bool {
        self.listing.is_some()
    }

    pub fn can_list(&self) -> bool {
        self.key().is_some() && !self.is_listing()
    }

    pub fn needs_key_lookup(&mut self) -> bool {
        !std::mem::replace(&mut self.key_looked_up, true)
    }

    pub fn accept_saved_key(&mut self, key: Option<Secret>) {
        if let (Some(key), true) = (key, self.api_key.is_empty()) {
            self.api_key = key.expose().to_string();
        }
    }

    pub fn start_listing(&mut self, job: u64) {
        self.listing = Some(job);
        self.error = None;
    }

    pub fn finish_listing(&mut self, job: u64, listed: Result<Vec<CloudWorkspace>, String>) {
        if self.listing != Some(job) {
            return;
        }
        self.listing = None;
        match listed {
            Ok(workspaces) => {
                let still_there = self
                    .selected
                    .as_ref()
                    .is_some_and(|id| workspaces.iter().any(|workspace| &workspace.id == id));
                if !still_there {
                    self.selected = workspaces.first().map(|workspace| workspace.id.clone());
                }
                self.workspaces = workspaces;
            }
            Err(reason) => self.error = Some(reason),
        }
    }

    pub fn selected_workspace(&self) -> Option<&CloudWorkspace> {
        let selected = self.selected.as_ref()?;
        self.workspaces
            .iter()
            .find(|workspace| &workspace.id == selected)
    }
}
