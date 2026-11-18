use roxmltree::Node;

pub const WSDL_NAMESPACE: &str = "http://schemas.xmlsoap.org/wsdl/";
pub const WSDL_2_NAMESPACE: &str = "http://www.w3.org/ns/wsdl";
pub const SCHEMA_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";
pub const SOAP_1_1_BINDING_NAMESPACE: &str = "http://schemas.xmlsoap.org/wsdl/soap/";
pub const SOAP_1_2_BINDING_NAMESPACE: &str = "http://schemas.xmlsoap.org/wsdl/soap12/";

const ESCAPED_CHARACTERS: [(char, &str); 4] = [
    ('&', "&amp;"),
    ('<', "&lt;"),
    ('>', "&gt;"),
    ('"', "&quot;"),
];

pub trait ElementNode<'a, 'input: 'a> {
    fn is(&self, namespace: &str, name: &str) -> bool;
    fn elements(
        self,
        namespace: &'static str,
        name: &'static str,
    ) -> impl Iterator<Item = Node<'a, 'input>>;
    fn element(self, namespace: &'static str, name: &'static str) -> Option<Node<'a, 'input>>;
    fn trimmed(&self, attribute: &str) -> Option<&'a str>;
}

impl<'a, 'input: 'a> ElementNode<'a, 'input> for Node<'a, 'input> {
    fn is(&self, namespace: &str, name: &str) -> bool {
        self.is_element()
            && self.tag_name().name() == name
            && self.tag_name().namespace() == Some(namespace)
    }

    fn elements(
        self,
        namespace: &'static str,
        name: &'static str,
    ) -> impl Iterator<Item = Node<'a, 'input>> {
        self.children()
            .filter(move |child| child.is(namespace, name))
    }

    fn element(self, namespace: &'static str, name: &'static str) -> Option<Node<'a, 'input>> {
        self.elements(namespace, name).next()
    }

    fn trimmed(&self, attribute: &str) -> Option<&'a str> {
        self.attribute(attribute)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }
}

pub fn escaped(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match ESCAPED_CHARACTERS
            .iter()
            .find(|(special, _)| *special == character)
        {
            Some((_, replacement)) => escaped.push_str(replacement),
            None => escaped.push(character),
        }
    }
    escaped
}
