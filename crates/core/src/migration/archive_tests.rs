use std::fs;
use std::io::{Cursor, Write};
use std::path::PathBuf;

use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use super::{PostmanExportFiles, WorkspaceArchive};
use crate::collection::{Auth, Collection, Item};
use crate::environment::Environment;
use crate::workspace::Workspace;

const ENVIRONMENT: &str = r#"{"name":"Staging","values":[{"key":"token","value":"s3cr3t"}]}"#;
const COLLECTION: &str = r#"{"info":{"name":"Orders","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},"item":[]}"#;

fn scratch_file(name: &str, bytes: &[u8]) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("requestor-archive-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join(format!("{name}.zip"));
    fs::write(&path, bytes).unwrap();
    path
}

fn zip_of(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, text) in entries {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(text.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn workspace() -> Workspace {
    let mut collection = Collection::new("Orders");
    collection.auth = Some(Auth::bearer("literal-token"));
    collection
        .add_item(None, Item::folder("Admin".to_string(), Vec::new()))
        .unwrap();
    let mut twin = Collection::new("orders");
    twin.ensure_ids();
    let mut workspace = Workspace::default();
    workspace.add_collection(collection);
    workspace.add_collection(twin);
    workspace.add_environment(serde_json::from_str::<Environment>(ENVIRONMENT).unwrap());
    workspace
}

#[test]
fn a_workspace_archive_imports_back_with_folders_and_without_credentials() {
    let bytes = WorkspaceArchive::of(&workspace(), false).unwrap();
    let path = scratch_file("roundtrip", &bytes);

    let report = PostmanExportFiles::discover(std::slice::from_ref(&path)).import();

    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert_eq!(report.collections.len(), 2);
    let orders = report
        .collections
        .iter()
        .find(|c| c.info.name == "Orders")
        .unwrap();
    assert_eq!(orders.item[0].name.as_deref(), Some("Admin"));
    assert_eq!(orders.literal_secret_count(), 0);
    assert_eq!(report.environments[0].values[0].value_text(), "");
    fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn credentials_stay_when_asked() {
    let bytes = WorkspaceArchive::of(&workspace(), true).unwrap();
    let path = scratch_file("credentials", &bytes);

    let report = PostmanExportFiles::discover(std::slice::from_ref(&path)).import();

    let orders = report
        .collections
        .iter()
        .find(|c| c.info.name == "Orders")
        .unwrap();
    assert_eq!(orders.literal_secret_count(), 1);
    assert_eq!(report.environments[0].values[0].value_text(), "s3cr3t");
    fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn a_postman_data_export_zip_skips_its_index_and_system_files() {
    let bytes = zip_of(&[
        ("export/archive.json", "{}"),
        ("export/collection/one.json", COLLECTION),
        ("export/environment/two.json", ENVIRONMENT),
        ("__MACOSX/export/._one.json", "binary"),
        ("export/.hidden/three.json", COLLECTION),
        ("export/readme.txt", "hello"),
        ("export/broken.json", "{"),
    ]);
    let path = scratch_file("postman", &bytes);

    let report = PostmanExportFiles::discover(std::slice::from_ref(&path)).import();

    assert_eq!(report.collections.len(), 1);
    assert_eq!(report.environments.len(), 1);
    assert_eq!(report.failures.len(), 1);
    assert!(report.failures[0].source.ends_with("export/broken.json"));
    fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn a_file_that_is_not_a_zip_is_reported() {
    let path = scratch_file("fake", b"not a zip");

    let report = PostmanExportFiles::discover(std::slice::from_ref(&path)).import();

    assert_eq!(report.failures.len(), 1);
    fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
