use requestor_core::collection::{Collection, Item, Request, Url};
use requestor_core::environment::Environment;
use requestor_core::workspace::{LoadedWorkspace, ThemeName, Workspace, WorkspaceStore};

use super::{Entry, Query};
use crate::state::{Action, AppState};

const STORE_ROOT: &str = "requestor-palette-tests";

fn state() -> (AppState, String, String) {
    let mut collection = Collection::new("Shop");
    let folder = collection
        .add_item(None, Item::folder("Orders".to_string(), Vec::new()))
        .unwrap();
    let request = Request::new("POST", Url::Raw("https://x.test".to_string()));
    let item_id = collection
        .add_item(
            Some(&folder),
            Item::for_request("Create order".to_string(), request),
        )
        .unwrap();
    let mut workspace = Workspace::default();
    let collection_id = workspace.add_collection(collection);
    workspace.add_environment(Environment::new("Staging"));
    let loaded = LoadedWorkspace {
        workspace,
        ..LoadedWorkspace::default()
    };
    let state = AppState::new(loaded, WorkspaceStore::new(STORE_ROOT));
    (state, collection_id, item_id)
}

fn titles(entries: &[&Entry]) -> Vec<String> {
    entries.iter().map(|entry| entry.title.clone()).collect()
}

#[test]
fn every_word_must_match_the_title_or_the_folders() {
    let (state, _, _) = state();
    let entries = Entry::all(&state);
    let ranked = Query::new("ORDER shop").rank(&entries);
    assert_eq!(titles(&ranked), ["Create order"]);
    assert_eq!(ranked[0].detail, "Shop / Orders");
    assert!(Query::new("order missing").rank(&entries).is_empty());
}

#[test]
fn earlier_matches_rank_first_and_an_empty_query_lists_everything() {
    let (state, _, _) = state();
    let entries = Entry::all(&state);
    let ranked = Query::new("tab").rank(&entries);
    assert_eq!(titles(&ranked[..2]), ["New tab", "Next tab"]);
    assert_eq!(Query::new("  ").rank(&entries).len(), entries.len());
}

#[test]
fn running_an_entry_opens_selects_or_applies_it() {
    let (mut state, collection_id, item_id) = state();
    let entries = Entry::all(&state);
    let mut actions = Vec::new();
    for query in ["create order", "theme midnight", "no environment"] {
        Query::new(query).rank(&entries)[0].run(&mut state, &mut actions);
    }
    assert!(matches!(
        &actions[0],
        Action::OpenItem { collection_id: opened, item_id: item } if *opened == collection_id && *item == item_id
    ));
    assert!(matches!(actions[1], Action::SettingsEdited));
    assert!(matches!(actions[2], Action::SelectEnvironment(None)));
    assert_eq!(
        state.workspace.settings.interface.theme,
        ThemeName::Midnight
    );
}

#[test]
fn the_postman_migration_is_reachable_from_the_palette() {
    let (mut state, _, _) = state();
    let entries = Entry::all(&state);
    let mut actions = Vec::new();
    Query::new("postman").rank(&entries)[0].run(&mut state, &mut actions);
    assert!(matches!(actions[..], [Action::OpenPostmanMigration]));
}
