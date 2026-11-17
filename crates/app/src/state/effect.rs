use std::path::PathBuf;

use requestor_core::prepared::PreparedRequest;
use requestor_core::settings::RequestSettings;
use requestor_core::workspace::{PendingDelete, PendingWrite};

use super::overlay::LibraryTarget;
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
    DeleteFile(PathBuf),
    WriteFile {
        path: PathBuf,
        contents: String,
    },
    CopyText(String),
    AbortSend(u64),
    PickImportFile,
    PickExportLocation {
        target: LibraryTarget,
        suggested: PathBuf,
    },
}

impl Effect {
    pub fn store(self) -> Result<(), String> {
        match self {
            Effect::Persist(write) => write.commit().map_err(|error| error.to_string()),
            Effect::DeleteFile(path) => PendingDelete::new(path)
                .commit()
                .map_err(|error| error.to_string()),
            Effect::WriteFile { path, contents } => PendingWrite::new(path, contents)
                .commit()
                .map_err(|error| error.to_string()),
            Effect::Send { .. }
            | Effect::CopyText(_)
            | Effect::AbortSend(_)
            | Effect::PickImportFile
            | Effect::PickExportLocation { .. } => Ok(()),
        }
    }
}
