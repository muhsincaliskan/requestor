use std::fmt;

pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_REQUEST_FAILED: u8 = 1;
pub const EXIT_USAGE: u8 = 2;

#[derive(Debug, PartialEq, Eq)]
pub enum CliError {
    Usage(String),
    Input(String),
}

impl CliError {
    pub fn usage(message: impl Into<String>) -> Self {
        CliError::Usage(message.into())
    }

    pub fn input(message: impl Into<String>) -> Self {
        CliError::Input(message.into())
    }

    pub fn is_usage(&self) -> bool {
        matches!(self, CliError::Usage(_))
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Usage(message) | CliError::Input(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for CliError {}
