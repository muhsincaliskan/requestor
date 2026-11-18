use std::collections::HashMap;

use roxmltree::Node;

use super::particle::{ElementDecl, Particle, SchemaScope, TypeDef, QUALIFIED};
use super::qname::QName;
use super::xml::{ElementNode, SCHEMA_NAMESPACE};

#[derive(Debug, Default)]
pub struct Schema {
    elements: HashMap<QName, ElementDecl>,
    types: HashMap<QName, TypeDef>,
    groups: HashMap<QName, Vec<Particle>>,
}

impl Schema {
    pub fn add(&mut self, schema: Node, inherited_namespace: Option<&str>) {
        let declared = schema.trimmed("targetNamespace");
        let target = declared.or(inherited_namespace).unwrap_or_default();
        let scope = SchemaScope {
            target_namespace: target.to_string(),
            chameleon: declared.is_none() && inherited_namespace.is_some(),
            qualified_elements: schema.attribute("elementFormDefault") == Some(QUALIFIED),
        };
        for child in schema.children().filter(Node::is_element) {
            self.add_declaration(child, &scope);
        }
    }

    fn add_declaration(&mut self, node: Node, scope: &SchemaScope) {
        if node.tag_name().namespace() != Some(SCHEMA_NAMESPACE) {
            return;
        }
        let Some(name) = node.trimmed("name") else {
            return;
        };
        let qualified = QName::new(scope.target_namespace.clone(), name);
        match node.tag_name().name() {
            "element" => {
                self.elements
                    .insert(qualified, ElementDecl::global(node, scope));
            }
            "complexType" | "simpleType" => {
                self.types
                    .insert(qualified, TypeDef::from_node(node, scope));
            }
            "group" => {
                self.groups
                    .insert(qualified, Particle::collect(node, scope));
            }
            _ => {}
        }
    }

    pub fn element(&self, name: &QName) -> Option<&ElementDecl> {
        self.elements.get(name)
    }

    pub fn type_def(&self, name: &QName) -> Option<&TypeDef> {
        self.types.get(name)
    }

    pub fn group(&self, name: &QName) -> Option<&[Particle]> {
        self.groups.get(name).map(Vec::as_slice)
    }

    pub fn references<'a>(schema: Node<'a, '_>) -> impl Iterator<Item = SchemaReference> + 'a {
        schema
            .children()
            .filter(|child| {
                child.is(SCHEMA_NAMESPACE, "import") || child.is(SCHEMA_NAMESPACE, "include")
            })
            .filter_map(|child| {
                Some(SchemaReference {
                    location: child.trimmed("schemaLocation")?.to_string(),
                    included: child.tag_name().name() == "include",
                })
            })
    }
}

pub struct SchemaReference {
    pub location: String,
    pub included: bool,
}
