use std::path::PathBuf;

use requestor_core::prepared::PreparedRequest;
use requestor_core::settings::RequestSettings;
use requestor_core::workspace::{PendingDelete, PendingWrite, StoreWrite};

use super::certificates::CertificateFile;
use super::collection_run::CollectionRunJob;
use super::folders::FolderPurpose;
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
    OpenSocket {
        tab: TabId,
        job: u64,
        request: Box<PreparedRequest>,
        settings: Box<RequestSettings>,
    },
    FetchSchema {
        tab: TabId,
        job: u64,
        request: Box<PreparedRequest>,
        settings: Box<RequestSettings>,
    },
    SocketSend {
        job: u64,
        text: String,
    },
    RunCollection(Box<CollectionRunJob>),
    Persist(StoreWrite),
    DeleteFile(PathBuf),
    WriteFile {
        path: PathBuf,
        contents: String,
    },
    CopyText(String),
    OpenUrl(String),
    AbortSend(u64),
    PickImportFile,
    DownloadImport {
        job: u64,
        address: String,
        settings: Box<RequestSettings>,
    },
    PickExportLocation {
        target: LibraryTarget,
        suggested: PathBuf,
        keep_credentials: bool,
    },
    PickFolder(FolderPurpose),
    PickCertificateFile(CertificateFile),
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
            | Effect::OpenSocket { .. }
            | Effect::SocketSend { .. }
            | Effect::RunCollection(_)
            | Effect::FetchSchema { .. }
            | Effect::CopyText(_)
            | Effect::OpenUrl(_)
            | Effect::AbortSend(_)
            | Effect::PickImportFile
            | Effect::DownloadImport { .. }
            | Effect::PickExportLocation { .. }
            | Effect::PickFolder(_)
            | Effect::PickCertificateFile(_) => Ok(()),
        }
    }
}
