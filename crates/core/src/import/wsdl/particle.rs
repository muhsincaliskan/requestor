use roxmltree::Node;

use super::qname::QName;
use super::xml::{ElementNode, SCHEMA_NAMESPACE};

pub const QUALIFIED: &str = "qualified";
const FLATTENED_GROUPS: [&str; 2] = ["sequence", "all"];
const EXTENSION: &str = "extension";
const ANNOTATION: &str = "annotation";
const CONTENT_DERIVATIONS: [&str; 2] = [EXTENSION, "restriction"];

#[derive(Debug)]
pub struct SchemaScope {
    pub target_namespace: String,
    pub qualified_elements: bool,
    pub chameleon: bool,
}

impl SchemaScope {
    fn qname(&self, node: Node, attribute: &str) -> Option<QName> {
        let name = QName::of_attribute(node, attribute)?;
        if self.chameleon && !name.is_qualified() {
            return Some(QName::new(self.target_namespace.as_str(), name.local));
        }
        Some(name)
    }
}

#[derive(Debug)]
pub struct ElementDecl {
    pub name: QName,
    pub kind: ElementType,
}

#[derive(Debug)]
pub enum ElementType {
    Named(QName),
    Inline(Box<TypeDef>),
    Any,
}

#[derive(Debug)]
pub enum Particle {
    Element(ElementDecl),
    Reference(QName),
    Group(QName),
}

#[derive(Debug)]
pub enum TypeDef {
    Simple(SimpleType),
    Complex(ComplexType),
}

#[derive(Debug)]
pub struct SimpleType {
    pub first_value: Option<String>,
}

#[derive(Debug)]
pub struct ComplexType {
    pub base: Option<QName>,
    pub attributes: Vec<String>,
    pub content: Content,
}

#[derive(Debug)]
pub enum Content {
    Text,
    Particles(Vec<Particle>),
}

impl ElementDecl {
    pub fn global(node: Node, scope: &SchemaScope) -> Self {
        ElementDecl::declared(node, scope, true)
    }

    fn declared(node: Node, scope: &SchemaScope, global: bool) -> Self {
        let local = node.trimmed("name").unwrap_or_default();
        let qualified = match node.attribute("form") {
            Some(form) => form == QUALIFIED,
            None => global || scope.qualified_elements,
        };
        let namespace = if qualified {
            scope.target_namespace.as_str()
        } else {
            ""
        };
        ElementDecl {
            name: QName::new(namespace, local),
            kind: ElementType::of(node, scope),
        }
    }
}

impl ElementType {
    fn of(node: Node, scope: &SchemaScope) -> Self {
        if let Some(name) = scope.qname(node, "type") {
            return ElementType::Named(name);
        }
        node.children()
            .find(|child| {
                child.is(SCHEMA_NAMESPACE, "complexType")
                    || child.is(SCHEMA_NAMESPACE, "simpleType")
            })
            .map_or(ElementType::Any, |inline| {
                ElementType::Inline(Box::new(TypeDef::from_node(inline, scope)))
            })
    }
}

impl Particle {
    pub fn collect(node: Node, scope: &SchemaScope) -> Vec<Particle> {
        let mut particles = Vec::new();
        for child in node.children().filter(Node::is_element) {
            Particle::collect_child(child, scope, &mut particles);
        }
        particles
    }

    fn collect_child(child: Node, scope: &SchemaScope, particles: &mut Vec<Particle>) {
        if child.tag_name().namespace() != Some(SCHEMA_NAMESPACE) {
            return;
        }
        match child.tag_name().name() {
            "element" => particles.push(Particle::of_element(child, scope)),
            "group" => particles.extend(scope.qname(child, "ref").map(Particle::Group)),
            "choice" => {
                if let Some(first) = child
                    .children()
                    .find(|option| option.is_element() && !option.is(SCHEMA_NAMESPACE, ANNOTATION))
                {
                    Particle::collect_child(first, scope, particles);
                }
            }
            name if FLATTENED_GROUPS.contains(&name) => {
                particles.extend(Particle::collect(child, scope));
            }
            _ => {}
        }
    }

    fn of_element(node: Node, scope: &SchemaScope) -> Particle {
        match scope.qname(node, "ref") {
            Some(reference) => Particle::Reference(reference),
            None => Particle::Element(ElementDecl::declared(node, scope, false)),
        }
    }
}

impl TypeDef {
    pub fn from_node(node: Node, scope: &SchemaScope) -> Self {
        if node.tag_name().name() == "complexType" {
            return TypeDef::Complex(ComplexType::from_node(node, scope));
        }
        TypeDef::Simple(SimpleType {
            first_value: node
                .descendants()
                .find(|child| child.is(SCHEMA_NAMESPACE, "enumeration"))
                .and_then(|enumeration| enumeration.attribute("value"))
                .map(str::to_string),
        })
    }
}

impl ComplexType {
    fn from_node(node: Node, scope: &SchemaScope) -> Self {
        if let Some(simple) = node.element(SCHEMA_NAMESPACE, "simpleContent") {
            return ComplexType {
                base: None,
                attributes: attribute_names(derivation(simple).unwrap_or(simple)),
                content: Content::Text,
            };
        }
        let Some(derived) = node
            .element(SCHEMA_NAMESPACE, "complexContent")
            .and_then(derivation)
        else {
            return ComplexType {
                base: None,
                attributes: attribute_names(node),
                content: Content::Particles(Particle::collect(node, scope)),
            };
        };
        let extends = derived.tag_name().name() == EXTENSION;
        ComplexType {
            base: scope.qname(derived, "base").filter(|_| extends),
            attributes: attribute_names(derived),
            content: Content::Particles(Particle::collect(derived, scope)),
        }
    }
}

fn derivation<'a, 'input>(content: Node<'a, 'input>) -> Option<Node<'a, 'input>> {
    content.children().find(|child| {
        CONTENT_DERIVATIONS
            .iter()
            .any(|name| child.is(SCHEMA_NAMESPACE, name))
    })
}

fn attribute_names(node: Node) -> Vec<String> {
    node.elements(SCHEMA_NAMESPACE, "attribute")
        .filter_map(|attribute| {
            attribute
                .trimmed("name")
                .map(str::to_string)
                .or_else(|| QName::of_attribute(attribute, "ref").map(|reference| reference.local))
        })
        .collect()
}
