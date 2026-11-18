use super::{open, response, state_with_request};
use crate::response::ResponseView;
use crate::state::{Action, AppState, ExampleRef, Exchange};

fn example_count(state: &AppState, collection_id: &str, item_id: &str) -> usize {
    state
        .workspace
        .collection(collection_id)
        .and_then(|collection| collection.find_item(item_id))
        .map_or(0, |item| item.example_summaries().count())
}

#[test]
fn a_response_is_saved_opened_and_deleted_as_an_example() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    state.tab_mut(tab).unwrap().exchange = Exchange::Completed(Box::new(
        ResponseView::from_response(response(b"{\"ok\":true}"), true),
    ));
    state.apply(Action::SaveExample(tab), 0.0);
    assert_eq!(example_count(&state, &collection_id, &item_id), 1);

    let reference = ExampleRef {
        collection_id: collection_id.clone(),
        item_id: item_id.clone(),
        index: 0,
    };
    state.apply(Action::OpenExample(reference.clone()), 0.0);
    let opened = state.active().unwrap();
    assert_ne!(opened.id, tab);
    assert_eq!(opened.draft.name, "list › 200 OK");
    assert!(opened.binding.is_none());
    assert!(opened
        .source
        .as_ref()
        .unwrap()
        .points_at(&collection_id, &item_id));
    assert!(matches!(opened.exchange, Exchange::Completed(_)));

    state.apply(Action::DeleteExample(reference), 0.0);
    assert_eq!(example_count(&state, &collection_id, &item_id), 0);
}

#[test]
fn an_unsaved_request_cannot_keep_examples() {
    let (mut state, collection_id, item_id) = state_with_request();
    state.apply(Action::NewTab, 0.0);
    let tab = state.active_tab.unwrap();
    state.tab_mut(tab).unwrap().exchange =
        Exchange::Completed(Box::new(ResponseView::from_response(response(b"{}"), true)));
    state.apply(Action::SaveExample(tab), 0.0);
    assert_eq!(example_count(&state, &collection_id, &item_id), 0);
}
