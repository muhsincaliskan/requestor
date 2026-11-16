use crate::secret::{is_secret_name, REDACTED};

const FORM_FLAG: &str = "-F";
const FORM_STRING_FLAG: &str = "--form-string";
const FILE_MARK: char = '@';
const QUOTED_PATH_MARKS: [char; 3] = [';', ',', '"'];

#[derive(Debug, Clone, PartialEq, Default)]
pub enum BodySource {
    #[default]
    Inline,
    File(String),
    Form(Vec<FormField>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormField {
    pub name: String,
    pub value: FormValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormValue {
    Text(String),
    File {
        path: String,
        content_type: Option<String>,
    },
}

impl FormField {
    pub fn text(name: impl Into<String>, value: impl Into<String>) -> Self {
        FormField {
            name: name.into(),
            value: FormValue::Text(value.into()),
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

    pub fn curl_argument(&self, mask_secrets: bool) -> (&'static str, String) {
        match &self.value {
            FormValue::Text(text) => {
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
