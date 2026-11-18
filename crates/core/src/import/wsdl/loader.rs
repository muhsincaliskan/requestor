use std::collections::HashSet;
use std::fs;
use std::path::Path;

use roxmltree::{Document, Node};

use super::definitions::Definitions;
use super::schema::Schema;
use super::xml::{ElementNode, SCHEMA_NAMESPACE, WSDL_NAMESPACE};
use super::MAX_DOCUMENT_BYTES;
use crate::settings::BYTES_PER_MEGABYTE;

const MAX_DOCUMENTS: usize = 64;
const REMOTE_SCHEMES: [&str; 2] = ["http://", "https://"];
const STANDARD_SCHEMA_HOSTS: [&str; 3] = [
    "http://schemas.xmlsoap.org/",
    "http://www.w3.org/",
    "https://www.w3.org/",
];
const REMOTE_NOT_FETCHED: &str = "remote documents are not downloaded";
const PASTED_TEXT_HAS_NO_FOLDER: &str =
    "pasted text cannot load the files it imports; import the WSDL file instead";

pub trait DocumentSource {
    fn resolve(&self, base: &str, location: &str) -> String;
    fn read(&self, location: &str) -> Result<String, String>;
}

pub struct FileSource;

pub struct PastedSource;

impl DocumentSource for FileSource {
    fn resolve(&self, base: &str, location: &str) -> String {
        if is_remote(location) {
            return location.to_string();
        }
        Path::new(base)
            .parent()
            .map_or_else(
                || Path::new(location).to_path_buf(),
                |folder| folder.join(location),
            )
            .to_string_lossy()
            .into_owned()
    }

    fn read(&self, location: &str) -> Result<String, String> {
        if is_remote(location) {
            return Err(REMOTE_NOT_FETCHED.to_string());
        }
        let size = fs::metadata(location)
            .map_err(|error| error.to_string())?
            .len();
        if size > MAX_DOCUMENT_BYTES as u64 {
            return Err(format!(
                "it is larger than {} MB",
                MAX_DOCUMENT_BYTES / BYTES_PER_MEGABYTE
            ));
        }
        fs::read_to_string(location).map_err(|error| error.to_string())
    }
}

impl DocumentSource for PastedSource {
    fn resolve(&self, _base: &str, location: &str) -> String {
        location.to_string()
    }

    fn read(&self, _location: &str) -> Result<String, String> {
        Err(PASTED_TEXT_HAS_NO_FOLDER.to_string())
    }
}

pub struct Loader<'a> {
    source: &'a dyn DocumentSource,
    visited: HashSet<String>,
    limit_reported: bool,
    pub definitions: Definitions,
    pub warnings: Vec<String>,
}

impl<'a> Loader<'a> {
    pub fn new(source: &'a dyn DocumentSource, location: &str) -> Self {
        Loader {
            source,
            visited: HashSet::from([location.to_string()]),
            limit_reported: false,
            definitions: Definitions::default(),
            warnings: Vec::new(),
        }
    }

    pub fn add_definitions(&mut self, definitions: Node, location: &str) {
        self.definitions.add(definitions);
        let types = definitions.elements(WSDL_NAMESPACE, "types");
        for schema in types.flat_map(|types| types.elements(SCHEMA_NAMESPACE, "schema")) {
            self.add_schema(schema, location, None);
        }
        for import in definitions.elements(WSDL_NAMESPACE, "import") {
            if let Some(imported) = import.trimmed("location") {
                self.follow(location, imported, None);
            }
        }
    }

    fn add_schema(&mut self, schema: Node, location: &str, inherited_namespace: Option<&str>) {
        self.definitions.schema.add(schema, inherited_namespace);
        let target = schema.trimmed("targetNamespace").or(inherited_namespace);
        for reference in Schema::references(schema) {
            let inherited = if reference.included { target } else { None };
            self.follow(location, &reference.location, inherited);
        }
    }

    fn follow(&mut self, base: &str, location: &str, inherited_namespace: Option<&str>) {
        if is_standard_schema(location) {
            return;
        }
        let resolved = self.source.resolve(base, location);
        if self.visited.contains(&resolved) || self.over_limit() {
            return;
        }
        self.visited.insert(resolved.clone());
        let text = match self.source.read(&resolved) {
            Ok(text) => text,
            Err(reason) => return self.warn(location, &reason),
        };
        let document = match Document::parse(&text) {
            Ok(document) => document,
            Err(error) => return self.warn(location, &error.to_string()),
        };
        let root = document.root_element();
        if root.is(WSDL_NAMESPACE, "definitions") {
            self.add_definitions(root, &resolved);
        } else if root.is(SCHEMA_NAMESPACE, "schema") {
            self.add_schema(root, &resolved, inherited_namespace);
        } else {
            self.warn(location, "it is neither a WSDL nor an XML schema");
        }
    }

    fn over_limit(&mut self) -> bool {
        if self.visited.len() < MAX_DOCUMENTS {
            return false;
        }
        if !self.limit_reported {
            self.limit_reported = true;
            self.warnings.push(format!(
                "only the first {MAX_DOCUMENTS} imported documents were loaded"
            ));
        }
        true
    }

    fn warn(&mut self, location: &str, reason: &str) {
        self.warnings
            .push(format!("{location} was not loaded: {reason}"));
    }
}

fn is_remote(location: &str) -> bool {
    REMOTE_SCHEMES
        .iter()
        .any(|scheme| location.starts_with(scheme))
}

fn is_standard_schema(location: &str) -> bool {
    STANDARD_SCHEMA_HOSTS
        .iter()
        .any(|host| location.starts_with(host))
}
