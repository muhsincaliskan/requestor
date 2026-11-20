use super::Version;

const NAME_SYMBOLS: [char; 4] = ['.', '_', '-', '+'];
const PARENT_SEGMENT: &str = "..";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub page_url: String,
    pub download_base: String,
}

impl Release {
    pub fn is_newer_than(&self, current: &Version) -> bool {
        self.version > *current
    }

    pub fn download_url(&self, name: &str) -> Option<String> {
        is_plain_name(name).then(|| format!("{}{name}", self.download_base))
    }
}

pub(super) fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(PARENT_SEGMENT)
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || NAME_SYMBOLS.contains(&character))
}
