use std::time::Duration;

use super::*;
use crate::header::Header;
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::transport::{HttpTransport, TransportError};

struct LatestPage {
    status: u16,
    location: &'static str,
}

impl HttpTransport for LatestPage {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        let latest = request.url == LATEST_RELEASE_URL;
        let (status, headers) = if latest {
            (self.status, vec![Header::new(LOCATION, self.location)])
        } else {
            (NOT_FOUND, Vec::new())
        };
        Ok(HttpResponse {
            status,
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

fn client(status: u16, location: &'static str) -> GithubReleaseClient<LatestPage> {
    GithubReleaseClient::new(LatestPage { status, location }, LATEST_RELEASE_URL)
}

fn version(text: &str) -> Version {
    Version::parse(text).unwrap()
}

const LOCATION: &str = "Location";
const FOUND: u16 = 302;
const NOT_FOUND: u16 = 404;
const RELEASE_0_2_0: &str = "https://github.com/muhsincaliskan/requestor/releases/tag/v0.2.0";

#[test]
fn a_newer_release_is_offered() {
    let found = newer_release(&client(FOUND, RELEASE_0_2_0), &version("0.1.0")).unwrap();
    let release = found.unwrap();
    assert_eq!(release.version, version("0.2.0"));
    assert!(release.page_url.ends_with("/v0.2.0"));
}

#[test]
fn the_same_or_an_older_release_is_not_offered() {
    let latest = client(FOUND, RELEASE_0_2_0);
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
fn no_published_release_is_reported() {
    let none = client(
        FOUND,
        "https://github.com/muhsincaliskan/requestor/releases",
    );
    let result = newer_release(&none, &version("0.1.0"));
    assert_eq!(result, Err(UpdateError::NoStableRelease));
}

#[test]
fn a_redirect_outside_the_project_releases_is_refused() {
    for odd in [
        "file:///etc/passwd",
        "https://github.com/other/repo/releases/tag/v9.0.0",
        "https://github.com/muhsincaliskan/requestor/releases/tag/../v9.0.0",
        "https://github.com/muhsincaliskan/requestor/releases/tag/v9.0.0\" /evil",
    ] {
        let result = newer_release(&client(FOUND, odd), &version("0.1.0"));
        assert_eq!(result, Err(UpdateError::InvalidResponse), "{odd}");
    }
}

#[test]
fn failures_are_reported_without_an_offer() {
    let missing = newer_release(&client(NOT_FOUND, ""), &version("0.1.0"));
    assert_eq!(missing, Err(UpdateError::Status(NOT_FOUND)));
    let not_a_tag = newer_release(
        &client(
            FOUND,
            "https://github.com/muhsincaliskan/requestor/releases/tag/latest",
        ),
        &version("0.1.0"),
    );
    assert_eq!(not_a_tag, Err(UpdateError::InvalidResponse));
    let offline = GithubReleaseClient::new(Offline, LATEST_RELEASE_URL);
    assert!(matches!(
        newer_release(&offline, &version("0.1.0")),
        Err(UpdateError::Unreachable(_))
    ));
}

#[test]
fn update_checks_verify_tls_read_redirects_and_time_out() {
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
    assert!(relaxed.for_update_download().follow_redirects);
    assert!(checked.request_timeout_ms > 0);
    assert!(checked.connect_timeout_ms > 0);
}
