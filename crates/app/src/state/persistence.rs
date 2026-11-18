use requestor_core::workspace::{Session, StoreError, StoreWrite, UnsecuredSecrets};

use super::autosave::QUIET_SECONDS;
use super::tab::{RequestTab, TabId};
use super::{AppState, Effect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveStatus {
    Saved,
    Saving,
    Unsaved,
}

impl AppState {
    pub fn tick(&mut self, now: f64) -> Vec<Effect> {
        self.now = now;
        self.autosave_tabs(|tab| tab.is_due_for_autosave(now, QUIET_SECONDS));
        if self.unsaved.is_due(now) {
            self.pending_writes()
        } else {
            Vec::new()
        }
    }

    pub fn flush(&mut self) -> Vec<Effect> {
        self.autosave_tabs(|tab| tab.unsaved && tab.binding.is_some());
        self.unsaved.touch_session(self.now);
        self.pending_writes()
    }

    pub fn seconds_until_next_save(&self, now: f64) -> Option<f64> {
        let tabs = self
            .tabs
            .iter()
            .filter(|tab| tab.unsaved && tab.binding.is_some() && self.interface().autosave)
            .filter_map(|tab| tab.edited_at)
            .map(|edited| (QUIET_SECONDS - (now - edited)).max(0.0));
        tabs.chain(self.unsaved.seconds_until_due(now))
            .reduce(f64::min)
    }

    pub fn save_status(&self) -> SaveStatus {
        let autosave = self.interface().autosave;
        let waiting = |tab: &RequestTab| tab.unsaved && tab.binding.is_some();
        if !autosave && self.tabs.iter().any(waiting) {
            SaveStatus::Unsaved
        } else if self.tabs.iter().any(waiting) || !self.unsaved.is_empty() {
            SaveStatus::Saving
        } else {
            SaveStatus::Saved
        }
    }

    pub(super) fn restore_tabs(&mut self) {
        let saved = self.workspace.session.tabs.clone();
        for entry in &saved {
            let id = self.allocate_tab_id();
            self.tabs
                .push(RequestTab::from_session(id, entry, &self.workspace));
        }
        self.active_tab = self
            .workspace
            .session
            .active_tab
            .and_then(|index| self.tabs.get(index))
            .or(self.tabs.first())
            .map(|tab| tab.id);
    }

    pub(super) fn secure_loaded_secrets(&mut self, unsecured: &UnsecuredSecrets) {
        for id in &unsecured.environments {
            self.unsaved.touch_environment(id, self.now);
        }
        if unsecured.settings {
            self.unsaved.touch_settings(self.now);
        }
    }

    fn autosave_tabs(&mut self, is_due: impl Fn(&RequestTab) -> bool) {
        if !self.interface().autosave {
            return;
        }
        let due: Vec<TabId> = self
            .tabs
            .iter()
            .filter(|tab| is_due(tab))
            .map(|tab| tab.id)
            .collect();
        for id in due {
            self.commit_tab(id);
        }
    }

    fn pending_writes(&mut self) -> Vec<Effect> {
        let changes = self.unsaved.take();
        let mut writes: Vec<Result<StoreWrite, StoreError>> = Vec::new();
        writes.extend(
            changes
                .collections
                .iter()
                .filter_map(|id| self.workspace.collection(id))
                .map(|collection| self.store.collection_write(collection)),
        );
        writes.extend(
            changes
                .environments
                .iter()
                .filter_map(|id| self.workspace.environment(id))
                .map(|environment| self.store.environment_write(environment)),
        );
        if changes.settings {
            writes.push(self.store.settings_write(&self.workspace.settings));
        }
        if changes.session {
            self.workspace.session = self.session_snapshot();
            writes.push(
                self.store
                    .session_write(&self.workspace.session)
                    .map(StoreWrite::from),
            );
        }
        if changes.history {
            writes.push(
                self.store
                    .history_write(&self.workspace.history)
                    .map(StoreWrite::from),
            );
        }
        let mut effects = Vec::with_capacity(writes.len());
        for write in writes {
            self.persist_or_report(write, &mut effects);
        }
        effects
    }

    fn session_snapshot(&self) -> Session {
        Session {
            tabs: self.tabs.iter().map(RequestTab::to_session).collect(),
            active_tab: self.active_tab.and_then(|id| self.position_of(id)),
            active_environment: self.workspace.session.active_environment.clone(),
        }
    }
}
