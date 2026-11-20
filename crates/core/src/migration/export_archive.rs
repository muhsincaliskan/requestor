use std::fs::File;
use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

use super::export_files::{is_export_name, ImportBudget, MAX_FILES};
use super::MigrationReport;
use crate::import::import_text;

const ARCHIVE_EXTENSION: &str = "zip";
const ENTRY_SEPARATOR: char = ':';
const TOO_MANY_ENTRIES: &str = "Too many files in the archive; only this many were imported:";

pub(super) struct PostmanExportArchive;

impl PostmanExportArchive {
    pub(super) fn is_archive(path: &Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(ARCHIVE_EXTENSION))
    }

    pub(super) fn import_into(
        path: &Path,
        report: &mut MigrationReport,
        budget: &mut ImportBudget,
    ) {
        let source = path.to_string_lossy();
        let archive = File::open(path)
            .map_err(|error| error.to_string())
            .and_then(|file| ZipArchive::new(file).map_err(|error| error.to_string()));
        match archive {
            Ok(mut archive) => Self::import_entries(&source, &mut archive, report, budget),
            Err(reason) => report.fail(&source, reason),
        }
    }

    fn import_entries(
        source: &str,
        archive: &mut ZipArchive<File>,
        report: &mut MigrationReport,
        budget: &mut ImportBudget,
    ) {
        let mut imported = 0;
        for index in 0..archive.len() {
            let Ok(entry) = archive.by_index(index) else {
                continue;
            };
            let Ok(name) = entry.name().map(|name| name.into_owned()) else {
                continue;
            };
            if entry.is_dir() || !is_export_name(&name) {
                continue;
            }
            if imported == MAX_FILES {
                report
                    .warnings
                    .push(format!("{TOO_MANY_ENTRIES} {MAX_FILES}"));
                return;
            }
            imported += 1;
            let label = format!("{source}{ENTRY_SEPARATOR}{name}");
            match read_entry(entry, budget) {
                Ok(text) => report.record(&label, import_text(&text)),
                Err(reason) => report.fail(&label, reason),
            }
        }
    }
}

fn read_entry(entry: impl Read, budget: &mut ImportBudget) -> Result<String, String> {
    let limit = budget.next_file_limit();
    let mut bytes = Vec::new();
    entry
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    budget.spend(bytes.len() as u64)?;
    String::from_utf8(bytes).map_err(|error| error.to_string())
}
