use std::time::Duration;

use super::*;
use crate::header::{Header, LOCATION};
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::transport::{HttpTransport, TransportError};

struct FixedAnswer {
    status: u16,
    location: Option<&'static str>,
}

impl HttpTransport for FixedAnswer {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        assert_eq!(request.url, LATEST_RELEASE_URL);
        let headers = self
            .location
            .map(|location| vec![Header::new(LOCATION, location)])
            .unwrap_or_default();
        Ok(HttpResponse {
            status: self.status,
            reason: String::new(),
            version: String::new(),
            headers,
            body: Vec::new(),
            truncated: false,
            elapsed: Duration::ZERO,
            timing: None,
        })
    }
}

struct Offline;

impl HttpTransport for Offline {
    fn send(&self, _: PreparedRequest) -> Result<HttpResponse, TransportError> {
        Err(TransportError::ConnectionFailed("offline".to_string()))
    }
}

fn client(status: u16, location: Option<&'static str>) -> GithubReleaseClient<FixedAnswer> {
    GithubReleaseClient::new(FixedAnswer { status, location }, LATEST_RELEASE_URL)
}

fn redirect_to(location: &'static str) -> GithubReleaseClient<FixedAnswer> {
    client(302, Some(location))
}

fn version(text: &str) -> Version {
    Version::parse(text).unwrap()
}

const RELEASE_0_2_0: &str = "https://github.com/muhsincaliskan/requestor/releases/tag/v0.2.0";

#[test]
fn a_newer_release_is_offered() {
    let found = newer_release(&redirect_to(RELEASE_0_2_0), &version("0.1.0")).unwrap();
    let release = found.unwrap();
    assert_eq!(release.version, version("0.2.0"));
    assert_eq!(release.page_url, RELEASE_0_2_0);
    assert_eq!(
        release.download_url("SHA256SUMS"),
        "https://github.com/muhsincaliskan/requestor/releases/download/v0.2.0/SHA256SUMS"
    );
}

#[test]
fn the_same_or_an_older_release_is_not_offered() {
    let latest = redirect_to(RELEASE_0_2_0);
    assert_eq!(newer_release(&latest, &version("0.2.0")).unwrap(), None);
    assert_eq!(newer_release(&latest, &version("1.0.0")).unwrap(), None);
}

#[test]
fn versions_compare_by_number_not_text() {
    assert!(version("0.10.0") > version("0.9.3"));
    assert_eq!(version("v1.2.3"), version("1.2.3"));
    assert!(version("1.2.3-beta.1") < version("1.2.3"));
    assert!(version("1.2.3-beta.1") > version("1.2.2"));
    assert_eq!(version("1.2.3+build.7"), version("1.2.3"));
    assert_eq!(version("1.2.3").to_string(), "1.2.3");
    assert_eq!(Version::parse("1.2"), None);
    assert_eq!(Version::parse("latest"), None);
    assert_eq!(Version::current(), version(env!("CARGO_PKG_VERSION")));
}

#[test]
fn a_pre_release_is_not_offered() {
    let pre = "https://github.com/muhsincaliskan/requestor/releases/tag/v9.0.0-beta.1";
    let result = newer_release(&redirect_to(pre), &version("0.1.0"));
    assert_eq!(result, Err(UpdateError::NoStableRelease));
}

#[test]
fn a_redirect_outside_the_project_releases_is_refused() {
    for odd in [
        "file:///etc/passwd",
        "https://github.com/someone/else/releases/tag/v9.0.0",
        "https://github.com/muhsincaliskan/requestor/releases",
        "https://github.com/muhsincaliskan/requestor/releases/tag/",
    ] {
        let result = newer_release(&redirect_to(odd), &version("0.1.0"));
        assert_eq!(result, Err(UpdateError::InvalidResponse), "{odd}");
    }
}

#[test]
fn failures_are_reported_without_an_offer() {
    let missing = newer_release(&client(404, None), &version("0.1.0"));
    assert_eq!(missing, Err(UpdateError::Status(404)));
    let followed = newer_release(&client(200, None), &version("0.1.0"));
    assert_eq!(followed, Err(UpdateError::Status(200)));
    let headless = newer_release(&client(302, None), &version("0.1.0"));
    assert_eq!(headless, Err(UpdateError::InvalidResponse));
    let offline = GithubReleaseClient::new(Offline, LATEST_RELEASE_URL);
    assert!(matches!(
        newer_release(&offline, &version("0.1.0")),
        Err(UpdateError::Unreachable(_))
    ));
}

#[test]
fn update_checks_read_the_redirect_and_downloads_follow_it() {
    let relaxed = crate::settings::RequestSettings {
        verify_ssl: false,
        follow_redirects: false,
        request_timeout_ms: 0,
        connect_timeout_ms: 0,
        ..Default::default()
    };
    let checked = relaxed.for_update_check();
    assert!(checked.verify_ssl);
    assert!(!checked.follow_redirects);
    assert!(checked.request_timeout_ms > 0);
    assert!(checked.connect_timeout_ms > 0);
    let downloading = relaxed.for_update_download();
    assert!(downloading.verify_ssl);
    assert!(downloading.follow_redirects);
}

#[test]
fn a_tag_with_unsafe_characters_is_refused() {
    for odd in [
        "https://github.com/muhsincaliskan/requestor/releases/tag/v9.0.0/../../evil",
        "https://github.com/muhsincaliskan/requestor/releases/tag/v9.0.0\" /evil",
        "https://github.com/muhsincaliskan/requestor/releases/tag/v9.0.0?x=1",
    ] {
        let result = newer_release(&redirect_to(odd), &version("0.1.0"));
        assert_eq!(result, Err(UpdateError::InvalidResponse), "{odd}");
    }
}
