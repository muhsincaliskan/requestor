pub const JSON: &str = "application/json";
pub const XML: &str = "application/xml";
pub const SOAP_1_1: &str = "text/xml; charset=utf-8";
pub const SOAP_1_2: &str = "application/soap+xml; charset=utf-8";
pub const HTML: &str = "text/html";
pub const JAVASCRIPT: &str = "application/javascript";
pub const PLAIN_TEXT: &str = "text/plain";
pub const OCTET_STREAM: &str = "application/octet-stream";
pub const FORM_URLENCODED: &str = "application/x-www-form-urlencoded";
pub const MULTIPART_PREFIX: &str = "multipart/";
pub const MULTIPART_FORM_DATA: &str = "multipart/form-data";
pub const BOUNDARY_PARAMETER: &str = "boundary";

const PARAMETER_SEPARATOR: char = ';';
const VALUE_SEPARATOR: char = '=';
const VALUE_QUOTE: char = '"';

pub fn segments(text: &str) -> impl Iterator<Item = &str> {
    text.split(PARAMETER_SEPARATOR)
}

pub fn base(content_type: &str) -> &str {
    segments(content_type).next().unwrap_or(content_type).trim()
}

pub fn parameter<'a>(content_type: &'a str, name: &str) -> Option<&'a str> {
    segments(content_type).skip(1).find_map(|segment| {
        let (key, value) = segment.split_once(VALUE_SEPARATOR)?;
        key.trim()
            .eq_ignore_ascii_case(name)
            .then(|| value.trim().trim_matches(VALUE_QUOTE))
    })
}

#[cfg(test)]
mod tests {
    use super::{base, parameter};

    #[test]
    fn base_drops_parameters() {
        assert_eq!(base("application/json; charset=utf-8"), "application/json");
        assert_eq!(base("text/plain"), "text/plain");
    }

    #[test]
    fn parameter_finds_a_named_value() {
        let content_type = "multipart/form-data; Boundary=\"abc\"; charset=utf-8";
        assert_eq!(parameter(content_type, "boundary"), Some("abc"));
        assert_eq!(parameter(content_type, "charset"), Some("utf-8"));
        assert_eq!(parameter("text/plain", "charset"), None);
    }
}
