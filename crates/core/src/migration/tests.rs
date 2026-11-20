use std::fs;
use std::path::{Path, PathBuf};

use super::export_files::{MAX_FILES, MAX_FOLDER_DEPTH};
use super::{MigrationReport, PostmanExportFiles};
use crate::import::import_text;

const COLLECTION: &str = r#"{"info":{"name":"Orders","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},"item":[{"name":"Admin","item":[{"name":"List","request":{"method":"GET","url":"https://h/orders"}}]}]}"#;
const ENVIRONMENT: &str = r#"{"name":"Staging","values":[{"key":"host","value":"h"}]}"#;
const ARCHIVE_INDEX: &str = r#"{"collection":{"a":true},"environment":{"b":true}}"#;

fn scratch(name: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("requestor-migration-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

fn write(folder: &Path, name: &str, text: &str) -> PathBuf {
    let path = folder.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_postman_data_export_folder_imports_every_collection_and_environment() {
    let folder = scratch("export");
    write(&folder, "archive.json", ARCHIVE_INDEX);
    write(&folder, "collection/one.json", COLLECTION);
    write(&folder, "environment/two.json", ENVIRONMENT);
    write(&folder, "notes.txt", "not json");

    let files = PostmanExportFiles::discover(std::slice::from_ref(&folder));
    let report = files.import();

    assert_eq!(files.len(), 2);
    assert_eq!(report.collections.len(), 1);
    assert_eq!(
        report.collections[0].item[0].item.as_ref().unwrap().len(),
        1
    );
    assert_eq!(report.environments[0].name, "Staging");
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn one_broken_file_is_reported_without_stopping_the_rest() {
    let folder = scratch("broken");
    let good = write(&folder, "good.postman_collection.json", COLLECTION);
    let broken = write(&folder, "broken.json", "{\"info\":");
    let missing = folder.join("missing.json");

    let report = PostmanExportFiles::discover(&[good, broken.clone(), missing]).import();

    assert_eq!(report.collections.len(), 1);
    assert_eq!(report.failures.len(), 2);
    assert!(report
        .failures
        .iter()
        .any(|failure| failure.source == broken.to_string_lossy()));
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn a_file_dropped_twice_is_imported_once() {
    let folder = scratch("twice");
    let path = write(&folder, "c.json", COLLECTION);

    let files = PostmanExportFiles::discover(&[path.clone(), folder.clone(), path]);

    assert_eq!(files.len(), 1);
    fs::remove_dir_all(folder).unwrap();
}

#[cfg(unix)]
#[test]
fn links_inside_a_folder_are_not_followed() {
    let folder = scratch("links");
    let outside = scratch("links-outside");
    write(&outside, "secret.json", COLLECTION);
    std::os::unix::fs::symlink(&outside, folder.join("linked")).unwrap();

    assert!(PostmanExportFiles::discover(std::slice::from_ref(&folder)).is_empty());
    fs::remove_dir_all(folder).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn single_requests_are_kept_apart_from_library_documents() {
    let mut report = MigrationReport::default();
    report.record("a.txt", import_text("curl http://h/x"));
    report.record("b.json", import_text(ENVIRONMENT));
    report.record("c.json", import_text("{\"neither\":true}"));

    assert_eq!(report.requests.len(), 1);
    assert_eq!(report.environments.len(), 1);
    assert_eq!(report.imported_count(), 2);
    assert_eq!(report.failures[0].source, "c.json");
}

#[test]
fn hidden_folders_and_a_picked_export_index_are_left_out() {
    let folder = scratch("hidden");
    write(&folder, ".git/c.json", COLLECTION);
    let index = write(&folder, "archive.json", ARCHIVE_INDEX);
    let same = folder.join(".").join("archive.json");

    assert!(PostmanExportFiles::discover(&[folder.clone(), index, same]).is_empty());
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn the_same_file_by_two_spellings_is_imported_once() {
    let folder = scratch("spellings");
    let path = write(&folder, "c.json", COLLECTION);
    let other = folder.join(".").join("c.json");

    assert_eq!(PostmanExportFiles::discover(&[path, other]).len(), 1);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn scanning_stops_at_the_file_cap_and_says_so() {
    let folder = scratch("cap");
    for index in 0..=MAX_FILES {
        write(&folder, &format!("e{index}.json"), ENVIRONMENT);
    }

    let files = PostmanExportFiles::discover(std::slice::from_ref(&folder));

    assert_eq!(files.len(), MAX_FILES);
    assert_eq!(files.import().warnings.len(), 1);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn folders_deeper_than_the_limit_are_not_scanned() {
    let folder = scratch("deep");
    let deep: String = (0..=MAX_FOLDER_DEPTH + 1)
        .map(|level| format!("d{level}/"))
        .collect();
    write(&folder, &format!("{deep}c.json"), COLLECTION);
    write(&folder, "d0/c.json", COLLECTION);

    assert_eq!(
        PostmanExportFiles::discover(std::slice::from_ref(&folder)).len(),
        1
    );
    fs::remove_dir_all(folder).unwrap();
}
