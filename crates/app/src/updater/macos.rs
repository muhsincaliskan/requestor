use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use requestor_core::update::UpdatePackage;

pub const PACKAGE_NAME: &str = "Requestor-macos.zip";
const BUNDLE_EXTENSION: &str = "app";
const EXECUTABLES: [&str; 2] = ["Contents", "MacOS"];
const STAGING_PREFIX: &str = ".requestor-update-";
const PREVIOUS_PREFIX: &str = ".requestor-previous-";
const EXTRACTED_FOLDER: &str = "extracted";
const UNPACKER: &str = "/usr/bin/ditto";
const UNPACK_ARGUMENTS: [&str; 2] = ["-x", "-k"];
const SHELL: &str = "/bin/sh";
const RELAUNCH_SCRIPT: &str =
    "while kill -0 \"$1\" 2>/dev/null; do sleep 0.2; done; exec /usr/bin/open \"$2\"";
const RELAUNCH_NAME: &str = "requestor-relaunch";
const READ_ONLY_LOCATIONS: [&str; 2] = ["/Volumes/", "/AppTranslocation/"];
const NOT_UNPACKED: &str = "the update package could not be unpacked";
const NO_APP_INSIDE: &str = "the update package does not contain Requestor";
const BACKUP_KEPT_AT: &str = "Requestor could not be put back; the previous version is kept at";

pub struct AppBundle {
    path: PathBuf,
    executable: String,
}

impl AppBundle {
    pub fn current() -> Option<AppBundle> {
        let executable_path = std::env::current_exe().ok()?.canonicalize().ok()?;
        let executable = executable_path.file_name()?.to_str()?.to_string();
        let path = executable_path
            .ancestors()
            .find(|folder| {
                folder
                    .extension()
                    .is_some_and(|ext| ext == BUNDLE_EXTENSION)
            })?
            .to_path_buf();
        Some(AppBundle { path, executable })
    }

    pub fn is_replaceable(&self) -> bool {
        let location = self.path.to_string_lossy();
        let read_only = READ_ONLY_LOCATIONS
            .iter()
            .any(|marker| location.contains(marker));
        let writable_parent = self
            .path
            .parent()
            .and_then(|parent| fs::metadata(parent).ok())
            .is_some_and(|metadata| !metadata.permissions().readonly());
        !read_only && writable_parent
    }

    #[cfg(test)]
    fn at(path: PathBuf, executable: &str) -> AppBundle {
        AppBundle {
            path,
            executable: executable.to_string(),
        }
    }

    pub fn replace_with(&self, package: &UpdatePackage) -> io::Result<()> {
        let parent = self.path.parent().ok_or_else(|| other(NO_APP_INSIDE))?;
        let staging = parent.join(format!("{STAGING_PREFIX}{}", std::process::id()));
        let result = self.install_from(package, &staging);
        let _ = fs::remove_dir_all(&staging);
        result
    }

    pub fn relaunch_after_exit(&self) -> io::Result<()> {
        Command::new(SHELL)
            .args(["-c", RELAUNCH_SCRIPT, RELAUNCH_NAME])
            .arg(std::process::id().to_string())
            .arg(&self.path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(drop)
    }

    fn install_from(&self, package: &UpdatePackage, staging: &Path) -> io::Result<()> {
        let _ = fs::remove_dir_all(staging);
        fs::create_dir_all(staging)?;
        let archive = staging.join(&package.name);
        fs::write(&archive, &package.bytes)?;
        let extracted = staging.join(EXTRACTED_FOLDER);
        let unpacked = Command::new(UNPACKER)
            .args(UNPACK_ARGUMENTS)
            .arg(&archive)
            .arg(&extracted)
            .status()?;
        if !unpacked.success() {
            return Err(other(NOT_UNPACKED));
        }
        let replacement = self.bundle_in(&extracted)?;
        self.swap_in(&replacement)
    }

    fn bundle_in(&self, folder: &Path) -> io::Result<PathBuf> {
        fs::read_dir(folder)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.extension().is_some_and(|ext| ext == BUNDLE_EXTENSION)
                    && executable_of(path, &self.executable).is_file()
            })
            .ok_or_else(|| other(NO_APP_INSIDE))
    }

    fn swap_in(&self, replacement: &Path) -> io::Result<()> {
        let parent = self.path.parent().ok_or_else(|| other(NO_APP_INSIDE))?;
        let previous = parent.join(format!("{PREVIOUS_PREFIX}{}", std::process::id()));
        let _ = fs::remove_dir_all(&previous);
        fs::rename(&self.path, &previous)?;
        if let Err(error) = fs::rename(replacement, &self.path) {
            return match fs::rename(&previous, &self.path) {
                Ok(()) => Err(error),
                Err(_) => Err(other(&format!("{BACKUP_KEPT_AT} {}", previous.display()))),
            };
        }
        let _ = fs::remove_dir_all(&previous);
        Ok(())
    }
}

fn executable_of(bundle: &Path, executable: &str) -> PathBuf {
    EXECUTABLES
        .iter()
        .fold(bundle.to_path_buf(), |path, part| path.join(part))
        .join(executable)
}

fn other(reason: &str) -> io::Error {
    io::Error::other(reason)
}

#[cfg(test)]
mod tests;
