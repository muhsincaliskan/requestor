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
    let package = release.asset(name).ok_or(UpdateError::NoPackage)?;
    let checksums = release
        .asset(CHECKSUMS_ASSET)
        .ok_or(UpdateError::NoChecksums)?;
    let listed = client.download(&checksums.download_url)?;
    let checksums = ChecksumList::parse(&String::from_utf8_lossy(&listed));
    let bytes = client.download(&package.download_url)?;
    if !checksums.verifies(name, &bytes) {
        return Err(UpdateError::ChecksumMismatch);
    }
    Ok(UpdatePackage {
        name: name.to_string(),
        bytes,
    })
}
