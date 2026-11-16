use std::fmt;

const MASK: &str = "***";
const SCHEME_SEPARATOR: &str = "://";
const SENSITIVE_NAME_MARKERS: [&str; 8] = [
    "token",
    "key",
    "secret",
    "password",
    "passwd",
    "auth",
    "sig",
    "credential",
];

pub struct RedactedUrl(String);

impl RedactedUrl {
    pub fn from(url: &str) -> Self {
        let (without_fragment, fragment) = split_keeping(url, '#');
        let (base, query) = split_keeping(without_fragment, '?');
        let mut text = redact_userinfo(base);
        if let Some(query) = query {
            text.push('?');
            text.push_str(&redact_query(query));
        }
        if let Some(fragment) = fragment {
            text.push('#');
            text.push_str(fragment);
        }
        RedactedUrl(text)
    }
}

impl fmt::Display for RedactedUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn split_keeping(text: &str, separator: char) -> (&str, Option<&str>) {
    match text.split_once(separator) {
        Some((head, tail)) => (head, Some(tail)),
        None => (text, None),
    }
}

fn redact_userinfo(base: &str) -> String {
    let Some((scheme, rest)) = base.split_once(SCHEME_SEPARATOR) else {
        return base.to_string();
    };
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let (authority, path) = rest.split_at(authority_end);
    match authority.rsplit_once('@') {
        Some((_, host)) => format!("{scheme}{SCHEME_SEPARATOR}{MASK}@{host}{path}"),
        None => base.to_string(),
    }
}

fn redact_query(query: &str) -> String {
    query
        .split('&')
        .map(redact_pair)
        .collect::<Vec<_>>()
        .join("&")
}

fn redact_pair(pair: &str) -> String {
    match pair.split_once('=') {
        Some((name, _)) if is_sensitive(name) => format!("{name}={MASK}"),
        _ => pair.to_string(),
    }
}

fn is_sensitive(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    SENSITIVE_NAME_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::RedactedUrl;

    fn redact(url: &str) -> String {
        RedactedUrl::from(url).to_string()
    }

    #[test]
    fn masks_sensitive_query_values_only() {
        assert_eq!(
            redact("http://h/p?page=2&api_key=abc&Token=x#top"),
            "http://h/p?page=2&api_key=***&Token=***#top"
        );
    }

    #[test]
    fn masks_userinfo() {
        assert_eq!(redact("https://user:pw@h:8080/a"), "https://***@h:8080/a");
    }

    #[test]
    fn leaves_plain_urls_alone() {
        assert_eq!(redact("http://h/a?b=c"), "http://h/a?b=c");
        assert_eq!(redact("not a url"), "not a url");
    }
}
