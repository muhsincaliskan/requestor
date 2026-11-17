use std::path::{Path, PathBuf};

use rfd::FileDialog;

const IMPORT_TITLE: &str = "Import into Requestor";
const EXPORT_TITLE: &str = "Export from Requestor";

pub fn import_file() -> Option<PathBuf> {
    FileDialog::new().set_title(IMPORT_TITLE).pick_file()
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
