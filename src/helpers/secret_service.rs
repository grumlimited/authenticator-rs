extern crate secret_service;

use log::debug;

use secret_service::{EncryptionType, SsError};
use secret_service::{Item, SecretService};

use crate::helpers::ConfigManager;

pub type Result<T> = ::std::result::Result<T, SsError>;

const APPLICATION: &str = "Authenticator-rs";
const APPLICATION_ATTRS: (&str, &str) = ("application", "authenticator-rs");
const ACCOUNT_ID_KEY: &str = "account_id";

pub trait TotpSecretService {
    fn store(label: &str, account_id: u32, secret: &str) -> Result<()>;

    fn secret(account_id: u32) -> Result<Option<String>>;
}

impl TotpSecretService for ConfigManager {
    fn store(label: &str, account_id: u32, secret: &str) -> Result<()> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = ss.get_default_collection()?;

        collection.create_item(
            format!("{} TOTP ({})", APPLICATION, label).as_str(),
            vec![(ACCOUNT_ID_KEY, &format!("{}", account_id)), APPLICATION_ATTRS],
            secret.as_bytes(),
            false,
            "text/plain",
        )?;

        debug!("Saved {} ({})", label, account_id);

        Ok(())
    }

    fn secret(account_id: u32) -> Result<Option<String>> {
        let ss = SecretService::new(EncryptionType::Dh)?;

        let search_items: Vec<Item> = ss.search_items(vec![(ACCOUNT_ID_KEY, &format!("{}", account_id)), APPLICATION_ATTRS])?;

        search_items
            .get(0)
            .map(|i| i.get_secret())
            .map(|r: Result<Vec<u8>>| r.and_then(|s: Vec<u8>| String::from_utf8(s).map_err(|_| SsError::NoResult)))
            .map(|r| r.map(Some))
            .unwrap_or(Ok(None))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_create_collection_struct() {
        ConfigManager::store("x", 1, "secret");

        let result = ConfigManager::secret(1).unwrap().unwrap();

        assert_eq!("secret", result);
    }
}
