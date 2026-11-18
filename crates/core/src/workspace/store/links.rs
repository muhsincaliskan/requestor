use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::{pending, read_document, WorkspaceStore, JSON_EXTENSION};
use crate::collection::Collection;
use crate::workspace::folder::{read_folder, FolderLayout, FolderWrite, FOLDER_FILE};
use crate::workspace::{StoreError, StoreWrite};

const LINKS_FILE: &str = "linked-folders.json";
const HIDDEN_MARK: char = '.';
const NOT_EMPTY: &str = "the folder already holds request files; choose an empty folder or open it";
const ALREADY_OPEN: &str = "this folder is already open";

#[derive(Debug, Default)]
pub(super) struct LinkedFolders {
    folders: HashMap<String, LinkedFolder>,
}

#[derive(Debug)]
struct LinkedFolder {
    root: PathBuf,
    owned: Vec<PathBuf>,
}

impl WorkspaceStore {
    pub fn linked_folder(&self, collection_id: &str) -> Option<&Path> {
        self.linked
            .folders
            .get(collection_id)
            .map(|folder| folder.root.as_path())
    }

    pub fn link_collection(
        &mut self,
        collection: &Collection,
        root: PathBuf,
    ) -> Result<Vec<StoreWrite>, StoreError> {
        self.ensure_not_open(&root)?;
        ensure_no_request_files(&root)?;
        let layout = FolderLayout::of(collection).map_err(|error| StoreError::at(&root, error))?;
        let stale: Vec<PathBuf> = self
            .collections
            .release(collection.id())
            .into_iter()
            .collect();
        let owned = layout.paths().map(|path| root.join(path)).collect();
        let write = FolderWrite::new(root.clone(), layout, stale);
        self.linked
            .folders
            .insert(collection.id().to_string(), LinkedFolder { root, owned });
        Ok(vec![StoreWrite::Folder(write), self.links_write()?])
    }

    pub fn open_folder(&self, root: &Path) -> Result<(Collection, Vec<PathBuf>), StoreError> {
        self.ensure_not_open(root)?;
        read_folder(root)
    }

    pub fn remember_folder(
        &mut self,
        collection_id: &str,
        root: PathBuf,
        owned: Vec<PathBuf>,
    ) -> Result<StoreWrite, StoreError> {
        self.linked
            .folders
            .insert(collection_id.to_string(), LinkedFolder { root, owned });
        self.links_write()
    }

    pub fn unlink_collection(
        &mut self,
        collection_id: &str,
    ) -> Option<Result<StoreWrite, StoreError>> {
        self.linked.folders.remove(collection_id)?;
        Some(self.links_write())
    }

    pub(super) fn folder_write(
        &mut self,
        collection: &Collection,
    ) -> Option<Result<StoreWrite, StoreError>> {
        let folder = self.linked.folders.get_mut(collection.id())?;
        let layout = match FolderLayout::of(collection) {
            Ok(layout) => layout,
            Err(error) => return Some(Err(StoreError::at(&folder.root, error))),
        };
        let owned: Vec<PathBuf> = layout.paths().map(|path| folder.root.join(path)).collect();
        let kept: HashSet<&PathBuf> = owned.iter().collect();
        let stale = folder
            .owned
            .iter()
            .filter(|path| !kept.contains(path))
            .cloned()
            .collect();
        let write = FolderWrite::new(folder.root.clone(), layout, stale);
        folder.owned = owned;
        Some(Ok(StoreWrite::Folder(write)))
    }

    pub(super) fn load_linked(&mut self, warnings: &mut Vec<StoreError>) -> Vec<Collection> {
        let path = self.root.join(LINKS_FILE);
        if !path.exists() {
            return Vec::new();
        }
        let roots: Vec<PathBuf> = read_document(&path).unwrap_or_else(|error| {
            warnings.push(error);
            Vec::new()
        });
        let mut taken = HashSet::new();
        let mut collections = Vec::new();
        for root in roots {
            match read_folder(&root) {
                Ok((mut collection, owned)) => {
                    let changed = collection.claim_unique_id(&mut taken);
                    let id = collection.id().to_string();
                    self.linked.folders.insert(id, LinkedFolder { root, owned });
                    if changed {
                        let rewritten = self
                            .folder_write(&collection)
                            .and_then(|write| write.and_then(StoreWrite::commit).err());
                        warnings.extend(rewritten);
                    }
                    collections.push(collection);
                }
                Err(error) => warnings.push(error),
            }
        }
        collections
    }

    fn links_write(&self) -> Result<StoreWrite, StoreError> {
        let mut roots: Vec<&PathBuf> = self
            .linked
            .folders
            .values()
            .map(|folder| &folder.root)
            .collect();
        roots.sort();
        pending(self.root.join(LINKS_FILE), &roots).map(StoreWrite::from)
    }

    fn ensure_not_open(&self, root: &Path) -> Result<(), StoreError> {
        let open = self
            .linked
            .folders
            .values()
            .any(|folder| same_folder(&folder.root, root));
        if open {
            return Err(StoreError::at(root, ALREADY_OPEN));
        }
        Ok(())
    }
}

fn same_folder(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn ensure_no_request_files(root: &Path) -> Result<(), StoreError> {
    let Ok(entries) = fs::read_dir(root) else {
        return Ok(());
    };
    let holds_requests = entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        let name = entry.file_name();
        let visible = !name.to_string_lossy().starts_with(HIDDEN_MARK);
        let request_file = path.extension().is_some_and(|kind| kind == JSON_EXTENSION);
        visible && (request_file || path.join(FOLDER_FILE).is_file())
    });
    if holds_requests {
        return Err(StoreError::at(root, NOT_EMPTY));
    }
    Ok(())
}
