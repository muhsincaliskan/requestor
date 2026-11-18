use std::path::Path;

use super::{FolderWrite, PendingDelete, PendingWrite, StoreError};

#[derive(Debug, Clone, PartialEq)]
pub enum StoreWrite {
    File(PendingWrite),
    Folder(FolderWrite),
    Delete(PendingDelete),
    Batch(Vec<StoreWrite>),
}

impl From<PendingWrite> for StoreWrite {
    fn from(write: PendingWrite) -> Self {
        StoreWrite::File(write)
    }
}

impl StoreWrite {
    pub fn path(&self) -> &Path {
        match self {
            StoreWrite::File(write) => write.path(),
            StoreWrite::Folder(write) => write.root(),
            StoreWrite::Delete(delete) => delete.path(),
            StoreWrite::Batch(writes) => writes.first().map_or(Path::new(""), StoreWrite::path),
        }
    }

    pub fn commit(self) -> Result<(), StoreError> {
        match self {
            StoreWrite::File(write) => write.commit(),
            StoreWrite::Folder(write) => write.commit(),
            StoreWrite::Delete(delete) => delete.commit(),
            StoreWrite::Batch(writes) => writes.into_iter().try_for_each(StoreWrite::commit),
        }
    }
}
