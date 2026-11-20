use keyring::{Entry, Error};
use requestor_core::secret::Secret;

use super::SERVICE;

const ACCOUNT: &str = "postman-api-key";

pub struct PostmanKeychain;

impl PostmanKeychain {
    pub fn load() -> Result<Option<Secret>, String> {
        match Self::entry()?.get_password() {
            Ok(key) => Ok(Some(Secret::new(key))),
            Err(Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn store(key: Option<&Secret>) -> Result<(), String> {
        let entry = Self::entry()?;
        let stored = match key {
            Some(key) => entry.set_password(key.expose()),
            None => match entry.delete_credential() {
                Err(Error::NoEntry) => Ok(()),
                other => other,
            },
        };
        stored.map_err(|error| error.to_string())
    }

    fn entry() -> Result<Entry, String> {
        Entry::new(SERVICE, ACCOUNT).map_err(|error| error.to_string())
    }
}
