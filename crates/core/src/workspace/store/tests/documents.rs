use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::TemporaryRoot;
use crate::collection::Collection;

pub(super) const NO_ID_COLLECTION: &str = r#"{"info":{"name":"My API","schema":"s"},"item":[]}"#;

pub(super) fn collections_directory(root: &TemporaryRoot) -> PathBuf {
    let directory = root.0.join("collections");
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

pub(super) fn listing(directory: &Path) -> BTreeSet<String> {
    std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect()
}

pub(super) fn document_with_id(id: &str, name: &str) -> String {
    format!(r#"{{"info":{{"_postman_id":"{id}","name":"{name}","schema":"s"}},"item":[]}}"#)
}

#[test]
fn a_dropped_in_file_keeps_its_id_and_path_and_receives_edits() {
    let root = TemporaryRoot::new();
    let directory = collections_directory(&root);
    let dropped = directory.join("My API.json");
    std::fs::write(&dropped, NO_ID_COLLECTION).unwrap();
    let mut store = root.store();

    let first = store.load();
    assert!(first.warnings.is_empty());
    let mut collection = first.workspace.collections[0].clone();
    let id = collection.id().to_string();
    assert_eq!(listing(&directory), BTreeSet::from(["My API.json".into()]));

    let second = store.load();
    assert_eq!(second.workspace.collections[0].id(), id);
    let mut fresh_launch = root.store();
    assert_eq!(fresh_launch.load().workspace.collections[0].id(), id);

    collection.info.name = "Edited".to_string();
    let write = fresh_launch.collection_write(&collection).unwrap();
    assert_eq!(write.path(), dropped);
    write.commit().unwrap();
    assert_eq!(listing(&directory), BTreeSet::from(["My API.json".into()]));
    assert_eq!(
        root.store().load().workspace.collections[0].info.name,
        "Edited"
    );
}

#[test]
fn two_files_with_one_id_both_load_and_the_fresh_id_persists() {
    let root = TemporaryRoot::new();
    let directory = collections_directory(&root);
    std::fs::write(
        directory.join("a.json"),
        document_with_id("shared", "first"),
    )
    .unwrap();
    std::fs::write(
        directory.join("b.json"),
        document_with_id("shared", "second"),
    )
    .unwrap();
    let mut store = root.store();

    let loaded = store.load();
    assert!(loaded.warnings.is_empty());
    let workspace = loaded.workspace;
    assert_eq!(workspace.collections.len(), 2);
    assert_eq!(workspace.collection("shared").unwrap().info.name, "first");
    let second_id = workspace.collections[1].id().to_string();
    assert_ne!(second_id, "shared");

    let again = root.store().load().workspace;
    assert_eq!(again.collections[1].id(), second_id);
    assert_eq!(again.collection("shared").unwrap().info.name, "first");
    assert_eq!(
        listing(&directory),
        BTreeSet::from(["a.json".into(), "b.json".into()])
    );
    assert_eq!(
        store.release_collection_path(&second_id),
        Some(directory.join("b.json"))
    );
}

#[test]
fn a_new_document_never_takes_the_path_of_an_unparsable_file() {
    let root = TemporaryRoot::new();
    let directory = collections_directory(&root);
    std::fs::write(directory.join("X.json"), "{ not json").unwrap();
    let mut store = root.store();
    assert_eq!(store.load().warnings.len(), 1);

    let mut imported = Collection::new("Imported");
    imported.info.postman_id = Some("X".to_string());
    let write = store.collection_write(&imported).unwrap();
    assert_ne!(write.path(), directory.join("X.json"));
    assert_eq!(write.path().parent(), Some(directory.as_path()));
    write.commit().unwrap();

    assert_eq!(
        std::fs::read_to_string(directory.join("X.json")).unwrap(),
        "{ not json"
    );
    let rewritten = store.collection_write(&imported).unwrap();
    assert_ne!(rewritten.path(), directory.join("X.json"));
}

#[test]
fn releasing_an_id_that_was_never_remembered_yields_nothing_and_keeps_the_file() {
    let root = TemporaryRoot::new();
    let directory = collections_directory(&root);
    std::fs::write(directory.join("X.json"), "{ not json").unwrap();
    let mut store = root.store();
    store.load();
    let mut imported = Collection::new("Imported");
    imported.info.postman_id = Some("X".to_string());

    assert_eq!(store.release_collection_path(imported.id()), None);
    assert_eq!(store.release_environment_path("X"), None);
    assert!(directory.join("X.json").exists());
}

#[test]
fn duplicate_item_ids_are_fixed_once_and_stay_fixed() {
    let root = TemporaryRoot::new();
    let directory = collections_directory(&root);
    let item = r#"{"id":"same","name":"n","request":{"method":"GET","url":"u"}}"#;
    let text = format!(
        r#"{{"info":{{"_postman_id":"c","name":"c","schema":"s"}},"item":[{item},{item}]}}"#
    );
    std::fs::write(directory.join("c.json"), text).unwrap();

    let first = root.store().load().workspace.collections[0].clone();
    let second = root.store().load().workspace.collections[0].clone();
    assert_ne!(first.item[0].id, first.item[1].id);
    assert_eq!(first, second);
}

#[test]
fn deleting_a_remembered_document_removes_its_original_path() {
    let root = TemporaryRoot::new();
    let directory = collections_directory(&root);
    let original = directory.join("My API.json");
    std::fs::write(&original, NO_ID_COLLECTION).unwrap();
    let mut store = root.store();
    let id = store.load().workspace.collections[0].id().to_string();

    let path = store.release_collection_path(&id).unwrap();
    assert_eq!(path, original);
    std::fs::remove_file(path).unwrap();
    assert!(listing(&directory).is_empty());
}

#[test]
fn loading_never_removes_a_file() {
    let root = TemporaryRoot::new();
    let directory = collections_directory(&root);
    std::fs::write(directory.join("noid.json"), NO_ID_COLLECTION).unwrap();
    std::fs::write(directory.join("a.json"), document_with_id("d", "a")).unwrap();
    std::fs::write(directory.join("b.json"), document_with_id("d", "b")).unwrap();
    std::fs::write(directory.join("broken.json"), "nope").unwrap();
    std::fs::write(directory.join("notes.txt"), "keep").unwrap();
    let before = listing(&directory);
    let mut store = root.store();
    store.load();
    store.load();
    assert!(listing(&directory).is_superset(&before));
}
