use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::workspace::folder::{folded, FolderLayout, FolderWrite};

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

    pub(super) fn next_write(&mut self, layout: FolderLayout) -> FolderWrite {
        let owned: Vec<PathBuf> = layout.paths().map(|path| self.root.join(path)).collect();
        let kept: HashSet<&PathBuf> = owned.iter().collect();
        let stale: Vec<PathBuf> = self
            .owned
            .iter()
            .filter(|path| !kept.contains(path) && fs::symlink_metadata(path).is_ok())
            .cloned()
            .collect();
        let rewritten: HashSet<String> = owned.iter().map(|path| folded(path)).collect();
        let retried: Vec<PathBuf> = stale
            .iter()
            .filter(|path| !rewritten.contains(&folded(path)))
            .cloned()
            .collect();
        self.owned = owned;
        self.owned.extend(retried);
        FolderWrite::new(self.root.clone(), layout, stale)
    }
}

pub(super) fn same_folder(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}
