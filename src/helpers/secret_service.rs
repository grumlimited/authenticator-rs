extern crate secret_service;

use log::debug;

use secret_service::{EncryptionType, SsError};
use secret_service::{Item, SecretService};

pub type Result<T> = ::std::result::Result<T, SsError>;

const APPLICATION: &str = "Authenticator-rs";
const APPLICATION_ATTRS: (&str, &str) = ("application", "authenticator-rs");
const ACCOUNT_ID_KEY: &str = "account_id";

pub struct TotpSecretService;

impl TotpSecretService {
    fn store(label: &str, account_id: u32, secret: &str) -> Result<()> {
        Self::remove(account_id).unwrap();

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

    fn upsert(label: &str, account_id: u32, secret: &str) -> Result<()> {
        match Self::secret(account_id) {
            Ok(Some(_)) => {
                Self::remove(account_id)?;
                Self::store(label, account_id, secret)
            }
            Ok(None) => Self::store(label, account_id, secret),
            Err(e) => Err(e)
        }
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

    fn remove(account_id: u32) -> Result<()> {
        let ss = SecretService::new(EncryptionType::Dh)?;

        let search_items: Vec<Item> = ss.search_items(vec![(ACCOUNT_ID_KEY, &format!("{}", account_id)), APPLICATION_ATTRS])?;

        match search_items.get(0) {
            Some(i) => i.delete(),
            None => Err(SsError::NoResult),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_create_collection_struct() {
        TotpSecretService::store("x22", 1, "secret").unwrap();

        let result = TotpSecretService::secret(1).unwrap().unwrap();

        assert_eq!("secret", result);
    }
}
