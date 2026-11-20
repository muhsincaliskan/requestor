#[cfg(target_os = "macos")]
mod macos;

use requestor_core::update::UpdatePackage;

pub const UNSUPPORTED: &str = "automatic updates are not available on this platform yet";

#[cfg(target_os = "macos")]
pub const PACKAGE_NAME: Option<&str> = Some(macos::PACKAGE_NAME);
#[cfg(not(target_os = "macos"))]
pub const PACKAGE_NAME: Option<&str> = None;

pub fn can_install() -> bool {
    #[cfg(target_os = "macos")]
    return macos::AppBundle::current().is_some_and(|bundle| bundle.is_replaceable());
    #[cfg(not(target_os = "macos"))]
    return false;
}

pub fn install(package: &UpdatePackage) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return macos::AppBundle::current()
        .ok_or_else(|| UNSUPPORTED.to_string())?
        .replace_with(package)
        .map_err(|error| error.to_string());
    #[cfg(not(target_os = "macos"))]
    {
        let _ = package;
        Err(UNSUPPORTED.to_string())
    }
}

pub fn relaunch() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return macos::AppBundle::current()
        .ok_or_else(|| UNSUPPORTED.to_string())?
        .relaunch_after_exit()
        .map_err(|error| error.to_string());
    #[cfg(not(target_os = "macos"))]
    Err(UNSUPPORTED.to_string())
}
