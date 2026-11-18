use std::fs;
use std::path::Path;

use requestor_core::collection::Collection;
use requestor_core::curl_command::shell_quote;
use requestor_core::workspace::read_folder;

use crate::error::CliError;

const PLAIN_CHARACTERS: &str = "-_./:=@%+,";

pub fn read_text(path: &Path) -> Result<String, CliError> {
    fs::read_to_string(path)
        .map_err(|error| CliError::input(format!("cannot read {}: {error}", path.display())))
}

pub fn load_collection(path: &Path) -> Result<Collection, CliError> {
    if path.is_dir() {
        return read_folder(path)
            .map(|(collection, _)| collection)
            .map_err(|error| CliError::input(error.to_string()));
    }
    Collection::from_json(&read_text(path)?)
        .map_err(|error| CliError::input(format!("{}: {error}", path.display())))
}

pub struct CommandLine(String);

impl CommandLine {
    pub fn from(parts: &[String]) -> Self {
        match parts {
            [single] => CommandLine(single.clone()),
            many => CommandLine(
                many.iter()
                    .map(|part| quoted_word(part))
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
        }
    }

    pub fn into_text(self) -> String {
        self.0
    }
}

fn quoted_word(word: &str) -> String {
    let plain = !word.is_empty()
        && word.chars().all(|character| {
            character.is_ascii_alphanumeric() || PLAIN_CHARACTERS.contains(character)
        });
    if plain {
        return word.to_string();
    }
    shell_quote(word)
}

pub fn load_send_target(parts: &[String]) -> Result<String, CliError> {
    match parts {
        [single] if Path::new(single).is_file() => read_text(Path::new(single)),
        _ => Ok(CommandLine::from(parts).into_text()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(items: &[&str]) -> Vec<String> {
        items.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn single_argument_is_used_verbatim() {
        let line = CommandLine::from(&words(&["curl -X POST 'http://h'"]));
        assert_eq!(line.into_text(), "curl -X POST 'http://h'");
    }

    #[test]
    fn multiple_arguments_are_quoted_individually() {
        let line = CommandLine::from(&words(&["curl", "-d", "it's here", "http://h"]));
        assert_eq!(line.into_text(), "curl -d 'it'\\''s here' http://h");
    }

    #[test]
    fn a_collection_folder_loads_like_a_collection_file() {
        let root = std::env::temp_dir().join(format!(
            "requestor-cli-folder-{}",
            requestor_core::identity::random_uuid()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("collection.json"),
            r#"{"info":{"name":"Shop"},"order":["health.json"]}"#,
        )
        .unwrap();
        fs::write(
            root.join("health.json"),
            r#"{"name":"Health","request":{"url":"/h"}}"#,
        )
        .unwrap();
        let collection = load_collection(&root);
        fs::remove_dir_all(&root).unwrap();
        let collection = collection.unwrap();
        assert_eq!(collection.info.name, "Shop");
        assert_eq!(collection.item[0].name.as_deref(), Some("Health"));
    }

    #[test]
    fn missing_files_report_the_path() {
        let error = read_text(Path::new("/definitely/not/here.json")).unwrap_err();
        assert!(error.to_string().contains("/definitely/not/here.json"));
    }
}
