use std::collections::HashMap;

use roxmltree::Node;

use super::binding::{Binding, Service};
use super::qname::QName;
use super::schema::Schema;
use super::xml::{ElementNode, WSDL_NAMESPACE};

#[derive(Debug, Default)]
pub struct Definitions {
    pub name: Option<String>,
    messages: HashMap<QName, Message>,
    port_types: HashMap<QName, PortType>,
    pub bindings: Vec<Binding>,
    pub services: Vec<Service>,
    pub schema: Schema,
}

#[derive(Debug)]
pub struct Message {
    pub parts: Vec<Part>,
}

#[derive(Debug)]
pub struct Part {
    pub name: String,
    pub kind: PartKind,
}

#[derive(Debug)]
pub enum PartKind {
    Element(QName),
    Type(QName),
}

#[derive(Debug, Default)]
pub struct PortType {
    operations: HashMap<String, AbstractOperation>,
}

#[derive(Debug)]
pub struct AbstractOperation {
    pub input: Option<QName>,
    pub documentation: Option<String>,
}

impl Definitions {
    pub fn add(&mut self, definitions: Node) {
        let target = definitions.trimmed("targetNamespace").unwrap_or_default();
        if self.name.is_none() {
            self.name = definitions.trimmed("name").map(str::to_string);
        }
        for child in definitions.children().filter(|child| child.is_element()) {
            self.add_child(child, target);
        }
    }

    fn add_child(&mut self, child: Node, target: &str) {
        if child.tag_name().namespace() != Some(WSDL_NAMESPACE) {
            return;
        }
        let name = || QName::new(target, child.trimmed("name").unwrap_or_default());
        match child.tag_name().name() {
            "message" => {
                self.messages.insert(name(), Message::from_node(child));
            }
            "portType" => {
                self.port_types.insert(name(), PortType::from_node(child));
            }
            "binding" => self.bindings.extend(Binding::from_node(child, target)),
            "service" => self.services.push(Service::from_node(child)),
            _ => {}
        }
    }

    pub fn operation(&self, port_type: &QName, name: &str) -> Option<&AbstractOperation> {
        self.port_types.get(port_type)?.operations.get(name)
    }

    pub fn message(&self, name: &QName) -> Option<&Message> {
        self.messages.get(name)
    }

    pub fn binding(&self, name: &QName) -> Option<&Binding> {
        self.bindings.iter().find(|binding| &binding.name == name)
    }
}

impl Message {
    fn from_node(node: Node) -> Self {
        Message {
            parts: node
                .elements(WSDL_NAMESPACE, "part")
                .filter_map(Part::from_node)
                .collect(),
        }
    }
}

impl Part {
    fn from_node(node: Node) -> Option<Self> {
        let kind = QName::of_attribute(node, "element")
            .map(PartKind::Element)
            .or_else(|| QName::of_attribute(node, "type").map(PartKind::Type))?;
        Some(Part {
            name: node.trimmed("name")?.to_string(),
            kind,
        })
    }
}

impl PortType {
    fn from_node(node: Node) -> Self {
        PortType {
            operations: node
                .elements(WSDL_NAMESPACE, "operation")
                .filter_map(|operation| {
                    let name = operation.trimmed("name")?.to_string();
                    Some((name, AbstractOperation::from_node(operation)))
                })
                .collect(),
        }
    }
}

impl AbstractOperation {
    fn from_node(node: Node) -> Self {
        AbstractOperation {
            input: node
                .element(WSDL_NAMESPACE, "input")
                .and_then(|input| QName::of_attribute(input, "message")),
            documentation: node
                .element(WSDL_NAMESPACE, "documentation")
                .and_then(|documentation| documentation.text())
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(str::to_string),
        }
    }
}
