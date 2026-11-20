#[cfg(target_os = "macos")]
mod macos;

use std::sync::Arc;

const DEFAULT_PORTS: [(&str, u16); 2] = [("http", 80), ("https", 443)];

pub trait PacEvaluator: Send + Sync {
    fn find_proxy(&self, script_url: &str, target: &PacTarget) -> Option<String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PacTarget {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
}

impl PacTarget {
    pub fn new(scheme: &str, host: &str, port: Option<u16>) -> Self {
        let scheme = scheme.to_ascii_lowercase();
        let port = port.filter(|port| !DEFAULT_PORTS.contains(&(scheme.as_str(), *port)));
        PacTarget {
            scheme,
            host: host.to_ascii_lowercase(),
            port,
        }
    }

    pub fn script_url(&self) -> String {
        match self.port {
            Some(port) => format!("{}://{}:{port}/", self.scheme, self.host),
            None => format!("{}://{}/", self.scheme, self.host),
        }
    }
}

#[cfg(target_os = "macos")]
pub(super) fn system() -> Option<Arc<macos::SystemPacEvaluator>> {
    Some(Arc::new(macos::SystemPacEvaluator))
}

#[cfg(not(target_os = "macos"))]
pub(super) fn system() -> Option<Arc<UnsupportedSystem>> {
    None
}

#[cfg(not(target_os = "macos"))]
pub(super) struct UnsupportedSystem;

#[cfg(not(target_os = "macos"))]
impl UnsupportedSystem {
    pub(super) fn config(&self) -> Option<super::PacConfig> {
        None
    }
}

#[cfg(not(target_os = "macos"))]
impl PacEvaluator for UnsupportedSystem {
    fn find_proxy(&self, _script_url: &str, _target: &PacTarget) -> Option<String> {
        None
    }
}
