use std::fs;
use std::path::{Path, PathBuf};

use requestor_core::workspace::DataFolder;

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir()
        .join("requestor-data-folder-tests")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn workspace_at(folder: &Path, collection: &str) {
    fs::create_dir_all(folder.join("collections")).unwrap();
    fs::write(folder.join("settings.json"), "{}").unwrap();
    fs::write(folder.join("collections/api.json"), collection).unwrap();
}

fn collection_at(folder: &Path) -> String {
    fs::read_to_string(folder.join("collections/api.json")).unwrap()
}

#[test]
fn moving_to_an_empty_folder_copies_the_data_at_the_next_start() {
    let root = scratch("copy");
    let home = root.join("home");
    workspace_at(&home, "old");
    let folder = DataFolder::new(&home);
    assert!(folder.is_default());
    let synced = root.join("Dropbox/Requestor");
    folder.move_to(&synced).unwrap();
    assert_eq!(folder.location(), home);
    assert_eq!(folder.next_location(), synced);
    workspace_at(&home, "latest");
    folder.settle().unwrap();
    assert_eq!(folder.location(), synced);
    assert_eq!(collection_at(&synced), "latest");
    assert!(!synced.join("data-folder.txt").exists());
    assert!(!synced.join(".requestor-incoming").exists());
}

#[test]
fn moving_to_a_folder_with_data_adopts_it_without_copying() {
    let root = scratch("adopt");
    let home = root.join("home");
    workspace_at(&home, "local");
    let shared = root.join("shared");
    workspace_at(&shared, "shared");
    let folder = DataFolder::new(&home);
    folder.move_to(&shared).unwrap();
    folder.settle().unwrap();
    assert_eq!(collection_at(&shared), "shared");
    assert_eq!(folder.location(), shared);
}

#[test]
fn going_back_to_the_default_folder_brings_the_current_data_along() {
    let root = scratch("return");
    let home = root.join("home");
    workspace_at(&home, "stale");
    let synced = root.join("synced");
    let folder = DataFolder::new(&home);
    folder.move_to(&synced).unwrap();
    folder.settle().unwrap();
    workspace_at(&synced, "edited elsewhere");
    folder.reset().unwrap();
    assert_eq!(folder.location(), synced);
    folder.settle().unwrap();
    assert!(folder.is_default());
    assert_eq!(collection_at(&home), "edited elsewhere");
}

#[test]
fn choosing_the_current_folder_cancels_a_pending_move() {
    let root = scratch("cancel");
    let home = root.join("home");
    workspace_at(&home, "data");
    let folder = DataFolder::new(&home);
    folder.move_to(&root.join("elsewhere")).unwrap();
    folder.move_to(&home).unwrap();
    assert!(folder.is_default());
    assert_eq!(folder.next_location(), home);
}

#[test]
fn a_failed_copy_keeps_the_previous_folder_on_every_start() {
    let root = scratch("failed");
    let home = root.join("home");
    let missing = root.join("gone");
    workspace_at(&missing, "data");
    let folder = DataFolder::new(&home);
    fs::create_dir_all(&home).unwrap();
    folder.move_to(&missing).unwrap();
    folder.settle().unwrap();
    let target = root.join("target");
    folder.move_to(&target).unwrap();
    fs::remove_dir_all(&missing).unwrap();
    for _ in 0..2 {
        assert!(folder.settle().is_err());
        assert_eq!(folder.location(), missing);
        assert_eq!(folder.next_location(), target);
    }
}

#[test]
fn unsuitable_folders_are_refused_and_left_untouched() {
    let root = scratch("refused");
    let home = root.join("home");
    workspace_at(&home, "data");
    let folder = DataFolder::new(&home);
    assert!(folder.move_to(&home.join("inner")).is_err());
    assert!(!home.join("inner").exists());
    assert!(folder.move_to(&root).is_err());
    let documents = root.join("Documents");
    fs::create_dir_all(&documents).unwrap();
    fs::write(documents.join("notes.txt"), "mine").unwrap();
    assert!(folder.move_to(&documents).is_err());
    assert!(folder.is_default());
}

#[test]
fn a_missing_synced_folder_is_reported() {
    let root = scratch("unmounted");
    let home = root.join("home");
    workspace_at(&home, "data");
    let synced = root.join("drive");
    let folder = DataFolder::new(&home);
    folder.move_to(&synced).unwrap();
    folder.settle().unwrap();
    assert_eq!(folder.unavailable(), None);
    fs::remove_dir_all(&synced).unwrap();
    assert_eq!(folder.unavailable(), Some(synced));
    assert_eq!(folder.location(), home);
}
