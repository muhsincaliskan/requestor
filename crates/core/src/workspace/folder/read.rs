use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{COLLECTION_FILE, FOLDER_FILE, ITEMS_KEY, ORDER_KEY, REQUEST_KEY};
use crate::collection::{Collection, Item};
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
        let metadata_path = directory.join(metadata_file);
        let mut document = read_document(&metadata_path)?;
        self.owned.push(metadata_path);
        let order = take_order(&mut document);
        let mut items = Vec::new();
        let (listed, unlisted) = ordered_entries(directory, &order)?;
        let entries = listed
            .into_iter()
            .map(|entry| (entry, true))
            .chain(unlisted.into_iter().map(|entry| (entry, false)));
        for (entry, is_listed) in entries {
            let mark = self.owned.len();
            match self.item(&directory.join(&entry), is_listed) {
                Ok(item) => items.extend(item),
                Err(_) if !is_listed => self.owned.truncate(mark),
                Err(error) => return Err(error),
            }
        }
        if let Value::Object(members) = &mut document {
            members.insert(ITEMS_KEY.to_string(), Value::Array(items));
        }
        Ok(document)
    }

    fn item(&mut self, path: &Path, listed: bool) -> Result<Option<Value>, StoreError> {
        if path.is_dir() {
            return self.directory(path, FOLDER_FILE).map(Some);
        }
        self.request(path, listed)
    }

    fn request(&mut self, path: &Path, listed: bool) -> Result<Option<Value>, StoreError> {
        let text = read_text(path)?;
        if !listed && !text.contains(&format!("\"{REQUEST_KEY}\"")) {
            return Ok(None);
        }
        let document: Value =
            serde_json::from_str(&text).map_err(|error| StoreError::at(path, error))?;
        if !is_request(&document) {
            return Ok(None);
        }
        serde_json::from_value::<Item>(document.clone())
            .map_err(|error| StoreError::at(path, error))?;
        self.owned.push(path.to_path_buf());
        Ok(Some(document))
    }
}

fn read_text(path: &Path) -> Result<String, StoreError> {
    fs::read_to_string(path).map_err(|error| StoreError::at(path, error))
}

fn read_document(path: &Path) -> Result<Value, StoreError> {
    serde_json::from_str(&read_text(path)?).map_err(|error| StoreError::at(path, error))
}

fn is_request(document: &Value) -> bool {
    document
        .as_object()
        .is_some_and(|members| members.contains_key(REQUEST_KEY))
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

fn ordered_entries(
    directory: &Path,
    order: &[String],
) -> Result<(Vec<String>, Vec<String>), StoreError> {
    let mut present = child_entries(directory)?;
    let listed = order
        .iter()
        .filter_map(|name| {
            let folded_name = name.to_lowercase();
            let position = present.iter().position(|entry| entry == name).or_else(|| {
                present
                    .iter()
                    .position(|entry| entry.to_lowercase() == folded_name)
            })?;
            Some(present.remove(position))
        })
        .collect();
    present.sort();
    Ok((listed, present))
}

fn child_entries(directory: &Path) -> Result<Vec<String>, StoreError> {
    let entries = fs::read_dir(directory).map_err(|error| StoreError::at(directory, error))?;
    Ok(entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            is_item_entry(&entry, &name).then_some(name)
        })
        .collect())
}

fn is_item_entry(entry: &DirEntry, name: &str) -> bool {
    if name.starts_with(HIDDEN_MARK) || name == COLLECTION_FILE || name == FOLDER_FILE {
        return false;
    }
    let Ok(kind) = entry.file_type() else {
        return false;
    };
    let path = entry.path();
    if kind.is_dir() {
        return path.join(FOLDER_FILE).is_file();
    }
    kind.is_file() && path.extension().is_some_and(|kind| kind == JSON_EXTENSION)
}
