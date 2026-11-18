use std::path::PathBuf;

use super::{safe_file_stem, WorkspaceStore};

use crate::collection::Collection;
use crate::environment::Environment;
use crate::identity::random_uuid;
use crate::workspace::{AppSettings, Session, SessionTab, Workspace};

mod documents;
mod lifecycle;
mod links;

pub(super) struct TemporaryRoot(pub PathBuf);

impl TemporaryRoot {
    pub(super) fn new() -> Self {
        let path = std::env::temp_dir().join(format!("requestor-store-{}", random_uuid()));
        TemporaryRoot(path)
    }

    pub(super) fn store(&self) -> WorkspaceStore {
        WorkspaceStore::new(&self.0)
    }
}

impl Drop for TemporaryRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn an_empty_location_loads_an_empty_workspace_with_defaults() {
    let root = TemporaryRoot::new();
    let loaded = root.store().load();
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.workspace, Workspace::default());
    assert!(loaded.workspace.settings.interface.autosave);
}

#[test]
fn everything_written_is_read_back() {
    let root = TemporaryRoot::new();
    let mut store = root.store();
    let mut workspace = Workspace::default();
    let collection_id = workspace.add_collection(Collection::new("Sample"));
    let environment_id = workspace.add_environment(Environment::new("Staging"));
    workspace.settings.interface.two_pane_view = true;
    workspace.settings.request.verify_ssl = false;
    workspace.session = Session {
        tabs: vec![SessionTab {
            collection_id: Some(collection_id.clone()),
            name: "tab".to_string(),
            ..SessionTab::default()
        }],
        active_tab: Some(0),
        active_environment: Some(environment_id.clone()),
    };
    let writes = [
        store.collection_write(&workspace.collections[0]).unwrap(),
        store
            .environment_write(&workspace.environments[0])
            .unwrap()
            .into(),
        store.settings_write(&workspace.settings).unwrap().into(),
        store.session_write(&workspace.session).unwrap().into(),
    ];
    for write in writes {
        write.commit().unwrap();
    }
    let loaded = store.load();
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.workspace, workspace);
    store
        .release_collection(&collection_id)
        .unwrap()
        .unwrap()
        .commit()
        .unwrap();
    std::fs::remove_file(store.release_environment_path(&environment_id).unwrap()).unwrap();
    let emptied = store.load().workspace;
    assert!(emptied.collections.is_empty());
    assert!(emptied.session.active_environment.is_none());
}

#[test]
fn a_damaged_file_is_reported_and_left_in_place() {
    let root = TemporaryRoot::new();
    let mut store = root.store();
    store
        .collection_write(&Collection::new("Good"))
        .unwrap()
        .commit()
        .unwrap();
    let damaged = root.0.join("collections").join("damaged.json");
    std::fs::write(&damaged, "{ not json").unwrap();
    std::fs::write(root.0.join("settings.json"), "not json").unwrap();
    let loaded = store.load();
    assert_eq!(loaded.workspace.collections.len(), 1);
    assert_eq!(loaded.workspace.settings, AppSettings::default());
    assert_eq!(loaded.warnings.len(), 2);
    assert!(damaged.exists());
}

#[test]
fn writes_replace_the_file_and_leave_no_temporary_files() {
    let root = TemporaryRoot::new();
    let mut store = root.store();
    let mut collection = Collection::new("First");
    collection.ensure_ids();
    store
        .collection_write(&collection)
        .unwrap()
        .commit()
        .unwrap();
    collection.info.name = "Second".to_string();
    store
        .collection_write(&collection)
        .unwrap()
        .commit()
        .unwrap();
    let names: Vec<_> = std::fs::read_dir(root.0.join("collections"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, [format!("{}.json", collection.id())]);
    assert_eq!(store.load().workspace.collections[0].info.name, "Second");
}

#[cfg(unix)]
#[test]
fn files_are_readable_by_the_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let root = TemporaryRoot::new();
    let write = root
        .store()
        .settings_write(&AppSettings::default())
        .unwrap();
    let path = write.path().to_path_buf();
    write.commit().unwrap();
    let mode = std::fs::metadata(path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn identifiers_cannot_escape_the_storage_directory() {
    assert_eq!(safe_file_stem("../../etc/passwd"), "______etc_passwd");
    assert_eq!(safe_file_stem("a1-B2"), "a1-B2");
    assert_eq!(safe_file_stem(""), "unnamed");
    let root = TemporaryRoot::new();
    let mut collection = Collection::new("Escape");
    collection.info.postman_id = Some("../outside".to_string());
    let write = root.store().collection_write(&collection).unwrap();
    assert!(write.path().starts_with(root.0.join("collections")));
}
