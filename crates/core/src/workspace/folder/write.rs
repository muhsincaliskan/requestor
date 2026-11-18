use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{folded, FolderLayout};
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
        let written: HashSet<String> = self
            .layout
            .paths()
            .map(|path| folded(&self.root.join(path)))
            .collect();
        let (renamed, stale): (Vec<PathBuf>, Vec<PathBuf>) = self
            .stale
            .into_iter()
            .partition(|path| written.contains(&folded(path)));
        remove_files(renamed, &self.root)?;
        for (path, contents) in self.layout.files {
            PendingWrite::new(self.root.join(path), contents).commit()?;
        }
        remove_files(stale, &self.root)
    }
}

fn remove_files(paths: Vec<PathBuf>, root: &Path) -> Result<(), StoreError> {
    for path in paths {
        let parent = path.parent().map(Path::to_path_buf);
        PendingDelete::new(path).commit()?;
        if let Some(parent) = parent {
            remove_empty_directories(&parent, root);
        }
    }
    Ok(())
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
