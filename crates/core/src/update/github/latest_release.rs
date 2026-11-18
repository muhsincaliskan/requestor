use serde::Deserialize;

use crate::update::{Release, UpdateError, Version};

const RELEASE_PAGE_PREFIX: &str = "https://github.com/";

#[derive(Debug, Deserialize)]
pub(super) struct LatestReleaseResponse {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

impl LatestReleaseResponse {
    pub(super) fn to_domain(&self) -> Result<Release, UpdateError> {
        if self.draft || self.prerelease {
            return Err(UpdateError::NoStableRelease);
        }
        if !self.html_url.starts_with(RELEASE_PAGE_PREFIX) {
            return Err(UpdateError::InvalidResponse);
        }
        let version = Version::parse(&self.tag_name).ok_or(UpdateError::InvalidResponse)?;
        Ok(Release {
            version,
            page_url: self.html_url.clone(),
        })
    }
}
