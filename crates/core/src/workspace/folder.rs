mod names;
mod read;
mod write;

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::collection::{Collection, Item};
use names::EntryNames;

pub use read::read_folder;
pub use write::FolderWrite;

pub(super) const COLLECTION_FILE: &str = "collection.json";
pub(super) const FOLDER_FILE: &str = "folder.json";
const ORDER_KEY: &str = "order";
const ITEMS_KEY: &str = "item";
const REQUEST_KEY: &str = "request";
const LINE_END: char = '\n';

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FolderLayout {
    pub files: Vec<(PathBuf, String)>,
}

impl FolderLayout {
    pub fn of(collection: &Collection) -> Result<Self, serde_json::Error> {
        let mut layout = FolderLayout::default();
        let order = layout.add_children(Path::new(""), &collection.item)?;
        layout.add_file(PathBuf::from(COLLECTION_FILE), collection, order)?;
        Ok(layout)
    }

    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.files.iter().map(|(path, _)| path.as_path())
    }

    pub fn occupies_files_in(&self, root: &Path) -> bool {
        self.paths()
            .any(|path| fs::symlink_metadata(root.join(path)).is_ok())
    }

    fn add_children(
        &mut self,
        directory: &Path,
        items: &[Item],
    ) -> Result<Vec<String>, serde_json::Error> {
        let mut names = EntryNames::default();
        let mut order = Vec::with_capacity(items.len());
        for item in items {
            let entry = names.claim(item.name.as_deref(), item.item.is_some());
            let path = directory.join(&entry);
            match &item.item {
                Some(children) => {
                    let child_order = self.add_children(&path, children)?;
                    self.add_file(path.join(FOLDER_FILE), item, child_order)?;
                }
                None => self.add_file(path, item, Vec::new())?,
            }
            order.push(entry);
        }
        Ok(order)
    }

    fn add_file(
        &mut self,
        path: PathBuf,
        document: &impl Serialize,
        order: Vec<String>,
    ) -> Result<(), serde_json::Error> {
        let mut value = serde_json::to_value(document)?;
        if let Value::Object(members) = &mut value {
            if members.remove(ITEMS_KEY).is_some() {
                members.insert(ORDER_KEY.to_string(), Value::from(order));
            }
        }
        let mut text = serde_json::to_string_pretty(&value)?;
        text.push(LINE_END);
        self.files.push((path, text));
        Ok(())
    }
}

pub(crate) fn folded(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

#[cfg(test)]
mod tests;
