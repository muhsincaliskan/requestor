use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::folded;

#[derive(Debug, Default)]
pub struct FolderSurroundings {
    root: Option<PathBuf>,
    owned: HashSet<String>,
}

impl FolderSurroundings {
    pub fn of(root: &Path, owned: &[PathBuf]) -> Self {
        let owned = owned
            .iter()
            .flat_map(|path| path.ancestors().take_while(|ancestor| *ancestor != root))
            .map(folded)
            .collect();
        FolderSurroundings {
            root: Some(root.to_path_buf()),
            owned,
        }
    }

    pub(super) fn foreign_names_in(&self, directory: &Path) -> Vec<String> {
        let Some(root) = &self.root else {
            return Vec::new();
        };
        let Ok(entries) = fs::read_dir(root.join(directory)) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .filter(|entry| !self.owned.contains(&folded(&entry.path())))
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect()
    }
}
