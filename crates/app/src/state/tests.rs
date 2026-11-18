use std::time::Duration;

use requestor_core::collection::{Collection, Item, Request, Url};
use requestor_core::response::HttpResponse;
use requestor_core::workspace::{LoadedWorkspace, StoreWrite, Workspace, WorkspaceStore};

use super::autosave::QUIET_SECONDS;
use super::{Action, AppState, Effect, Exchange, LibraryTarget, Overlay, SaveTabForm, TabId};

const STORE_ROOT: &str = "requestor-state-tests";
const LATER: f64 = QUIET_SECONDS + 1.0;

fn state_with(workspace: Workspace) -> AppState {
    let loaded = LoadedWorkspace {
        workspace,
        ..LoadedWorkspace::default()
    };
    AppState::new(loaded, WorkspaceStore::new(STORE_ROOT))
}

fn state_with_request() -> (AppState, String, String) {
    let mut collection = Collection::new("Sample");
    let request = Request::new("GET", Url::Raw("https://example.test/users".to_string()));
    let item_id = collection
        .add_item(None, Item::for_request("list".to_string(), request))
        .unwrap();
    let mut workspace = Workspace::default();
    let collection_id = workspace.add_collection(collection);
    (state_with(workspace), collection_id, item_id)
}

fn open(state: &mut AppState, collection_id: &str, item_id: &str) -> TabId {
    state.apply(
        Action::OpenItem {
            collection_id: collection_id.to_string(),
            item_id: item_id.to_string(),
        },
        0.0,
    );
    state.active_tab.unwrap()
}

fn edit_url(state: &mut AppState, tab: TabId, url: &str, now: f64) {
    state.tab_mut(tab).unwrap().draft.url = url.to_string();
    state.apply(Action::TabEdited(tab), now);
}

fn stored_url(state: &AppState, collection_id: &str, item_id: &str) -> String {
    let request = state
        .workspace
        .collection(collection_id)
        .and_then(|collection| collection.find_item(item_id))
        .and_then(Item::full_request)
        .unwrap();
    request.url.unwrap().to_sendable_text()
}

fn response(body: &[u8]) -> HttpResponse {
    HttpResponse {
        status: 200,
        reason: "OK".to_string(),
        version: "HTTP/1.1".to_string(),
        headers: Vec::new(),
        body: body.to_vec(),
        truncated: false,
        elapsed: Duration::ZERO,
    }
}

fn persisted_count(effects: &[Effect]) -> usize {
    effects
        .iter()
        .filter(|effect| matches!(effect, Effect::Persist(_)))
        .count()
}

#[test]
fn opening_the_same_item_twice_reuses_its_tab() {
    let (mut state, collection_id, item_id) = state_with_request();
    let first = open(&mut state, &collection_id, &item_id);
    state.apply(Action::NewTab, 0.0);
    let again = open(&mut state, &collection_id, &item_id);
    assert_eq!(first, again);
    assert_eq!(state.tabs.len(), 2);
}

#[test]
fn edits_are_saved_only_after_the_quiet_period() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    edit_url(&mut state, tab, "https://example.test/changed", 10.0);

    assert!(state.tick(10.0 + QUIET_SECONDS / 2.0).is_empty());
    assert!(state.tab(tab).unwrap().unsaved);
    assert!(state.seconds_until_next_save(10.0).is_some());

    let effects = state.tick(10.0 + LATER);
    assert!(!state.tab(tab).unwrap().unsaved);
    assert_eq!(
        stored_url(&state, &collection_id, &item_id),
        "https://example.test/changed"
    );
    assert!(effects.is_empty());
    assert_eq!(persisted_count(&state.tick(10.0 + 2.0 * LATER)), 2);
    assert!(state.seconds_until_next_save(100.0).is_none());
}

#[test]
fn with_autosave_off_edits_wait_for_an_explicit_save() {
    let (mut state, collection_id, item_id) = state_with_request();
    state.workspace.settings.interface.autosave = false;
    let tab = open(&mut state, &collection_id, &item_id);
    edit_url(&mut state, tab, "https://example.test/changed", 0.0);
    state.tick(LATER);
    assert!(state.tab(tab).unwrap().unsaved);
    assert_eq!(
        stored_url(&state, &collection_id, &item_id),
        "https://example.test/users"
    );

    state.apply(Action::CloseTab(tab), LATER);
    assert_eq!(state.overlay, Some(Overlay::ConfirmCloseTab(tab)));

    state.apply(Action::SaveTab(tab), LATER);
    assert_eq!(
        stored_url(&state, &collection_id, &item_id),
        "https://example.test/changed"
    );
    state.apply(Action::ForceCloseTab(tab), LATER);
    assert!(state.tabs.is_empty());
    assert_eq!(state.overlay, None);
}

#[test]
fn closing_writes_pending_edits_when_autosave_is_on() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    edit_url(&mut state, tab, "https://example.test/changed", 0.0);
    state.apply(Action::CloseTab(tab), 0.1);
    assert!(state.tabs.is_empty());
    assert_eq!(
        stored_url(&state, &collection_id, &item_id),
        "https://example.test/changed"
    );
}

#[test]
fn flushing_saves_everything_without_waiting() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    edit_url(&mut state, tab, "https://example.test/changed", 0.0);
    let effects = state.flush();
    assert_eq!(persisted_count(&effects), 2);
    assert_eq!(
        stored_url(&state, &collection_id, &item_id),
        "https://example.test/changed"
    );
}

#[test]
fn an_untouched_scratch_tab_closes_without_asking_and_an_edited_one_asks() {
    let mut state = state_with(Workspace::default());
    state.apply(Action::NewTab, 0.0);
    let untouched = state.active_tab.unwrap();
    state.apply(Action::CloseTab(untouched), 0.0);
    assert!(state.tabs.is_empty());

    state.apply(Action::NewTab, 0.0);
    let edited = state.active_tab.unwrap();
    edit_url(&mut state, edited, "https://example.test", 0.0);
    state.apply(Action::CloseTab(edited), 0.0);
    assert_eq!(state.overlay, Some(Overlay::ConfirmCloseTab(edited)));
    assert_eq!(state.tabs.len(), 1);
}

#[test]
fn a_scratch_tab_is_saved_into_a_collection_and_becomes_bound() {
    let (mut state, collection_id, _) = state_with_request();
    state.apply(Action::NewTab, 0.0);
    let tab = state.active_tab.unwrap();
    edit_url(&mut state, tab, "https://example.test/new", 0.0);
    state.apply(Action::SaveTab(tab), 0.0);
    assert!(matches!(state.overlay, Some(Overlay::SaveTab(_))));

    state.apply(
        Action::SaveTabInto {
            tab,
            name: "created".to_string(),
            collection_id: collection_id.clone(),
            folder_id: None,
            close_after: false,
        },
        0.0,
    );
    let saved = state.tab(tab).unwrap();
    let binding = saved.binding.clone().unwrap();
    assert!(!saved.unsaved);
    assert_eq!(state.overlay, None);
    assert_eq!(
        stored_url(&state, &collection_id, &binding.item_id),
        "https://example.test/new"
    );
}

#[test]
fn a_collection_created_while_saving_returns_to_the_save_form() {
    let mut state = state_with(Workspace::default());
    state.apply(Action::NewTab, 0.0);
    let tab = state.active_tab.unwrap();
    state.apply(Action::SaveTab(tab), 0.0);
    let Some(Overlay::SaveTab(form)) = state.overlay.clone() else {
        panic!("expected the save form, got {:?}", state.overlay);
    };
    assert_eq!(form.collection_id, None);

    state.apply(Action::NewCollectionForSave(form), 0.0);
    let created = state.workspace.collections[0].id().to_string();
    let Some(Overlay::SaveTab(form)) = state.overlay.clone() else {
        panic!("expected the save form, got {:?}", state.overlay);
    };
    assert_eq!(form.collection_id, Some(created.clone()));
    assert_eq!(form.folder_id, None);
    let unnamed = SaveTabForm {
        name: "  ".to_string(),
        ..form.clone()
    };
    assert!(unnamed.submission().is_none());

    state.apply(form.submission().unwrap(), 0.0);
    let saved = state.tab(tab).unwrap();
    assert_eq!(saved.binding.as_ref().unwrap().collection_id, created);
    assert_eq!(state.overlay, None);
}

#[test]
fn sending_produces_one_effect_and_stale_results_are_ignored() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    let effects = state.apply(Action::Send(tab), 1.0);
    let [Effect::Send { job, request, .. }] = effects.as_slice() else {
        panic!("expected one send effect");
    };
    assert_eq!(request.url, "https://example.test/users");
    assert!(state.tab(tab).unwrap().is_sending());

    state.apply(
        Action::SendFinished {
            tab,
            job: job + 1,
            result: Ok(response(b"stale")),
        },
        2.0,
    );
    assert!(state.tab(tab).unwrap().is_sending());

    state.apply(
        Action::SendFinished {
            tab,
            job: *job,
            result: Ok(response(b"{}")),
        },
        2.0,
    );
    assert!(matches!(
        state.tab(tab).unwrap().exchange,
        Exchange::Completed(_)
    ));
}

#[test]
fn a_cancelled_send_ignores_the_late_response() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    let effects = state.apply(Action::Send(tab), 0.0);
    let [Effect::Send { job, .. }] = effects.as_slice() else {
        panic!("expected one send effect");
    };
    let cancelled = state.apply(Action::CancelSend(tab), 0.5);
    assert!(matches!(cancelled.as_slice(), [Effect::AbortSend(aborted)] if aborted == job));
    state.apply(
        Action::SendFinished {
            tab,
            job: *job,
            result: Err("late".to_string()),
        },
        1.0,
    );
    assert!(matches!(state.tab(tab).unwrap().exchange, Exchange::Idle));
}

#[test]
fn only_the_active_tab_keeps_a_display_copy_of_its_response() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    let effects = state.apply(Action::Send(tab), 0.0);
    let [Effect::Send { job, .. }] = effects.as_slice() else {
        panic!("expected one send effect");
    };
    state.apply(
        Action::SendFinished {
            tab,
            job: *job,
            result: Ok(response(b"{\"a\":1}")),
        },
        0.0,
    );
    let holds_display = |state: &mut AppState| match &mut state.tab_mut(tab).unwrap().exchange {
        Exchange::Completed(view) => view.holds_display(),
        _ => false,
    };
    if let Exchange::Completed(view) = &mut state.tab_mut(tab).unwrap().exchange {
        view.display();
    }
    assert!(holds_display(&mut state));
    state.apply(Action::NewTab, 0.0);
    assert!(!holds_display(&mut state));
}

#[test]
fn deleting_an_item_unbinds_its_open_tab_instead_of_losing_the_work() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    state.apply(
        Action::Delete(LibraryTarget::Item {
            collection_id: collection_id.clone(),
            item_id,
        }),
        0.0,
    );
    let orphan = state.tab(tab).unwrap();
    assert!(orphan.binding.is_none());
    assert!(orphan.unsaved);
    assert_eq!(orphan.draft.url, "https://example.test/users");
}

#[test]
fn deleting_a_saved_collection_removes_its_file_and_never_rewrites_it() {
    let (mut state, collection_id, _) = state_with_request();
    state.apply(Action::CollectionEdited(collection_id.clone()), 0.0);
    assert_eq!(persisted_count(&state.tick(LATER)), 1);
    state.apply(Action::CollectionEdited(collection_id.clone()), LATER);
    let effects = state.apply(
        Action::Delete(LibraryTarget::Collection {
            collection_id: collection_id.clone(),
        }),
        LATER,
    );
    assert!(matches!(
        effects.as_slice(),
        [Effect::Persist(StoreWrite::Delete(delete))]
            if delete.path().ends_with(format!("{collection_id}.json"))
    ));
    assert_eq!(persisted_count(&state.tick(LATER * 3.0)), 1);
}

#[test]
fn deleting_before_the_first_save_deletes_no_file() {
    let (mut state, collection_id, _) = state_with_request();
    let effects = state.apply(
        Action::Delete(LibraryTarget::Collection { collection_id }),
        0.0,
    );
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        Effect::DeleteFile(_) | Effect::Persist(StoreWrite::Delete(_))
    )));
}

#[test]
fn renaming_an_item_renames_its_open_tab() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    state.apply(
        Action::Rename {
            target: LibraryTarget::Item {
                collection_id,
                item_id,
            },
            name: "  renamed ".to_string(),
        },
        0.0,
    );
    assert_eq!(state.tab(tab).unwrap().draft.name, "renamed");
}

#[test]
fn the_session_is_restored_with_unsaved_work() {
    let (mut state, collection_id, item_id) = state_with_request();
    state.workspace.settings.interface.autosave = false;
    let bound = open(&mut state, &collection_id, &item_id);
    edit_url(&mut state, bound, "https://example.test/draft", 0.0);
    state.apply(Action::NewTab, 0.0);
    state.flush();

    let restored = state_with(state.workspace.clone());
    assert_eq!(restored.tabs.len(), 2);
    assert_eq!(restored.tabs[0].draft.url, "https://example.test/draft");
    assert!(restored.tabs[0].unsaved);
    assert!(restored.tabs[0].is_bound_to(&collection_id, &item_id));
    assert_eq!(restored.active_tab, Some(restored.tabs[1].id));
}

#[test]
fn a_new_environment_is_put_in_use() {
    let mut state = state_with(Workspace::default());
    state.apply(Action::NewEnvironment, 0.0);
    let id = state.workspace.environments[0].id.clone();
    assert!(id.is_some());
    assert_eq!(state.workspace.session.active_environment, id);
}

#[test]
fn neighbour_activation_wraps_around() {
    let mut state = state_with(Workspace::default());
    for _ in 0..3 {
        state.apply(Action::NewTab, 0.0);
    }
    let ids: Vec<TabId> = state.tabs.iter().map(|tab| tab.id).collect();
    state.apply(Action::ActivateNeighbourTab { forward: true }, 0.0);
    assert_eq!(state.active_tab, Some(ids[0]));
    state.apply(Action::ActivateNeighbourTab { forward: false }, 0.0);
    assert_eq!(state.active_tab, Some(ids[2]));
}

#[test]
fn saving_from_the_close_prompt_also_closes_the_tab() {
    let (mut state, collection_id, item_id) = state_with_request();
    state.workspace.settings.interface.autosave = false;
    let tab = open(&mut state, &collection_id, &item_id);
    edit_url(&mut state, tab, "https://example.test/changed", 0.0);
    state.apply(Action::CloseTab(tab), 0.0);
    state.apply(Action::SaveAndCloseTab(tab), 0.0);
    assert!(state.tabs.is_empty());
    assert_eq!(state.overlay, None);
    assert_eq!(
        stored_url(&state, &collection_id, &item_id),
        "https://example.test/changed"
    );
}

#[test]
fn closing_other_tabs_asks_about_the_first_unsaved_one() {
    let mut state = state_with(Workspace::default());
    for _ in 0..3 {
        state.apply(Action::NewTab, 0.0);
    }
    let ids: Vec<TabId> = state.tabs.iter().map(|tab| tab.id).collect();
    edit_url(&mut state, ids[0], "https://example.test/kept", 0.0);
    state.apply(Action::CloseOtherTabs(ids[2]), 0.0);
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(state.overlay, Some(Overlay::ConfirmCloseTab(ids[0])));
    assert_eq!(state.active_tab, Some(ids[2]));
}

#[test]
fn unsaved_work_restored_from_the_session_is_autosaved() {
    let (mut state, collection_id, item_id) = state_with_request();
    state.workspace.settings.interface.autosave = false;
    let tab = open(&mut state, &collection_id, &item_id);
    edit_url(&mut state, tab, "https://example.test/draft", 0.0);
    state.flush();

    let mut workspace = state.workspace.clone();
    workspace.settings.interface.autosave = true;
    let mut restored = state_with(workspace);
    restored.tick(LATER);
    assert!(!restored.tabs[0].unsaved);
    assert_eq!(
        stored_url(&restored, &collection_id, &item_id),
        "https://example.test/draft"
    );
}

mod certificates;
mod changes;
mod code;
mod examples;
mod folders;
mod history;
mod rules;
mod schema;
mod secrets;
mod sockets;
mod streaming;
mod transfer;
mod unsaved;
mod updates;
