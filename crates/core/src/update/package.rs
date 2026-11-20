use super::checksums::{ChecksumList, CHECKSUMS_ASSET};
use super::{Release, ReleaseClient, UpdateError};

#[derive(Clone, PartialEq, Eq)]
pub struct UpdatePackage {
    pub name: String,
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for UpdatePackage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UpdatePackage")
            .field("name", &self.name)
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

pub fn download_package(
    client: &impl ReleaseClient,
    release: &Release,
    name: &str,
) -> Result<UpdatePackage, UpdateError> {
    let package_url = release.download_url(name).ok_or(UpdateError::NoPackage)?;
    let checksums_url = release
        .download_url(CHECKSUMS_ASSET)
        .ok_or(UpdateError::NoChecksums)?;
    let listed = client
        .download(&checksums_url)
        .map_err(|error| error.when_missing(UpdateError::NoChecksums))?;
    let checksums = ChecksumList::parse(&String::from_utf8_lossy(&listed));
    let bytes = client
        .download(&package_url)
        .map_err(|error| error.when_missing(UpdateError::NoPackage))?;
    if !checksums.verifies(name, &bytes) {
        return Err(UpdateError::ChecksumMismatch);
    }
    Ok(UpdatePackage {
        name: name.to_string(),
        bytes,
    })
}
