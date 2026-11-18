use requestor_core::collection::{Auth, Param};

use super::{open, state_with_request};

#[test]
fn an_open_request_shows_what_it_inherits() {
    let (mut state, collection_id, item_id) = state_with_request();
    let collection = state.workspace.collection_mut(&collection_id).unwrap();
    collection.auth = Some(Auth::bearer("root"));
    collection.set_shared_headers(&[Param::text("X-Tenant", "shop")]);
    let tab = open(&mut state, &collection_id, &item_id);
    let inheritance = state.inheritance(tab);
    let auth = inheritance.auth.unwrap();
    assert_eq!(auth.kind, "Bearer token");
    assert_eq!(auth.owner, "Sample");
    assert_eq!(inheritance.headers.len(), 1);
}

#[test]
fn a_scratch_request_inherits_nothing() {
    let (mut state, _, _) = state_with_request();
    state.apply(crate::state::Action::NewTab, 0.0);
    let tab = state.active_tab.unwrap();
    assert_eq!(state.inheritance(tab), Default::default());
}
