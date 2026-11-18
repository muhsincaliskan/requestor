use roxmltree::Node;

use super::qname::QName;
use super::xml::{
    ElementNode, SOAP_1_1_BINDING_NAMESPACE, SOAP_1_2_BINDING_NAMESPACE, WSDL_NAMESPACE,
};
use crate::soap::SoapVersion;

const RPC_STYLE: &str = "rpc";
const PARTS_SEPARATOR: char = ' ';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Style {
    #[default]
    Document,
    Rpc,
}

#[derive(Debug)]
pub struct Binding {
    pub name: QName,
    pub port_type: QName,
    pub version: SoapVersion,
    style: Style,
    pub operations: Vec<BindingOperation>,
}

#[derive(Debug)]
pub struct BindingOperation {
    pub name: String,
    pub action: Option<String>,
    style: Option<Style>,
    pub body_namespace: Option<String>,
    body_parts: Option<Vec<String>>,
    pub headers: Vec<HeaderPart>,
}

#[derive(Debug)]
pub struct HeaderPart {
    pub message: QName,
    pub part: String,
}

#[derive(Debug)]
pub struct Service {
    pub name: String,
    pub ports: Vec<Port>,
}

#[derive(Debug)]
pub struct Port {
    pub name: String,
    pub binding: QName,
    pub address: String,
}

impl Style {
    fn of(node: Node) -> Option<Self> {
        node.trimmed("style").map(|style| match style {
            RPC_STYLE => Style::Rpc,
            _ => Style::Document,
        })
    }
}

impl Binding {
    pub fn from_node(node: Node, target_namespace: &str) -> Option<Self> {
        let (version, soap_namespace) = soap_version(node)?;
        let soap = node.element(soap_namespace, "binding");
        Some(Binding {
            name: QName::new(target_namespace, node.trimmed("name")?),
            port_type: QName::of_attribute(node, "type")?,
            version,
            style: soap.and_then(Style::of).unwrap_or_default(),
            operations: node
                .elements(WSDL_NAMESPACE, "operation")
                .filter_map(|operation| BindingOperation::from_node(operation, soap_namespace))
                .collect(),
        })
    }

    pub fn style_of(&self, operation: &BindingOperation) -> Style {
        operation.style.unwrap_or(self.style)
    }
}

impl BindingOperation {
    fn from_node(node: Node, soap_namespace: &'static str) -> Option<Self> {
        let soap = node.element(soap_namespace, "operation");
        let input = node.element(WSDL_NAMESPACE, "input");
        let body = input.and_then(|input| input.element(soap_namespace, "body"));
        Some(BindingOperation {
            name: node.trimmed("name")?.to_string(),
            action: soap
                .and_then(|soap| soap.trimmed("soapAction"))
                .map(str::to_string),
            style: soap.and_then(Style::of),
            body_namespace: body
                .and_then(|body| body.trimmed("namespace"))
                .map(str::to_string),
            body_parts: body.and_then(|body| body.attribute("parts")).map(|parts| {
                parts
                    .split(PARTS_SEPARATOR)
                    .filter(|part| !part.is_empty())
                    .map(str::to_string)
                    .collect()
            }),
            headers: input
                .into_iter()
                .flat_map(|input| input.elements(soap_namespace, "header"))
                .filter_map(HeaderPart::from_node)
                .collect(),
        })
    }

    pub fn sends_in_body(&self, message: &QName, part: &str) -> bool {
        match &self.body_parts {
            Some(parts) => parts.iter().any(|name| name == part),
            None => !self
                .headers
                .iter()
                .any(|header| &header.message == message && header.part == part),
        }
    }
}

impl HeaderPart {
    fn from_node(node: Node) -> Option<Self> {
        Some(HeaderPart {
            message: QName::of_attribute(node, "message")?,
            part: node.trimmed("part")?.to_string(),
        })
    }
}

impl Service {
    pub fn from_node(node: Node) -> Self {
        Service {
            name: node.trimmed("name").unwrap_or_default().to_string(),
            ports: node
                .elements(WSDL_NAMESPACE, "port")
                .filter_map(Port::from_node)
                .collect(),
        }
    }
}

impl Port {
    fn from_node(node: Node) -> Option<Self> {
        let address = [SOAP_1_1_BINDING_NAMESPACE, SOAP_1_2_BINDING_NAMESPACE]
            .into_iter()
            .find_map(|namespace| node.element(namespace, "address"))?;
        Some(Port {
            name: node.trimmed("name")?.to_string(),
            binding: QName::of_attribute(node, "binding")?,
            address: address.trimmed("location").unwrap_or_default().to_string(),
        })
    }
}

fn soap_version(node: Node) -> Option<(SoapVersion, &'static str)> {
    [
        (SoapVersion::V1_1, SOAP_1_1_BINDING_NAMESPACE),
        (SoapVersion::V1_2, SOAP_1_2_BINDING_NAMESPACE),
    ]
    .into_iter()
    .find(|(_, namespace)| node.element(namespace, "binding").is_some())
}
