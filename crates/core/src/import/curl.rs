use super::shell_words;
use super::{ImportError, ImportedRequest};
use crate::collection::{Auth, Body, Param, Request, Url, UrlParts};
use crate::encoding::{percent_decode, percent_encode};
use crate::media_type;

const SHORT_FLAGS_WITH_VALUE: &str = "XHdFuAbeoTmxUEwcKCrtz";
const LONG_OPTIONS_WITH_VALUE: [&str; 24] = [
    "request",
    "header",
    "data",
    "data-raw",
    "data-binary",
    "data-ascii",
    "data-urlencode",
    "form",
    "user",
    "user-agent",
    "cookie",
    "referer",
    "url",
    "json",
    "output",
    "upload-file",
    "max-time",
    "connect-timeout",
    "proxy",
    "proxy-user",
    "cacert",
    "cert",
    "key",
    "write-out",
];
const DEFAULT_SCHEME: &str = "http://";
const COMMAND_NAME: &str = "curl";
const FILE_MARK: char = '@';

#[derive(Default)]
struct CurlArguments {
    method: Option<String>,
    url: Option<String>,
    headers: Vec<(String, String)>,
    data: Vec<String>,
    url_encoded: Vec<Param>,
    form: Vec<Param>,
    user: Option<String>,
    query_data: bool,
    head: bool,
    json: bool,
    warnings: Vec<String>,
}

impl ImportedRequest {
    pub fn from_curl(command: &str) -> Result<Self, ImportError> {
        let words = shell_words::split(command)?;
        let mut arguments = CurlArguments::default();
        let mut remaining = words
            .into_iter()
            .skip_while(|word| word == COMMAND_NAME)
            .peekable();
        while let Some(word) = remaining.next() {
            arguments.consume(&word, &mut remaining);
        }
        arguments.into_imported()
    }
}

impl CurlArguments {
    fn consume(&mut self, word: &str, remaining: &mut impl Iterator<Item = String>) {
        if let Some(long) = word.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value.to_string())),
                None => (long, None),
            };
            let value = if LONG_OPTIONS_WITH_VALUE.contains(&name) {
                inline.or_else(|| remaining.next())
            } else {
                None
            };
            self.apply(name, value);
        } else if word.len() > 1 && word.starts_with('-') {
            self.consume_short_cluster(&word[1..], remaining);
        } else {
            self.url.get_or_insert_with(|| word.to_string());
        }
    }

    fn consume_short_cluster(
        &mut self,
        cluster: &str,
        remaining: &mut impl Iterator<Item = String>,
    ) {
        for (position, flag) in cluster.char_indices() {
            if SHORT_FLAGS_WITH_VALUE.contains(flag) {
                let rest = &cluster[position + flag.len_utf8()..];
                let value = if rest.is_empty() {
                    remaining.next()
                } else {
                    Some(rest.to_string())
                };
                self.apply(short_to_long(flag), value);
                return;
            }
            self.apply(short_to_long(flag), None);
        }
    }

    fn apply(&mut self, name: &str, value: Option<String>) {
        let value = value.unwrap_or_default();
        match name {
            "request" => self.method = Some(value.to_ascii_uppercase()),
            "header" => self.push_header(&value),
            "data" | "data-raw" | "data-binary" | "data-ascii" => self.data.push(value),
            "data-urlencode" => self.push_url_encoded(&value),
            "form" => self.push_form(&value),
            "user" => self.user = Some(value),
            "user-agent" => self.headers.push(("User-Agent".to_string(), value)),
            "referer" => self.headers.push(("Referer".to_string(), value)),
            "cookie" => self.push_cookie(value),
            "url" => self.url = Some(value),
            "json" => self.push_json(value),
            "get" => self.query_data = true,
            "head" => self.head = true,
            "insecure" => self.warnings.push(
                "--insecure was ignored; turn off SSL verification in the settings instead"
                    .to_string(),
            ),
            "silent" | "show-error" | "location" | "compressed" | "verbose" | "include"
            | "fail" | "globoff" | "next" => {}
            other => self
                .warnings
                .push(format!("unsupported curl option ignored: {other}")),
        }
    }

    fn push_header(&mut self, line: &str) {
        let (name, value) = match line.split_once(':') {
            Some((name, value)) => (name, value),
            None => (line.trim_end_matches(';'), ""),
        };
        self.headers
            .push((name.trim().to_string(), value.trim().to_string()));
    }

    fn push_url_encoded(&mut self, text: &str) {
        let param = match text.split_once('=') {
            Some(("", value)) => Param::text("", value),
            Some((key, value)) => Param::text(key, value),
            None => Param::text("", text),
        };
        self.url_encoded.push(param);
    }

    fn push_form(&mut self, text: &str) {
        let Some((name, value)) = text.split_once('=') else {
            self.warnings
                .push(format!("form field without a value ignored: {text}"));
            return;
        };
        match value.strip_prefix(FILE_MARK) {
            Some(path) => self
                .form
                .push(Param::file(name, path.split(';').next().unwrap_or(path))),
            None => self.form.push(Param::text(name, value)),
        }
    }

    fn push_cookie(&mut self, value: String) {
        if value.contains('=') {
            self.headers.push(("Cookie".to_string(), value));
        } else {
            self.warnings
                .push("cookie files are not supported; the option was ignored".to_string());
        }
    }

    fn push_json(&mut self, value: String) {
        self.json = true;
        self.data.push(value);
    }

    fn into_imported(self) -> Result<ImportedRequest, ImportError> {
        let url_text = self.url.clone().ok_or(ImportError::MissingUrl)?;
        let url_text = if url_text.contains("://") {
            url_text
        } else {
            format!("{DEFAULT_SCHEME}{url_text}")
        };
        let has_body =
            !self.data.is_empty() || !self.url_encoded.is_empty() || !self.form.is_empty();
        let method = self.resolved_method(has_body);
        let url_text = self.url_with_query_data(url_text);
        let mut request = Request::new(&method, Url::Parts(Box::new(UrlParts::parse(&url_text))));
        for (name, value) in &self.headers {
            request.push_header(name.as_str(), value.as_str());
        }
        if self.json {
            self.add_json_headers(&mut request);
        }
        request.body = self.build_body(&request);
        request.auth = self.user.as_deref().map(basic_auth);
        let name = format!("{method} {}", display_path(&url_text));
        Ok(ImportedRequest {
            name,
            request,
            warnings: self.warnings,
        })
    }

    fn resolved_method(&self, has_body: bool) -> String {
        match (&self.method, self.head, has_body && !self.query_data) {
            (Some(method), _, _) => method.clone(),
            (None, true, _) => "HEAD".to_string(),
            (None, false, true) => "POST".to_string(),
            (None, false, false) => "GET".to_string(),
        }
    }

    fn url_with_query_data(&self, url: String) -> String {
        if !self.query_data || self.data.is_empty() {
            return url;
        }
        let separator = if url.contains('?') { '&' } else { '?' };
        format!("{url}{separator}{}", self.data.join("&"))
    }

    fn add_json_headers(&self, request: &mut Request) {
        for (name, value) in [
            ("Content-Type", media_type::JSON),
            ("Accept", media_type::JSON),
        ] {
            if !request.has_header(name) {
                request.push_header(name, value);
            }
        }
    }

    fn build_body(&self, request: &Request) -> Option<Body> {
        if !self.form.is_empty() {
            return Some(Body::formdata(self.form.clone()));
        }
        if !self.url_encoded.is_empty() {
            return Some(Body::urlencoded(
                self.url_encoded.iter().map(encode_pair).collect(),
            ));
        }
        if self.data.is_empty() || self.query_data {
            return None;
        }
        Some(body_from_data(
            &self.data.join("&"),
            request.header_value("content-type"),
        ))
    }
}

fn short_to_long(flag: char) -> &'static str {
    match flag {
        'X' => "request",
        'H' => "header",
        'd' => "data",
        'F' => "form",
        'u' => "user",
        'A' => "user-agent",
        'b' => "cookie",
        'e' => "referer",
        'G' => "get",
        'I' => "head",
        'k' => "insecure",
        's' => "silent",
        'S' => "show-error",
        'L' => "location",
        'v' => "verbose",
        'i' => "include",
        'f' => "fail",
        _ => "other",
    }
}

fn basic_auth(credentials: &str) -> Auth {
    let (username, password) = credentials.split_once(':').unwrap_or((credentials, ""));
    Auth::basic(username, password)
}

fn encode_pair(param: &Param) -> Param {
    let value = param.value.as_deref().unwrap_or_default();
    Param::text(param.key.clone().unwrap_or_default(), percent_encode(value))
}

fn body_from_data(data: &str, content_type: Option<&str>) -> Body {
    let kind = content_type.map(str::to_ascii_lowercase);
    match kind.as_deref() {
        Some(kind) if kind.contains("json") => Body::raw(data, Some("json")),
        Some(kind) if kind.contains("xml") => Body::raw(data, Some("xml")),
        Some(kind) if !kind.contains("x-www-form-urlencoded") => Body::raw(data, None),
        _ => form_pairs(data).map_or_else(|| Body::raw(data, None), Body::urlencoded),
    }
}

fn form_pairs(data: &str) -> Option<Vec<Param>> {
    data.split('&')
        .map(|pair| {
            pair.split_once('=')
                .map(|(key, value)| Param::text(percent_decode(key), percent_decode(value)))
        })
        .collect()
}

fn display_path(url: &str) -> String {
    let without_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    without_scheme
        .split(['?', '#'])
        .next()
        .unwrap_or(without_scheme)
        .to_string()
}

#[cfg(test)]
mod tests {
    use crate::collection::BodyMode;
    use crate::import::{ImportError, ImportedRequest};

    fn import(command: &str) -> ImportedRequest {
        ImportedRequest::from_curl(command).unwrap()
    }

    #[test]
    fn simple_get_defaults_to_http_scheme() {
        let imported = import("curl example.com/items?a=1");
        assert_eq!(imported.request.method.as_deref(), Some("GET"));
        assert_eq!(
            imported.request.url.unwrap().to_text(),
            "http://example.com/items?a=1"
        );
        assert_eq!(imported.name, "GET example.com/items");
    }

    #[test]
    fn json_post_with_headers_and_basic_auth() {
        let command = "curl -X POST 'https://api.test/v1/users' \\\n  -H 'Content-Type: application/json' \\\n  -H \"Accept: */*\" \\\n  -u ada:s3cret \\\n  -d '{\"name\":\"Ada\"}'";
        let imported = import(command);
        let request = imported.request;
        assert_eq!(request.method.as_deref(), Some("POST"));
        assert_eq!(request.header.as_ref().unwrap().len(), 2);
        let body = request.body.unwrap();
        assert_eq!(body.mode, Some(BodyMode::Raw));
        assert_eq!(body.raw.as_deref(), Some("{\"name\":\"Ada\"}"));
        assert_eq!(request.auth.unwrap().value_of("username"), Some("ada"));
    }

    #[test]
    fn data_without_method_posts_and_form_pairs_become_urlencoded() {
        let imported = import("curl https://x.test -d 'a=1&b=hello%20world' -d c=3");
        assert_eq!(imported.request.method.as_deref(), Some("POST"));
        let params = imported.request.body.unwrap().urlencoded.unwrap();
        assert_eq!(params.len(), 3);
        assert_eq!(params[1].value.as_deref(), Some("hello world"));
    }

    #[test]
    fn multipart_forms_distinguish_text_and_files() {
        let imported =
            import("curl -F title=Hi -F 'doc=@/tmp/a.pdf;type=application/pdf' https://x.test/up");
        let params = imported.request.body.unwrap().formdata.unwrap();
        assert_eq!(params[0].value.as_deref(), Some("Hi"));
        assert_eq!(params[1].kind.as_deref(), Some("file"));
    }

    #[test]
    fn get_flag_moves_data_to_the_query_and_head_sets_the_method() {
        let imported = import("curl -G https://x.test/s -d q=rust -d page=2");
        assert_eq!(
            imported.request.url.unwrap().to_text(),
            "https://x.test/s?q=rust&page=2"
        );
        assert_eq!(imported.request.method.as_deref(), Some("GET"));
        assert_eq!(
            import("curl -I https://x.test").request.method.as_deref(),
            Some("HEAD")
        );
    }

    #[test]
    fn clustered_short_flags_and_attached_values() {
        let imported = import("curl -sSLXPUT -H'X-A: 1' https://x.test");
        assert_eq!(imported.request.method.as_deref(), Some("PUT"));
        assert_eq!(imported.request.header_value("x-a"), Some("1"));
    }

    #[test]
    fn json_option_sets_headers_and_body() {
        let imported = import("curl --json '{\"a\":1}' https://x.test");
        assert_eq!(
            imported.request.header_value("content-type"),
            Some("application/json")
        );
        assert_eq!(imported.request.method.as_deref(), Some("POST"));
    }

    #[test]
    fn unsupported_options_are_reported_not_fatal() {
        let imported = import("curl -k --retry 3 --limit-rate 1k https://x.test");
        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("--insecure")));
        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("limit-rate")));
    }

    #[test]
    fn missing_url_is_an_error() {
        assert_eq!(
            ImportedRequest::from_curl("curl -X POST").unwrap_err(),
            ImportError::MissingUrl
        );
    }
}
