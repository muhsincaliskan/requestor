use std::fs;

use super::folder_safety::{item_names, save, with_request};
use super::links::{folder_in, link, shop};
use super::TemporaryRoot;
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
