use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use self::index::DocumentIndex;
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
pub(super) const JSON_EXTENSION: &str = "json";
const FILE_NAME_FILLER: char = '_';
const UNNAMED_FILE: &str = "unnamed";

#[derive(Debug)]
pub struct WorkspaceStore {
    root: PathBuf,
    collections: DocumentIndex,
    environments: DocumentIndex,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LoadedWorkspace {
    pub workspace: Workspace,
    pub warnings: Vec<StoreError>,
}

impl WorkspaceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        WorkspaceStore {
            root: root.into(),
            collections: DocumentIndex::default(),
            environments: DocumentIndex::default(),
        }
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

    pub fn load(&mut self) -> LoadedWorkspace {
        let mut warnings = Vec::new();
        let (collections, collection_index) = load_documents(
            &self.root.join(COLLECTIONS_DIRECTORY),
            Collection::claim_unique_id,
            |collection| collection.id().to_string(),
            &mut warnings,
        );
        let (environments, environment_index) = load_documents(
            &self.root.join(ENVIRONMENTS_DIRECTORY),
            Environment::claim_unique_id,
            |environment| environment.id.clone().unwrap_or_default(),
            &mut warnings,
        );
        self.collections = collection_index;
        self.environments = environment_index;
        let mut workspace = Workspace {
            collections,
            environments,
            settings: self.load_file(SETTINGS_FILE, &mut warnings),
            session: self.load_file(SESSION_FILE, &mut warnings),
        };
        workspace.normalize();
        LoadedWorkspace {
            workspace,
            warnings,
        }
    }

    pub fn collection_write(
        &mut self,
        collection: &Collection,
    ) -> Result<PendingWrite, StoreError> {
        let directory = self.root.join(COLLECTIONS_DIRECTORY);
        let path = self.collections.path_for(&directory, collection.id());
        pending(path, collection)
    }

    pub fn environment_write(
        &mut self,
        environment: &Environment,
    ) -> Result<PendingWrite, StoreError> {
        let directory = self.root.join(ENVIRONMENTS_DIRECTORY);
        let id = environment.id.as_deref().unwrap_or_default();
        let path = self.environments.path_for(&directory, id);
        pending(path, environment)
    }

    pub fn settings_write(&self, settings: &AppSettings) -> Result<PendingWrite, StoreError> {
        pending(self.root.join(SETTINGS_FILE), settings)
    }

    pub fn session_write(&self, session: &Session) -> Result<PendingWrite, StoreError> {
        pending(self.root.join(SESSION_FILE), session)
    }

    pub fn release_collection_path(&mut self, id: &str) -> Option<PathBuf> {
        self.collections.release(id)
    }

    pub fn release_environment_path(&mut self, id: &str) -> Option<PathBuf> {
        self.environments.release(id)
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

fn load_documents<T: DeserializeOwned + Serialize>(
    directory: &Path,
    claim_id: fn(&mut T, &mut HashSet<String>) -> bool,
    id_of: fn(&T) -> String,
    warnings: &mut Vec<StoreError>,
) -> (Vec<T>, DocumentIndex) {
    let mut taken = HashSet::new();
    let mut index = DocumentIndex::default();
    let mut documents = Vec::new();
    for path in json_paths(directory) {
        let mut document: T = match read_document(&path) {
            Ok(document) => document,
            Err(error) => {
                warnings.push(error);
                continue;
            }
        };
        if claim_id(&mut document, &mut taken) {
            let rewritten = pending(path.clone(), &document).and_then(PendingWrite::commit);
            warnings.extend(rewritten.err());
        }
        index.remember(id_of(&document), path);
        documents.push(document);
    }
    (documents, index)
}

fn json_paths(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|kind| kind == JSON_EXTENSION))
        .collect();
    paths.sort();
    paths
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

pub(super) fn safe_file_stem(id: &str) -> String {
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

mod index;
#[cfg(test)]
mod tests;
