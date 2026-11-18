use requestor_core::settings::{ProxyChoice, RequestOverrides};

use super::{open, state_with_request};
use crate::state::tab::RequestTab;
use crate::state::{Action, Effect};

fn insecure_without_proxy() -> RequestOverrides {
    RequestOverrides {
        verify_ssl: Some(false),
        proxy: ProxyChoice::Direct,
        ..RequestOverrides::default()
    }
}

#[test]
fn a_sent_request_uses_its_own_settings_on_top_of_the_app_settings() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    state.tab_mut(tab).unwrap().draft.overrides = insecure_without_proxy();
    let effects = state.apply(Action::Send(tab), 1.0);
    let [Effect::Send { settings, .. }] = effects.as_slice() else {
        panic!("expected one send effect");
    };
    assert!(!settings.verify_ssl);
    assert!(!settings.use_system_proxy);
    assert_eq!(
        settings.request_timeout_ms,
        state.workspace.settings.request.request_timeout_ms
    );
    assert!(state.workspace.settings.request.verify_ssl);
}

#[test]
fn saved_request_settings_are_stored_on_the_item_and_reopened() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    state.tab_mut(tab).unwrap().draft.overrides = insecure_without_proxy();
    state.apply(Action::TabEdited(tab), 1.0);
    state.apply(Action::SaveTab(tab), 2.0);
    let item = state
        .workspace
        .collection(&collection_id)
        .and_then(|collection| collection.find_item(&item_id))
        .unwrap();
    assert_eq!(item.extra["protocolProfileBehavior"]["strictSSL"], false);
    assert!(!state.tab(tab).unwrap().unsaved);

    let reopened =
        RequestTab::for_item(tab, state.tab(tab).unwrap().binding.clone().unwrap(), item).unwrap();
    assert_eq!(reopened.draft.overrides, insecure_without_proxy());
    assert!(reopened.draft.is_unchanged());
}

#[test]
fn unsaved_request_settings_survive_a_restart() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    state.tab_mut(tab).unwrap().draft.overrides = insecure_without_proxy();
    state.apply(Action::TabEdited(tab), 1.0);
    let session = state.tab(tab).unwrap().to_session();
    let restored = RequestTab::from_session(tab, &session, &state.workspace);
    assert_eq!(restored.draft.overrides, insecure_without_proxy());
    assert!(restored.unsaved);
}
