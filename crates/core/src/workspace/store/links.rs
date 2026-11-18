use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::linked_folder::{nested, same_folder, LinkedFolder};
use super::{pending, read_document, WorkspaceStore};
use crate::collection::Collection;
use crate::workspace::folder::{read_folder, COLLECTION_FILE, FOLDER_FILE};
use crate::workspace::{PendingDelete, StoreError, StoreWrite};

pub(super) const LINKS_FILE: &str = "linked-folders.json";
const HOLDS_A_COLLECTION: &str = "this folder already holds a collection; open it instead";
const NOT_A_FOLDER: &str = "this is a file, not a folder";
const ALREADY_OPEN: &str = "this folder is already open";
const OVERLAPS: &str = "this folder is inside, or holds, a folder that is already open";

#[derive(Debug, Default)]
pub(super) struct LinkedFolders {
    folders: HashMap<String, LinkedFolder>,
    unavailable: Vec<PathBuf>,
}

impl LinkedFolders {
    fn insert(&mut self, collection_id: &str, folder: LinkedFolder) {
        self.unavailable.retain(|root| !folder.is_at(root));
        self.folders.insert(collection_id.to_string(), folder);
    }

    fn roots(&self) -> Vec<&Path> {
        let mut roots: Vec<&Path> = self
            .folders
            .values()
            .map(LinkedFolder::root)
            .chain(self.unavailable.iter().map(PathBuf::as_path))
            .collect();
        roots.sort();
        roots
    }

    fn ensure_apart(&self, root: &Path) -> Result<(), StoreError> {
        if self.folders.values().any(|folder| folder.is_at(root)) {
            return Err(StoreError::at(root, ALREADY_OPEN));
        }
        let overlapping = self.folders.values().any(|folder| folder.overlaps(root))
            || self
                .unavailable
                .iter()
                .any(|known| nested(known, root) && !same_folder(known, root));
        if overlapping {
            return Err(StoreError::at(root, OVERLAPS));
        }
        Ok(())
    }
}

impl WorkspaceStore {
    pub fn linked_folder(&self, collection_id: &str) -> Option<&Path> {
        self.linked
            .folders
            .get(collection_id)
            .map(LinkedFolder::root)
    }

    pub fn link_collection(
        &mut self,
        collection: &Collection,
        root: PathBuf,
    ) -> Result<StoreWrite, StoreError> {
        self.linked.ensure_apart(&root)?;
        ensure_free(&root)?;
        let mut folder = LinkedFolder::new(root, Vec::new());
        let write = folder
            .next_write(collection)
            .map_err(|error| StoreError::at(folder.root(), error))?;
        let mut writes = vec![StoreWrite::Folder(write)];
        self.linked.insert(collection.id(), folder);
        writes.push(self.links_write()?);
        let internal = self.collections.path_of(collection.id());
        writes.extend(
            internal.map(|path| StoreWrite::Delete(PendingDelete::new(path.to_path_buf()))),
        );
        Ok(StoreWrite::Batch(writes))
    }

    pub fn open_folder(&self, root: &Path) -> Result<(Collection, Vec<PathBuf>), StoreError> {
        self.linked.ensure_apart(root)?;
        read_folder(root)
    }

    pub fn remember_folder(
        &mut self,
        collection_id: &str,
        root: PathBuf,
        owned: Vec<PathBuf>,
    ) -> Result<StoreWrite, StoreError> {
        self.linked
            .insert(collection_id, LinkedFolder::new(root, owned));
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
        let write = folder
            .next_write(collection)
            .map(StoreWrite::Folder)
            .map_err(|error| StoreError::at(folder.root(), error));
        Some(write)
    }

    pub(super) fn load_linked(
        &mut self,
        taken: &mut HashSet<String>,
        warnings: &mut Vec<StoreError>,
    ) -> Vec<Collection> {
        let path = self.root.join(LINKS_FILE);
        if !path.exists() {
            return Vec::new();
        }
        let roots: Vec<PathBuf> = read_document(&path).unwrap_or_else(|error| {
            warnings.push(error);
            Vec::new()
        });
        let mut collections = Vec::new();
        for root in roots {
            match read_folder(&root) {
                Ok((collection, owned)) => {
                    collections.push(self.adopt(collection, root, owned, taken, warnings));
                }
                Err(error) => {
                    warnings.push(error);
                    self.linked.unavailable.push(root);
                }
            }
        }
        collections
    }

    fn adopt(
        &mut self,
        mut collection: Collection,
        root: PathBuf,
        owned: Vec<PathBuf>,
        taken: &mut HashSet<String>,
        warnings: &mut Vec<StoreError>,
    ) -> Collection {
        let stored_id = collection.info.postman_id.clone();
        collection.claim_unique_id(taken);
        self.linked
            .insert(collection.id(), LinkedFolder::new(root, owned));
        if stored_id.as_deref() != Some(collection.id()) {
            let rewritten = self
                .folder_write(&collection)
                .and_then(|write| write.and_then(StoreWrite::commit).err());
            warnings.extend(rewritten);
        }
        collection
    }

    fn links_write(&self) -> Result<StoreWrite, StoreError> {
        pending(self.root.join(LINKS_FILE), &self.linked.roots()).map(StoreWrite::from)
    }
}

fn ensure_free(root: &Path) -> Result<(), StoreError> {
    if root.is_file() {
        return Err(StoreError::at(root, NOT_A_FOLDER));
    }
    let holds_items = [COLLECTION_FILE, FOLDER_FILE]
        .iter()
        .any(|file| fs::symlink_metadata(root.join(file)).is_ok());
    if holds_items {
        return Err(StoreError::at(root, HOLDS_A_COLLECTION));
    }
    Ok(())
}
