use std::fs;
use std::path::{Path, PathBuf};

use requestor_core::identity::random_uuid;
use requestor_core::workspace::{StoreWrite, Workspace};

use super::{state_with, state_with_request, LATER};
use crate::state::{Action, AppState, Effect, FolderPurpose, LibraryTarget, NoticeLevel};

const FOLDER_PREFIX: &str = "requestor-app-folder";

struct TemporaryFolder(PathBuf);

impl TemporaryFolder {
    fn new() -> Self {
        TemporaryFolder(std::env::temp_dir().join(format!("{FOLDER_PREFIX}-{}", random_uuid())))
    }
}

impl Drop for TemporaryFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn pick(state: &mut AppState, purpose: FolderPurpose, path: &Path, now: f64) -> Vec<Effect> {
    state.apply(
        Action::FolderPicked {
            purpose,
            path: path.to_path_buf(),
        },
        now,
    )
}

fn folder_writes(write: &StoreWrite) -> Vec<&StoreWrite> {
    match write {
        StoreWrite::Batch(writes) => writes.iter().flat_map(folder_writes).collect(),
        StoreWrite::Folder(_) => vec![write],
        _ => Vec::new(),
    }
}

fn folder_roots(effects: &[Effect]) -> Vec<&Path> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Persist(write) => Some(folder_writes(write)),
            _ => None,
        })
        .flatten()
        .map(StoreWrite::path)
        .collect()
}

fn commit_folders(effects: &[Effect]) {
    for effect in effects {
        if let Effect::Persist(write) = effect {
            for folder in folder_writes(write) {
                folder.clone().commit().unwrap();
            }
        }
    }
}

fn last_error(state: &AppState, now: f64) -> Option<&str> {
    state
        .notices
        .current(now)
        .filter(|notice| notice.level == NoticeLevel::Error)
        .map(|notice| notice.text.as_str())
}

#[test]
fn a_collection_saved_to_a_folder_keeps_saving_there() {
    let folder = TemporaryFolder::new();
    let (mut state, collection_id, _) = state_with_request();
    let purpose = FolderPurpose::SaveCollection(collection_id.clone());
    let effects = pick(&mut state, purpose, &folder.0, 0.0);
    assert_eq!(folder_roots(&effects), [folder.0.as_path()]);
    assert_eq!(
        state.linked_folder(&collection_id),
        Some(folder.0.as_path())
    );

    state.apply(Action::CollectionEdited(collection_id), 0.0);
    assert_eq!(folder_roots(&state.tick(LATER)), [folder.0.as_path()]);
}

#[test]
fn a_saved_folder_opens_as_a_collection_only_once() {
    let folder = TemporaryFolder::new();
    let (mut source, collection_id, _) = state_with_request();
    let purpose = FolderPurpose::SaveCollection(collection_id);
    commit_folders(&pick(&mut source, purpose, &folder.0, 0.0));

    let mut state = state_with(Workspace::default());
    pick(&mut state, FolderPurpose::OpenCollection, &folder.0, 0.0);
    assert_eq!(state.workspace.collections.len(), 1);
    let opened = state.workspace.collections[0].id().to_string();
    assert_eq!(state.linked_folder(&opened), Some(folder.0.as_path()));

    pick(&mut state, FolderPurpose::OpenCollection, &folder.0, 1.0);
    assert_eq!(state.workspace.collections.len(), 1);
    assert!(last_error(&state, 1.0).is_some());
}

#[test]
fn a_folder_holding_another_collection_is_never_taken_over() {
    let folder = TemporaryFolder::new();
    fs::create_dir_all(&folder.0).unwrap();
    fs::write(folder.0.join("collection.json"), "{}").unwrap();
    let (mut state, collection_id, _) = state_with_request();
    let purpose = FolderPurpose::SaveCollection(collection_id.clone());
    let effects = pick(&mut state, purpose, &folder.0, 0.0);
    assert!(effects.is_empty());
    assert!(state.linked_folder(&collection_id).is_none());
    assert!(last_error(&state, 0.0).is_some());
}

#[test]
fn stopping_or_deleting_leaves_the_folder_files_in_place() {
    let folder = TemporaryFolder::new();
    let (mut state, collection_id, _) = state_with_request();
    let purpose = FolderPurpose::SaveCollection(collection_id.clone());
    pick(&mut state, purpose, &folder.0, 0.0);

    state.apply(Action::StopSavingToFolder(collection_id.clone()), 0.0);
    assert!(state.linked_folder(&collection_id).is_none());
    assert!(folder_roots(&state.tick(LATER)).is_empty());

    let purpose = FolderPurpose::SaveCollection(collection_id.clone());
    pick(&mut state, purpose, &folder.0, LATER);
    let effects = state.apply(
        Action::Delete(LibraryTarget::Collection { collection_id }),
        LATER,
    );
    assert!(effects.iter().all(|effect| match effect {
        Effect::Persist(write) => !write.path().starts_with(&folder.0),
        _ => true,
    }));
}
