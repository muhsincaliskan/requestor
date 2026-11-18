use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct FileIdentity(u64, u64);

#[cfg(unix)]
impl FileIdentity {
    pub(super) fn of(path: &Path) -> Option<Self> {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(path).ok()?;
        Some(FileIdentity(metadata.dev(), metadata.ino()))
    }
}

#[cfg(not(unix))]
impl FileIdentity {
    pub(super) fn of(path: &Path) -> Option<Self> {
        use std::hash::{DefaultHasher, Hash, Hasher};
        fs::symlink_metadata(path).ok()?;
        let mut hasher = DefaultHasher::new();
        super::folded(path).hash(&mut hasher);
        Some(FileIdentity(0, hasher.finish()))
    }
}
