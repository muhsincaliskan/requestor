use std::io;
use std::process::Command;
use std::thread;

const WAITER_NAME: &str = "requestor-open-url";
const SECURE_URL_PREFIX: &str = "https://";
const NOT_A_WEB_ADDRESS: &str = "only https addresses can be opened";

#[cfg(target_os = "macos")]
const OPENER: (&str, &[&str]) = ("open", &[]);
#[cfg(windows)]
const OPENER: (&str, &[&str]) = ("rundll32", &["url.dll,FileProtocolHandler"]);
#[cfg(not(any(target_os = "macos", windows)))]
const OPENER: (&str, &[&str]) = ("xdg-open", &[]);

pub fn open(url: &str) -> io::Result<()> {
    if !url.starts_with(SECURE_URL_PREFIX) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            NOT_A_WEB_ADDRESS,
        ));
    }
    let (program, arguments) = OPENER;
    let mut child = Command::new(program).args(arguments).arg(url).spawn()?;
    thread::Builder::new()
        .name(WAITER_NAME.to_string())
        .spawn(move || child.wait())?;
    Ok(())
}
