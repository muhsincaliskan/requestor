mod check_settings;
mod error;
mod github;
mod release;
mod version;

pub use error::UpdateError;
pub use github::{GithubReleaseClient, LATEST_RELEASE_URL};
pub use release::Release;
pub use version::Version;

pub trait ReleaseClient {
    fn latest_release(&self) -> Result<Release, UpdateError>;
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
