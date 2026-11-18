use super::super::{FormField, FormValue};
use super::form_part::file_name;
use super::literal::{quoted, quoted_entries, quoted_lines, ENTRY_INDENT};
use super::snippet::{Snippet, SnippetBody};

const PREAMBLE: &str = "import requests\n\n";
const NO_JOIN: &str = "";
const READ_BINARY: &str = "\"rb\"";
const ENCODED_DATA: &str = "data=data.encode()";
const RESPONSE_OUTPUT: &str = "\nprint(response.status_code)\nprint(response.text)\n";

impl Snippet {
    pub(super) fn python(&self) -> String {
        let mut code = format!("{PREAMBLE}url = {}\n", quoted(&self.url));
        let mut arguments = vec![quoted(self.method_name()), "url".to_string()];
        let headers = self.library_headers(&[]);
        if !headers.is_empty() {
            code.push_str(&format!(
                "headers = {{\n{}}}\n",
                quoted_entries(&headers, header_value)
            ));
            arguments.push("headers=headers".to_string());
        }
        if let Some((definition, argument)) = self.python_body() {
            code.push_str(&definition);
            arguments.push(argument.to_string());
        }
        code.push_str(&format!(
            "response = requests.request({})\n",
            arguments.join(", ")
        ));
        code.push_str(RESPONSE_OUTPUT);
        code
    }

    fn python_body(&self) -> Option<(String, &'static str)> {
        match &self.body {
            SnippetBody::Empty => None,
            SnippetBody::Text(text) if text.contains('\n') => Some((
                format!(
                    "data = (\n{ENTRY_INDENT}{}\n)\n",
                    quoted_lines(text, NO_JOIN, ENTRY_INDENT)
                ),
                ENCODED_DATA,
            )),
            SnippetBody::Text(text) => Some((format!("data = {}\n", quoted(text)), ENCODED_DATA)),
            SnippetBody::File(path) => Some((
                format!("data = open({}, {READ_BINARY})\n", quoted(path)),
                "data=data",
            )),
            SnippetBody::Form { fields, .. } => {
                let lines: String = fields.iter().map(FormField::python_file_entry).collect();
                Some((format!("files = [\n{lines}]\n"), "files=files"))
            }
        }
    }
}

impl FormField {
    fn python_file_entry(&self) -> String {
        let mut members = match &self.value {
            FormValue::Text { text, .. } => vec!["None".to_string(), quoted(text)],
            FormValue::File { path, .. } => vec![
                quoted(&file_name(path)),
                format!("open({}, {READ_BINARY})", quoted(path)),
            ],
        };
        members.extend(self.part_content_type().map(quoted));
        format!(
            "{ENTRY_INDENT}({}, ({})),\n",
            quoted(&self.name),
            members.join(", ")
        )
    }
}

fn header_value(value: &str) -> String {
    if value.is_ascii() {
        quoted(value)
    } else {
        format!("{}.encode()", quoted(value))
    }
}
