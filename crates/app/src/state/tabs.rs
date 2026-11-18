use requestor_core::collection::Item;

use super::overlay::{Overlay, SaveTabForm};
use super::tab::{ItemBinding, RequestTab, TabId};
use super::{Action, AppState, Effect};

const COPY_SUFFIX: &str = " Copy";

impl AppState {
    pub(super) fn dispatch_tab_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::NewTab => self.open_scratch_tab(),
            Action::OpenItem {
                collection_id,
                item_id,
            } => self.open_item(&collection_id, &item_id),
            Action::ActivateTab(tab) => self.activate(tab),
            Action::ActivateNeighbourTab { forward } => self.activate_neighbour(forward),
            Action::CloseTab(tab) => self.close_tab(tab),
            Action::ForceCloseTab(tab) => self.force_close_tab(tab),
            Action::CloseOtherTabs(tab) => self.close_other_tabs(tab),
            Action::DuplicateTab(tab) => self.duplicate_tab(tab),
            Action::TabEdited(tab) => self.tab_edited(tab),
            Action::SaveTab(tab) => self.save_tab(tab, false),
            Action::SaveAndCloseTab(tab) => self.save_tab(tab, true),
            Action::SaveTabInto {
                tab,
                name,
                collection_id,
                folder_id,
                close_after,
            } => {
                self.save_tab_into(tab, name, &collection_id, folder_id.as_deref());
                self.close_saved_tab(tab, close_after);
            }
            other => self.dispatch_sending_action(other, effects),
        }
    }

    pub(super) fn open_tab(&mut self, tab: RequestTab) {
        let id = tab.id;
        self.tabs.push(tab);
        self.activate(id);
    }

    pub(super) fn open_scratch_tab(&mut self) {
        let id = self.allocate_tab_id();
        self.open_tab(RequestTab::scratch(id));
    }

    pub(super) fn open_item(&mut self, collection_id: &str, item_id: &str) {
        if let Some(existing) = self
            .tabs
            .iter()
            .find(|tab| tab.is_bound_to(collection_id, item_id))
        {
            let id = existing.id;
            self.activate(id);
            return;
        }
        let id = self.allocate_tab_id();
        let binding = ItemBinding::new(collection_id, item_id);
        let Some(tab) = self
            .workspace
            .collection(collection_id)
            .and_then(|collection| collection.find_item(item_id))
            .and_then(|item| RequestTab::for_item(id, binding, item))
        else {
            return;
        };
        match self.reusable_tab_position() {
            Some(position) => {
                self.tabs[position] = tab;
                self.activate(id);
            }
            None => self.open_tab(tab),
        }
    }

    pub(super) fn activate(&mut self, id: TabId) {
        if self.position_of(id).is_none() {
            return;
        }
        if let Some(previous) = self.active_mut().filter(|tab| tab.id != id) {
            previous.release_display();
        }
        self.active_tab = Some(id);
        self.unsaved.touch_session(self.now);
    }

    pub(super) fn commit_tab(&mut self, id: TabId) -> bool {
        let Some(position) = self.position_of(id) else {
            return false;
        };
        let tab = &mut self.tabs[position];
        let Some(binding) = tab.binding.clone() else {
            return false;
        };
        let Some(item) = self
            .workspace
            .collection_mut(&binding.collection_id)
            .and_then(|collection| collection.find_item_mut(&binding.item_id))
        else {
            tab.binding = None;
            return false;
        };
        item.name = Some(tab.draft.name.clone());
        tab.draft.rules.write_into(&mut item.event);
        tab.draft.overrides.write_into(&mut item.extra);
        item.replace_request(tab.draft.commit());
        tab.mark_saved();
        self.unsaved
            .touch_collection(&binding.collection_id, self.now);
        self.unsaved.touch_session(self.now);
        true
    }

    fn reusable_tab_position(&self) -> Option<usize> {
        if self.interface().open_sidebar_item_in_new_tab {
            return None;
        }
        let active = self.active()?;
        let reusable = active.binding.is_some() && !active.unsaved && !active.is_sending();
        reusable.then(|| self.position_of(active.id)).flatten()
    }

    fn activate_neighbour(&mut self, forward: bool) {
        let count = self.tabs.len();
        let Some(position) = self.active_tab.and_then(|id| self.position_of(id)) else {
            return;
        };
        let step = if forward { 1 } else { count - 1 };
        let id = self.tabs[(position + step) % count].id;
        self.activate(id);
    }

    fn duplicate_tab(&mut self, id: TabId) {
        let Some(original) = self.tab(id) else { return };
        let request = original.draft.to_request();
        let rules = original.draft.rules.clone();
        let overrides = original.draft.overrides.clone();
        let name = format!("{}{COPY_SUFFIX}", original.draft.name);
        let copy_id = self.allocate_tab_id();
        let mut copy = RequestTab::for_request(copy_id, &name, &request);
        copy.draft.rules = rules;
        copy.draft.overrides = overrides;
        self.open_tab(copy);
    }

    fn tab_edited(&mut self, id: TabId) {
        let now = self.now;
        if let Some(tab) = self.tab_mut(id) {
            tab.mark_edited(now);
            self.unsaved.touch_session(now);
        }
    }

    fn save_tab(&mut self, id: TabId, close_after: bool) {
        let Some(tab) = self.tab(id) else { return };
        if tab.binding.is_some() {
            self.commit_tab(id);
            self.close_saved_tab(id, close_after);
            return;
        }
        self.overlay = Some(Overlay::SaveTab(SaveTabForm {
            tab: id,
            name: tab.draft.name.clone(),
            collection_id: tab
                .source
                .as_ref()
                .map(|source| source.collection_id.as_str())
                .filter(|id| self.workspace.collection(id).is_some())
                .or_else(|| self.workspace.collections.first().map(|first| first.id()))
                .map(str::to_string),
            folder_id: None,
            close_after,
        }));
    }

    fn save_tab_into(
        &mut self,
        id: TabId,
        name: String,
        collection_id: &str,
        folder_id: Option<&str>,
    ) {
        let Some(position) = self.position_of(id) else {
            return;
        };
        let request = self.tabs[position].draft.to_request();
        let created = self
            .workspace
            .collection_mut(collection_id)
            .and_then(|collection| {
                collection.add_item(folder_id, Item::for_request(name.clone(), request))
            });
        let Some(item_id) = created else {
            self.report_error("The request could not be saved to that location");
            return;
        };
        let tab = &mut self.tabs[position];
        tab.draft.name = name;
        tab.binding = Some(ItemBinding::new(collection_id, item_id));
        self.overlay = None;
        self.commit_tab(id);
    }
}
