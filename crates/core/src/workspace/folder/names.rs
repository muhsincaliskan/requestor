use std::collections::HashSet;

use super::{COLLECTION_FILE, FOLDER_FILE};

const UNTITLED_STEM: &str = "untitled";
const MAX_STEM_CHARS: usize = 60;
const SEPARATOR: char = '-';
const REQUEST_EXTENSION: &str = ".json";
const FIRST_SUFFIX: usize = 2;

#[derive(Debug)]
pub(super) struct EntryNames {
    taken: HashSet<String>,
}

impl Default for EntryNames {
    fn default() -> Self {
        EntryNames {
            taken: [COLLECTION_FILE, FOLDER_FILE]
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }
}

impl EntryNames {
    pub(super) fn claim(&mut self, name: Option<&str>, is_folder: bool) -> String {
        let stem = stem_of(name.unwrap_or_default());
        let extension = if is_folder { "" } else { REQUEST_EXTENSION };
        let mut entry = format!("{stem}{extension}");
        let mut suffix = FIRST_SUFFIX;
        while !self.taken.insert(entry.to_lowercase()) {
            entry = format!("{stem}{SEPARATOR}{suffix}{extension}");
            suffix += 1;
        }
        entry
    }
}

fn stem_of(name: &str) -> String {
    let mut stem = String::with_capacity(name.len());
    for character in name.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            stem.push(character);
        } else if !stem.is_empty() && !stem.ends_with(SEPARATOR) {
            stem.push(SEPARATOR);
        }
    }
    let stem: String = stem
        .trim_end_matches(SEPARATOR)
        .chars()
        .take(MAX_STEM_CHARS)
        .collect();
    let stem = stem.trim_end_matches(SEPARATOR);
    if stem.is_empty() {
        UNTITLED_STEM.to_string()
    } else {
        stem.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::EntryNames;

    #[test]
    fn names_are_readable_unique_and_never_reserved() {
        let mut names = EntryNames::default();
        assert_eq!(
            names.claim(Some("List Users (v2)"), false),
            "list-users-v2.json"
        );
        assert_eq!(
            names.claim(Some("list users v2"), false),
            "list-users-v2-2.json"
        );
        assert_eq!(names.claim(Some("Users"), true), "users");
        assert_eq!(names.claim(Some("USERS"), true), "users-2");
        assert_eq!(names.claim(Some("Collection"), false), "collection-2.json");
        assert_eq!(names.claim(Some("folder"), false), "folder-2.json");
        assert_eq!(
            names.claim(Some("Kullanıcı Ekle"), false),
            "kullanıcı-ekle.json"
        );
        assert_eq!(names.claim(None, false), "untitled.json");
        assert_eq!(names.claim(Some("../.."), false), "untitled-2.json");
    }
}
