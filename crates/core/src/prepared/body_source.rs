use std::fmt;

use crate::media_type;
use crate::secret::{is_secret_name, REDACTED};
use crate::variables::placeholder_names;

const FORM_FLAG: &str = "-F";
const FORM_STRING_FLAG: &str = "--form-string";
const FILE_MARK: char = '@';
const QUOTED_PATH_MARKS: [char; 3] = [';', ',', '"'];

#[derive(Clone, PartialEq, Default)]
pub enum BodySource {
    #[default]
    Inline,
    File(String),
    Form(Vec<FormField>),
}

#[derive(Clone, PartialEq)]
pub struct FormField {
    pub name: String,
    pub value: FormValue,
}

#[derive(Clone, PartialEq)]
pub enum FormValue {
    Text {
        text: String,
        content_type: Option<String>,
    },
    File {
        path: String,
        content_type: Option<String>,
    },
}

impl fmt::Debug for BodySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BodySource::Inline => formatter.write_str("Inline"),
            BodySource::File(_) => formatter.write_str("File"),
            BodySource::Form(fields) => write!(formatter, "Form({} fields)", fields.len()),
        }
    }
}

impl fmt::Debug for FormField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FormField")
            .field("name", &self.name)
            .field("value", &REDACTED)
            .finish()
    }
}

impl fmt::Debug for FormValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(REDACTED)
    }
}

impl FormField {
    pub fn text(name: impl Into<String>, value: impl Into<String>) -> Self {
        FormField::typed_text(name, value, None)
    }

    pub fn typed_text(
        name: impl Into<String>,
        value: impl Into<String>,
        content_type: Option<&str>,
    ) -> Self {
        FormField {
            name: name.into(),
            value: FormValue::Text {
                text: value.into(),
                content_type: content_type.map(str::to_string),
            },
        }
    }

    pub fn file(
        name: impl Into<String>,
        path: impl Into<String>,
        content_type: Option<&str>,
    ) -> Self {
        FormField {
            name: name.into(),
            value: FormValue::File {
                path: path.into(),
                content_type: content_type.map(str::to_string),
            },
        }
    }

    pub fn part_content_type(&self) -> Option<&str> {
        match &self.value {
            FormValue::Text { content_type, .. } => content_type.as_deref(),
            FormValue::File { content_type, .. } => {
                Some(content_type.as_deref().unwrap_or(media_type::OCTET_STREAM))
            }
        }
    }

    pub fn placeholder_names(&self) -> Vec<String> {
        let mut names = placeholder_names(&self.name);
        match &self.value {
            FormValue::Text { text, .. } => names.extend(placeholder_names(text)),
            FormValue::File { path, .. } => names.extend(placeholder_names(path)),
        }
        names
    }

    pub fn curl_argument(&self, mask_secrets: bool) -> (&'static str, String) {
        match &self.value {
            FormValue::Text { text, .. } => {
                let shown = if mask_secrets && is_secret_name(&self.name) {
                    REDACTED
                } else {
                    text
                };
                (FORM_STRING_FLAG, format!("{}={shown}", self.name))
            }
            FormValue::File { path, content_type } => {
                let mut argument = format!("{}={FILE_MARK}{}", self.name, quoted_path(path));
                if let Some(content_type) = content_type {
                    argument.push_str(&format!(";type={content_type}"));
                }
                (FORM_FLAG, argument)
            }
        }
    }
}

fn quoted_path(path: &str) -> String {
    if path.contains(QUOTED_PATH_MARKS) {
        format!("\"{}\"", path.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        path.to_string()
    }
}
