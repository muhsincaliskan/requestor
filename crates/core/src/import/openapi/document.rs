use serde_json::Value;

use super::operation::Operation;
use super::reference::Resolver;
use super::security;
use super::tree::ItemTree;
use super::values::{description_of, non_empty_text};
use super::{BASE_URL_VARIABLE, MAX_OPERATIONS};
use crate::collection::{Auth, Collection, Item, Variable, SCHEME_SEPARATOR};
use crate::import::ImportError;

const DEFAULT_SERVER_URL: &str = "/";

const SUPPORTED_VERSION_PREFIX: &str = "3.";
const DEFAULT_TITLE: &str = "Imported API";
const CONVERT_HINT: &str = "convert to OpenAPI 3 JSON";

pub struct Document<'a> {
    root: &'a Value,
}

impl<'a> Document<'a> {
    pub fn parse(root: &'a Value) -> Result<Self, ImportError> {
        let version = root.get("openapi").and_then(Value::as_str);
        match version {
            Some(version) if version.starts_with(SUPPORTED_VERSION_PREFIX) => Ok(Document { root }),
            _ if root.get("swagger").is_some() => Err(ImportError::UnsupportedFormat(format!(
                "Swagger 2.0 documents are not supported; {CONVERT_HINT}"
            ))),
            _ => Err(ImportError::UnsupportedFormat(format!(
                "OpenAPI version {} is not supported; {CONVERT_HINT}",
                version.unwrap_or("unknown")
            ))),
        }
    }

    pub fn root(&self) -> &'a Value {
        self.root
    }

    pub fn auth_for(&self, resolver: &mut Resolver<'a>, requirements: &'a Value) -> Option<Auth> {
        let schemes = self.root.pointer("/components/securitySchemes");
        security::auth_for(resolver, schemes, requirements)
    }

    pub fn to_collection(&self, resolver: &mut Resolver<'a>) -> Collection {
        let info = self.root.get("info");
        let title = info.and_then(|info| non_empty_text(info, "title"));
        let mut collection = Collection::new(title.unwrap_or(DEFAULT_TITLE));
        collection.info.description = info.and_then(description_of);
        collection.variable = Some(vec![Variable::text(
            BASE_URL_VARIABLE,
            &self.base_url(resolver),
        )]);
        collection.auth = self
            .root
            .get("security")
            .and_then(|requirements| self.auth_for(resolver, requirements));
        collection.item = self.items(resolver);
        collection
    }

    fn base_url(&self, resolver: &mut Resolver<'a>) -> String {
        let server = self
            .root
            .get("servers")
            .and_then(Value::as_array)
            .and_then(|servers| servers.first());
        let declared = server.and_then(|server| server.get("url")?.as_str());
        if declared.is_none() {
            resolver.warn(format!(
                "no servers are defined; {DEFAULT_SERVER_URL} was assumed, set the {BASE_URL_VARIABLE} variable"
            ));
        }
        let mut url = declared.unwrap_or(DEFAULT_SERVER_URL).to_string();
        let variables = server.and_then(|server| server.get("variables")?.as_object());
        for (name, variable) in variables.into_iter().flatten() {
            if let Some(default) = variable.get("default").and_then(Value::as_str) {
                url = url.replace(&format!("{{{name}}}"), default);
            }
        }
        if !url.contains(SCHEME_SEPARATOR) {
            resolver.warn(format!(
                "the server URL {url} is relative; set the {BASE_URL_VARIABLE} variable to an absolute URL"
            ));
        }
        url.trim_end_matches('/').to_string()
    }

    fn items(&self, resolver: &mut Resolver<'a>) -> Vec<Item> {
        let paths = self.root.get("paths").and_then(Value::as_object);
        let operations: Vec<Operation<'a>> = paths
            .into_iter()
            .flatten()
            .filter_map(|(path, item)| Some((path.as_str(), resolver.resolve(item)?)))
            .flat_map(|(path, item)| Operation::of_path_item(path, item))
            .take(MAX_OPERATIONS + 1)
            .collect();
        if operations.len() > MAX_OPERATIONS {
            resolver.warn(format!(
                "only the first {MAX_OPERATIONS} operations were imported"
            ));
        }
        let mut tree = ItemTree::default();
        for operation in operations.iter().take(MAX_OPERATIONS) {
            tree.push(operation.folder(), operation.to_item(resolver, self));
        }
        tree.into_items()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Document;
    use crate::import::openapi::reference::Resolver;

    #[test]
    fn only_openapi_three_is_accepted() {
        let swagger = json!({"swagger": "2.0"});
        let error = Document::parse(&swagger).err().unwrap().to_string();
        assert!(error.contains("Swagger 2.0") && error.contains("convert to OpenAPI 3 JSON"));
        let future = json!({"openapi": "4.0.0"});
        assert!(Document::parse(&future).is_err());
        assert!(Document::parse(&json!({"openapi": "3.1.0"})).is_ok());
    }

    #[test]
    fn server_variables_are_substituted_with_their_defaults() {
        let root = json!({"openapi": "3.0.0", "servers": [{
            "url": "https://{region}.api.test/{base}/",
            "variables": {"region": {"default": "eu"}, "base": {"default": "v2"}}
        }]});
        let mut resolver = Resolver::new(&root);
        let document = Document::parse(&root).unwrap();
        let collection = document.to_collection(&mut resolver);
        let variable = &collection.variable.unwrap()[0];
        assert_eq!(variable.text_value().unwrap(), "https://eu.api.test/v2");
        assert!(resolver.into_warnings().is_empty());
    }

    #[test]
    fn relative_and_missing_servers_warn() {
        for root in [
            json!({"openapi": "3.0.0", "servers": [{"url": "/api"}]}),
            json!({"openapi": "3.0.0"}),
        ] {
            let mut resolver = Resolver::new(&root);
            Document::parse(&root).unwrap().to_collection(&mut resolver);
            let warnings = resolver.into_warnings();
            assert!(warnings.iter().any(|warning| warning.contains("relative")));
        }
    }
}
