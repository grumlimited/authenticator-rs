extern crate secret_service;

use log::debug;

use crate::helpers::RepositoryError;
use crate::model::{Account, AccountGroup};
use secret_service::{EncryptionType, SsError};
use secret_service::{Item, SecretService};

type Result<T> = ::std::result::Result<T, SsError>;

const APPLICATION: &str = "Authenticator-rs";
const APPLICATION_ATTRS: (&str, &str) = ("application", "authenticator-rs");
const ACCOUNT_ID_KEY: &str = "account_id";

pub struct Keyring;

impl Keyring {
    fn store(ss: &SecretService, label: &str, account_id: u32, secret: &str) -> Result<()> {
        let _ = Self::remove(ss, account_id);

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

    pub fn upsert(label: &str, account_id: u32, secret: &str) -> std::result::Result<(), RepositoryError> {
        let ss = SecretService::new(EncryptionType::Dh)?;

        let result = match Self::secret(account_id) {
            Ok(Some(_)) => {
                Self::remove(&ss, account_id)?;
                Self::store(&ss, label, account_id, secret)
            }
            Ok(None) => Self::store(&ss, label, account_id, secret),
            Err(e) => Err(e),
        };

        result.map_err(RepositoryError::KeyringError)
    }

    pub fn secret(account_id: u32) -> Result<Option<String>> {
        let ss = SecretService::new(EncryptionType::Dh)?;

        let search_items: Vec<Item> = ss.search_items(vec![(ACCOUNT_ID_KEY, &format!("{}", account_id)), APPLICATION_ATTRS])?;

        search_items
            .get(0)
            .map(|i| i.get_secret())
            .map(|r: Result<Vec<u8>>| r.and_then(|s: Vec<u8>| String::from_utf8(s).map_err(|_| SsError::NoResult)))
            .map(|r| r.map(Some))
            .unwrap_or(Ok(None))
    }

    fn remove(ss: &SecretService, account_id: u32) -> Result<()> {
        let search_items: Vec<Item> = ss.search_items(vec![(ACCOUNT_ID_KEY, &format!("{}", account_id)), APPLICATION_ATTRS])?;

        match search_items.get(0) {
            Some(i) => i.delete(),
            None => Err(SsError::NoResult),
        }
    }

    pub fn groups_account_secret(group_accounts: &mut Vec<AccountGroup>) -> std::result::Result<(), RepositoryError> {
        let ss = SecretService::new(EncryptionType::Dh)?;

        let collection = ss.get_default_collection()?;
        let results = collection.search_items(vec![APPLICATION_ATTRS])?;

        let all_secrets: Vec<(String, String)> = results
            .iter()
            .map(|v| {
                let secret = v
                    .get_secret()
                    .map_err(RepositoryError::KeyringError)
                    .and_then(|v| String::from_utf8(v).map_err(RepositoryError::KeyringDecodingError))
                    .ok();

                let account_id = match v.get_attributes() {
                    Ok(attributes) => attributes
                        .into_iter()
                        .filter(|a| a.0 == ACCOUNT_ID_KEY)
                        .map(|t| t.1)
                        .collect::<Vec<String>>()
                        .first()
                        .map(|v| v.clone()),
                    Err(_) => None,
                };

                (account_id, secret)
            })
            .filter(|v| v.0.is_some())
            .filter(|v| v.1.is_some())
            .map(|v| (v.0.unwrap(), v.1.unwrap()))
            .collect::<Vec<(String, String)>>();

        group_accounts.iter_mut().try_for_each(|g| Self::group_account_secret(&ss, g, &all_secrets))
    }

    fn group_account_secret(
        ss: &SecretService,
        group_account: &mut AccountGroup,
        all_secrets: &Vec<(String, String)>,
    ) -> std::result::Result<(), RepositoryError> {
        group_account.entries.iter_mut().try_for_each(|a| Self::account_secret(ss, a, all_secrets))
    }

    fn account_secret(ss: &SecretService, account: &mut Account, all_secrets: &Vec<(String, String)>) -> std::result::Result<(), RepositoryError> {
        debug!("Loading keyring secret for {} ({})", account.label, account.id);

        match all_secrets.into_iter().find(|v| v.0 == format!("{}", account.id)) {
            Some(secret) => account.secret = secret.1.clone(),
            None => Self::store(&ss, account.label.as_str(), account.id, account.secret.as_str())?,
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_create_collection_struct() {
        let ss = SecretService::new(EncryptionType::Dh).unwrap();
        Keyring::store(&ss, "x22", 1, "secret").unwrap();

        let result = Keyring::secret(1).unwrap().unwrap();

        assert_eq!("secret", result);
    }
}
