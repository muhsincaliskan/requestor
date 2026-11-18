use super::binding::{Binding, BindingOperation, Style};
use super::definitions::{Definitions, Part};
use super::qname::QName;
use super::sample::SampleWriter;
use crate::collection::{Body, Collection, Item, Request, Url, UrlParts};
use crate::soap::SOAP_ACTION_HEADER;
use crate::text::Description;

const SOAP_METHOD: &str = "POST";
const XML_LANGUAGE: &str = "xml";
const DEFAULT_COLLECTION_NAME: &str = "WSDL";
const PART_SEPARATOR: &str = "\n";
const NO_BINDINGS: &str = "the WSDL has no SOAP bindings";
const NO_OPERATIONS: &str = "the WSDL defines no operations";
const MISSING_ADDRESS: &str = "some operations have no service address; fill in their URL";
pub const MAX_OPERATIONS: usize = 2000;

pub struct Endpoint<'d> {
    folder: String,
    binding: &'d Binding,
    address: String,
}

impl Definitions {
    pub fn to_collection(&self, warnings: &mut Vec<String>) -> Collection {
        let mut collection = Collection::new(self.collection_name());
        let endpoints = self.endpoints();
        warnings.extend(Endpoint::warnings(&endpoints));
        let mut remaining = MAX_OPERATIONS;
        let single = endpoints.len() == 1;
        for endpoint in endpoints {
            let items = endpoint.to_items(self, &mut remaining);
            if single {
                collection.item.extend(items);
            } else {
                collection.item.push(Item::folder(endpoint.folder, items));
            }
        }
        collection
    }

    fn collection_name(&self) -> String {
        self.name
            .clone()
            .or_else(|| self.services.first().map(|service| service.name.clone()))
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| DEFAULT_COLLECTION_NAME.to_string())
    }

    fn endpoints(&self) -> Vec<Endpoint<'_>> {
        let several_services = self.services.len() > 1;
        let ports: Vec<Endpoint> = self
            .services
            .iter()
            .flat_map(|service| service.ports.iter().map(move |port| (service, port)))
            .filter_map(|(service, port)| {
                Some(Endpoint {
                    folder: if several_services {
                        format!("{} {}", service.name, port.name)
                    } else {
                        port.name.clone()
                    },
                    binding: self.binding(&port.binding)?,
                    address: port.address.clone(),
                })
            })
            .collect();
        if !ports.is_empty() {
            return ports;
        }
        self.bindings
            .iter()
            .map(|binding| Endpoint {
                folder: binding.name.local.clone(),
                binding,
                address: String::new(),
            })
            .collect()
    }

    fn input_sample(&self, binding: &Binding, operation: &BindingOperation) -> InputSample {
        let input = self
            .operation(&binding.port_type, &operation.name)
            .and_then(|abstract_operation| abstract_operation.input.as_ref());
        let parts: Vec<&Part> = input
            .and_then(|name| Some((name, self.message(name)?)))
            .map(|(name, message)| {
                message
                    .parts
                    .iter()
                    .filter(|part| operation.sends_in_body(name, &part.name))
                    .collect()
            })
            .unwrap_or_default();
        let body = match binding.style_of(operation) {
            Style::Rpc => {
                let namespace = operation.body_namespace.clone().unwrap_or_default();
                let wrapper = QName::new(namespace, operation.name.as_str());
                SampleWriter::rpc(&self.schema, &wrapper, &parts)
            }
            Style::Document => self.samples(parts.into_iter()),
        };
        let header_parts = operation.headers.iter().filter_map(|header| {
            self.message(&header.message)?
                .parts
                .iter()
                .find(|part| part.name == header.part)
        });
        InputSample {
            header: self.samples(header_parts),
            body,
        }
    }

    fn samples<'p>(&self, parts: impl Iterator<Item = &'p Part>) -> String {
        parts
            .map(|part| SampleWriter::part(&self.schema, part))
            .collect::<Vec<_>>()
            .join(PART_SEPARATOR)
    }
}

struct InputSample {
    header: String,
    body: String,
}

impl Endpoint<'_> {
    fn warnings(endpoints: &[Endpoint]) -> Vec<String> {
        let total: usize = endpoints
            .iter()
            .map(|endpoint| endpoint.binding.operations.len())
            .sum();
        let missing_address = endpoints.iter().any(|endpoint| endpoint.address.is_empty());
        [
            (endpoints.is_empty(), NO_BINDINGS.to_string()),
            (
                total == 0 && !endpoints.is_empty(),
                NO_OPERATIONS.to_string(),
            ),
            (missing_address, MISSING_ADDRESS.to_string()),
            (
                total > MAX_OPERATIONS,
                format!("only the first {MAX_OPERATIONS} operations were imported"),
            ),
        ]
        .into_iter()
        .filter_map(|(applies, warning)| applies.then_some(warning))
        .collect()
    }

    fn to_items(&self, definitions: &Definitions, remaining: &mut usize) -> Vec<Item> {
        let mut items = Vec::new();
        for operation in &self.binding.operations {
            if *remaining == 0 {
                break;
            }
            *remaining -= 1;
            items.push(self.to_item(definitions, operation));
        }
        items
    }

    fn to_item(&self, definitions: &Definitions, operation: &BindingOperation) -> Item {
        let version = self.binding.version;
        let mut request = Request::new(SOAP_METHOD, self.url());
        let sample = definitions.input_sample(self.binding, operation);
        let envelope = version.envelope_with_header(&sample.header, &sample.body);
        request.body = Some(Body::raw(envelope, Some(XML_LANGUAGE)));
        if let Some(action) = operation.action.as_deref() {
            request.push_header(SOAP_ACTION_HEADER, action);
        }
        request.description = definitions
            .operation(&self.binding.port_type, &operation.name)
            .and_then(|abstract_operation| abstract_operation.documentation.clone())
            .map(Description::Text);
        Item::for_request(operation.name.clone(), request)
    }

    fn url(&self) -> Url {
        if self.address.is_empty() {
            return Url::Raw(String::new());
        }
        Url::Parts(Box::new(UrlParts::parse(&self.address)))
    }
}
