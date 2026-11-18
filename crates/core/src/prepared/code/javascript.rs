use super::super::{FormField, FormValue};
use super::form_part::file_name;
use super::literal::{quoted, quoted_entries, quoted_lines, ENTRY_INDENT};
use super::snippet::{Snippet, SnippetBody};
use crate::method::HttpMethod;

const FILE_IMPORT: &str = "import { openAsBlob } from \"node:fs\";\n\n";
const FORM_START: &str = "const form = new FormData();\n";
const BODY_LEFT_OUT: &str =
    "// fetch cannot send a body with GET or HEAD, so the body is left out.\n";
const UNSUPPORTED_HEADERS: [&str; 7] = [
    "Connection",
    "Content-Length",
    "Expect",
    "Host",
    "Keep-Alive",
    "Transfer-Encoding",
    "Upgrade",
];
const LINE_JOIN: &str = " +";
const RESPONSE_OUTPUT: &str =
    "});\n\nconsole.log(response.status);\nconsole.log(await response.text());\n";

impl Snippet {
    pub(super) fn javascript(&self) -> String {
        let sends_body = self.fetch_sends_body();
        let mut code = String::new();
        if sends_body && self.has_files() {
            code.push_str(FILE_IMPORT);
        }
        match &self.body {
            SnippetBody::Empty => {}
            _ if !sends_body => code.push_str(BODY_LEFT_OUT),
            SnippetBody::Form { fields, .. } => {
                code.push_str(FORM_START);
                fields
                    .iter()
                    .for_each(|field| code.push_str(&field.form_data_line()));
                code.push('\n');
            }
            SnippetBody::Text(_) | SnippetBody::File(_) => {}
        }
        code.push_str(&format!(
            "const response = await fetch({}, {{\n  method: {},\n",
            quoted(&self.url),
            quoted(self.method_name())
        ));
        code.push_str(&self.javascript_headers());
        if sends_body {
            code.push_str(&self.javascript_body());
        }
        code.push_str(RESPONSE_OUTPUT);
        code
    }

    fn fetch_sends_body(&self) -> bool {
        !matches!(self.method, HttpMethod::Get | HttpMethod::Head)
    }

    fn javascript_headers(&self) -> String {
        let headers = self.library_headers(&UNSUPPORTED_HEADERS);
        if headers.is_empty() {
            return String::new();
        }
        format!("  headers: {{\n{}  }},\n", quoted_entries(&headers))
    }

    fn javascript_body(&self) -> String {
        match &self.body {
            SnippetBody::Empty => String::new(),
            SnippetBody::Text(text) => {
                format!("  body: {},\n", quoted_lines(text, LINE_JOIN, ENTRY_INDENT))
            }
            SnippetBody::File(path) => format!("  body: await openAsBlob({}),\n", quoted(path)),
            SnippetBody::Form { .. } => "  body: form,\n".to_string(),
        }
    }
}

impl FormField {
    fn form_data_line(&self) -> String {
        let name = quoted(&self.name);
        match &self.value {
            FormValue::Text { text, .. } => format!("form.append({name}, {});\n", quoted(text)),
            FormValue::File { path, .. } => {
                let options = self
                    .part_content_type()
                    .map(|kind| format!(", {{ type: {} }}", quoted(kind)))
                    .unwrap_or_default();
                format!(
                    "form.append({name}, await openAsBlob({}{options}), {});\n",
                    quoted(path),
                    quoted(&file_name(path))
                )
            }
        }
    }
}
