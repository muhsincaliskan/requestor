use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use super::adoption::{Adoption, LoadedDocument, StoredDocument};
use super::pending_write::{PendingWrite, StoreError};
use super::settings::AppSettings;
use super::{Session, Workspace};
use crate::collection::Collection;
use crate::environment::Environment;

const HOME_OVERRIDE_VARIABLE: &str = "REQUESTOR_HOME";
const COLLECTIONS_DIRECTORY: &str = "collections";
const ENVIRONMENTS_DIRECTORY: &str = "environments";
const SETTINGS_FILE: &str = "settings.json";
const SESSION_FILE: &str = "session.json";
const JSON_EXTENSION: &str = "json";
const FILE_NAME_FILLER: char = '_';
const UNNAMED_FILE: &str = "unnamed";

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceStore {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LoadedWorkspace {
    pub workspace: Workspace,
    pub warnings: Vec<StoreError>,
}

impl WorkspaceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        WorkspaceStore { root: root.into() }
    }

    pub fn in_default_location() -> Option<Self> {
        let variable = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
        variable(HOME_OVERRIDE_VARIABLE)
            .map(PathBuf::from)
            .or_else(|| platform_root(variable))
            .map(WorkspaceStore::new)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn load(&self) -> LoadedWorkspace {
        let mut warnings = Vec::new();
        let mut workspace = Workspace {
            collections: self.load_documents(COLLECTIONS_DIRECTORY, &mut warnings),
            environments: self.load_documents(ENVIRONMENTS_DIRECTORY, &mut warnings),
            settings: self.load_file(SETTINGS_FILE, &mut warnings),
            session: self.load_file(SESSION_FILE, &mut warnings),
        };
        workspace.normalize();
        LoadedWorkspace {
            workspace,
            warnings,
        }
    }

    pub fn collection_write(&self, collection: &Collection) -> Result<PendingWrite, StoreError> {
        self.document_write(COLLECTIONS_DIRECTORY, collection.id(), collection)
    }

    pub fn environment_write(&self, environment: &Environment) -> Result<PendingWrite, StoreError> {
        let id = environment.id.as_deref().unwrap_or_default();
        self.document_write(ENVIRONMENTS_DIRECTORY, id, environment)
    }

    pub fn settings_write(&self, settings: &AppSettings) -> Result<PendingWrite, StoreError> {
        pending(self.root.join(SETTINGS_FILE), settings)
    }

    pub fn session_write(&self, session: &Session) -> Result<PendingWrite, StoreError> {
        pending(self.root.join(SESSION_FILE), session)
    }

    pub fn delete_collection(&self, id: &str) -> Result<(), StoreError> {
        remove_if_present(&self.document_path(COLLECTIONS_DIRECTORY, id))
    }

    pub fn delete_environment(&self, id: &str) -> Result<(), StoreError> {
        remove_if_present(&self.document_path(ENVIRONMENTS_DIRECTORY, id))
    }

    fn document_write<T: Serialize>(
        &self,
        directory: &str,
        id: &str,
        document: &T,
    ) -> Result<PendingWrite, StoreError> {
        pending(self.document_path(directory, id), document)
    }

    fn document_path(&self, directory: &str, id: &str) -> PathBuf {
        self.root
            .join(directory)
            .join(format!("{}.{JSON_EXTENSION}", safe_file_stem(id)))
    }

    fn load_documents<T: DeserializeOwned + Serialize + StoredDocument>(
        &self,
        directory: &str,
        warnings: &mut Vec<StoreError>,
    ) -> Vec<T> {
        let loaded = self.load_directory(directory, warnings);
        let adoption = Adoption::of(loaded, |id| self.document_path(directory, id));
        let mut documents: Vec<T> = adoption
            .settled
            .into_iter()
            .map(|entry| entry.document)
            .collect();
        for entry in adoption.misplaced {
            if let Err(error) = self.move_home(directory, &entry) {
                warnings.push(error);
            }
            documents.push(entry.document);
        }
        documents
    }

    fn move_home<T: Serialize + StoredDocument>(
        &self,
        directory: &str,
        entry: &LoadedDocument<T>,
    ) -> Result<(), StoreError> {
        let id = entry.document.stored_id().unwrap_or_default();
        self.document_write(directory, id, &entry.document)?
            .commit()?;
        remove_if_present(&entry.path)
    }

    fn load_directory<T: DeserializeOwned>(
        &self,
        directory: &str,
        warnings: &mut Vec<StoreError>,
    ) -> Vec<LoadedDocument<T>> {
        let Ok(entries) = fs::read_dir(self.root.join(directory)) else {
            return Vec::new();
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| Some(entry.ok()?.path()))
            .filter(|path| path.extension().is_some_and(|kind| kind == JSON_EXTENSION))
            .collect();
        paths.sort();
        paths
            .into_iter()
            .filter_map(|path| match read_document(&path) {
                Ok(document) => Some(LoadedDocument { path, document }),
                Err(error) => {
                    warnings.push(error);
                    None
                }
            })
            .collect()
    }

    fn load_file<T: DeserializeOwned + Default>(
        &self,
        name: &str,
        warnings: &mut Vec<StoreError>,
    ) -> T {
        let path = self.root.join(name);
        if !path.exists() {
            return T::default();
        }
        read_document(&path).unwrap_or_else(|error| {
            warnings.push(error);
            T::default()
        })
    }
}

fn platform_root(variable: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    const APPLICATION_DIRECTORY: &str = "Requestor";
    if cfg!(target_os = "macos") {
        let home = PathBuf::from(variable("HOME")?);
        return Some(
            home.join("Library")
                .join("Application Support")
                .join(APPLICATION_DIRECTORY),
        );
    }
    if cfg!(windows) {
        return Some(PathBuf::from(variable("APPDATA")?).join(APPLICATION_DIRECTORY));
    }
    let configuration = variable("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(variable("HOME")?).join(".config")))?;
    Some(configuration.join(APPLICATION_DIRECTORY.to_ascii_lowercase()))
}

fn pending<T: Serialize>(path: PathBuf, document: &T) -> Result<PendingWrite, StoreError> {
    serde_json::to_string_pretty(document)
        .map(|contents| PendingWrite::new(path.clone(), contents))
        .map_err(|error| StoreError::at(&path, error))
}

fn read_document<T: DeserializeOwned>(path: &Path) -> Result<T, StoreError> {
    let text = fs::read_to_string(path).map_err(|error| StoreError::at(path, error))?;
    serde_json::from_str(&text).map_err(|error| StoreError::at(path, error))
}

fn remove_if_present(path: &Path) -> Result<(), StoreError> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(StoreError::at(path, error))
        }
        _ => Ok(()),
    }
}

fn safe_file_stem(id: &str) -> String {
    let stem: String = id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                FILE_NAME_FILLER
            }
        })
        .collect();
    if stem.is_empty() {
        UNNAMED_FILE.to_string()
    } else {
        stem
    }
}

#[cfg(test)]
mod tests;
