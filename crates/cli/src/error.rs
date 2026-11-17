use std::fmt;
use std::io;

pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_REQUEST_FAILED: u8 = 1;
pub const EXIT_USAGE: u8 = 2;

#[derive(Debug, PartialEq, Eq)]
pub enum CliError {
    Usage(String),
    Input(String),
    OutputClosed,
}

impl CliError {
    pub fn usage(message: impl Into<String>) -> Self {
        CliError::Usage(message.into())
    }

    pub fn input(message: impl Into<String>) -> Self {
        CliError::Input(message.into())
    }

    pub fn from_write(error: &io::Error) -> Self {
        if error.kind() == io::ErrorKind::BrokenPipe {
            CliError::OutputClosed
        } else {
            CliError::input(format!("cannot write output: {error}"))
        }
    }

    pub fn is_usage(&self) -> bool {
        matches!(self, CliError::Usage(_))
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            CliError::OutputClosed => EXIT_SUCCESS,
            CliError::Usage(_) | CliError::Input(_) => EXIT_USAGE,
        }
    }

    pub fn is_silent(&self) -> bool {
        *self == CliError::OutputClosed
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Usage(message) | CliError::Input(message) => write!(formatter, "{message}"),
            CliError::OutputClosed => write!(formatter, "output closed"),
        }
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_pipes_are_silent_successes() {
        let closed = CliError::from_write(&io::Error::from(io::ErrorKind::BrokenPipe));
        assert!(closed.is_silent());
        assert_eq!(closed.exit_code(), EXIT_SUCCESS);
    }

    #[test]
    fn other_write_failures_are_reported() {
        let failed = CliError::from_write(&io::Error::other("disk full"));
        assert!(!failed.is_silent());
        assert_eq!(failed.exit_code(), EXIT_USAGE);
    }
}
