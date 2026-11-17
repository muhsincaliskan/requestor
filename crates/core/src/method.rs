#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Trace,
    Connect,
    Custom(String),
}

impl HttpMethod {
    pub const COMMON: [HttpMethod; 7] = [
        HttpMethod::Get,
        HttpMethod::Post,
        HttpMethod::Put,
        HttpMethod::Patch,
        HttpMethod::Delete,
        HttpMethod::Head,
        HttpMethod::Options,
    ];

    pub fn is_common(name: &str) -> bool {
        HttpMethod::COMMON
            .iter()
            .any(|method| method.as_str() == name)
    }

    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "PATCH" => HttpMethod::Patch,
            "DELETE" => HttpMethod::Delete,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            "TRACE" => HttpMethod::Trace,
            "CONNECT" => HttpMethod::Connect,
            other => HttpMethod::Custom(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Trace => "TRACE",
            HttpMethod::Connect => "CONNECT",
            HttpMethod::Custom(name) => name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HttpMethod;

    #[test]
    fn common_methods_are_recognised_by_exact_name() {
        assert!(HttpMethod::is_common("PATCH"));
        assert!(!HttpMethod::is_common("PURGE"));
        assert!(!HttpMethod::is_common("get"));
    }

    #[test]
    fn parses_known_methods_case_insensitively() {
        assert_eq!(HttpMethod::parse("post"), HttpMethod::Post);
        assert_eq!(HttpMethod::parse(" Delete "), HttpMethod::Delete);
    }

    #[test]
    fn keeps_unknown_methods_as_custom() {
        let method = HttpMethod::parse("purge");
        assert_eq!(method, HttpMethod::Custom("PURGE".to_string()));
        assert_eq!(method.as_str(), "PURGE");
    }

    #[test]
    fn defaults_to_get() {
        assert_eq!(HttpMethod::default().as_str(), "GET");
    }
}
