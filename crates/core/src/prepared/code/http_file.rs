use super::form_part::FormPart;
use super::snippet::{Snippet, SnippetBody};

const FILE_MARK: &str = "< ";
const LINE_BREAK: &str = "\n";

impl Snippet {
    pub(super) fn http_file(&self) -> String {
        let mut code = format!("{} {}{LINE_BREAK}", self.method_name(), self.url);
        for header in &self.headers {
            code.push_str(&format!("{}: {}{LINE_BREAK}", header.name, header.value));
        }
        let body = match &self.body {
            SnippetBody::Empty => return code,
            SnippetBody::Text(text) => text.clone(),
            SnippetBody::File(path) => format!("{FILE_MARK}{path}"),
            SnippetBody::Form { .. } => self
                .form_parts(LINE_BREAK)
                .iter()
                .map(FormPart::http_text)
                .collect(),
        };
        code.push_str(LINE_BREAK);
        code.push_str(&body);
        code
    }
}

impl FormPart {
    fn http_text(&self) -> String {
        match self {
            FormPart::Text(text) => text.clone(),
            FormPart::File(path) => format!("{FILE_MARK}{path}"),
        }
    }
}
