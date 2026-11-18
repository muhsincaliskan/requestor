use super::qname::QName;
use super::xml::escaped;

const INDENT: &str = "  ";
const PREFIX: &str = "ns";
const ATTRIBUTE_PLACEHOLDER: &str = "?";
const TAG_NAME_ENDS: [char; 3] = [' ', '>', '/'];

#[derive(Default)]
pub struct XmlWriter {
    namespaces: Vec<String>,
    out: String,
    pub depth: usize,
}

impl XmlWriter {
    pub fn open(&mut self, name: &QName, attributes: &[String]) {
        let start = self.start_tag(name, attributes);
        self.line(&format!("{start}>"));
        self.depth += 1;
    }

    pub fn close(&mut self, name: &QName) {
        self.depth -= 1;
        let tag = self.tag(name);
        self.line(&format!("</{tag}>"));
    }

    pub fn leaf(&mut self, name: &QName, attributes: &[String], value: &str) {
        let start = self.start_tag(name, attributes);
        let tag = self.tag(name);
        self.line(&format!("{start}>{}</{tag}>", escaped(value)));
    }

    pub fn empty(&mut self, name: &QName, attributes: &[String]) {
        let start = self.start_tag(name, attributes);
        self.line(&format!("{start}/>"));
    }

    fn start_tag(&mut self, name: &QName, attributes: &[String]) -> String {
        let mut start = format!("<{}", self.tag(name));
        for attribute in attributes {
            start.push_str(&format!(" {attribute}=\"{ATTRIBUTE_PLACEHOLDER}\""));
        }
        start
    }

    fn tag(&mut self, name: &QName) -> String {
        if !name.is_qualified() {
            return name.local.clone();
        }
        let index = match self
            .namespaces
            .iter()
            .position(|known| known == &name.namespace)
        {
            Some(index) => index,
            None => {
                self.namespaces.push(name.namespace.clone());
                self.namespaces.len() - 1
            }
        };
        format!("{PREFIX}{}:{}", index + 1, name.local)
    }

    fn line(&mut self, text: &str) {
        self.out.push_str(&INDENT.repeat(self.depth));
        self.out.push_str(text);
        self.out.push('\n');
    }

    pub fn finish(self) -> String {
        let declarations: String = self
            .namespaces
            .iter()
            .enumerate()
            .map(|(index, namespace)| {
                format!(" xmlns:{PREFIX}{}=\"{}\"", index + 1, escaped(namespace))
            })
            .collect();
        let mut out = self.out.trim_end().to_string();
        if let Some(end) = out.find(TAG_NAME_ENDS) {
            out.insert_str(end, &declarations);
        }
        out
    }
}
