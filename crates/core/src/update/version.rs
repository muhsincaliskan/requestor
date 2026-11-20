use std::fmt;

const TAG_PREFIX: char = 'v';
const PART_SEPARATOR: char = '.';
const PRE_RELEASE_MARK: char = '-';
const BUILD_MARK: char = '+';
const PART_COUNT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    stable: bool,
}

impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        let bare = trimmed.strip_prefix(TAG_PREFIX).unwrap_or(trimmed);
        let without_build = bare.split(BUILD_MARK).next()?;
        let (core, stable) = match without_build.split_once(PRE_RELEASE_MARK) {
            Some((core, _)) => (core, false),
            None => (without_build, true),
        };
        let parts: Vec<u64> = core
            .split(PART_SEPARATOR)
            .map(|part| part.parse().ok())
            .collect::<Option<_>>()?;
        let [major, minor, patch] = <[u64; PART_COUNT]>::try_from(parts).ok()?;
        Some(Version {
            major,
            minor,
            patch,
            stable,
        })
    }

    pub fn is_stable(&self) -> bool {
        self.stable
    }

    pub fn current() -> Self {
        Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or(Version {
            major: 0,
            minor: 0,
            patch: 0,
            stable: false,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
