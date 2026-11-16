use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

const UUID_VERSION_4_MASK: u64 = 0xffff_ffff_ffff_0fff;
const UUID_VERSION_4_BITS: u64 = 0x0000_0000_0000_4000;
const UUID_VARIANT_MASK: u64 = 0x3fff_ffff_ffff_ffff;
const UUID_VARIANT_BITS: u64 = 0x8000_0000_0000_0000;

pub fn random_u64() -> u64 {
    RandomState::new().build_hasher().finish()
}

pub fn random_uuid() -> String {
    let high = random_u64() & UUID_VERSION_4_MASK | UUID_VERSION_4_BITS;
    let low = random_u64() & UUID_VARIANT_MASK | UUID_VARIANT_BITS;
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        high >> 32,
        high >> 16 & 0xffff,
        high & 0xffff,
        low >> 48,
        low & 0xffff_ffff_ffff
    )
}

#[cfg(test)]
mod tests {
    use super::random_uuid;

    #[test]
    fn uuid_has_version_four_shape() {
        let uuid = random_uuid();
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(
            parts.iter().map(|part| part.len()).collect::<Vec<_>>(),
            [8, 4, 4, 4, 12]
        );
        assert!(parts[2].starts_with('4'));
        assert!(matches!(
            parts[3].chars().next(),
            Some('8' | '9' | 'a' | 'b')
        ));
    }

    #[test]
    fn uuids_differ_between_calls() {
        assert_ne!(random_uuid(), random_uuid());
    }
}
