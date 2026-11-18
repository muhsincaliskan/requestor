mod binding;
mod definitions;
mod loader;
mod particle;
mod qname;
mod requests;
mod sample;
mod schema;
mod xml;

use roxmltree::Document;

use super::{ImportError, ImportedCollection};
use crate::settings::BYTES_PER_MEGABYTE;
use loader::Loader;
use xml::{ElementNode, WSDL_2_NAMESPACE, WSDL_NAMESPACE};

pub use loader::{DocumentSource, FileSource, PastedSource};
pub use requests::MAX_OPERATIONS;

pub const MAX_DOCUMENT_BYTES: usize = 16 * BYTES_PER_MEGABYTE;
const XML_START: char = '<';

pub fn is_xml(text: &str) -> bool {
    text.trim_start().starts_with(XML_START)
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
