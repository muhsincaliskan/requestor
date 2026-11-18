use std::path::Path;

use requestor_core::identity::random_uuid;
use requestor_core::secret::Secret;
use requestor_core::vault::{VaultSlot, VaultStore};

use super::KeychainVaultStore;

const VALUE: &str = "keychain-check";

#[test]
#[ignore = "touches the real system keychain"]
fn a_value_is_saved_read_back_and_forgotten() {
    let root = std::env::temp_dir().join(format!("requestor-keychain-{}", random_uuid()));
    let slot = VaultSlot::environment(&random_uuid());
    let store = KeychainVaultStore::for_workspace(&root);
    store.save(&slot, Some(&Secret::new(VALUE))).unwrap();
    let fresh = KeychainVaultStore::for_workspace(Path::new(&root));
    assert_eq!(fresh.load(&slot).unwrap(), Some(Secret::new(VALUE)));
    fresh.save(&slot, None).unwrap();
    assert_eq!(
        KeychainVaultStore::for_workspace(&root)
            .load(&slot)
            .unwrap(),
        None
    );
}
