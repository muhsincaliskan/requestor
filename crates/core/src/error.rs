use std::fmt;

#[derive(Debug)]
pub enum CoreError {
    InvalidJson(serde_json::Error),
    MissingUrl,
    UnreadableFile { path: String, reason: String },
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::InvalidJson(source) => write!(formatter, "invalid JSON: {source}"),
            CoreError::MissingUrl => write!(formatter, "the request has no URL"),
            CoreError::UnreadableFile { path, reason } => {
                write!(formatter, "cannot read {path}: {reason}")
            }
        }
    }
}

impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CoreError::InvalidJson(source) => Some(source),
            CoreError::MissingUrl | CoreError::UnreadableFile { .. } => None,
        }
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(source: serde_json::Error) -> Self {
        CoreError::InvalidJson(source)
    }
}
