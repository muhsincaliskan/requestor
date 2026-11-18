mod action;
mod autosave;
mod effect;
mod environments;
mod exchange;
mod folders;
mod history;
mod library;
mod notice;
mod overlay;
mod persistence;
mod sections;
mod sending;
mod socket;
mod sockets;
mod stream;
mod streaming;
mod tab;
mod tab_closing;
mod tabs;
mod transfer;
mod updates;

pub use action::Action;
pub use effect::Effect;
pub use exchange::Exchange;
pub use folders::FolderPurpose;
pub use notice::{Notice, NoticeLevel};
pub use overlay::{
    DeleteConfirmation, ExportForm, ImportForm, LibraryTarget, Overlay, PaletteForm, RenameForm,
    SaveTabForm,
};
pub use persistence::SaveStatus;
pub use sections::{EditorSection, ResponseSection};
pub use socket::{SocketPhase, SocketSession};
pub use stream::{LiveStream, StreamUpdate};
pub use tab::{ItemBinding, RequestTab, TabId};

use requestor_core::update::Release;
use requestor_core::variables::VariableScope;
use requestor_core::workspace::{InterfaceSettings, LoadedWorkspace, Workspace, WorkspaceStore};

use autosave::UnsavedChanges;
use notice::Notices;

pub struct AppState {
    pub workspace: Workspace,
    pub tabs: Vec<RequestTab>,
    pub active_tab: Option<TabId>,
    pub overlay: Option<Overlay>,
    pub available_update: Option<Release>,
    notices: Notices,
    store: WorkspaceStore,
    unsaved: UnsavedChanges,
    next_tab: u64,
    next_job: u64,
    abandoned_jobs: Vec<u64>,
    now: f64,
}

impl AppState {
    pub fn new(loaded: LoadedWorkspace, store: WorkspaceStore) -> Self {
        let mut state = AppState {
            workspace: loaded.workspace,
            tabs: Vec::new(),
            active_tab: None,
            overlay: None,
            available_update: None,
            notices: Notices::default(),
            store,
            unsaved: UnsavedChanges::default(),
            next_tab: 0,
            next_job: 0,
            abandoned_jobs: Vec::new(),
            now: 0.0,
        };
        for warning in &loaded.warnings {
            state.warn(format!("Could not load {warning}"));
        }
        if loaded.secrets_to_secure {
            state.secure_loaded_secrets();
        }
        state.restore_tabs();
        state
    }

    pub fn apply(&mut self, action: Action, now: f64) -> Vec<Effect> {
        self.now = now;
        let mut effects = Vec::new();
        self.dispatch(action, &mut effects);
        effects.extend(self.abandoned_jobs.drain(..).map(Effect::AbortSend));
        effects
    }

    pub fn interface(&self) -> &InterfaceSettings {
        &self.workspace.settings.interface
    }

    pub fn tab(&self, id: TabId) -> Option<&RequestTab> {
        self.tabs.iter().find(|tab| tab.id == id)
    }

    pub fn tab_mut(&mut self, id: TabId) -> Option<&mut RequestTab> {
        self.tabs.iter_mut().find(|tab| tab.id == id)
    }

    pub fn active(&self) -> Option<&RequestTab> {
        self.tab(self.active_tab?)
    }

    pub fn active_mut(&mut self) -> Option<&mut RequestTab> {
        self.tab_mut(self.active_tab?)
    }

    pub fn tabs_awaiting_save(&self) -> impl Iterator<Item = &RequestTab> {
        let autosave = self.interface().autosave;
        self.tabs
            .iter()
            .filter(move |tab| tab.awaits_manual_save(autosave))
    }

    pub fn unsaved_list(&self) -> Vec<&RequestTab> {
        if self.interface().autosave {
            return Vec::new();
        }
        self.tabs_awaiting_save().collect()
    }

    pub fn variable_scope(&self, tab: TabId) -> VariableScope {
        let binding = self.tab(tab).and_then(RequestTab::scope);
        self.workspace.scope_for(
            binding.map(|bound| bound.collection_id.as_str()),
            binding.map(|bound| bound.item_id.as_str()),
        )
    }

    pub fn notice(&self, now: f64) -> Option<&Notice> {
        self.notices.current(now)
    }

    pub fn notice_expires_in(&self, now: f64) -> Option<f64> {
        self.notices.expires_in(now)
    }

    pub fn inform(&mut self, text: impl Into<String>) {
        self.notices.raise(NoticeLevel::Info, text, self.now);
    }

    pub fn warn(&mut self, text: impl Into<String>) {
        self.notices.raise(NoticeLevel::Warning, text, self.now);
    }

    pub fn report_error(&mut self, text: impl Into<String>) {
        self.notices.raise(NoticeLevel::Error, text, self.now);
    }

    fn dispatch(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::Show(overlay) => self.overlay = Some(overlay),
            Action::DismissNotice => self.notices.dismiss(),
            Action::CopyText(text) => effects.push(Effect::CopyText(text)),
            Action::SettingsEdited => self.unsaved.touch_settings(self.now),
            Action::UpdateAvailable(release) => self.available_update = Some(release),
            Action::OpenReleasePage => effects.extend(self.release_page()),
            other => self.dispatch_tab_action(other, effects),
        }
    }

    fn abandon_send(&mut self, id: TabId) {
        let job = self.tab_mut(id).and_then(RequestTab::abandon_send);
        self.abandoned_jobs.extend(job);
    }

    fn position_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }

    fn allocate_tab_id(&mut self) -> TabId {
        self.next_tab += 1;
        TabId(self.next_tab)
    }
}

#[cfg(test)]
mod tests;
