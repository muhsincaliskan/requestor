use std::fs;

use super::folder_safety::{item_names, save, with_request};
use super::links::{folder_in, link, shop};
use super::TemporaryRoot;
#[cfg(unix)]
use crate::collection::Item;

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

#[test]
fn a_file_the_store_did_not_write_is_never_overwritten() {
    let root = TemporaryRoot::new();
    let folder = folder_in(&root);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("package.json"), r#"{"name":"app"}"#).unwrap();
    link(&mut root.store(), &with_request(shop(), "Package"), &folder);
    assert_eq!(
        fs::read_to_string(folder.join("package.json")).unwrap(),
        r#"{"name":"app"}"#
    );
    let loaded = root.store().load().workspace.collections;
    assert_eq!(item_names(&loaded[0]), ["Health", "Orders", "Package"]);
}

#[test]
fn json_files_that_do_not_parse_are_skipped_unless_the_order_lists_them() {
    let root = TemporaryRoot::new();
    let folder = folder_in(&root);
    link(&mut root.store(), &shop(), &folder);
    fs::write(folder.join("tsconfig.json"), "{ // comment\n }").unwrap();
    let loaded = root.store().load();
    assert!(loaded.warnings.is_empty());
    fs::write(folder.join("health.json"), "<<<<<<< ours").unwrap();
    let loaded = root.store().load();
    assert!(loaded.workspace.collections.is_empty());
    assert_eq!(loaded.warnings.len(), 1);
}

#[cfg(unix)]
#[test]
fn a_linked_directory_is_never_written_through() {
    let root = TemporaryRoot::new();
    let folder = folder_in(&root);
    let outside = root.0.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(&folder).unwrap();
    std::os::unix::fs::symlink(&outside, folder.join("shared")).unwrap();
    let mut collection = shop();
    collection
        .item
        .push(Item::folder("Shared".to_string(), Vec::new()));
    collection.ensure_ids();
    link(&mut root.store(), &collection, &folder);
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
    assert!(folder.join("shared-2").join("folder.json").exists());
}

#[test]
fn unreadable_or_misshaped_files_the_order_does_not_list_are_skipped() {
    let root = TemporaryRoot::new();
    let folder = folder_in(&root);
    link(&mut root.store(), &shop(), &folder);
    fs::write(folder.join("utf16.json"), [0xFF, 0xFE, 0x7B, 0x00]).unwrap();
    fs::write(folder.join("odd.json"), r#"{"request": 5}"#).unwrap();
    fs::create_dir_all(folder.join("broken")).unwrap();
    fs::write(folder.join("broken").join("folder.json"), "{").unwrap();
    let loaded = root.store().load();
    assert!(loaded.warnings.is_empty());
    let collection = &loaded.workspace.collections[0];
    assert_eq!(item_names(collection), ["Health", "Orders"]);
    save(&root, collection);
    assert!(folder.join("odd.json").exists());
    assert!(folder.join("broken").join("folder.json").exists());
}

#[test]
fn folders_inside_or_around_an_open_folder_cannot_be_linked() {
    let root = TemporaryRoot::new();
    let folder = folder_in(&root);
    let mut store = root.store();
    link(&mut store, &shop(), &folder);
    let inner = folder.join("orders");
    assert!(store.link_collection(&shop(), inner.clone()).is_err());
    assert!(store.open_folder(&inner).is_err());
    assert!(store.link_collection(&shop(), root.0.join("repo")).is_err());
}

#[cfg(unix)]
#[test]
fn a_foreign_file_differing_only_by_case_from_an_owned_one_is_kept() {
    let root = TemporaryRoot::new();
    let folder = folder_in(&root);
    link(&mut root.store(), &with_request(shop(), "Login"), &folder);
    fs::rename(folder.join("login.json"), folder.join("Login.json")).unwrap();
    fs::write(folder.join("login.json"), r#"{"foreign":true}"#).unwrap();
    if fs::read_to_string(folder.join("Login.json")).unwrap() == r#"{"foreign":true}"# {
        return;
    }
    let loaded = root.store().load().workspace.collections;
    save(&root, &loaded[0]);
    assert_eq!(
        fs::read_to_string(folder.join("login.json")).unwrap(),
        r#"{"foreign":true}"#
    );
}
