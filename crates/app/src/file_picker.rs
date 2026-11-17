use std::path::{Path, PathBuf};

use rfd::FileDialog;

const IMPORT_TITLE: &str = "Import into Requestor";
const EXPORT_TITLE: &str = "Export from Requestor";
const DOCUMENT_FILTER_NAME: &str = "Collections, environments, OpenAPI, cURL";
const DOCUMENT_EXTENSIONS: [&str; 5] = ["json", "txt", "sh", "http", "curl"];

pub fn import_file() -> Option<PathBuf> {
    FileDialog::new()
        .set_title(IMPORT_TITLE)
        .add_filter(DOCUMENT_FILTER_NAME, &DOCUMENT_EXTENSIONS)
        .pick_file()
}

pub fn export_location(suggested: &Path) -> Option<PathBuf> {
    let mut dialog = FileDialog::new().set_title(EXPORT_TITLE);
    if let Some(directory) = suggested.parent().filter(|directory| directory.is_dir()) {
        dialog = dialog.set_directory(directory);
    }
    if let Some(name) = suggested.file_name() {
        dialog = dialog.set_file_name(name.to_string_lossy());
    }
    dialog.save_file()
}
