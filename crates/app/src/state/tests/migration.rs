use std::path::PathBuf;

use requestor_core::collection::Collection;
use requestor_core::import::import_text;
use requestor_core::migration::MigrationReport;
use requestor_core::workspace::Workspace;

use super::state_with;
use crate::state::{Action, Effect, FolderPurpose, Overlay};

const ENVIRONMENT: &str = r#"{"name":"Staging","values":[]}"#;

fn started_job(effects: &[Effect]) -> u64 {
    match effects {
        [Effect::ImportFiles { job, .. }] => *job,
        other => panic!("expected an import job, got {other:?}"),
    }
}

#[test]
fn importing_files_opens_the_migration_dialog_and_works_in_the_background() {
    let mut state = state_with(Workspace::default());
    let effects = state.apply(Action::ImportFiles(vec![PathBuf::from("a.json")]), 0.0);

    started_job(&effects);
    assert_eq!(state.overlay, Some(Overlay::PostmanMigration));
    assert!(state.migration.is_working());
}

#[test]
fn a_finished_migration_adds_everything_and_lists_what_failed() {
    let mut state = state_with(Workspace::default());
    let job = started_job(&state.apply(Action::ImportFiles(Vec::new()), 0.0));
    let mut report = MigrationReport::default();
    report.add_collection(Collection::new("Orders"));
    report.record("staging.json", import_text(ENVIRONMENT));
    report.fail("broken.json", "invalid JSON");

    state.apply(
        Action::MigrationFinished {
            job,
            report: Box::new(report),
        },
        0.0,
    );

    assert_eq!(state.workspace.collections.len(), 1);
    assert_eq!(state.workspace.environments.len(), 1);
    assert_eq!(state.overlay, Some(Overlay::PostmanMigration));
    assert!(!state.migration.is_working());
    let outcome = state.migration.outcome.as_ref().unwrap();
    assert_eq!(outcome.failures[0].source, "broken.json");
}

#[test]
fn a_migration_finished_after_the_dialog_closed_still_imports() {
    let mut state = state_with(Workspace::default());
    let job = started_job(&state.apply(Action::ImportFiles(Vec::new()), 0.0));
    state.overlay = None;
    let mut report = MigrationReport::default();
    report.add_collection(Collection::new("Orders"));

    state.apply(
        Action::MigrationFinished {
            job,
            report: Box::new(report),
        },
        0.0,
    );

    assert_eq!(state.workspace.collections.len(), 1);
    assert!(state.migration.outcome.is_some());
}

#[test]
fn a_second_import_waits_for_the_first() {
    let mut state = state_with(Workspace::default());
    started_job(&state.apply(Action::ImportFiles(Vec::new()), 0.0));

    let effects = state.apply(Action::ImportFiles(Vec::new()), 0.0);

    assert!(effects.is_empty());
}

#[test]
fn dropping_files_keeps_another_open_dialog() {
    let mut state = state_with(Workspace::default());
    state.apply(Action::Show(Overlay::Settings), 0.0);

    let paths = vec![PathBuf::from("a.json"), PathBuf::from("b.json")];
    let effects = state.apply(Action::FilesDropped(paths), 0.0);

    started_job(&effects);
    assert_eq!(state.overlay, Some(Overlay::Settings));
}

#[test]
fn requests_among_dropped_files_open_as_tabs() {
    let mut state = state_with(Workspace::default());
    let job = started_job(&state.apply(Action::ImportFiles(Vec::new()), 0.0));
    let mut report = MigrationReport::default();
    report.record("a.curl", import_text("curl https://example.test/a"));

    state.apply(
        Action::MigrationFinished {
            job,
            report: Box::new(report),
        },
        0.0,
    );

    assert_eq!(state.active().unwrap().draft.url, "https://example.test/a");
}

#[test]
fn choosing_postman_exports_asks_for_files_or_a_folder() {
    let mut state = state_with(Workspace::default());
    let effects = state.apply(Action::ChooseImportFiles, 0.0);
    assert!(matches!(effects.as_slice(), [Effect::PickImportFiles]));
    let effects = state.apply(
        Action::FolderPicked {
            purpose: FolderPurpose::PostmanExport,
            path: PathBuf::from("export"),
        },
        0.0,
    );
    started_job(&effects);
}
