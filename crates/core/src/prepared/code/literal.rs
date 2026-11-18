use crate::header::Header;

pub(super) const ENTRY_INDENT: &str = "    ";
const LINE_END: char = '\n';

pub(super) fn quoted(text: &str) -> String {
    serde_json::Value::from(text).to_string()
}

pub(super) fn quoted_lines(text: &str, separator: &str, indent: &str) -> String {
    if text.is_empty() {
        return quoted(text);
    }
    text.split_inclusive(LINE_END)
        .map(quoted)
        .collect::<Vec<_>>()
        .join(&format!("{separator}\n{indent}"))
}

pub(super) fn quoted_entries(headers: &[Header], value_literal: fn(&str) -> String) -> String {
    headers
        .iter()
        .map(|header| {
            format!(
                "{ENTRY_INDENT}{}: {},\n",
                quoted(&header.name),
                value_literal(&header.value)
            )
        })
        .collect()
}
