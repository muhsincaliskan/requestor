use std::path::{Path, PathBuf};

use rfd::FileDialog;

use crate::state::FolderPurpose;

const IMPORT_TITLE: &str = "Import into Requestor";
const EXPORT_TITLE: &str = "Export from Requestor";
const SAVE_FOLDER_TITLE: &str = "Choose an empty folder for the collection";
const OPEN_FOLDER_TITLE: &str = "Open a collection folder";
const DATA_FOLDER_TITLE: &str = "Choose where Requestor keeps its data";
const POSTMAN_FILES_TITLE: &str = "Choose Postman export files";
const POSTMAN_FOLDER_TITLE: &str = "Choose a folder of Postman exports";
const JSON_FILTER_NAME: &str = "JSON";
const JSON_EXTENSIONS: [&str; 1] = ["json"];
const CERTIFICATE_TITLE: &str = "Choose a PEM certificate or key";
const PEM_FILTER_NAME: &str = "PEM";
const PEM_EXTENSIONS: [&str; 4] = ["pem", "crt", "cer", "key"];

pub fn import_file() -> Option<PathBuf> {
    FileDialog::new().set_title(IMPORT_TITLE).pick_file()
}

pub fn postman_export_files() -> Option<Vec<PathBuf>> {
    FileDialog::new()
        .set_title(POSTMAN_FILES_TITLE)
        .add_filter(JSON_FILTER_NAME, &JSON_EXTENSIONS)
        .pick_files()
}

pub fn certificate_file() -> Option<PathBuf> {
    FileDialog::new()
        .set_title(CERTIFICATE_TITLE)
        .add_filter(PEM_FILTER_NAME, &PEM_EXTENSIONS)
        .pick_file()
}

pub fn folder(purpose: &FolderPurpose) -> Option<PathBuf> {
    let title = match purpose {
        FolderPurpose::SaveCollection(_) => SAVE_FOLDER_TITLE,
        FolderPurpose::OpenCollection => OPEN_FOLDER_TITLE,
        FolderPurpose::DataFolder => DATA_FOLDER_TITLE,
        FolderPurpose::PostmanExport => POSTMAN_FOLDER_TITLE,
    };
    FileDialog::new().set_title(title).pick_folder()
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
