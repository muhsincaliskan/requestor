pub const JSON: &str = "application/json";
pub const XML: &str = "application/xml";
pub const HTML: &str = "text/html";
pub const JAVASCRIPT: &str = "application/javascript";
pub const PLAIN_TEXT: &str = "text/plain";
pub const OCTET_STREAM: &str = "application/octet-stream";
pub const FORM_URLENCODED: &str = "application/x-www-form-urlencoded";
pub const MULTIPART_PREFIX: &str = "multipart/";
pub const MULTIPART_FORM_DATA: &str = "multipart/form-data";

const PARAMETER_SEPARATOR: char = ';';

pub fn segments(text: &str) -> impl Iterator<Item = &str> {
    text.split(PARAMETER_SEPARATOR)
}

pub fn base(content_type: &str) -> &str {
    segments(content_type).next().unwrap_or(content_type).trim()
}

#[cfg(test)]
mod tests {
    use super::base;

    #[test]
    fn base_drops_parameters() {
        assert_eq!(base("application/json; charset=utf-8"), "application/json");
        assert_eq!(base("text/plain"), "text/plain");
    }
}
