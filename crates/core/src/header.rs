#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

impl Header {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Header {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn is_named(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
    }
}

pub fn find_header<'a>(headers: &'a [Header], name: &str) -> Option<&'a Header> {
    headers.iter().find(|header| header.is_named(name))
}
