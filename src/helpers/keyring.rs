use std::collections::HashMap;

use log::{debug, warn};
use rusqlite::Connection;
use secret_service::blocking::SecretService;
use secret_service::{EncryptionType, Error as SsError};

use crate::helpers::repository_error::RepositoryError;
use crate::helpers::{Database, SecretType};
use crate::model::{Account, AccountGroup};

type Result<T> = ::std::result::Result<T, RepositoryError>;

const APPLICATION: &str = "Authenticator-rs";
const APPLICATION_KEY: &str = "application";
const APPLICATION_VALUE: &str = "authenticator-rs";
const ACCOUNT_ID_KEY: &str = "account_id";

pub struct Keyring;

impl Keyring {
    fn connect<'a>() -> Result<SecretService<'a>> {
        SecretService::connect(EncryptionType::Dh).map_err(RepositoryError::KeyringError)
    }

    pub fn ensure_unlocked() -> Result<()> {
        let ss = Self::connect()?;
        let collection = ss.get_default_collection().map_err(RepositoryError::KeyringError)?;

        collection.unlock()?;
        collection.ensure_unlocked().map_err(RepositoryError::KeyringError)
    }

    fn store(ss: &SecretService, label: &str, account_id: u32, secret: &str) -> Result<()> {
        let collection = ss.get_default_collection().map_err(RepositoryError::KeyringError)?;

        let str_account_id = format!("{}", account_id);

        let mut attributes: HashMap<&str, &str> = HashMap::new();
        attributes.insert(ACCOUNT_ID_KEY, str_account_id.as_str());
        attributes.insert(APPLICATION_KEY, APPLICATION_VALUE);

        collection.create_item(
            format!("{} TOTP ({})", APPLICATION, label).as_str(),
            attributes,
            secret.as_bytes(),
            true,
            "text/plain",
        )?;

        debug!("Saved {} ({}) to keyring", label, account_id);
        Ok(())
    }

    pub fn upsert(label: &str, account_id: u32, secret: &str) -> Result<()> {
        let ss = Self::connect()?;
        Self::store(&ss, label, account_id, secret)
    }

    pub fn secret(account_id: u32) -> Result<Option<String>> {
        let ss = Self::connect()?;
        let collection = ss.get_default_collection()?;

        let str_account_id = format!("{}", account_id);
        let mut attributes: HashMap<&str, &str> = HashMap::new();
        attributes.insert(ACCOUNT_ID_KEY, str_account_id.as_str());
        attributes.insert(APPLICATION_KEY, APPLICATION_VALUE);

        let search_items = collection.search_items(attributes)?;

        if let Some(item) = search_items.first() {
            let bytes = item.get_secret()?;
            let secret = String::from_utf8(bytes).map_err(RepositoryError::KeyringDecodingError)?;
            Ok(Some(secret))
        } else {
            Ok(None)
        }
    }

    pub fn remove(account_id: u32) -> Result<()> {
        let ss = Self::connect()?;
        let collection = ss.get_default_collection()?;

        let mut attributes = HashMap::from([(APPLICATION_KEY, APPLICATION_VALUE)]);
        let str_account_id = format!("{}", account_id);
        attributes.insert(ACCOUNT_ID_KEY, str_account_id.as_str());

        let search_items = collection.search_items(attributes)?;

        match search_items.first() {
            Some(i) => i.delete().map_err(RepositoryError::KeyringError),
            None => Err(RepositoryError::KeyringError(SsError::NoResult)),
        }
    }

    pub fn all_secrets() -> Result<Vec<(String, String)>> {
        let ss = Self::connect()?;
        let collection = ss.get_default_collection()?;

        let attributes = HashMap::from([(APPLICATION_KEY, APPLICATION_VALUE)]);
        let results = collection.search_items(attributes)?;

        let secrets = results
            .iter()
            .map(|item| {
                let secret = item
                    .get_secret()
                    .map_err(RepositoryError::KeyringError)
                    .and_then(|v| String::from_utf8(v).map_err(RepositoryError::KeyringDecodingError))
                    .ok();

                let account_id = match item.get_attributes() {
                    Ok(attributes) => attributes
                        .into_iter()
                        .filter(|(key, _)| key == ACCOUNT_ID_KEY)
                        .map(|(_, account_id)| account_id)
                        .collect::<Vec<String>>()
                        .first()
                        .cloned(),
                    Err(_) => None,
                };

                (account_id, secret)
            })
            .filter(|(account_id, secret)| account_id.is_some() && secret.is_some())
            .map(|(account_id, secret)| (account_id.unwrap(), secret.unwrap()))
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
        let ss = Self::connect()?;
        group_accounts
            .iter_mut()
            .try_for_each(|group| Self::group_account_secret(&ss, group, all_secrets, connection))
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

        if let Some((_, secret)) = all_secrets.iter().find(|(account_id, _)| *account_id == format!("{}", account.id)) {
            account.secret = secret.clone();
            account.secret_type = SecretType::KEYRING;
            return Ok(());
        }

        warn!("No secret found in keyring for {} ({}). Creating one.", account.label, account.id);

        Self::store(ss, account.label.as_str(), account.id, account.secret.as_str())?;
        account.secret_type = SecretType::KEYRING;
        account.secret.clear();

        Database::update_account(connection, account).map(|_| ())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore]
    fn should_create_collection_struct() {
        if let Ok(ss) = SecretService::connect(EncryptionType::Dh) {
            let _ = Keyring::store(&ss, "x22", 1, "secret");
            if let Ok(Some(result)) = Keyring::secret(1) {
                assert_eq!("secret", result);
            }
        }
    }
}
