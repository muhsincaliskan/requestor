use super::sha256::Sha256Digest;

pub const CHECKSUMS_ASSET: &str = "SHA256SUMS";
const BINARY_MARK: char = '*';
const HEX_DIGEST_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecksumList {
    entries: Vec<(String, String)>,
}

impl ChecksumList {
    pub fn parse(text: &str) -> ChecksumList {
        let entries = text
            .lines()
            .filter_map(|line| {
                let (digest, name) = line.trim().split_once(char::is_whitespace)?;
                let digest = digest.to_ascii_lowercase();
                let valid = digest.len() == HEX_DIGEST_LENGTH
                    && digest
                        .chars()
                        .all(|character| character.is_ascii_hexdigit());
                let name = name.trim_start().trim_start_matches(BINARY_MARK);
                valid.then(|| (name.to_string(), digest))
            })
            .collect();
        ChecksumList { entries }
    }

    pub fn verifies(&self, name: &str, data: &[u8]) -> bool {
        self.entries
            .iter()
            .find(|(listed, _)| listed == name)
            .is_some_and(|(_, digest)| *digest == Sha256Digest::of(data).to_hex())
    }
}
