use std::net::Ipv4Addr;

const PAC_ENABLED_KEY: &str = "ProxyAutoConfigEnable";
const PAC_URL_KEY: &str = "ProxyAutoConfigURLString";
const DISCOVERY_ENABLED_KEY: &str = "ProxyAutoDiscoveryEnable";
const EXCEPTIONS_KEY: &str = "ExceptionsList";
const ENABLED_VALUE: &str = "1";
const KEY_SEPARATOR: &str = " : ";
const ARRAY_END: &str = "}";
const DISCOVERY_SCRIPT_URL: &str = "http://wpad/wpad.dat";
const WILDCARD_PREFIX: &str = "*.";
const PREFIX_SEPARATOR: char = '/';
const OCTET_SEPARATOR: char = '.';
const IPV4_BITS: u32 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacConfig {
    pub script_url: String,
    pub exceptions: Vec<String>,
}

impl PacConfig {
    pub fn from_system_settings(text: &str) -> Option<PacConfig> {
        let mut enabled = false;
        let mut discovery = false;
        let mut script_url = None;
        let mut exceptions = Vec::new();
        let mut in_exceptions = false;
        for line in text.lines().map(str::trim) {
            if in_exceptions {
                in_exceptions = line != ARRAY_END;
                exceptions.extend(entry_value(line).map(str::to_string));
                continue;
            }
            match line.split_once(KEY_SEPARATOR) {
                Some((PAC_ENABLED_KEY, value)) => enabled = value == ENABLED_VALUE,
                Some((DISCOVERY_ENABLED_KEY, value)) => discovery = value == ENABLED_VALUE,
                Some((PAC_URL_KEY, value)) => script_url = Some(value.to_string()),
                Some((EXCEPTIONS_KEY, _)) => in_exceptions = true,
                _ => {}
            }
        }
        let script_url = match (enabled, discovery) {
            (true, _) => script_url?,
            (false, true) => DISCOVERY_SCRIPT_URL.to_string(),
            (false, false) => return None,
        };
        Some(PacConfig {
            script_url,
            exceptions,
        })
    }

    pub fn bypasses(&self, host: &str) -> bool {
        let host = host.to_ascii_lowercase();
        self.exceptions
            .iter()
            .any(|exception| exception_matches(&exception.to_ascii_lowercase(), &host))
    }
}

fn entry_value(line: &str) -> Option<&str> {
    line.split_once(KEY_SEPARATOR).map(|(_, value)| value)
}

fn exception_matches(exception: &str, host: &str) -> bool {
    if let Some(suffix) = exception.strip_prefix(WILDCARD_PREFIX) {
        return host.ends_with(&format!(".{suffix}")) || host == suffix;
    }
    match exception.split_once(PREFIX_SEPARATOR) {
        Some((network, bits)) => in_network(host, network, bits),
        None => host == exception,
    }
}

fn in_network(host: &str, network: &str, bits: &str) -> bool {
    let (Ok(address), Ok(bits)) = (host.parse::<Ipv4Addr>(), bits.parse::<u32>()) else {
        return false;
    };
    let Some(network) = padded_ipv4(network) else {
        return false;
    };
    let mask = u32::MAX
        .checked_shl(IPV4_BITS.saturating_sub(bits))
        .unwrap_or(0);
    u32::from(address) & mask == u32::from(network) & mask
}

fn padded_ipv4(partial: &str) -> Option<Ipv4Addr> {
    let mut octets = partial
        .split(OCTET_SEPARATOR)
        .map(str::parse::<u8>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    octets.resize(4, 0);
    Some(Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]))
}
