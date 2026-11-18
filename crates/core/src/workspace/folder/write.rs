use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::identity::FileIdentity;
use super::FolderLayout;
use crate::workspace::{PendingDelete, PendingWrite, StoreError};

#[derive(Debug, Clone, PartialEq)]
pub struct FolderWrite {
    root: PathBuf,
    layout: FolderLayout,
    stale: Vec<PathBuf>,
}

impl FolderWrite {
    pub(crate) fn new(root: PathBuf, layout: FolderLayout, stale: Vec<PathBuf>) -> Self {
        FolderWrite {
            root,
            layout,
            stale,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn commit(self) -> Result<(), StoreError> {
        let mut written = HashSet::new();
        for (path, contents) in self.layout.files {
            let path = self.root.join(path);
            PendingWrite::new(path.clone(), contents).commit()?;
            written.extend(FileIdentity::of(&path));
        }
        for path in self.stale {
            if FileIdentity::of(&path).is_some_and(|identity| written.contains(&identity)) {
                continue;
            }
            let parent = path.parent().map(Path::to_path_buf);
            PendingDelete::new(path).commit()?;
            if let Some(parent) = parent {
                remove_empty_directories(&parent, &self.root);
            }
        }
        Ok(())
    }
}

fn remove_empty_directories(directory: &Path, root: &Path) {
    let mut current = Some(directory);
    while let Some(directory) = current.filter(|directory| directory.starts_with(root)) {
        if directory == root || fs::remove_dir(directory).is_err() {
            return;
        }
        current = directory.parent();
    }
}
