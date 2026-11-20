use crate::response::{HttpResponse, StatusClass};
use crate::update::release::is_plain_name;
use crate::update::{Release, UpdateError, Version};

const RELEASES_PAGE: &str = "https://github.com/muhsincaliskan/requestor/releases";
const TAG_PAGE_PREFIX: &str = "https://github.com/muhsincaliskan/requestor/releases/tag/";
const DOWNLOAD_PREFIX: &str = "https://github.com/muhsincaliskan/requestor/releases/download/";
const LOCATION: &str = "location";

#[derive(Debug)]
pub(super) struct LatestReleaseRedirect {
    tag: String,
}

impl LatestReleaseRedirect {
    pub(super) fn from_response(response: &HttpResponse) -> Result<Self, UpdateError> {
        if StatusClass::of(response.status) != StatusClass::Redirect {
            return Err(UpdateError::Status(response.status));
        }
        let location = response
            .header(LOCATION)
            .ok_or(UpdateError::InvalidResponse)?;
        if location.trim_end_matches('/') == RELEASES_PAGE {
            return Err(UpdateError::NoStableRelease);
        }
        let tag = location
            .strip_prefix(TAG_PAGE_PREFIX)
            .filter(|tag| is_plain_name(tag))
            .ok_or(UpdateError::InvalidResponse)?;
        Ok(LatestReleaseRedirect {
            tag: tag.to_string(),
        })
    }

    pub(super) fn to_domain(&self) -> Result<Release, UpdateError> {
        let version = Version::parse(&self.tag).ok_or(UpdateError::InvalidResponse)?;
        Ok(Release {
            version,
            page_url: format!("{TAG_PAGE_PREFIX}{}", self.tag),
            download_base: format!("{DOWNLOAD_PREFIX}{}/", self.tag),
        })
    }
}
