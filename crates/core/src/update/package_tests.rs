use std::collections::HashMap;
use std::time::Duration;

use super::*;
use crate::header::{Header, LOCATION};
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::transport::{HttpTransport, TransportError};

const DOWNLOADS: &str = "https://github.com/muhsincaliskan/requestor/releases/download/v9.0.0/";
const RELEASE_PAGE: &str = "https://github.com/muhsincaliskan/requestor/releases/tag/v9.0.0";
const PACKAGE: &str = "Requestor-macos.zip";
const PACKAGE_BYTES: &[u8] = b"abc";
const ABC_DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

struct Files(HashMap<String, Vec<u8>>);

impl HttpTransport for Files {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        let (status, headers, body) = if request.url == LATEST_RELEASE_URL {
            (302, vec![Header::new(LOCATION, RELEASE_PAGE)], Vec::new())
        } else {
            match self.0.get(&request.url) {
                Some(body) => (200, Vec::new(), body.clone()),
                None => (404, Vec::new(), Vec::new()),
            }
        };
        Ok(HttpResponse {
            status,
            reason: String::new(),
            version: String::new(),
            headers,
            body,
            truncated: false,
            elapsed: Duration::ZERO,
            timing: None,
        })
    }
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
    let unlisted = client(None);
    let release = unlisted.latest_release().unwrap();
    assert_eq!(
        download_package(&unlisted, &release, PACKAGE),
        Err(UpdateError::Status(404))
    );
    let listed = client(Some(&format!("{ABC_DIGEST}  {PACKAGE}\n")));
    assert_eq!(
        download_package(&listed, &release, "Requestor-linux.tar.gz"),
        Err(UpdateError::Status(404))
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
    let settings = RequestSettings::default();
    let checker = GithubReleaseClient::new(
        ReqwestTransport::new(&settings.for_update_check()).unwrap(),
        LATEST_RELEASE_URL,
    );
    let release = checker.latest_release().unwrap();
    let downloader = GithubReleaseClient::new(
        ReqwestTransport::new(&settings.for_update_download()).unwrap(),
        LATEST_RELEASE_URL,
    );
    let package = download_package(&downloader, &release, PACKAGE).unwrap();
    assert!(package.bytes.len() > 1_000_000);
}
