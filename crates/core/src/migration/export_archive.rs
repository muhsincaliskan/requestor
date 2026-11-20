use std::fs::File;
use std::io::Read;
use std::path::Path;

use zip::read::ZipFile;
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
            if imported == MAX_FILES {
                report
                    .warnings
                    .push(format!("{TOO_MANY_ENTRIES} {MAX_FILES} ({source})"));
                return;
            }
            match archive.by_index(index) {
                Ok(entry) => imported += Self::import_entry(source, entry, report, budget),
                Err(error) => {
                    report.fail(
                        &format!("{source}{ENTRY_SEPARATOR}{index}"),
                        error.to_string(),
                    );
                    imported += 1;
                }
            }
        }
    }

    fn import_entry(
        source: &str,
        entry: ZipFile<'_, File>,
        report: &mut MigrationReport,
        budget: &mut ImportBudget,
    ) -> usize {
        let name = match entry.name() {
            Ok(name) => name.into_owned(),
            Err(error) => {
                report.fail(source, error.to_string());
                return 1;
            }
        };
        if entry.is_dir() || !is_export_name(&name) {
            return 0;
        }
        let label = format!("{source}{ENTRY_SEPARATOR}{name}");
        match read_entry(entry, budget) {
            Ok(text) => report.record(&label, import_text(&text)),
            Err(reason) => report.fail(&label, reason),
        }
        1
    }
}

fn read_entry(entry: ZipFile<'_, File>, budget: &mut ImportBudget) -> Result<String, String> {
    let declared = entry.size();
    budget.check(declared)?;
    let limit = budget.next_file_limit();
    let mut bytes = Vec::with_capacity(usize::try_from(declared.min(limit)).unwrap_or_default());
    entry
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    budget.spend(bytes.len() as u64)?;
    String::from_utf8(bytes).map_err(|error| error.to_string())
}
