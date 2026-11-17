use std::path::PathBuf;

use requestor_core::collection::Collection;
use requestor_core::workspace::Workspace;

use super::{state_with, state_with_request};
use crate::state::{Action, Effect, LibraryTarget, Overlay};

const IMPORT_DIRECTORY: &str = "requestor-transfer-tests";

fn scratch_file(name: &str, contents: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("{IMPORT_DIRECTORY}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn imported_text_becomes_a_collection_a_tab_or_an_error_on_the_form() {
    let mut state = state_with(Workspace::default());
    state.apply(
        Action::ImportText("curl https://example.test/a".to_string()),
        0.0,
    );
    assert_eq!(state.active().unwrap().draft.url, "https://example.test/a");

    let collection = Collection::new("Imported").to_json_pretty().unwrap();
    state.apply(Action::ImportText(collection), 0.0);
    assert_eq!(state.workspace.collections.len(), 1);

    state.apply(Action::Show(Overlay::Import(Default::default())), 0.0);
    state.apply(Action::ImportText("{\"neither\":true}".to_string()), 0.0);
    let Some(Overlay::Import(form)) = &state.overlay else {
        panic!("the import form stays open");
    };
    assert!(form.error.is_some());
}

#[test]
fn choosing_files_asks_the_shell_for_a_native_picker() {
    let (mut state, collection_id, _) = state_with_request();
    let effects = state.apply(Action::ChooseImportFile, 0.0);
    assert!(matches!(effects.as_slice(), [Effect::PickImportFile]));
    let target = LibraryTarget::Collection { collection_id };
    let effects = state.apply(
        Action::ChooseExportLocation {
            target: target.clone(),
            suggested: "Sample.postman_collection.json".into(),
        },
        0.0,
    );
    assert!(matches!(
        effects.as_slice(),
        [Effect::PickExportLocation { target: chosen, .. }] if *chosen == target
    ));
}

#[test]
fn an_imported_file_closes_only_the_import_dialog() {
    let path = scratch_file(
        "dropped.postman_collection.json",
        &Collection::new("Dropped").to_json_pretty().unwrap(),
    );
    let mut state = state_with(Workspace::default());
    state.apply(Action::Show(Overlay::Settings), 0.0);
    state.apply(Action::ImportFile(path.clone()), 0.0);
    assert_eq!(state.overlay, Some(Overlay::Settings));
    assert_eq!(state.workspace.collections.len(), 1);
    state.apply(Action::Show(Overlay::Import(Default::default())), 0.0);
    state.apply(Action::ImportFile(path), 0.0);
    assert!(state.overlay.is_none());
    assert_eq!(state.workspace.collections.len(), 2);
}

#[test]
fn an_unreadable_file_reports_on_the_form_or_in_the_status_bar() {
    let missing = std::env::temp_dir().join("requestor-transfer-tests-missing.json");
    let mut state = state_with(Workspace::default());
    state.apply(Action::Show(Overlay::Import(Default::default())), 0.0);
    state.apply(Action::ImportFile(missing.clone()), 0.0);
    let Some(Overlay::Import(form)) = &state.overlay else {
        panic!("the import dialog should stay open");
    };
    assert!(form
        .error
        .as_deref()
        .is_some_and(|error| error.starts_with("Cannot read")));
    state.overlay = None;
    state.apply(Action::ImportFile(missing), 0.0);
    assert!(state.overlay.is_none());
    assert!(state.notices.current(0.0).is_some());
}
