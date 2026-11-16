use requestor_core::collection::{Collection, Item};
use requestor_core::environment::Environment;
use requestor_core::import::{import_text, Imported, ImportedRequest};

use crate::args::ImportOptions;
use crate::error::CliError;
use crate::input::read_text;

pub struct PostmanDocument {
    json: Result<String, String>,
}

impl PostmanDocument {
    pub fn from(imported: Imported) -> Self {
        let json = match imported {
            Imported::Collection(collection) => collection.to_json_pretty(),
            Imported::Environment(environment) => Environment::to_json_pretty(&environment),
            Imported::Request(request) => Self::collection_of(request).to_json_pretty(),
        };
        PostmanDocument {
            json: json.map_err(|error| error.to_string()),
        }
    }

    pub fn into_json(self) -> Result<String, CliError> {
        self.json.map_err(CliError::input)
    }

    fn collection_of(imported: ImportedRequest) -> Collection {
        let mut collection = Collection::new(imported.name.clone());
        collection.item.push(Item {
            id: None,
            name: Some(imported.name),
            description: None,
            variable: None,
            event: None,
            auth: None,
            item: None,
            request: Some(requestor_core::collection::RequestSpec::Full(Box::new(
                imported.request,
            ))),
            response: None,
            extra: Default::default(),
        });
        collection
    }
}

pub fn execute(options: &ImportOptions, out: &mut impl std::io::Write) -> Result<bool, CliError> {
    let text = read_text(&options.file)?;
    let imported = import_text(&text)
        .map_err(|error| CliError::input(format!("{}: {error}", options.file.display())))?;
    let json = PostmanDocument::from(imported).into_json()?;
    writeln!(out, "{json}").map_err(|error| CliError::input(error.to_string()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_request_becomes_a_one_item_collection() {
        let imported = import_text("curl -X POST http://h/x").unwrap();
        let json = PostmanDocument::from(imported).into_json().unwrap();
        let collection = Collection::from_json(&json).unwrap();
        assert_eq!(collection.request_count(), 1);
        assert!(json.contains("collection/v2.1.0"));
    }

    #[test]
    fn environments_keep_their_postman_shape() {
        let imported = import_text(r#"{"name":"E","values":[{"key":"a","value":"1"}]}"#).unwrap();
        let json = PostmanDocument::from(imported).into_json().unwrap();
        assert!(Environment::from_json(&json).is_ok());
    }
}
