extern crate secret_service;

use std::collections::HashMap;

use log::{debug, warn};
use rusqlite::Connection;
use secret_service::SecretService;
use secret_service::{EncryptionType, Error};

use crate::helpers::repository_error::RepositoryError;
use crate::helpers::{Database, SecretType};
use crate::model::{Account, AccountGroup};

type Result<T> = ::std::result::Result<T, Error>;

const APPLICATION: &str = "Authenticator-rs";
const APPLICATION_KEY: &str = "application";
const APPLICATION_VALUE: &str = "authenticator-rs";
const ACCOUNT_ID_KEY: &str = "account_id";

pub struct Keyring;

impl Keyring {
    pub fn ensure_unlocked() -> Result<()> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = ss.get_default_collection()?;

        collection.unlock()?;
        collection.ensure_unlocked()
    }

    fn store(ss: &SecretService, label: &str, account_id: u32, secret: &str) -> Result<()> {
        let _ = Self::remove(account_id);

        let collection = ss.get_default_collection()?;

        let mut attributes = HashMap::new();
        let str_account_id = format!("{}", account_id);
        attributes.insert(ACCOUNT_ID_KEY, str_account_id.as_str());
        attributes.insert(APPLICATION_KEY, APPLICATION_VALUE);

        collection.create_item(
            format!("{} TOTP ({})", APPLICATION, label).as_str(),
            attributes,
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
                Self::remove(account_id)?;
                Self::store(&ss, label, account_id, secret)
            }
            Ok(None) => Self::store(&ss, label, account_id, secret),
            Err(e) => Err(e),
        };

        result.map_err(RepositoryError::KeyringError)
    }

    pub fn secret(account_id: u32) -> Result<Option<String>> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = ss.get_default_collection()?;

        let mut attributes = HashMap::new();
        let str_account_id = format!("{}", account_id);
        attributes.insert(ACCOUNT_ID_KEY, str_account_id.as_str());
        attributes.insert(APPLICATION_KEY, APPLICATION_VALUE);

        let search_items = collection.search_items(attributes)?;

        search_items
            .get(0)
            .map(|i| i.get_secret())
            .map(|r: Result<Vec<u8>>| r.and_then(|s: Vec<u8>| String::from_utf8(s).map_err(|_| Error::NoResult)))
            .map(|r| r.map(Some))
            .unwrap_or(Ok(None))
    }

    pub fn remove(account_id: u32) -> Result<()> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = ss.get_default_collection()?;

        let mut attributes = HashMap::new();
        let str_account_id = format!("{}", account_id);
        attributes.insert(ACCOUNT_ID_KEY, str_account_id.as_str());
        attributes.insert(APPLICATION_KEY, APPLICATION_VALUE);

        let search_items = collection.search_items(attributes)?;

        match search_items.get(0) {
            Some(i) => i.delete(),
            None => Err(Error::NoResult),
        }
    }

    pub fn all_secrets() -> std::result::Result<Vec<(String, String)>, RepositoryError> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = ss.get_default_collection()?;

        let mut attributes = HashMap::new();
        attributes.insert(APPLICATION_KEY, APPLICATION_VALUE);

        let results = collection.search_items(attributes)?;

        let secrets = results
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
                        .filter(|t| t.0 == ACCOUNT_ID_KEY)
                        .map(|t| t.1)
                        .collect::<Vec<String>>()
                        .first()
                        .cloned(),
                    Err(_) => None,
                };

                (account_id, secret)
            })
            .filter(|v| v.0.is_some())
            .filter(|v| v.1.is_some())
            .map(|v| (v.0.unwrap(), v.1.unwrap()))
            .collect::<Vec<(String, String)>>();

        Ok(secrets)
    }

    pub fn set_secrets(group_accounts: &mut [AccountGroup], connection: &Connection) -> std::result::Result<(), RepositoryError> {
        let all_secrets = Self::all_secrets()?;

        Self::associate_secrets(group_accounts, &all_secrets, connection)
    }

    pub fn associate_secrets(
        group_accounts: &mut [AccountGroup],
        all_secrets: &[(String, String)],
        connection: &Connection,
    ) -> std::result::Result<(), RepositoryError> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        group_accounts
            .iter_mut()
            .try_for_each(|g| Self::group_account_secret(&ss, g, all_secrets, connection))
    }

    fn group_account_secret(
        ss: &SecretService,
        group_account: &mut AccountGroup,
        all_secrets: &[(String, String)],
        connection: &Connection,
    ) -> std::result::Result<(), RepositoryError> {
        group_account
            .entries
            .iter_mut()
            .try_for_each(|a| Self::account_secret(ss, a, all_secrets, connection))
    }

    fn account_secret(
        ss: &SecretService,
        account: &mut Account,
        all_secrets: &[(String, String)],
        connection: &Connection,
    ) -> std::result::Result<(), RepositoryError> {
        debug!("Loading keyring secret for {} ({})", account.label, account.id);

        match all_secrets.iter().find(|v| v.0 == format!("{}", account.id)) {
            Some(secret) => account.secret = secret.1.clone(),
            None => {
                warn!("No secret found in keyring for {} ({}). Creating one.", account.label, account.id);
                Self::store(ss, account.label.as_str(), account.id, account.secret.as_str())?;
                account.secret_type = SecretType::KEYRING;
                account.secret = "".to_string();
                Database::update_account(connection, account)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore]
    fn should_create_collection_struct() {
        let ss = SecretService::new(EncryptionType::Dh).unwrap();
        Keyring::store(&ss, "x22", 1, "secret").unwrap();

        let result = Keyring::secret(1).unwrap().unwrap();

        assert_eq!("secret", result);
    }
}
