use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::store::{COLLECTIONS_DIRECTORY, ENVIRONMENTS_DIRECTORY, SETTINGS_FILE};

const POINTER_FILE: &str = "data-folder.txt";
const STAGING_DIRECTORY: &str = ".requestor-incoming";
const LINE_BREAK: &str = "\n";
const WORKSPACE_MARKERS: [&str; 3] = [SETTINGS_FILE, COLLECTIONS_DIRECTORY, ENVIRONMENTS_DIRECTORY];
const NESTED_TARGET: &str =
    "choose a folder that neither holds nor sits inside the current data folder";
const FOREIGN_TARGET: &str = "choose an empty folder or one that already holds Requestor data";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFolder {
    home: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Pointer {
    target: PathBuf,
    copy_from: Option<PathBuf>,
}

impl DataFolder {
    pub fn new(home: impl Into<PathBuf>) -> Self {
        DataFolder { home: home.into() }
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn location(&self) -> PathBuf {
        match self.pointer() {
            Some(Pointer {
                copy_from: Some(source),
                ..
            }) => source,
            Some(pointer) if pointer.target.is_dir() => pointer.target,
            _ => self.home.clone(),
        }
    }

    pub fn next_location(&self) -> PathBuf {
        self.pointer()
            .map_or_else(|| self.home.clone(), |pointer| pointer.target)
    }

    pub fn unavailable(&self) -> Option<PathBuf> {
        self.pointer()
            .filter(|pointer| pointer.copy_from.is_none() && !pointer.target.is_dir())
            .map(|pointer| pointer.target)
    }

    pub fn is_default(&self) -> bool {
        self.pointer().is_none()
    }

    pub fn move_to(&self, target: &Path) -> io::Result<()> {
        let current = self.location();
        if same_folder(&current, target) {
            return self.stay_at(current);
        }
        if inside(target, &current) || inside(&current, target) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, NESTED_TARGET));
        }
        let returning_home = same_folder(target, &self.home);
        if !returning_home && !holds_workspace(target) && !is_empty(target) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, FOREIGN_TARGET));
        }
        fs::create_dir_all(target)?;
        let copies = returning_home || !holds_workspace(target);
        self.write(&Pointer {
            target: target.to_path_buf(),
            copy_from: copies.then_some(current),
        })
    }

    pub fn reset(&self) -> io::Result<()> {
        self.move_to(&self.home.clone())
    }

    pub fn settle(&self) -> io::Result<()> {
        let Some(Pointer {
            target,
            copy_from: Some(source),
        }) = self.pointer()
        else {
            return Ok(());
        };
        replace_contents(&source, &target)?;
        if same_folder(&target, &self.home) {
            return self.forget_pointer();
        }
        self.write(&Pointer {
            target,
            copy_from: None,
        })
    }

    fn stay_at(&self, current: PathBuf) -> io::Result<()> {
        if same_folder(&current, &self.home) {
            return self.forget_pointer();
        }
        self.write(&Pointer {
            target: current,
            copy_from: None,
        })
    }

    fn pointer(&self) -> Option<Pointer> {
        let text = fs::read_to_string(self.pointer_file()).ok()?;
        let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
        let target = PathBuf::from(lines.next()?);
        let copy_from = lines.next().map(PathBuf::from);
        target
            .is_absolute()
            .then_some(Pointer { target, copy_from })
    }

    fn write(&self, pointer: &Pointer) -> io::Result<()> {
        let mut text = format!("{}{LINE_BREAK}", pointer.target.display());
        if let Some(source) = &pointer.copy_from {
            text.push_str(&format!("{}{LINE_BREAK}", source.display()));
        }
        fs::create_dir_all(&self.home)?;
        fs::write(self.pointer_file(), text)
    }

    fn forget_pointer(&self) -> io::Result<()> {
        match fs::remove_file(self.pointer_file()) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }

    fn pointer_file(&self) -> PathBuf {
        self.home.join(POINTER_FILE)
    }
}

fn holds_workspace(folder: &Path) -> bool {
    WORKSPACE_MARKERS
        .iter()
        .any(|marker| folder.join(marker).exists())
}

fn is_empty(folder: &Path) -> bool {
    fs::read_dir(folder).map_or(true, |mut entries| entries.next().is_none())
}

fn same_folder(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn inside(folder: &Path, parent: &Path) -> bool {
    match (fs::canonicalize(folder), fs::canonicalize(parent)) {
        (Ok(folder), Ok(parent)) => folder.starts_with(parent),
        _ => folder.starts_with(parent),
    }
}

fn replace_contents(source: &Path, target: &Path) -> io::Result<()> {
    let staging = target.join(STAGING_DIRECTORY);
    discard(&staging)?;
    fs::create_dir_all(&staging)?;
    if let Err(error) = copy_contents(source, &staging) {
        discard(&staging)?;
        return Err(error);
    }
    for entry in fs::read_dir(&staging)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        discard(&destination)?;
        fs::rename(entry.path(), destination)?;
    }
    fs::remove_dir(&staging)
}

fn discard(path: &Path) -> io::Result<()> {
    let result = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(error) => Err(error),
    };
    match result {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

fn copy_contents(source: &Path, target: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == POINTER_FILE || name == STAGING_DIRECTORY {
            continue;
        }
        let kind = entry.file_type()?;
        let destination = target.join(name);
        if kind.is_dir() {
            fs::create_dir_all(&destination)?;
            copy_contents(&entry.path(), &destination)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}
