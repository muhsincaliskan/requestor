use super::{edit_url, open, state_with_request};
use crate::state::Action;

#[test]
fn with_autosave_on_only_scratch_tabs_await_a_manual_save_and_no_list_is_shown() {
    let (mut state, collection_id, item_id) = state_with_request();
    let bound = open(&mut state, &collection_id, &item_id);
    edit_url(&mut state, bound, "https://example.test/edited", 0.0);
    state.apply(Action::NewTab, 0.0);
    let scratch = state.active_tab.unwrap();
    let awaiting: Vec<_> = state.tabs_awaiting_save().map(|tab| tab.id).collect();
    assert_eq!(awaiting, vec![scratch]);
    assert!(state.unsaved_list().is_empty());
}

#[test]
fn with_autosave_off_every_edited_tab_is_listed_until_saved() {
    let (mut state, collection_id, item_id) = state_with_request();
    state.workspace.settings.interface.autosave = false;
    let bound = open(&mut state, &collection_id, &item_id);
    assert!(state.unsaved_list().is_empty());
    edit_url(&mut state, bound, "https://example.test/edited", 0.0);
    let listed: Vec<_> = state.unsaved_list().iter().map(|tab| tab.id).collect();
    assert_eq!(listed, vec![bound]);
    state.apply(Action::SaveTab(bound), 1.0);
    assert!(state.unsaved_list().is_empty());
}
