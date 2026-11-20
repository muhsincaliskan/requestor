use std::collections::HashSet;
use std::fmt;
use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::collection::Collection;
use crate::environment::Environment;
use crate::workspace::Workspace;

const COLLECTIONS_FOLDER: &str = "collections";
const ENVIRONMENTS_FOLDER: &str = "environments";
const COLLECTION_SUFFIX: &str = ".postman_collection.json";
const ENVIRONMENT_SUFFIX: &str = ".postman_environment.json";
const UNSAFE_NAME_CHARACTERS: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
const SAFE_REPLACEMENT: &str = "_";
const UNTITLED: &str = "untitled";
const FIRST_DUPLICATE: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveError(String);

impl fmt::Display for ArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ArchiveError {}

pub struct WorkspaceArchive {
    writer: ZipWriter<Cursor<Vec<u8>>>,
    names: HashSet<String>,
    keep_credentials: bool,
}

impl WorkspaceArchive {
    pub fn of(workspace: &Workspace, keep_credentials: bool) -> Result<Vec<u8>, ArchiveError> {
        let mut archive = WorkspaceArchive {
            writer: ZipWriter::new(Cursor::new(Vec::new())),
            names: HashSet::new(),
            keep_credentials,
        };
        for collection in &workspace.collections {
            archive.add_collection(collection)?;
        }
        for environment in &workspace.environments {
            archive.add_environment(environment)?;
        }
        archive
            .writer
            .finish()
            .map(Cursor::into_inner)
            .map_err(failed)
    }

    fn add_collection(&mut self, collection: &Collection) -> Result<(), ArchiveError> {
        let shared = if self.keep_credentials {
            collection.clone()
        } else {
            collection.without_literal_secrets()
        };
        let json = shared.to_export_json().map_err(failed)?;
        let path = self.unique_path(COLLECTIONS_FOLDER, &collection.info.name, COLLECTION_SUFFIX);
        self.add(&path, &json)
    }

    fn add_environment(&mut self, environment: &Environment) -> Result<(), ArchiveError> {
        let shared = if self.keep_credentials {
            environment.clone()
        } else {
            environment.without_unmarked_secrets()
        };
        let json = shared.to_export_json().map_err(failed)?;
        let path = self.unique_path(ENVIRONMENTS_FOLDER, &environment.name, ENVIRONMENT_SUFFIX);
        self.add(&path, &json)
    }

    fn add(&mut self, path: &str, json: &str) -> Result<(), ArchiveError> {
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        self.writer.start_file(path, options).map_err(failed)?;
        self.writer.write_all(json.as_bytes()).map_err(failed)
    }

    fn unique_path(&mut self, folder: &str, name: &str, suffix: &str) -> String {
        let cleaned = name
            .trim()
            .replace(UNSAFE_NAME_CHARACTERS, SAFE_REPLACEMENT);
        let stem = if cleaned.is_empty() {
            UNTITLED
        } else {
            cleaned.as_str()
        };
        let mut path = format!("{folder}/{stem}{suffix}");
        let mut number = FIRST_DUPLICATE;
        while !self.names.insert(path.to_lowercase()) {
            path = format!("{folder}/{stem} {number}{suffix}");
            number += 1;
        }
        path
    }
}

fn failed(error: impl fmt::Display) -> ArchiveError {
    ArchiveError(error.to_string())
}
