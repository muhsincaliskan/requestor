use std::net::Ipv4Addr;

const PAC_ENABLED_KEY: &str = "ProxyAutoConfigEnable";
const PAC_URL_KEY: &str = "ProxyAutoConfigURLString";
const DISCOVERY_ENABLED_KEY: &str = "ProxyAutoDiscoveryEnable";
const SIMPLE_HOSTS_KEY: &str = "ExcludeSimpleHostnames";
const EXCEPTIONS_KEY: &str = "ExceptionsList";
const ENABLED_VALUE: &str = "1";
const KEY_SEPARATOR: &str = " : ";
const BLOCK_OPEN: char = '{';
const BLOCK_CLOSE: &str = "}";
const TOP_LEVEL: usize = 1;
const DISCOVERY_SCRIPT_URL: &str = "http://wpad/wpad.dat";
const WILDCARD_PREFIX: &str = "*.";
const DOMAIN_SEPARATOR: char = '.';
const PREFIX_SEPARATOR: char = '/';
const IPV4_OCTETS: usize = 4;
const IPV4_BITS: u32 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacConfig {
    pub script_url: String,
    pub exceptions: Vec<String>,
    pub exclude_simple_hosts: bool,
}

#[derive(Default)]
struct SettingsReader {
    enabled: bool,
    discovery: bool,
    exclude_simple_hosts: bool,
    script_url: Option<String>,
    exceptions: Vec<String>,
    depth: usize,
    exceptions_depth: Option<usize>,
}

impl SettingsReader {
    fn read(&mut self, line: &str) {
        if line == BLOCK_CLOSE {
            self.depth = self.depth.saturating_sub(1);
            if self
                .exceptions_depth
                .is_some_and(|depth| depth > self.depth)
            {
                self.exceptions_depth = None;
            }
            return;
        }
        let entry = line.split_once(KEY_SEPARATOR);
        match (entry, self.depth) {
            (Some((_, value)), depth) if self.exceptions_depth == Some(depth) => {
                self.exceptions.push(value.to_string());
            }
            (Some((key, value)), TOP_LEVEL) => self.top_level(key, value),
            _ => {}
        }
        if line.ends_with(BLOCK_OPEN) {
            self.depth += 1;
        }
    }

    fn top_level(&mut self, key: &str, value: &str) {
        let enabled = value == ENABLED_VALUE;
        match key {
            PAC_ENABLED_KEY => self.enabled = enabled,
            DISCOVERY_ENABLED_KEY => self.discovery = enabled,
            SIMPLE_HOSTS_KEY => self.exclude_simple_hosts = enabled,
            PAC_URL_KEY => self.script_url = Some(value.to_string()),
            EXCEPTIONS_KEY => self.exceptions_depth = Some(TOP_LEVEL + 1),
            _ => {}
        }
    }

    fn into_config(self) -> Option<PacConfig> {
        let configured = self.script_url.filter(|_| self.enabled);
        let discovered = self.discovery.then(|| DISCOVERY_SCRIPT_URL.to_string());
        Some(PacConfig {
            script_url: configured.or(discovered)?,
            exceptions: self.exceptions,
            exclude_simple_hosts: self.exclude_simple_hosts,
        })
    }
}

impl PacConfig {
    pub fn from_system_settings(text: &str) -> Option<PacConfig> {
        let mut reader = SettingsReader::default();
        text.lines()
            .map(str::trim)
            .for_each(|line| reader.read(line));
        reader.into_config()
    }

    pub fn bypasses(&self, host: &str) -> bool {
        let host = host.to_ascii_lowercase();
        if self.exclude_simple_hosts && !host.contains(DOMAIN_SEPARATOR) {
            return true;
        }
        self.exceptions
            .iter()
            .any(|exception| exception_matches(&exception.to_ascii_lowercase(), &host))
    }
}

fn exception_matches(exception: &str, host: &str) -> bool {
    if let Some(suffix) = exception.strip_prefix(WILDCARD_PREFIX) {
        return host.ends_with(&format!("{DOMAIN_SEPARATOR}{suffix}")) || host == suffix;
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
        .split(DOMAIN_SEPARATOR)
        .map(str::parse::<u8>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if octets.len() > IPV4_OCTETS {
        return None;
    }
    octets.resize(IPV4_OCTETS, 0);
    let octets: [u8; IPV4_OCTETS] = octets.try_into().ok()?;
    Some(Ipv4Addr::from(octets))
}
