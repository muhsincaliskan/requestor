use std::mem;
use std::path::Path;

use super::super::multipart::escape;
use super::super::{FormField, FormValue};

#[derive(Debug, Clone, PartialEq)]
pub(super) enum FormPart {
    Text(String),
    File(String),
}

impl FormPart {
    pub(super) fn layout(fields: &[FormField], boundary: &str, line_break: &str) -> Vec<FormPart> {
        let mut parts = Vec::new();
        let mut pending = String::new();
        for field in fields {
            pending.push_str(&format!(
                "--{boundary}{line_break}Content-Disposition: form-data; {}{line_break}",
                disposition(field)
            ));
            if let Some(content_type) = field.part_content_type() {
                pending.push_str(&format!("Content-Type: {content_type}{line_break}"));
            }
            pending.push_str(line_break);
            match &field.value {
                FormValue::Text { text, .. } => {
                    pending.push_str(text);
                    pending.push_str(line_break);
                }
                FormValue::File { path, .. } => {
                    parts.push(FormPart::Text(mem::take(&mut pending)));
                    parts.push(FormPart::File(path.clone()));
                    pending.push_str(line_break);
                }
            }
        }
        pending.push_str(&format!("--{boundary}--{line_break}"));
        parts.push(FormPart::Text(pending));
        parts
    }
}

pub(super) fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn disposition(field: &FormField) -> String {
    let name = escape(&field.name);
    match &field.value {
        FormValue::Text { .. } => format!("name=\"{name}\""),
        FormValue::File { path, .. } => {
            format!("name=\"{name}\"; filename=\"{}\"", escape(&file_name(path)))
        }
    }
}
