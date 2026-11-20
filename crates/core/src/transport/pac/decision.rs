const ENTRY_SEPARATOR: char = ';';
const DIRECT: &str = "DIRECT";
const HTTP_KINDS: [&str; 2] = ["PROXY", "HTTP"];
const HTTPS_KIND: &str = "HTTPS";
const SOCKS_KINDS: [&str; 2] = ["SOCKS", "SOCKS5"];
const HTTP_SCHEME: &str = "http";
const HTTPS_SCHEME: &str = "https";
const SOCKS_SCHEME: &str = "socks5";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyDecision {
    Direct,
    Through(String),
}

impl ProxyDecision {
    pub fn from_pac_result(result: &str) -> ProxyDecision {
        result
            .split(ENTRY_SEPARATOR)
            .find_map(ProxyDecision::from_entry)
            .unwrap_or(ProxyDecision::Direct)
    }

    fn from_entry(entry: &str) -> Option<ProxyDecision> {
        let mut words = entry.split_whitespace();
        let kind = words.next()?.to_ascii_uppercase();
        if kind == DIRECT {
            return Some(ProxyDecision::Direct);
        }
        let address = words.next()?;
        let scheme = if HTTP_KINDS.contains(&kind.as_str()) {
            HTTP_SCHEME
        } else if kind == HTTPS_KIND {
            HTTPS_SCHEME
        } else if SOCKS_KINDS.contains(&kind.as_str()) {
            SOCKS_SCHEME
        } else {
            return None;
        };
        Some(ProxyDecision::Through(format!("{scheme}://{address}")))
    }

    pub fn proxy_url(&self) -> Option<&str> {
        match self {
            ProxyDecision::Direct => None,
            ProxyDecision::Through(url) => Some(url),
        }
    }
}
