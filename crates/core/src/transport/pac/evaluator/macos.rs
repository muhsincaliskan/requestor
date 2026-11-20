use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::{PacEvaluator, PacTarget};
use crate::transport::pac::PacConfig;

const SETTINGS_COMMAND: &str = "/usr/sbin/scutil";
const SETTINGS_ARGUMENT: &str = "--proxy";
const SCRIPT_COMMAND: &str = "/usr/bin/osascript";
const SCRIPT_LANGUAGE: [&str; 2] = ["-l", "JavaScript"];
const SCRIPT_FLAG: &str = "-e";
const RUNNER: &str = include_str!("runner.js");
const TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

pub struct SystemPacEvaluator;

impl SystemPacEvaluator {
    pub fn config(&self) -> Option<PacConfig> {
        let output = Command::new(SETTINGS_COMMAND)
            .arg(SETTINGS_ARGUMENT)
            .output()
            .ok()?;
        PacConfig::from_system_settings(&String::from_utf8_lossy(&output.stdout))
    }
}

impl PacEvaluator for SystemPacEvaluator {
    fn find_proxy(&self, script_url: &str, target: &PacTarget) -> Option<String> {
        let child = Command::new(SCRIPT_COMMAND)
            .args(SCRIPT_LANGUAGE)
            .args([SCRIPT_FLAG, RUNNER])
            .args([script_url, &target.script_url(), &target.host])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        finished_output(child)
    }
}

fn finished_output(mut child: Child) -> Option<String> {
    let started = Instant::now();
    while child.try_wait().ok()?.is_none() {
        if started.elapsed() > TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    let mut output = String::new();
    child.stdout.take()?.read_to_string(&mut output).ok()?;
    Some(output.trim().to_string())
}
