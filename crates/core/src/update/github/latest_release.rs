use serde::Deserialize;

use crate::update::{Release, ReleaseAsset, UpdateError, Version};

const RELEASE_PAGE_PREFIX: &str = "https://github.com/";
const DOWNLOAD_PREFIX: &str = "https://github.com/muhsincaliskan/requestor/releases/download/";
const PARENT_SEGMENT: &str = "..";
const UNSAFE_URL_CHARACTERS: [char; 10] = ['"', '\'', '\\', '<', '>', '`', '^', '|', '{', '}'];

#[derive(Debug, Deserialize)]
pub(super) struct LatestReleaseResponse {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<AssetResponse>,
}

#[derive(Debug, Deserialize)]
struct AssetResponse {
    name: String,
    browser_download_url: String,
}

impl AssetResponse {
    fn to_domain(&self) -> Option<ReleaseAsset> {
        let trusted = self.browser_download_url.starts_with(DOWNLOAD_PREFIX)
            && is_safe_url(&self.browser_download_url)
            && !self.browser_download_url.contains(PARENT_SEGMENT);
        trusted.then(|| ReleaseAsset {
            name: self.name.clone(),
            download_url: self.browser_download_url.clone(),
        })
    }
}

impl LatestReleaseResponse {
    pub(super) fn to_domain(&self) -> Result<Release, UpdateError> {
        if self.draft || self.prerelease {
            return Err(UpdateError::NoStableRelease);
        }
        if !self.is_release_page() {
            return Err(UpdateError::InvalidResponse);
        }
        let version = Version::parse(&self.tag_name).ok_or(UpdateError::InvalidResponse)?;
        Ok(Release {
            version,
            page_url: self.html_url.clone(),
            assets: self
                .assets
                .iter()
                .filter_map(AssetResponse::to_domain)
                .collect(),
        })
    }

    fn is_release_page(&self) -> bool {
        self.html_url.starts_with(RELEASE_PAGE_PREFIX) && is_safe_url(&self.html_url)
    }
}

fn is_safe_url(url: &str) -> bool {
    url.chars().all(|character| {
        character.is_ascii_graphic() && !UNSAFE_URL_CHARACTERS.contains(&character)
    })
}
