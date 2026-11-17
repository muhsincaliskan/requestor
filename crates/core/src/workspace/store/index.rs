use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::{safe_file_stem, JSON_EXTENSION};
use crate::identity::random_uuid;

const CLASH_SUFFIX_LENGTH: usize = 8;

#[derive(Debug, Default)]
pub(super) struct DocumentIndex {
    paths: HashMap<String, PathBuf>,
}

impl DocumentIndex {
    pub(super) fn remember(&mut self, id: String, path: PathBuf) {
        self.paths.insert(id, path);
    }

    pub(super) fn path_for(&mut self, directory: &Path, id: &str) -> PathBuf {
        if let Some(path) = self.paths.get(id).filter(|path| path.exists()) {
            return path.clone();
        }
        self.paths.remove(id);
        let path = self.free_path(directory, &safe_file_stem(id));
        self.paths.insert(id.to_string(), path.clone());
        path
    }

    pub(super) fn release(&mut self, id: &str) -> Option<PathBuf> {
        self.paths.remove(id)
    }

    fn free_path(&self, directory: &Path, stem: &str) -> PathBuf {
        let plain = default_path(directory, stem);
        if self.is_free(&plain) {
            return plain;
        }
        loop {
            let suffix = random_uuid();
            let short = &suffix[..CLASH_SUFFIX_LENGTH];
            let candidate = default_path(directory, &format!("{stem}-{short}"));
            if self.is_free(&candidate) {
                return candidate;
            }
        }
    }

    fn is_free(&self, path: &Path) -> bool {
        let name = lowercase_name(path);
        !path.exists()
            && !self
                .paths
                .values()
                .any(|taken| lowercase_name(taken) == name)
    }
}

fn default_path(directory: &Path, stem: &str) -> PathBuf {
    directory.join(format!("{stem}.{JSON_EXTENSION}"))
}

fn lowercase_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}
