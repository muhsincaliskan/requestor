use std::collections::BTreeSet;
use std::path::PathBuf;

use super::documents::{collections_directory, listing, NO_ID_COLLECTION};
use super::TemporaryRoot;
use crate::collection::Collection;
use crate::environment::Environment;
use crate::workspace::{PendingDelete, PendingWrite};

const ENVIRONMENT_JSON: &str = r#"{"id":"shared","name":"Env","values":[]}"#;

fn environments_directory(root: &TemporaryRoot) -> PathBuf {
    let directory = root.0.join("environments");
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

fn named(id: &str) -> Collection {
    let mut collection = Collection::new("Case");
    collection.info.postman_id = Some(id.to_string());
    collection
}

fn lowercase_name(write: &PendingWrite) -> String {
    write
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_lowercase()
}

#[test]
fn ids_differing_only_in_case_never_share_a_file_name() {
    let root = TemporaryRoot::new();
    let mut store = root.store();
    let lower = store.collection_write(&named("abc")).unwrap();
    let upper = store.collection_write(&named("ABC")).unwrap();
    assert_ne!(lowercase_name(&lower), lowercase_name(&upper));
}

#[test]
fn a_remembered_file_that_was_renamed_away_is_written_to_a_new_path() {
    let root = TemporaryRoot::new();
    let directory = collections_directory(&root);
    let original = directory.join("My API.json");
    std::fs::write(&original, NO_ID_COLLECTION).unwrap();
    let mut store = root.store();
    let collection = store.load().workspace.collections[0].clone();
    std::fs::rename(&original, directory.join("Renamed.json")).unwrap();

    let write = store.collection_write(&collection).unwrap();
    assert_ne!(write.path(), original);
    write.commit().unwrap();
    assert_eq!(
        listing(&directory),
        BTreeSet::from(["Renamed.json".into(), format!("{}.json", collection.id())])
    );
}

#[test]
fn environments_keep_their_path_and_a_duplicate_id_gets_a_persisted_fresh_id() {
    let root = TemporaryRoot::new();
    let directory = environments_directory(&root);
    std::fs::write(directory.join("a.json"), ENVIRONMENT_JSON).unwrap();
    std::fs::write(directory.join("b.json"), ENVIRONMENT_JSON).unwrap();
    let mut store = root.store();

    let loaded = store.load();
    assert!(loaded.warnings.is_empty());
    let environments = loaded.workspace.environments;
    assert_eq!(environments.len(), 2);
    let fresh_id = environments[1].id.clone().unwrap();
    assert_ne!(fresh_id, "shared");

    let again = root.store().load().workspace.environments;
    assert_eq!(again[1].id.as_deref(), Some(fresh_id.as_str()));
    let write = store.environment_write(&environments[0]).unwrap();
    assert_eq!(write.path(), directory.join("a.json"));
    assert_eq!(
        store.release_environment_path(&fresh_id),
        Some(directory.join("b.json"))
    );
    let other = store.environment_write(&Environment::new("Other")).unwrap();
    assert_eq!(other.path().parent(), Some(directory.as_path()));
}

#[test]
fn a_file_rewritten_on_load_keeps_unknown_fields() {
    let root = TemporaryRoot::new();
    let directory = collections_directory(&root);
    let text = r#"{"info":{"name":"n","schema":"s"},"x-custom":{"keep":1},
        "item":[{"name":"i","x-custom":"item","request":{"method":"GET","url":"u"}}]}"#;
    std::fs::write(directory.join("c.json"), text).unwrap();
    root.store().load();

    let rewritten: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(directory.join("c.json")).unwrap()).unwrap();
    assert!(rewritten["info"]["_postman_id"].is_string());
    assert_eq!(rewritten["x-custom"]["keep"], 1);
    assert_eq!(rewritten["item"][0]["x-custom"], "item");
}

#[test]
fn a_pending_delete_removes_the_file_and_tolerates_a_missing_one() {
    let root = TemporaryRoot::new();
    let directory = collections_directory(&root);
    let path = directory.join("gone.json");
    std::fs::write(&path, "{}").unwrap();
    PendingDelete::new(path.clone()).commit().unwrap();
    assert!(!path.exists());
    PendingDelete::new(path).commit().unwrap();
}
