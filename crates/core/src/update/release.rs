use super::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub page_url: String,
    pub downloads_url: String,
}

impl Release {
    pub fn is_newer_than(&self, current: &Version) -> bool {
        self.version > *current
    }

    pub fn download_url(&self, asset: &str) -> String {
        format!("{}{asset}", self.downloads_url)
    }
}
