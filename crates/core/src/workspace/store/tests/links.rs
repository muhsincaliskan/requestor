use std::fs;
use std::path::{Path, PathBuf};

use super::documents::listing;
use super::TemporaryRoot;
use crate::collection::{Collection, Item, Request, Url};
use crate::workspace::WorkspaceStore;

pub(super) fn link(store: &mut WorkspaceStore, collection: &Collection, folder: &Path) {
    store
        .link_collection(collection, folder.to_path_buf())
        .unwrap()
        .commit()
        .unwrap();
}

pub(super) fn shop() -> Collection {
    let mut collection = Collection::new("Shop");
    collection.item = vec![
        Item::for_request(
            "Health".to_string(),
            Request::new("GET", Url::Raw("https://x.test/h".to_string())),
        ),
        Item::folder(
            "Orders".to_string(),
            vec![Item::for_request(
                "List".to_string(),
                Request::new("GET", Url::Raw("https://x.test/o".to_string())),
            )],
        ),
    ];
    collection.ensure_ids();
    collection
}

pub(super) fn folder_in(root: &TemporaryRoot) -> PathBuf {
    root.0.join("repo").join("shop")
}

#[test]
fn a_linked_collection_lives_in_its_folder_and_loads_from_there() {
    let root = TemporaryRoot::new();
    let mut store = root.store();
    let collection = shop();
    store
        .collection_write(&collection)
        .unwrap()
        .commit()
        .unwrap();
    let folder = folder_in(&root);
    link(&mut store, &collection, &folder);
    assert!(listing(&root.0.join("collections")).is_empty());
    assert_eq!(
        listing(&folder),
        ["collection.json", "health.json", "orders"]
            .map(str::to_string)
            .into()
    );
    let mut reopened = root.store();
    let loaded = reopened.load();
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.workspace.collections, vec![collection.clone()]);
    assert_eq!(
        reopened.linked_folder(collection.id()),
        Some(folder.as_path())
    );
}

#[test]
fn removed_requests_lose_their_files_and_other_files_stay() {
    let root = TemporaryRoot::new();
    let mut store = root.store();
    let mut collection = shop();
    let folder = folder_in(&root);
    link(&mut store, &collection, &folder);
    fs::write(folder.join("README.md"), "docs").unwrap();
    collection.item.truncate(1);
    store
        .collection_write(&collection)
        .unwrap()
        .commit()
        .unwrap();
    assert_eq!(
        listing(&folder),
        ["README.md", "collection.json", "health.json"]
            .map(str::to_string)
            .into()
    );
}

#[test]
fn deleting_a_linked_collection_only_forgets_the_folder() {
    let root = TemporaryRoot::new();
    let mut store = root.store();
    let collection = shop();
    let folder = folder_in(&root);
    link(&mut store, &collection, &folder);
    store
        .release_collection(collection.id())
        .unwrap()
        .unwrap()
        .commit()
        .unwrap();
    assert!(folder.join("collection.json").exists());
    assert!(root.store().load().workspace.collections.is_empty());
}

#[test]
fn folders_holding_a_collection_cannot_be_linked_and_open_ones_cannot_be_opened_twice() {
    let root = TemporaryRoot::new();
    let mut store = root.store();
    let folder = folder_in(&root);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("package.json"), "{}").unwrap();
    fs::write(folder.join("collection.json"), "{}").unwrap();
    assert!(store.link_collection(&shop(), folder.clone()).is_err());
    fs::remove_file(folder.join("collection.json")).unwrap();
    let collection = shop();
    link(&mut store, &collection, &folder);
    assert!(store.open_folder(&folder).is_err());
    let (opened, owned) = root.store().open_folder(&folder).unwrap();
    assert_eq!(opened, collection);
    assert_eq!(owned.len(), 4);
}
