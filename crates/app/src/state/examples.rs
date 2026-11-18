use requestor_core::collection::Example;

use super::exchange::Exchange;
use super::tab::{ItemBinding, RequestTab, TabId};
use super::{Action, AppState, Effect};
use crate::response::ResponseView;

const EXAMPLE_SAVED: &str = "Saved the response as an example";
const NEEDS_SAVED_REQUEST: &str = "Save the request to a collection first, then save its examples";
const NOT_STORABLE: &str = "Only complete text responses can be saved as examples";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleRef {
    pub collection_id: String,
    pub item_id: String,
    pub index: usize,
}

impl AppState {
    pub(super) fn dispatch_example_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::OpenExample(example) => self.open_example(&example),
            Action::SaveExample(tab) => self.save_example(tab),
            Action::DeleteExample(example) => self.delete_example(&example),
            other => self.dispatch_library_action(other, effects),
        }
    }

    fn open_example(&mut self, reference: &ExampleRef) {
        let Some(item) = self
            .workspace
            .collection(&reference.collection_id)
            .and_then(|collection| collection.find_item(&reference.item_id))
        else {
            return;
        };
        let Some(example) = item.example(reference.index) else {
            return;
        };
        let Some(request) = example.request().or_else(|| item.full_request()) else {
            return;
        };
        let name = format!("{} › {}", item.display_name(), example.display_name());
        let detect_format = self.interface().response_format_detection;
        let id = self.allocate_tab_id();
        let mut tab = RequestTab::for_request(id, &name, &request);
        tab.source = Some(ItemBinding::new(
            &reference.collection_id,
            &reference.item_id,
        ));
        tab.exchange = Exchange::Completed(Box::new(ResponseView::from_response(
            example.to_response(),
            detect_format,
        )));
        self.open_tab(tab);
    }

    fn save_example(&mut self, id: TabId) {
        let Some(tab) = self.tab(id) else {
            return;
        };
        let (Some(binding), Exchange::Completed(view)) = (tab.binding.clone(), &tab.exchange)
        else {
            self.warn(NEEDS_SAVED_REQUEST);
            return;
        };
        if !view.is_storable_as_text() {
            self.warn(NOT_STORABLE);
            return;
        }
        let example = Example::from_exchange(&tab.draft.to_request(), &view.to_response());
        let Some(item) = self
            .workspace
            .collection_mut(&binding.collection_id)
            .and_then(|collection| collection.find_item_mut(&binding.item_id))
        else {
            return;
        };
        item.add_example(&example);
        self.unsaved
            .touch_collection(&binding.collection_id, self.now);
        self.inform(EXAMPLE_SAVED);
    }

    fn delete_example(&mut self, reference: &ExampleRef) {
        let removed = self
            .workspace
            .collection_mut(&reference.collection_id)
            .and_then(|collection| collection.find_item_mut(&reference.item_id))
            .is_some_and(|item| item.remove_example(reference.index));
        if removed {
            self.unsaved
                .touch_collection(&reference.collection_id, self.now);
        }
    }
}
