use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::{PacConfig, PacEvaluator, PacResolver, PacTarget, ProxyDecision};

const WORK_SETTINGS: &str = "<dictionary> {
  ExceptionsList : <array> {
    0 : *.local
    1 : 169.254/16
    2 : chatgpt.com
  }
  FTPPassive : 1
  HTTPEnable : 0
  HTTPSEnable : 0
  ProxyAutoConfigEnable : 1
  ProxyAutoConfigURLString : http://wpad/wpad.dat
  ProxyAutoDiscoveryEnable : 1
  SOCKSEnable : 0
}";

struct CountingEvaluator {
    result: &'static str,
    calls: AtomicUsize,
}

impl PacEvaluator for CountingEvaluator {
    fn find_proxy(&self, _script_url: &str, _target: &PacTarget) -> Option<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Some(self.result.to_string())
    }
}

fn config() -> PacConfig {
    PacConfig::from_system_settings(WORK_SETTINGS).unwrap()
}

#[test]
fn system_settings_give_the_script_url_and_exceptions() {
    let config = config();
    assert_eq!(config.script_url, "http://wpad/wpad.dat");
    assert_eq!(config.exceptions, ["*.local", "169.254/16", "chatgpt.com"]);
}

#[test]
fn settings_without_auto_configuration_have_no_pac() {
    let manual = "<dictionary> {\n  HTTPEnable : 1\n  ProxyAutoConfigEnable : 0\n}";
    assert_eq!(PacConfig::from_system_settings(manual), None);
    let discovery = "<dictionary> {\n  ProxyAutoDiscoveryEnable : 1\n}";
    assert_eq!(
        PacConfig::from_system_settings(discovery)
            .unwrap()
            .script_url,
        "http://wpad/wpad.dat"
    );
}

#[test]
fn exceptions_match_suffixes_hosts_and_networks() {
    let config = config();
    assert!(config.bypasses("printer.local"));
    assert!(config.bypasses("ChatGPT.com"));
    assert!(config.bypasses("169.254.10.2"));
    assert!(!config.bypasses("169.255.0.1"));
    assert!(!config.bypasses("efes.example.com"));
    assert!(!config.bypasses("notchatgpt.com"));
}

#[test]
fn pac_results_choose_the_first_usable_entry() {
    assert_eq!(
        ProxyDecision::from_pac_result("PROXY proxy.corp:8080; DIRECT"),
        ProxyDecision::Through("http://proxy.corp:8080".to_string())
    );
    assert_eq!(
        ProxyDecision::from_pac_result("SOCKS5 s:1080"),
        ProxyDecision::Through("socks5://s:1080".to_string())
    );
    assert_eq!(
        ProxyDecision::from_pac_result("QUIC q:443; HTTPS h:443"),
        ProxyDecision::Through("https://h:443".to_string())
    );
    assert_eq!(
        ProxyDecision::from_pac_result("DIRECT"),
        ProxyDecision::Direct
    );
    assert_eq!(ProxyDecision::from_pac_result(""), ProxyDecision::Direct);
}

#[test]
fn targets_leave_out_the_path_and_default_ports() {
    assert_eq!(
        PacTarget::new("HTTPS", "Api.Test", Some(443)).script_url(),
        "https://api.test/"
    );
    assert_eq!(
        PacTarget::new("http", "api.test", Some(8080)).script_url(),
        "http://api.test:8080/"
    );
}

#[test]
fn decisions_are_cached_per_target_and_exceptions_skip_the_script() {
    let evaluator = Arc::new(CountingEvaluator {
        result: "PROXY proxy.corp:8080",
        calls: AtomicUsize::new(0),
    });
    let resolver = PacResolver::new(config(), evaluator.clone());
    let target = PacTarget::new("https", "efes.example.com", None);
    for _ in 0..3 {
        assert_eq!(
            resolver.decide(&target).proxy_url(),
            Some("http://proxy.corp:8080")
        );
    }
    let local = PacTarget::new("http", "printer.local", None);
    assert_eq!(resolver.decide(&local), ProxyDecision::Direct);
    assert_eq!(evaluator.calls.load(Ordering::SeqCst), 1);
}
