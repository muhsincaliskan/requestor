use super::{open, response, state_with_request};
use crate::response::{BodyLayout, ResponseView};
use crate::state::{Action, AppState, Effect, Exchange, TabId};

#[test]
fn repeated_sends_keep_the_tree_layout_and_apply_the_query_to_the_new_body() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    finish(&mut state, tab, Ok(br#"{"items":[1]}"#));
    let view = completed(&mut state, tab);
    view.set_layout(BodyLayout::Tree);
    view.set_filter("$.items[*]".to_string());
    assert_eq!(view.filter_result_count(), Some(1));

    finish(&mut state, tab, Err("connection closed"));
    finish(&mut state, tab, Ok(br#"{"items":[2,3]}"#));
    let view = completed(&mut state, tab);
    assert_eq!(view.layout(), BodyLayout::Tree);
    assert_eq!(view.inspection().filter_text(), "$.items[*]");
    assert_eq!(view.filter_result_count(), Some(2));
}

#[test]
fn retrying_keeps_the_last_valid_filter_while_an_edit_is_incomplete() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    finish(&mut state, tab, Ok(br#"{"items":[1]}"#));
    let view = completed(&mut state, tab);
    view.set_filter("$.items[*]".to_string());
    view.set_filter("$.items[".to_string());

    finish(&mut state, tab, Ok(br#"{"items":[2,3,4]}"#));
    let view = completed(&mut state, tab);
    assert_eq!(view.inspection().filter_text(), "$.items[");
    assert!(view.inspection().error().is_some());
    assert_eq!(view.filter_result_count(), Some(3));
}

fn finish(state: &mut AppState, tab: TabId, result: Result<&[u8], &str>) {
    let effects = state.apply(Action::Send(tab), 0.0);
    let [Effect::Send { job, .. }] = effects.as_slice() else {
        panic!("expected a send, got {effects:?}");
    };
    state.apply(
        Action::SendFinished {
            tab,
            job: *job,
            result: result.map(response).map_err(str::to_string),
        },
        0.0,
    );
}

fn completed(state: &mut AppState, tab: TabId) -> &mut ResponseView {
    match &mut state.tab_mut(tab).unwrap().exchange {
        Exchange::Completed(view) => view,
        _ => panic!("expected a completed response"),
    }
}
