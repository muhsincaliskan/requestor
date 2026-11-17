use std::path::PathBuf;

use requestor_core::response::HttpResponse;

use super::overlay::{LibraryTarget, Overlay};
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
    SendFinished {
        tab: TabId,
        job: u64,
        result: Result<HttpResponse, String>,
    },
    CopyAsCurl {
        tab: TabId,
        include_secrets: bool,
    },
    CopyText(String),
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
    Export {
        target: LibraryTarget,
        file_path: PathBuf,
    },
    SettingsEdited,
    Show(Overlay),
    DismissNotice,
}
