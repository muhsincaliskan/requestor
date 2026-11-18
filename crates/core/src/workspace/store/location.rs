use std::ffi::OsString;
use std::path::PathBuf;

pub(super) fn platform_root(variable: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    const APPLICATION_DIRECTORY: &str = "Requestor";
    if cfg!(target_os = "macos") {
        let home = PathBuf::from(variable("HOME")?);
        return Some(
            home.join("Library")
                .join("Application Support")
                .join(APPLICATION_DIRECTORY),
        );
    }
    if cfg!(windows) {
        return Some(PathBuf::from(variable("APPDATA")?).join(APPLICATION_DIRECTORY));
    }
    let configuration = variable("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(variable("HOME")?).join(".config")))?;
    Some(configuration.join(APPLICATION_DIRECTORY.to_ascii_lowercase()))
}
