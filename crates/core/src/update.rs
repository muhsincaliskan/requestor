mod check_settings;
mod checksums;
mod error;
mod github;
mod package;
mod release;
mod sha256;
mod version;

pub use checksums::{ChecksumList, CHECKSUMS_ASSET};
pub use error::UpdateError;
pub use github::{GithubReleaseClient, LATEST_RELEASE_URL};
pub use package::{download_package, UpdatePackage};
pub use release::{Release, ReleaseAsset};
pub use sha256::Sha256Digest;
pub use version::Version;

pub trait ReleaseClient {
    fn latest_release(&self) -> Result<Release, UpdateError>;
    fn download(&self, url: &str) -> Result<Vec<u8>, UpdateError>;
}

pub fn newer_release(
    client: &impl ReleaseClient,
    current: &Version,
) -> Result<Option<Release>, UpdateError> {
    let latest = client.latest_release()?;
    Ok(latest.is_newer_than(current).then_some(latest))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod package_tests;
