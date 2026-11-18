mod binding;
mod definitions;
mod loader;
mod particle;
mod qname;
mod remote;
mod requests;
mod sample;
mod schema;
mod xml;
mod xml_writer;

use roxmltree::Document;

use super::{ImportError, ImportedCollection, WEB_SCHEMES};
use crate::settings::BYTES_PER_MEGABYTE;
use loader::Loader;
use xml::{ElementNode, WSDL_2_NAMESPACE, WSDL_NAMESPACE};

pub use loader::{DocumentSource, FileSource, PastedSource};
pub use remote::RemoteSource;
pub use requests::MAX_OPERATIONS;

pub const MAX_DOCUMENT_BYTES: usize = 16 * BYTES_PER_MEGABYTE;
const XML_START: char = '<';
const WSDL_EXTENSION: &str = ".wsdl";
const WSDL_QUERIES: [&str; 2] = ["wsdl", "singlewsdl"];
const QUERY_SEPARATOR: char = '&';
const QUERY_VALUE_SEPARATOR: char = '=';

pub fn is_xml(text: &str) -> bool {
    text.trim_start().starts_with(XML_START)
}

pub fn is_wsdl_address(text: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(text.trim()) else {
        return false;
    };
    let path = url.path().to_ascii_lowercase();
    let query = url.query().unwrap_or_default().to_ascii_lowercase();
    let wsdl_query = query.split(QUERY_SEPARATOR).any(|pair| {
        let name = pair.split(QUERY_VALUE_SEPARATOR).next().unwrap_or_default();
        WSDL_QUERIES.contains(&name)
    });
    WEB_SCHEMES.contains(&url.scheme()) && (path.ends_with(WSDL_EXTENSION) || wsdl_query)
}

pub fn from_xml(
    text: &str,
    location: &str,
    source: &dyn DocumentSource,
) -> Result<ImportedCollection, ImportError> {
    if text.len() > MAX_DOCUMENT_BYTES {
        return Err(ImportError::UnsupportedFormat(format!(
            "XML documents larger than {} MB are not supported",
            MAX_DOCUMENT_BYTES / BYTES_PER_MEGABYTE
        )));
    }
    let document =
        Document::parse(text).map_err(|error| ImportError::InvalidXml(error.to_string()))?;
    let root = document.root_element();
    if root.tag_name().namespace() == Some(WSDL_2_NAMESPACE) {
        return Err(ImportError::UnsupportedFormat(
            "WSDL 2.0 is not supported; use a WSDL 1.1 document".to_string(),
        ));
    }
    if !root.is(WSDL_NAMESPACE, "definitions") {
        return Err(ImportError::UnsupportedFormat(
            "the XML document is not a WSDL 1.1 file".to_string(),
        ));
    }
    let mut loader = Loader::new(source, location);
    loader.add_definitions(root, location);
    let mut warnings = loader.warnings;
    let collection = loader.definitions.to_collection(&mut warnings);
    Ok(ImportedCollection {
        collection,
        warnings,
    })
}

fn too_large() -> String {
    format!(
        "it is larger than {} MB",
        MAX_DOCUMENT_BYTES / BYTES_PER_MEGABYTE
    )
}
