use std::time::Duration;

use super::*;
use crate::header::ACCEPT;
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::transport::{HttpTransport, TransportError};

struct FixedAnswer {
    status: u16,
    body: &'static str,
}

impl HttpTransport for FixedAnswer {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        assert_eq!(request.url, LATEST_RELEASE_URL);
        assert_eq!(request.header(ACCEPT), Some(github::GITHUB_JSON));
        Ok(HttpResponse {
            status: self.status,
            reason: String::new(),
            version: String::new(),
            headers: Vec::new(),
            body: self.body.as_bytes().to_vec(),
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

fn client(status: u16, body: &'static str) -> GithubReleaseClient<FixedAnswer> {
    GithubReleaseClient::new(FixedAnswer { status, body }, LATEST_RELEASE_URL)
}

fn version(text: &str) -> Version {
    Version::parse(text).unwrap()
}

const RELEASE_0_2_0: &str = r#"{"tag_name":"v0.2.0","html_url":"https://github.com/muhsincaliskan/requestor/releases/tag/v0.2.0","draft":false,"prerelease":false,"assets":[]}"#;

#[test]
fn a_newer_release_is_offered() {
    let found = newer_release(&client(200, RELEASE_0_2_0), &version("0.1.0")).unwrap();
    let release = found.unwrap();
    assert_eq!(release.version, version("0.2.0"));
    assert!(release.page_url.ends_with("/v0.2.0"));
}

#[test]
fn the_same_or_an_older_release_is_not_offered() {
    let latest = client(200, RELEASE_0_2_0);
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
fn a_pre_release_or_draft_is_not_offered() {
    let pre = r#"{"tag_name":"v9.0.0","html_url":"https://github.com/x/y","prerelease":true}"#;
    let result = newer_release(&client(200, pre), &version("0.1.0"));
    assert_eq!(result, Err(UpdateError::NoStableRelease));
}

#[test]
fn a_page_outside_github_is_refused() {
    let odd = r#"{"tag_name":"v9.0.0","html_url":"file:///etc/passwd"}"#;
    let result = newer_release(&client(200, odd), &version("0.1.0"));
    assert_eq!(result, Err(UpdateError::InvalidResponse));
}

#[test]
fn failures_are_reported_without_an_offer() {
    let missing = newer_release(&client(404, "{}"), &version("0.1.0"));
    assert_eq!(missing, Err(UpdateError::Status(404)));
    let garbled = newer_release(&client(200, "<html>"), &version("0.1.0"));
    assert_eq!(garbled, Err(UpdateError::InvalidResponse));
    let offline = GithubReleaseClient::new(Offline, LATEST_RELEASE_URL);
    assert!(matches!(
        newer_release(&offline, &version("0.1.0")),
        Err(UpdateError::Unreachable(_))
    ));
}

#[test]
fn update_checks_verify_tls_follow_redirects_and_time_out() {
    let relaxed = crate::settings::RequestSettings {
        verify_ssl: false,
        follow_redirects: false,
        request_timeout_ms: 0,
        connect_timeout_ms: 0,
        ..Default::default()
    };
    let checked = relaxed.for_update_check();
    assert!(checked.verify_ssl);
    assert!(checked.follow_redirects);
    assert!(checked.request_timeout_ms > 0);
    assert!(checked.connect_timeout_ms > 0);
}

#[test]
fn a_page_with_unsafe_characters_is_refused() {
    let odd = r#"{"tag_name":"v9.0.0","html_url":"https://github.com/x/y\" /evil"}"#;
    let result = newer_release(&client(200, odd), &version("0.1.0"));
    assert_eq!(result, Err(UpdateError::InvalidResponse));
}
