use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{COLLECTION_FILE, FOLDER_FILE, ITEMS_KEY, ORDER_KEY};
use crate::collection::Collection;
use crate::workspace::store::JSON_EXTENSION;
use crate::workspace::StoreError;

const HIDDEN_MARK: char = '.';

#[derive(Debug, Default)]
struct FolderReader {
    owned: Vec<PathBuf>,
}

pub fn read_folder(root: &Path) -> Result<(Collection, Vec<PathBuf>), StoreError> {
    let mut reader = FolderReader::default();
    let document = reader.directory(root, COLLECTION_FILE)?;
    let collection =
        serde_json::from_value(document).map_err(|error| StoreError::at(root, error))?;
    Ok((collection, reader.owned))
}

impl FolderReader {
    fn directory(&mut self, directory: &Path, metadata_file: &str) -> Result<Value, StoreError> {
        let mut document = self.document(&directory.join(metadata_file))?;
        let order = take_order(&mut document);
        let mut items = Vec::new();
        for entry in ordered_entries(directory, &order)? {
            let path = directory.join(&entry);
            let item = if path.is_dir() {
                self.directory(&path, FOLDER_FILE)?
            } else {
                self.document(&path)?
            };
            items.push(item);
        }
        if let Value::Object(members) = &mut document {
            members.insert(ITEMS_KEY.to_string(), Value::Array(items));
        }
        Ok(document)
    }

    fn document(&mut self, path: &Path) -> Result<Value, StoreError> {
        let text = fs::read_to_string(path).map_err(|error| StoreError::at(path, error))?;
        let value = serde_json::from_str(&text).map_err(|error| StoreError::at(path, error))?;
        self.owned.push(path.to_path_buf());
        Ok(value)
    }
}

fn take_order(document: &mut Value) -> Vec<String> {
    let Some(Value::Array(order)) = document
        .as_object_mut()
        .and_then(|members| members.remove(ORDER_KEY))
    else {
        return Vec::new();
    };
    order
        .into_iter()
        .filter_map(|entry| entry.as_str().map(str::to_string))
        .collect()
}

fn ordered_entries(directory: &Path, order: &[String]) -> Result<Vec<String>, StoreError> {
    let mut present = child_entries(directory)?;
    let mut entries: Vec<String> = order
        .iter()
        .filter_map(|name| {
            let position = present.iter().position(|entry| entry == name)?;
            Some(present.remove(position))
        })
        .collect();
    present.sort();
    entries.extend(present);
    Ok(entries)
}

fn child_entries(directory: &Path) -> Result<Vec<String>, StoreError> {
    let entries = fs::read_dir(directory).map_err(|error| StoreError::at(directory, error))?;
    Ok(entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            is_item_entry(&entry.path(), &name).then_some(name)
        })
        .collect())
}

fn is_item_entry(path: &Path, name: &str) -> bool {
    if name.starts_with(HIDDEN_MARK) || name == COLLECTION_FILE || name == FOLDER_FILE {
        return false;
    }
    if path.is_dir() {
        return path.join(FOLDER_FILE).is_file();
    }
    path.extension().is_some_and(|kind| kind == JSON_EXTENSION)
}
