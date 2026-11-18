use std::io;
use std::process::Command;
use std::thread;

const WAITER_NAME: &str = "requestor-open-url";

#[cfg(target_os = "macos")]
const OPENER: (&str, &[&str]) = ("open", &[]);
#[cfg(windows)]
const OPENER: (&str, &[&str]) = ("rundll32", &["url.dll,FileProtocolHandler"]);
#[cfg(not(any(target_os = "macos", windows)))]
const OPENER: (&str, &[&str]) = ("xdg-open", &[]);

pub fn open(url: &str) -> io::Result<()> {
    let (program, arguments) = OPENER;
    let mut child = Command::new(program).args(arguments).arg(url).spawn()?;
    thread::Builder::new()
        .name(WAITER_NAME.to_string())
        .spawn(move || child.wait())?;
    Ok(())
}
