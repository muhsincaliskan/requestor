use std::fs;

use super::documents::listing;
use super::links::{folder_in, link, shop};
use super::TemporaryRoot;
use crate::collection::Collection;

const LINKS_FILE: &str = "linked-folders.json";

fn item_names(collection: &Collection) -> Vec<&str> {
    collection
        .item
        .iter()
        .filter_map(|item| item.name.as_deref())
        .collect()
}

fn collection_names(root: &TemporaryRoot) -> Vec<String> {
    root.store()
        .load()
        .workspace
        .collections
        .iter()
        .map(|collection| collection.info.name.clone())
        .collect()
}

fn save(root: &TemporaryRoot, collection: &Collection) {
    let mut store = root.store();
    store.load();
    store
        .collection_write(collection)
        .unwrap()
        .commit()
        .unwrap();
}

#[test]
fn a_linked_folder_with_an_id_already_in_use_never_replaces_the_other_collection() {
    let root = TemporaryRoot::new();
    let personal = shop();
    let mut store = root.store();
    store.collection_write(&personal).unwrap().commit().unwrap();
    let other = TemporaryRoot::new();
    let mut team = personal.clone();
    team.info.name = "Team".to_string();
    let folder = folder_in(&other);
    link(&mut other.store(), &team, &folder);
    fs::write(
        root.0.join(LINKS_FILE),
        serde_json::to_string(&[&folder]).unwrap(),
    )
    .unwrap();

    let mut reopened = root.store();
    let loaded = reopened.load().workspace.collections;
    assert_eq!(loaded.len(), 2);
    assert_ne!(loaded[0].id(), loaded[1].id());
    let team = loaded[1].clone();
    reopened
        .release_collection(team.id())
        .unwrap()
        .unwrap()
        .commit()
        .unwrap();
    reopened.collection_write(&team).unwrap().commit().unwrap();
    let mut names = collection_names(&root);
    names.sort();
    assert_eq!(names, ["Shop", "Team"]);
}

#[test]
fn a_folder_that_fails_to_load_is_still_remembered() {
    let root = TemporaryRoot::new();
    let folder = folder_in(&root);
    link(&mut root.store(), &shop(), &folder);
    let moved = folder.with_extension("away");
    fs::rename(&folder, &moved).unwrap();

    let mut store = root.store();
    let loaded = store.load();
    assert!(loaded.workspace.collections.is_empty());
    assert_eq!(loaded.warnings.len(), 1);
    link(&mut store, &Collection::new("Other"), &root.0.join("other"));

    fs::rename(&moved, &folder).unwrap();
    assert_eq!(collection_names(&root), ["Other", "Shop"]);
}

#[test]
fn files_left_by_a_failed_save_are_removed_by_the_next_one() {
    let root = TemporaryRoot::new();
    let mut store = root.store();
    let mut collection = shop();
    let folder = folder_in(&root);
    link(&mut store, &collection, &folder);
    collection.item.truncate(1);
    drop(store.collection_write(&collection).unwrap());
    store
        .collection_write(&collection)
        .unwrap()
        .commit()
        .unwrap();
    assert!(!folder.join("orders").exists());
}

#[test]
fn a_hand_named_request_keeps_its_content_when_renamed_by_case() {
    let root = TemporaryRoot::new();
    let folder = folder_in(&root);
    link(&mut root.store(), &Collection::new("Shop"), &folder);
    fs::write(
        folder.join("Login.json"),
        r#"{"name":"Login","request":{"url":"/login"}}"#,
    )
    .unwrap();
    let (collection, _) = root.store().open_folder(&folder).unwrap();
    save(&root, &collection);
    assert!(listing(&folder).contains("login.json"));
    assert!(!listing(&folder).contains("Login.json"));
    let reloaded = root.store().load().workspace.collections;
    assert_eq!(item_names(&reloaded[0]), ["Login"]);
}

#[test]
fn json_files_that_are_not_requests_are_never_read_or_removed() {
    let root = TemporaryRoot::new();
    let folder = folder_in(&root);
    link(&mut root.store(), &shop(), &folder);
    fs::write(folder.join("renovate.json"), r#"{"extends":["base"]}"#).unwrap();
    let loaded = root.store().load().workspace.collections;
    assert_eq!(item_names(&loaded[0]), ["Health", "Orders"]);
    save(&root, &loaded[0]);
    assert!(folder.join("renovate.json").exists());
}

#[cfg(unix)]
#[test]
fn linked_directories_inside_the_folder_are_never_followed() {
    let root = TemporaryRoot::new();
    let folder = folder_in(&root);
    link(&mut root.store(), &shop(), &folder);
    let outside = root.0.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("folder.json"), r#"{"name":"Shared"}"#).unwrap();
    fs::write(
        outside.join("ping.json"),
        r#"{"name":"Ping","request":{"url":"/p"}}"#,
    )
    .unwrap();
    std::os::unix::fs::symlink(&outside, folder.join("shared")).unwrap();
    let loaded = root.store().load().workspace.collections;
    assert_eq!(item_names(&loaded[0]), ["Health", "Orders"]);
    save(&root, &loaded[0]);
    assert!(outside.join("ping.json").exists());
}
