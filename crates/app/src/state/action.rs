use std::path::PathBuf;

use requestor_core::history::HistoryEntry;
use requestor_core::response::HttpResponse;
use requestor_core::transport::SocketEvent;
use requestor_core::update::Release;

use super::folders::FolderPurpose;
use super::overlay::{LibraryTarget, Overlay};
use super::stream::StreamUpdate;
use super::tab::TabId;

#[derive(Debug)]
pub enum Action {
    NewTab,
    OpenItem {
        collection_id: String,
        item_id: String,
    },
    ActivateTab(TabId),
    ActivateNeighbourTab {
        forward: bool,
    },
    CloseTab(TabId),
    ForceCloseTab(TabId),
    CloseOtherTabs(TabId),
    DuplicateTab(TabId),
    TabEdited(TabId),
    SaveTab(TabId),
    SaveAndCloseTab(TabId),
    SaveTabInto {
        tab: TabId,
        name: String,
        collection_id: String,
        folder_id: Option<String>,
        close_after: bool,
    },
    Send(TabId),
    CancelSend(TabId),
    FetchSchema(TabId),
    SchemaFetched {
        tab: TabId,
        job: u64,
        result: Result<HttpResponse, String>,
    },
    SendFinished {
        tab: TabId,
        job: u64,
        result: Result<HttpResponse, String>,
    },
    StreamProgress {
        tab: TabId,
        job: u64,
        update: StreamUpdate,
    },
    SocketEvent {
        tab: TabId,
        job: u64,
        event: SocketEvent,
    },
    SocketEnded {
        tab: TabId,
        job: u64,
        result: Result<(), String>,
    },
    SendSocketMessage(TabId),
    CopyAsCurl {
        tab: TabId,
        include_secrets: bool,
    },
    CopyText(String),
    OpenHistoryEntry(Box<HistoryEntry>),
    ClearHistory,
    NewCollection,
    AddFolder {
        collection_id: String,
        parent_id: Option<String>,
    },
    AddRequest {
        collection_id: String,
        parent_id: Option<String>,
    },
    DuplicateItem {
        collection_id: String,
        item_id: String,
    },
    MoveItem {
        collection_id: String,
        item_id: String,
        parent_id: Option<String>,
        index: usize,
    },
    Rename {
        target: LibraryTarget,
        name: String,
    },
    Delete(LibraryTarget),
    CollectionEdited(String),
    NewEnvironment,
    SelectEnvironment(Option<String>),
    EnvironmentEdited(String),
    ImportText(String),
    ImportFile(PathBuf),
    ChooseImportFile,
    ChooseExportLocation {
        target: LibraryTarget,
        suggested: PathBuf,
    },
    Export {
        target: LibraryTarget,
        file_path: PathBuf,
    },
    ChooseFolder(FolderPurpose),
    FolderPicked {
        purpose: FolderPurpose,
        path: PathBuf,
    },
    StopSavingToFolder(String),
    SettingsEdited,
    UpdateAvailable(Release),
    OpenReleasePage,
    Show(Overlay),
    DismissNotice,
}
