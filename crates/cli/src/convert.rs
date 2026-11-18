use requestor_core::error::CoreError;
use requestor_core::import::{import_address, import_file_text, Imported};
use requestor_core::settings::RequestSettings;
use requestor_core::transport::ReqwestTransport;

use crate::args::ImportOptions;
use crate::error::CliError;
use crate::input::read_text;

pub struct PostmanDocument(Result<String, CoreError>);

impl PostmanDocument {
    pub fn from(imported: Imported) -> Self {
        PostmanDocument(match imported {
            Imported::Collection(imported) => imported.collection.to_json_pretty(),
            Imported::Environment(environment) => environment.to_json_pretty(),
            Imported::Request(imported) => imported.into_collection().to_json_pretty(),
        })
    }

    pub fn into_json(self) -> Result<String, CliError> {
        self.0.map_err(|error| CliError::input(error.to_string()))
    }
}

pub fn execute(options: &ImportOptions, out: &mut impl std::io::Write) -> Result<bool, CliError> {
    let imported = load(options)?;
    for warning in imported.warnings() {
        eprintln!("warning: {warning}");
    }
    let json = PostmanDocument::from(imported).into_json()?;
    writeln!(out, "{json}").map_err(|error| CliError::from_write(&error))?;
    Ok(true)
}

fn load(options: &ImportOptions) -> Result<Imported, CliError> {
    let imported = match options.web_address() {
        Some(address) => {
            let transport = ReqwestTransport::new(&RequestSettings::default())
                .map_err(|error| CliError::input(error.to_string()))?;
            import_address(address, &transport)
        }
        None => import_file_text(&read_text(&options.file)?, &options.file),
    };
    imported.map_err(|error| CliError::input(format!("{}: {error}", options.file.display())))
}

#[cfg(test)]
mod tests {
    use requestor_core::collection::Collection;
    use requestor_core::environment::Environment;
    use requestor_core::import::import_text;

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

    #[test]
    fn collections_pass_through_unchanged_in_shape() {
        let imported = import_text(r#"{"info":{"name":"c"},"item":[]}"#).unwrap();
        let json = PostmanDocument::from(imported).into_json().unwrap();
        assert_eq!(Collection::from_json(&json).unwrap().info.name, "c");
    }
}
