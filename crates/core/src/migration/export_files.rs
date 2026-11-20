use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::export_archive::PostmanExportArchive;
use super::MigrationReport;
use crate::import::{import_file_text, MAX_IMPORT_FILE_BYTES};
use crate::settings::BYTES_PER_MEGABYTE;

const JSON_EXTENSION: &str = "json";
const EXPORT_INDEX_FILE: &str = "archive.json";
const HIDDEN_PREFIX: char = '.';
pub(super) const MAX_FOLDER_DEPTH: usize = 8;
pub(super) const MAX_FILES: usize = 2_000;
const MAX_TOTAL_BYTES: u64 = 512 * BYTES_PER_MEGABYTE as u64;
const TOO_MANY_FILES: &str = "Too many files; only this many were imported:";
const TOO_LARGE: &str = "the file is too large to import";
const OVER_TOTAL: &str = "skipped because the files together are too large";

#[derive(Debug)]
pub(super) struct ImportBudget {
    remaining: u64,
}

impl ImportBudget {
    pub(super) fn new() -> Self {
        ImportBudget {
            remaining: MAX_TOTAL_BYTES,
        }
    }

    pub(super) fn next_file_limit(&self) -> u64 {
        MAX_IMPORT_FILE_BYTES.min(self.remaining)
    }

    pub(super) fn check(&self, size: u64) -> Result<(), String> {
        if size > MAX_IMPORT_FILE_BYTES {
            return Err(TOO_LARGE.to_string());
        }
        if size > self.remaining {
            return Err(OVER_TOTAL.to_string());
        }
        Ok(())
    }

    pub(super) fn spend(&mut self, size: u64) -> Result<(), String> {
        self.check(size)?;
        self.remaining -= size;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PostmanExportFiles {
    files: BTreeSet<PathBuf>,
    archives: BTreeSet<PathBuf>,
    unreadable: Vec<(PathBuf, String)>,
    truncated: bool,
}

impl PostmanExportFiles {
    pub fn discover(paths: &[PathBuf]) -> Self {
        let mut found = PostmanExportFiles::default();
        for path in paths {
            if path.is_dir() {
                found.scan_folder(path, 0);
            } else if PostmanExportArchive::is_archive(path) {
                found
                    .archives
                    .insert(fs::canonicalize(path).unwrap_or_else(|_| path.clone()));
            } else if !is_export_index(path) {
                found.add(path);
            }
        }
        found
    }

    pub fn is_archive(path: &Path) -> bool {
        PostmanExportArchive::is_archive(path)
    }

    pub fn len(&self) -> usize {
        self.files.len() + self.archives.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn import(&self) -> MigrationReport {
        let mut report = MigrationReport::default();
        for (path, reason) in &self.unreadable {
            report.fail(&path.to_string_lossy(), reason.clone());
        }
        let mut budget = ImportBudget::new();
        for path in &self.files {
            let source = path.to_string_lossy();
            match read_text(path, &mut budget) {
                Ok(text) => report.record(&source, import_file_text(&text, path)),
                Err(reason) => report.fail(&source, reason),
            }
        }
        for archive in &self.archives {
            PostmanExportArchive::import_into(archive, &mut report, &mut budget);
        }
        if self.truncated {
            report
                .warnings
                .push(format!("{TOO_MANY_FILES} {MAX_FILES}"));
        }
        report
    }

    fn add(&mut self, path: &Path) {
        let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if self.files.contains(&path) {
            return;
        }
        if self.files.len() < MAX_FILES {
            self.files.insert(path);
        } else {
            self.truncated = true;
        }
    }

    fn scan_folder(&mut self, folder: &Path, depth: usize) {
        if depth > MAX_FOLDER_DEPTH || self.truncated {
            return;
        }
        let entries = match fs::read_dir(folder) {
            Ok(entries) => entries,
            Err(error) => {
                self.unreadable
                    .push((folder.to_path_buf(), error.to_string()));
                return;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if self.truncated || is_hidden(&path) {
                continue;
            }
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => self.scan_folder(&path, depth + 1),
                Ok(kind) if kind.is_file() && is_export_document(&path) => self.add(&path),
                _ => {}
            }
        }
    }
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with(HIDDEN_PREFIX))
}

fn is_export_index(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(EXPORT_INDEX_FILE))
}

fn is_export_document(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| is_export_name(&name.to_string_lossy()))
}

pub(super) fn is_export_name(name: &str) -> bool {
    let file_name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let is_json = Path::new(file_name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(JSON_EXTENSION));
    let hidden = name
        .split(['/', '\\'])
        .any(|segment| segment.starts_with(HIDDEN_PREFIX));
    is_json && !hidden && !file_name.eq_ignore_ascii_case(EXPORT_INDEX_FILE)
}

fn read_text(path: &Path, budget: &mut ImportBudget) -> Result<String, String> {
    let size = fs::metadata(path).map_err(|error| error.to_string())?.len();
    budget.spend(size)?;
    fs::read_to_string(path).map_err(|error| error.to_string())
}
