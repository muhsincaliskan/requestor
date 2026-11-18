use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::linked_folder::LinkedFolder;
use super::{pending, read_document, WorkspaceStore};
use crate::collection::Collection;
use crate::workspace::folder::{read_folder, FolderLayout};
use crate::workspace::{PendingDelete, StoreError, StoreWrite};

const LINKS_FILE: &str = "linked-folders.json";
const OCCUPIED: &str =
    "the folder already holds files with the same names; choose another folder or open it";
const NOT_A_FOLDER: &str = "this is a file, not a folder";
const ALREADY_OPEN: &str = "this folder is already open";

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

    fn ensure_not_open(&self, root: &Path) -> Result<(), StoreError> {
        if self.folders.values().any(|folder| folder.is_at(root)) {
            return Err(StoreError::at(root, ALREADY_OPEN));
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
        self.linked.ensure_not_open(&root)?;
        let layout = FolderLayout::of(collection).map_err(|error| StoreError::at(&root, error))?;
        ensure_free(&root, &layout)?;
        let mut folder = LinkedFolder::new(root, Vec::new());
        let mut writes = vec![StoreWrite::Folder(folder.next_write(layout))];
        self.linked.insert(collection.id(), folder);
        writes.push(self.links_write()?);
        let released = self.collections.release(collection.id());
        writes.extend(released.map(|path| StoreWrite::Delete(PendingDelete::new(path))));
        Ok(StoreWrite::Batch(writes))
    }

    pub fn open_folder(&self, root: &Path) -> Result<(Collection, Vec<PathBuf>), StoreError> {
        self.linked.ensure_not_open(root)?;
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
        let write = FolderLayout::of(collection)
            .map(|layout| StoreWrite::Folder(folder.next_write(layout)))
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

fn ensure_free(root: &Path, layout: &FolderLayout) -> Result<(), StoreError> {
    if root.is_file() {
        return Err(StoreError::at(root, NOT_A_FOLDER));
    }
    if layout.occupies_files_in(root) {
        return Err(StoreError::at(root, OCCUPIED));
    }
    Ok(())
}
