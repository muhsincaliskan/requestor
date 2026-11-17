use std::path::PathBuf;

use super::action::Action;
use super::tab::TabId;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImportForm {
    pub text: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SaveTabForm {
    pub tab: TabId,
    pub name: String,
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
    pub close_after: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LibraryTarget {
    Collection {
        collection_id: String,
    },
    Item {
        collection_id: String,
        item_id: String,
    },
    Environment {
        environment_id: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenameForm {
    pub target: LibraryTarget,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeleteConfirmation {
    pub target: LibraryTarget,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportForm {
    pub target: LibraryTarget,
    pub file_path: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    Settings,
    Import(ImportForm),
    EnvironmentEditor { environment_id: String },
    CollectionEditor { collection_id: String },
    ConfirmCloseTab(TabId),
    ConfirmDelete(DeleteConfirmation),
    SaveTab(SaveTabForm),
    Rename(RenameForm),
    Export(ExportForm),
}

const COLLECTION_FILE_SUFFIX: &str = ".postman_collection.json";
const ENVIRONMENT_FILE_SUFFIX: &str = ".postman_environment.json";
const HOME_VARIABLE: &str = "HOME";
const EXPORT_DIRECTORY: &str = "Downloads";
const UNSAFE_FILE_CHARACTERS: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

impl LibraryTarget {
    fn file_suffix(&self) -> &'static str {
        match self {
            LibraryTarget::Environment { .. } => ENVIRONMENT_FILE_SUFFIX,
            LibraryTarget::Collection { .. } | LibraryTarget::Item { .. } => COLLECTION_FILE_SUFFIX,
        }
    }
}

impl ExportForm {
    pub fn suggested(target: LibraryTarget, name: &str) -> Self {
        let stem = name.replace(UNSAFE_FILE_CHARACTERS, "_");
        let file_name = format!("{stem}{}", target.file_suffix());
        let file_path = std::env::var_os(HOME_VARIABLE)
            .map(std::path::PathBuf::from)
            .map_or_else(
                || file_name.clone(),
                |home| {
                    home.join(EXPORT_DIRECTORY)
                        .join(&file_name)
                        .display()
                        .to_string()
                },
            );
        ExportForm {
            target,
            file_path,
            error: None,
        }
    }
}

impl ExportForm {
    pub fn submission(&self) -> Option<Action> {
        let file_path = self.file_path.trim();
        (!file_path.is_empty()).then(|| Action::Export {
            target: self.target.clone(),
            file_path: PathBuf::from(file_path),
        })
    }

    pub fn location_choice(&self) -> Action {
        Action::ChooseExportLocation {
            target: self.target.clone(),
            suggested: PathBuf::from(self.file_path.trim()),
        }
    }
}

impl SaveTabForm {
    pub fn submission(&self) -> Option<Action> {
        let name = self.name.trim();
        if name.is_empty() {
            return None;
        }
        Some(Action::SaveTabInto {
            tab: self.tab,
            name: name.to_string(),
            collection_id: self.collection_id.clone()?,
            folder_id: self.folder_id.clone(),
            close_after: self.close_after,
        })
    }
}
