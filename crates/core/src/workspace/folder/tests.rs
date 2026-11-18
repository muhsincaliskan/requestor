use std::fs;
use std::path::{Path, PathBuf};

use super::{read_folder, FolderLayout, FolderWrite};
use crate::collection::Collection;
use crate::identity::random_uuid;

const FIXTURE: &str = include_str!("../../../tests/fixtures/collection.json");

struct TemporaryFolder(PathBuf);

impl TemporaryFolder {
    fn new() -> Self {
        TemporaryFolder(std::env::temp_dir().join(format!("requestor-folder-{}", random_uuid())))
    }
}

impl Drop for TemporaryFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> Collection {
    let mut collection = Collection::from_json(FIXTURE).unwrap();
    collection.ensure_ids();
    collection
}

fn write(root: &Path, collection: &Collection, stale: Vec<PathBuf>) {
    let layout = FolderLayout::of(collection).unwrap();
    FolderWrite::new(root.to_path_buf(), layout, stale)
        .commit()
        .unwrap();
}

#[test]
fn a_collection_round_trips_through_a_folder() {
    let folder = TemporaryFolder::new();
    let collection = fixture();
    write(&folder.0, &collection, Vec::new());
    let (read, owned) = read_folder(&folder.0).unwrap();
    assert_eq!(read, collection);
    assert_eq!(
        owned.len(),
        FolderLayout::of(&collection).unwrap().files.len()
    );
}

#[test]
fn each_request_is_its_own_file_and_folders_are_directories() {
    let mut collection = Collection::new("Shop");
    collection.item = serde_json::from_str(
        r#"[{"name":"Health","request":{"url":"/h"}},
            {"name":"Orders","item":[{"name":"List orders","request":{"url":"/o"}}]}]"#,
    )
    .unwrap();
    let layout = FolderLayout::of(&collection).unwrap();
    let paths: Vec<_> = layout.paths().map(Path::to_path_buf).collect();
    assert_eq!(
        paths,
        [
            PathBuf::from("health.json"),
            PathBuf::from("orders/list-orders.json"),
            PathBuf::from("orders/folder.json"),
            PathBuf::from("collection.json"),
        ]
    );
    let (_, root) = layout.files.last().unwrap();
    assert!(root.contains(r#""order": ["#) && root.contains(r#""orders""#));
    assert!(!root.contains(r#""item""#));
    assert!(root.ends_with('\n'));
}

#[test]
fn files_added_by_someone_else_are_read_after_the_ordered_ones() {
    let folder = TemporaryFolder::new();
    let mut collection = Collection::new("Shop");
    collection.item = serde_json::from_str(r#"[{"name":"Zeta","request":{"url":"/z"}}]"#).unwrap();
    write(&folder.0, &collection, Vec::new());
    fs::write(
        folder.0.join("alpha.json"),
        r#"{"name":"Alpha","request":{"url":"/a"},"x-team":1}"#,
    )
    .unwrap();
    fs::write(folder.0.join("notes.txt"), "not a request").unwrap();
    let (read, _) = read_folder(&folder.0).unwrap();
    let names: Vec<_> = read
        .item
        .iter()
        .filter_map(|item| item.name.as_deref())
        .collect();
    assert_eq!(names, ["Zeta", "Alpha"]);
    assert!(read.item[1].extra.contains_key("x-team"));
}

#[test]
fn stale_files_are_removed_with_their_empty_directories_only() {
    let folder = TemporaryFolder::new();
    let mut collection = Collection::new("Shop");
    collection.item = serde_json::from_str(
        r#"[{"name":"Orders","item":[{"name":"List","request":{"url":"/o"}}]}]"#,
    )
    .unwrap();
    write(&folder.0, &collection, Vec::new());
    fs::write(folder.0.join("README.md"), "keep me").unwrap();
    let (_, owned) = read_folder(&folder.0).unwrap();
    collection.item.clear();
    let kept: Vec<PathBuf> = FolderLayout::of(&collection)
        .unwrap()
        .paths()
        .map(|path| folder.0.join(path))
        .collect();
    let stale = owned
        .into_iter()
        .filter(|path| !kept.contains(path))
        .collect();
    write(&folder.0, &collection, stale);
    assert!(!folder.0.join("orders").exists());
    assert!(folder.0.join("README.md").exists());
    assert!(folder.0.join("collection.json").exists());
}
