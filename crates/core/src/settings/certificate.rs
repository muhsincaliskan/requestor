use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::BYTES_PER_MEGABYTE;

const EVERY_HOST: &str = "*";
const SUBDOMAIN_WILDCARD: &str = "*.";
const PORT_SEPARATOR: char = ':';
const MAX_PEM_BYTES: usize = BYTES_PER_MEGABYTE;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientCertificate {
    pub host: String,
    pub certificate_path: PathBuf,
    pub key_path: Option<PathBuf>,
}

impl ClientCertificate {
    pub fn for_every_host(certificate_path: PathBuf, key_path: Option<PathBuf>) -> Self {
        ClientCertificate {
            host: EVERY_HOST.to_string(),
            certificate_path,
            key_path,
        }
    }

    pub fn certificate_path_text(&self) -> String {
        self.certificate_path.to_string_lossy().into_owned()
    }

    pub fn key_path_text(&self) -> String {
        self.key_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn set_certificate_path_text(&mut self, text: &str) {
        self.certificate_path = PathBuf::from(text.trim());
    }

    pub fn set_key_path_text(&mut self, text: &str) {
        let text = text.trim();
        self.key_path = (!text.is_empty()).then(|| PathBuf::from(text));
    }

    pub fn is_complete(&self) -> bool {
        !self.host.trim().is_empty() && !self.certificate_path.as_os_str().is_empty()
    }

    pub fn matches(&self, host: &str, port: u16) -> bool {
        let pattern = self.host.trim();
        if pattern == EVERY_HOST {
            return true;
        }
        let (pattern_host, pattern_port) = split_port(pattern);
        pattern_port.is_none_or(|expected| expected == port)
            && host_matches(pattern_host, host.trim_end_matches('.'))
    }

    pub fn pem(&self) -> Result<Vec<u8>, String> {
        let mut pem = read_pem(&self.certificate_path)?;
        if let Some(key_path) = self
            .key_path
            .as_ref()
            .filter(|path| !path.as_os_str().is_empty())
        {
            pem.push(b'\n');
            pem.extend(read_pem(key_path)?);
        }
        Ok(pem)
    }
}

fn split_port(pattern: &str) -> (&str, Option<u16>) {
    match pattern.rsplit_once(PORT_SEPARATOR) {
        Some((host, port)) => match port.parse() {
            Ok(port) => (host, Some(port)),
            Err(_) => (pattern, None),
        },
        None => (pattern, None),
    }
}

fn host_matches(pattern: &str, host: &str) -> bool {
    match pattern.strip_prefix(SUBDOMAIN_WILDCARD) {
        Some(domain) => {
            let suffix = format!(".{}", domain.to_ascii_lowercase());
            host.len() > suffix.len() && host.to_ascii_lowercase().ends_with(&suffix)
        }
        None => pattern.eq_ignore_ascii_case(host),
    }
}

fn read_pem(path: &Path) -> Result<Vec<u8>, String> {
    let unreadable = |reason: String| format!("cannot read {}: {reason}", path.display());
    let file = File::open(path).map_err(|error| unreadable(error.to_string()))?;
    let mut bytes = Vec::new();
    file.take(MAX_PEM_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| unreadable(error.to_string()))?;
    if bytes.len() > MAX_PEM_BYTES {
        return Err(unreadable(
            "the file is too large for a certificate".to_string(),
        ));
    }
    Ok(bytes)
}
