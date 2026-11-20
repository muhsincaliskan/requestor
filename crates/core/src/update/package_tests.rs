use std::collections::HashMap;
use std::time::Duration;

use super::github::GITHUB_JSON;
use super::*;
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::transport::{HttpTransport, TransportError};

const DOWNLOADS: &str = "https://github.com/muhsincaliskan/requestor/releases/download/v9.0.0/";
const PACKAGE: &str = "Requestor-macos.zip";
const PACKAGE_BYTES: &[u8] = b"abc";
const ABC_DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

struct Files(HashMap<String, Vec<u8>>);

impl HttpTransport for Files {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        let body = if request.url == LATEST_RELEASE_URL {
            assert_eq!(request.header("Accept"), Some(GITHUB_JSON));
            release_json().into_bytes()
        } else {
            self.0.get(&request.url).cloned().unwrap_or_default()
        };
        Ok(HttpResponse {
            status: if body.is_empty() { 404 } else { 200 },
            reason: String::new(),
            version: String::new(),
            headers: Vec::new(),
            body,
            truncated: false,
            elapsed: Duration::ZERO,
            timing: None,
        })
    }
}

fn release_json() -> String {
    format!(
        r#"{{"tag_name":"v9.0.0","html_url":"https://github.com/muhsincaliskan/requestor/releases/tag/v9.0.0","assets":[
            {{"name":"{PACKAGE}","browser_download_url":"{DOWNLOADS}{PACKAGE}"}},
            {{"name":"SHA256SUMS","browser_download_url":"{DOWNLOADS}SHA256SUMS"}},
            {{"name":"elsewhere.zip","browser_download_url":"https://example.com/elsewhere.zip"}},
            {{"name":"escaped.zip","browser_download_url":"{DOWNLOADS}../../../../other/repo/escaped.zip"}}
        ]}}"#
    )
}

fn client(checksums: Option<&str>) -> GithubReleaseClient<Files> {
    let mut files = HashMap::new();
    files.insert(format!("{DOWNLOADS}{PACKAGE}"), PACKAGE_BYTES.to_vec());
    if let Some(checksums) = checksums {
        files.insert(
            format!("{DOWNLOADS}SHA256SUMS"),
            checksums.as_bytes().to_vec(),
        );
    }
    GithubReleaseClient::new(Files(files), LATEST_RELEASE_URL)
}

#[test]
fn sha256_matches_the_published_test_vectors() {
    assert_eq!(
        Sha256Digest::of(b"").to_hex(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(Sha256Digest::of(b"abc").to_hex(), ABC_DIGEST);
    assert_eq!(
        Sha256Digest::of(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq").to_hex(),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    for (length, digest) in [
        (
            55,
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
        ),
        (
            56,
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
        ),
        (
            64,
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
        ),
        (
            119,
            "31eba51c313a5c08226adf18d4a359cfdfd8d2e816b13f4af952f7ea6584dcfb",
        ),
    ] {
        assert_eq!(Sha256Digest::of(&vec![b'a'; length]).to_hex(), digest);
    }
    assert_eq!(
        Sha256Digest::of(&[b'a'; 1_000]).to_hex(),
        "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
    );
}

#[test]
fn only_release_downloads_from_the_project_are_kept() {
    let release = client(None).latest_release().unwrap();
    let names: Vec<&str> = release
        .assets
        .iter()
        .map(|asset| asset.name.as_str())
        .collect();
    assert_eq!(names, [PACKAGE, CHECKSUMS_ASSET]);
}

#[test]
fn a_package_is_returned_only_when_its_checksum_matches() {
    let listed = format!("{ABC_DIGEST}  {PACKAGE}\n");
    let good = client(Some(&listed));
    let release = good.latest_release().unwrap();
    let package = download_package(&good, &release, PACKAGE).unwrap();
    assert_eq!(package.bytes, PACKAGE_BYTES);
    let wrong = format!("{}  {PACKAGE}\n", "0".repeat(64));
    let bad = client(Some(&wrong));
    assert_eq!(
        download_package(&bad, &release, PACKAGE),
        Err(UpdateError::ChecksumMismatch)
    );
    let unlisted = client(Some(&format!("{ABC_DIGEST}  other.zip\n")));
    assert_eq!(
        download_package(&unlisted, &release, PACKAGE),
        Err(UpdateError::ChecksumMismatch)
    );
}

#[test]
fn missing_packages_or_checksum_lists_are_refused() {
    let client = client(None);
    let release = client.latest_release().unwrap();
    assert_eq!(
        download_package(&client, &release, "Requestor-linux.tar.gz"),
        Err(UpdateError::NoPackage)
    );
    let mut without_list = release.clone();
    without_list
        .assets
        .retain(|asset| asset.name != CHECKSUMS_ASSET);
    assert_eq!(
        download_package(&client, &without_list, PACKAGE),
        Err(UpdateError::NoChecksums)
    );
}

#[test]
fn checksum_lists_accept_binary_marks_and_ignore_junk() {
    let list = ChecksumList::parse(&format!(
        "not a line\n{}  *{PACKAGE}\nzz  bad\n",
        ABC_DIGEST.to_uppercase()
    ));
    assert!(list.verifies(PACKAGE, PACKAGE_BYTES));
    assert!(!list.verifies(PACKAGE, b"abd"));
}

#[test]
#[ignore = "downloads the latest release from GitHub"]
fn the_published_release_package_verifies() {
    use crate::settings::RequestSettings;
    use crate::transport::ReqwestTransport;
    let settings = RequestSettings::default().for_update_download();
    let client = GithubReleaseClient::new(
        ReqwestTransport::new(&settings).unwrap(),
        LATEST_RELEASE_URL,
    );
    let release = client.latest_release().unwrap();
    let package = download_package(&client, &release, PACKAGE).unwrap();
    assert!(package.bytes.len() > 1_000_000);
}
