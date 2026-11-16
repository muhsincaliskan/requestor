use super::{options, CurlArguments};
use crate::collection::Param;

const FILE_MARK: char = '@';
const STDIN_MARK: &str = "-";
const TYPE_PREFIX: &str = "type=";
const USER_AGENT: &str = "User-Agent";
const REFERER: &str = "Referer";
const COOKIE: &str = "Cookie";
const INSECURE_WARNING: &str =
    "--insecure was ignored; turn off SSL verification in the settings instead";
const COOKIE_FILE_WARNING: &str = "cookie files are not supported; the option was ignored";
const STDIN_WARNING: &str =
    "reading data from standard input is not supported; the option was ignored";

impl CurlArguments {
    pub(super) fn consume(&mut self, word: &str, remaining: &mut impl Iterator<Item = String>) {
        if let Some(long) = word.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value.to_string())),
                None => (long, None),
            };
            let value = if options::long_takes_value(name) {
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
            let name = options::short_to_long(flag);
            if options::short_takes_value(flag) {
                let rest = &cluster[position + flag.len_utf8()..];
                let value = if rest.is_empty() {
                    remaining.next()
                } else {
                    Some(rest.to_string())
                };
                self.apply(&name, value);
                return;
            }
            self.apply(&name, None);
        }
    }

    fn apply(&mut self, name: &str, value: Option<String>) {
        let value = value.unwrap_or_default();
        match name {
            "request" => self.method = Some(value.to_ascii_uppercase()),
            "header" => self.push_header(&value),
            "data" | "data-ascii" | "data-binary" => self.push_data(value),
            "data-raw" => self.data.push(value),
            "data-urlencode" => self.push_url_encoded(&value),
            "form" => self.push_form(&value, true),
            "form-string" => self.push_form(&value, false),
            "user" => self.user = Some(value),
            "user-agent" => self.headers.push((USER_AGENT.to_string(), value)),
            "referer" => self.headers.push((REFERER.to_string(), value)),
            "cookie" => self.push_cookie(value),
            "url" => self.url = Some(value),
            "json" => self.push_json(value),
            "get" => self.query_data = true,
            "head" => self.head = true,
            "insecure" => self.warnings.push(INSECURE_WARNING.to_string()),
            "silent" | "show-error" | "location" | "compressed" | "verbose" | "include"
            | "fail" | "globoff" | "next" => {}
            other => self.warnings.push(format!(
                "unsupported curl option ignored: {}",
                options::display_flag(other)
            )),
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

    fn push_data(&mut self, value: String) {
        match value.strip_prefix(FILE_MARK) {
            Some(path) => self.set_data_file(path),
            None => self.data.push(value),
        }
    }

    fn set_data_file(&mut self, path: &str) {
        if path == STDIN_MARK {
            self.warnings.push(STDIN_WARNING.to_string());
        } else if self.data_file.is_some() {
            self.warnings.push(format!(
                "only one file body can be imported; the file {path} was ignored"
            ));
        } else {
            self.data_file = Some(path.to_string());
        }
    }

    fn push_url_encoded(&mut self, text: &str) {
        match text.find(['=', FILE_MARK]) {
            Some(position) if text[position..].starts_with(FILE_MARK) => {
                self.warnings.push(format!(
                    "--data-urlencode content from the file {} is not supported and was ignored",
                    &text[position + 1..]
                ));
            }
            Some(position) => self
                .url_encoded
                .push(Param::text(&text[..position], &text[position + 1..])),
            None => self.url_encoded.push(Param::text("", text)),
        }
    }

    fn push_form(&mut self, text: &str, allow_files: bool) {
        let Some((name, value)) = text.split_once('=') else {
            self.warnings
                .push(format!("form field without a value ignored: {text}"));
            return;
        };
        match value.strip_prefix(FILE_MARK).filter(|_| allow_files) {
            Some(spec) => {
                let mut segments = spec.split(';');
                let path = segments.next().unwrap_or(spec);
                let content_type = segments.find_map(|segment| segment.strip_prefix(TYPE_PREFIX));
                self.form
                    .push(Param::file_with_type(name, path, content_type));
            }
            None => self.form.push(Param::text(name, value)),
        }
    }

    fn push_cookie(&mut self, value: String) {
        if value.contains('=') {
            self.headers.push((COOKIE.to_string(), value));
        } else {
            self.warnings.push(COOKIE_FILE_WARNING.to_string());
        }
    }

    fn push_json(&mut self, value: String) {
        self.json = true;
        match value.strip_prefix(FILE_MARK) {
            Some(path) => self.set_data_file(path),
            None => self.json_data.push(value),
        }
    }
}
