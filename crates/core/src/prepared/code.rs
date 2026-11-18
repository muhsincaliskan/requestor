mod form_part;
mod http_file;
mod java;
mod javascript;
mod literal;
#[cfg(test)]
mod portability_tests;
mod python;
mod snippet;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use super::exposure::SecretExposure;
use super::PreparedRequest;
use crate::secret::Redactor;
use snippet::Snippet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeLanguage {
    Http,
    #[serde(rename = "javascript")]
    JavaScript,
    Python,
    Java,
    #[default]
    #[serde(other)]
    Curl,
}

impl CodeLanguage {
    pub const ALL: [CodeLanguage; 5] = [
        CodeLanguage::Curl,
        CodeLanguage::Http,
        CodeLanguage::JavaScript,
        CodeLanguage::Python,
        CodeLanguage::Java,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CodeLanguage::Curl => "cURL",
            CodeLanguage::Http => "HTTP",
            CodeLanguage::JavaScript => "JavaScript",
            CodeLanguage::Python => "Python",
            CodeLanguage::Java => "Java",
        }
    }

    pub fn library(self) -> &'static str {
        match self {
            CodeLanguage::Curl => "command line",
            CodeLanguage::Http => ".http file",
            CodeLanguage::JavaScript => "fetch",
            CodeLanguage::Python => "requests",
            CodeLanguage::Java => "HttpClient",
        }
    }
}

impl PreparedRequest {
    pub fn to_code(&self, language: CodeLanguage, redactor: &Redactor) -> String {
        self.code(language, SecretExposure::Masked(redactor))
    }

    pub fn to_code_with_secrets(&self, language: CodeLanguage) -> String {
        self.code(language, SecretExposure::Included)
    }

    fn code(&self, language: CodeLanguage, exposure: SecretExposure) -> String {
        match language {
            CodeLanguage::Curl => self.curl_command(exposure),
            CodeLanguage::Http => Snippet::new(self, exposure).http_file(),
            CodeLanguage::JavaScript => Snippet::new(self, exposure).javascript(),
            CodeLanguage::Python => Snippet::new(self, exposure).python(),
            CodeLanguage::Java => Snippet::new(self, exposure).java(),
        }
    }
}
