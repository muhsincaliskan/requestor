use roxmltree::Node;

use super::xml::ElementNode;

const PREFIX_SEPARATOR: char = ':';

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QName {
    pub namespace: String,
    pub local: String,
}

impl QName {
    pub fn new(namespace: impl Into<String>, local: impl Into<String>) -> Self {
        QName {
            namespace: namespace.into(),
            local: local.into(),
        }
    }

    pub fn unqualified(local: impl Into<String>) -> Self {
        QName::new(String::new(), local)
    }

    pub fn resolve(node: Node, value: &str) -> Self {
        let (prefix, local) = match value.split_once(PREFIX_SEPARATOR) {
            Some((prefix, local)) => (Some(prefix), local),
            None => (None, value),
        };
        let namespace = node.lookup_namespace_uri(prefix).unwrap_or_default();
        QName::new(namespace, local)
    }

    pub fn of_attribute(node: Node, attribute: &str) -> Option<Self> {
        node.trimmed(attribute)
            .map(|value| QName::resolve(node, value))
    }

    pub fn is_qualified(&self) -> bool {
        !self.namespace.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::QName;

    #[test]
    fn prefixes_resolve_through_the_declarations_in_scope() {
        let xml =
            r#"<a xmlns="urn:default" xmlns:t="urn:types"><b type="t:Name" plain="Other"/></a>"#;
        let document = roxmltree::Document::parse(xml).unwrap();
        let node = document.root_element().first_element_child().unwrap();
        assert_eq!(
            QName::of_attribute(node, "type"),
            Some(QName::new("urn:types", "Name"))
        );
        assert_eq!(
            QName::of_attribute(node, "plain"),
            Some(QName::new("urn:default", "Other"))
        );
        assert_eq!(QName::of_attribute(node, "missing"), None);
    }
}
