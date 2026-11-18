use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::folded;
use super::identity::FileIdentity;

#[derive(Debug, Default)]
pub struct FolderSurroundings {
    root: Option<PathBuf>,
    owned: HashSet<PathBuf>,
    owned_by_fold: HashMap<String, PathBuf>,
}

impl FolderSurroundings {
    pub fn of(root: &Path, owned: &[PathBuf]) -> Self {
        let owned: HashSet<PathBuf> = owned
            .iter()
            .flat_map(|path| path.ancestors().take_while(|ancestor| *ancestor != root))
            .map(Path::to_path_buf)
            .collect();
        let owned_by_fold = owned
            .iter()
            .map(|path| (folded(path), path.clone()))
            .collect();
        FolderSurroundings {
            root: Some(root.to_path_buf()),
            owned,
            owned_by_fold,
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
            .filter(|entry| !self.owns(&entry.path()))
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect()
    }

    fn owns(&self, path: &Path) -> bool {
        if self.owned.contains(path) {
            return true;
        }
        let Some(owned) = self.owned_by_fold.get(&folded(path)) else {
            return false;
        };
        FileIdentity::of(path).is_some_and(|identity| FileIdentity::of(owned) == Some(identity))
    }
}
