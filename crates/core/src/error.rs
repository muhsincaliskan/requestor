use std::fmt;

#[derive(Debug)]
pub enum CoreError {
    InvalidJson(serde_json::Error),
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::InvalidJson(source) => write!(formatter, "invalid JSON: {source}"),
        }
    }
}

impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CoreError::InvalidJson(source) => Some(source),
        }
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(source: serde_json::Error) -> Self {
        CoreError::InvalidJson(source)
    }
}
