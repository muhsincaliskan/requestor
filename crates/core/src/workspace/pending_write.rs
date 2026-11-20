use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::vault_write::KEYCHAIN_LABEL;

const TEMPORARY_SUFFIX: &str = "tmp";

#[derive(Debug, Clone, PartialEq)]
pub struct StoreError {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingWrite {
    path: PathBuf,
    contents: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingDelete {
    path: PathBuf,
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path.display(), self.reason)
    }
}

impl std::error::Error for StoreError {}

impl StoreError {
    pub(super) fn keychain(reason: impl fmt::Display) -> Self {
        StoreError::at(Path::new(KEYCHAIN_LABEL), reason)
    }

    pub(super) fn at(path: &Path, reason: impl fmt::Display) -> Self {
        StoreError {
            path: path.to_path_buf(),
            reason: reason.to_string(),
        }
    }
}

impl PendingWrite {
    pub fn new(path: PathBuf, contents: impl Into<Vec<u8>>) -> Self {
        PendingWrite {
            path,
            contents: contents.into(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn commit(self) -> Result<(), StoreError> {
        let failure = |error: std::io::Error| StoreError::at(&self.path, error);
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(failure)?;
        }
        let temporary = self.temporary_path();
        let written = write_private(&temporary, &self.contents)
            .and_then(|()| fs::rename(&temporary, &self.path));
        if written.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        written.map_err(failure)
    }

    fn temporary_path(&self) -> PathBuf {
        let mut name = self.path.file_name().unwrap_or_default().to_os_string();
        name.push(format!(".{}.{TEMPORARY_SUFFIX}", std::process::id()));
        self.path.with_file_name(name)
    }
}

impl PendingDelete {
    pub fn new(path: PathBuf) -> Self {
        PendingDelete { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn commit(self) -> Result<(), StoreError> {
        match fs::remove_file(&self.path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(StoreError::at(&self.path, error))
            }
            _ => Ok(()),
        }
    }
}

fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut file = private_file_options().open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

#[cfg(unix)]
fn private_file_options() -> fs::OpenOptions {
    use std::os::unix::fs::OpenOptionsExt;

    const OWNER_READ_WRITE: u32 = 0o600;
    let mut options = fs::OpenOptions::new();
    options
        .write(true)
        .create(true)
        .truncate(true)
        .mode(OWNER_READ_WRITE);
    options
}

#[cfg(not(unix))]
fn private_file_options() -> fs::OpenOptions {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    options
}
