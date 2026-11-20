use crate::update::{Release, UpdateError, Version};

const TAG_PAGE_PREFIX: &str = "https://github.com/muhsincaliskan/requestor/releases/tag/";
const DOWNLOAD_PREFIX: &str = "https://github.com/muhsincaliskan/requestor/releases/download/";
const DOWNLOAD_SEPARATOR: char = '/';
const TAG_PUNCTUATION: [char; 4] = ['.', '-', '+', '_'];

pub(super) struct LatestReleaseRedirect<'a> {
    location: Option<&'a str>,
}

impl<'a> LatestReleaseRedirect<'a> {
    pub(super) fn new(location: Option<&'a str>) -> Self {
        LatestReleaseRedirect { location }
    }

    pub(super) fn to_domain(&self) -> Result<Release, UpdateError> {
        let location = self.location.ok_or(UpdateError::InvalidResponse)?;
        let tag = location
            .strip_prefix(TAG_PAGE_PREFIX)
            .filter(|tag| is_safe_tag(tag))
            .ok_or(UpdateError::InvalidResponse)?;
        let version = Version::parse(tag).ok_or(UpdateError::InvalidResponse)?;
        if !version.is_stable() {
            return Err(UpdateError::NoStableRelease);
        }
        Ok(Release {
            version,
            page_url: location.to_string(),
            downloads_url: format!("{DOWNLOAD_PREFIX}{tag}{DOWNLOAD_SEPARATOR}"),
        })
    }
}

fn is_safe_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag.chars().all(|character| {
            character.is_ascii_alphanumeric() || TAG_PUNCTUATION.contains(&character)
        })
}
