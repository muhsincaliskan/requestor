use requestor_core::history::HistoryEntry;

use super::tab::RequestTab;
use super::{Action, AppState, Effect};

impl AppState {
    pub(super) fn dispatch_history_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::OpenHistoryEntry(entry) => self.open_history_entry(&entry),
            Action::ClearHistory => self.clear_history(),
            other => self.dispatch_library_action(other, effects),
        }
    }

    pub(super) fn remember_sent(&mut self, record: Option<HistoryEntry>) {
        let Some(record) = record else { return };
        self.workspace.history.record(record);
        self.unsaved.touch_history(self.now);
    }

    fn open_history_entry(&mut self, entry: &HistoryEntry) {
        let id = self.allocate_tab_id();
        self.open_tab(RequestTab::for_request(id, &entry.name, &entry.request));
    }

    fn clear_history(&mut self) {
        self.workspace.history.clear();
        self.unsaved.touch_history(self.now);
    }
}
