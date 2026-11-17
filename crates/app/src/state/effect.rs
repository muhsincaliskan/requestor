use std::path::PathBuf;

use requestor_core::prepared::PreparedRequest;
use requestor_core::settings::RequestSettings;
use requestor_core::workspace::{PendingWrite, WorkspaceStore};

use super::tab::TabId;

#[derive(Debug)]
pub enum Effect {
    Send {
        tab: TabId,
        job: u64,
        request: Box<PreparedRequest>,
        settings: Box<RequestSettings>,
    },
    Persist(PendingWrite),
    DeleteCollectionFile(String),
    DeleteEnvironmentFile(String),
    WriteFile {
        path: PathBuf,
        contents: String,
    },
    CopyText(String),
}

impl Effect {
    pub fn store(self, store: &WorkspaceStore) -> Result<(), String> {
        match self {
            Effect::Persist(write) => write.commit().map_err(|error| error.to_string()),
            Effect::DeleteCollectionFile(id) => store
                .delete_collection(&id)
                .map_err(|error| error.to_string()),
            Effect::DeleteEnvironmentFile(id) => store
                .delete_environment(&id)
                .map_err(|error| error.to_string()),
            Effect::WriteFile { path, contents } => std::fs::write(&path, contents)
                .map_err(|error| format!("{}: {error}", path.display())),
            Effect::Send { .. } | Effect::CopyText(_) => Ok(()),
        }
    }
}
