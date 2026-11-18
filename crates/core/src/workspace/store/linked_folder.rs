use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::collection::Collection;
use crate::workspace::folder::{FolderLayout, FolderSurroundings, FolderWrite};

#[derive(Debug)]
pub(super) struct LinkedFolder {
    root: PathBuf,
    owned: Vec<PathBuf>,
}

impl LinkedFolder {
    pub(super) fn new(root: PathBuf, owned: Vec<PathBuf>) -> Self {
        LinkedFolder { root, owned }
    }

    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    pub(super) fn is_at(&self, root: &Path) -> bool {
        same_folder(&self.root, root)
    }

    pub(super) fn overlaps(&self, root: &Path) -> bool {
        nested(&self.root, root)
    }

    pub(super) fn next_write(
        &mut self,
        collection: &Collection,
    ) -> Result<FolderWrite, serde_json::Error> {
        let surroundings = FolderSurroundings::of(&self.root, &self.owned);
        let layout = FolderLayout::around(collection, &surroundings)?;
        let owned: Vec<PathBuf> = layout.paths().map(|path| self.root.join(path)).collect();
        let kept: HashSet<&PathBuf> = owned.iter().collect();
        let stale: Vec<PathBuf> = self
            .owned
            .iter()
            .filter(|path| !kept.contains(path))
            .cloned()
            .collect();
        let retried: Vec<PathBuf> = stale
            .iter()
            .filter(|path| fs::symlink_metadata(path).is_ok())
            .cloned()
            .collect();
        self.owned = owned;
        self.owned.extend(retried);
        Ok(FolderWrite::new(self.root.clone(), layout, stale))
    }
}

pub(super) fn same_folder(left: &Path, right: &Path) -> bool {
    resolved(left) == resolved(right)
}

pub(super) fn nested(left: &Path, right: &Path) -> bool {
    let (left, right) = (resolved(left), resolved(right));
    left.starts_with(&right) || right.starts_with(&left)
}

fn resolved(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
