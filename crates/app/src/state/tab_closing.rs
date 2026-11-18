use super::overlay::Overlay;
use super::tab::TabId;
use super::AppState;

impl AppState {
    pub(super) fn close_tab(&mut self, id: TabId) {
        let Some(tab) = self.tab(id) else { return };
        let autosaved = tab.binding.is_some() && self.interface().autosave;
        let untouched_scratch = tab.binding.is_none() && tab.draft.is_unchanged();
        let unsaved = tab.unsaved && !untouched_scratch;
        if unsaved && !autosaved && self.interface().ask_when_closing_unsaved_tabs {
            self.overlay = Some(Overlay::ConfirmCloseTab(id));
            return;
        }
        if unsaved && autosaved {
            self.commit_tab(id);
        }
        self.force_close_tab(id);
    }

    pub(super) fn force_close_tab(&mut self, id: TabId) {
        self.abandon_send(id);
        self.cancel_schema(id);
        let Some(position) = self.position_of(id) else {
            return;
        };
        self.tabs.remove(position);
        if self.overlay == Some(Overlay::ConfirmCloseTab(id)) {
            self.overlay = None;
        }
        if self.active_tab == Some(id) {
            let neighbour = position.min(self.tabs.len().saturating_sub(1));
            self.active_tab = self.tabs.get(neighbour).map(|tab| tab.id);
        }
        self.unsaved.touch_session(self.now);
    }

    pub(super) fn close_other_tabs(&mut self, keep: TabId) {
        let others: Vec<TabId> = self
            .tabs
            .iter()
            .map(|tab| tab.id)
            .filter(|id| *id != keep)
            .collect();
        self.overlay = None;
        let mut awaiting_confirmation = None;
        for id in others {
            self.close_tab(id);
            awaiting_confirmation = awaiting_confirmation.or(self.overlay.take());
        }
        self.overlay = awaiting_confirmation;
        self.activate(keep);
    }

    pub(super) fn close_saved_tab(&mut self, id: TabId, wanted: bool) {
        if wanted && self.tab(id).is_some_and(|tab| !tab.unsaved) {
            self.force_close_tab(id);
        }
    }
}
