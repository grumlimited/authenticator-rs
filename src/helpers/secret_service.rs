extern crate secret_service;

use secret_service::{EncryptionType, SsError};
use secret_service::{Item, SecretService};

use crate::helpers::ConfigManager;
use std::string::FromUtf8Error;

pub type Result<T> = ::std::result::Result<T, SsError>;

const APPLICATION_ATTRS: (&str, &str) = ("application", "authenticator-rs");
const ACCOUNT_ID_KEY: &str = "account_id";

trait TotpSecretService {
    fn store(label: &str, account_id: u32, secret: &str) -> Result<()>;

    fn secret(account_id: u32) -> Result<Option<String>>;
}

impl TotpSecretService for ConfigManager {
    fn store(label: &str, account_id: u32, secret: &str) -> Result<()> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = ss.get_default_collection()?;

        collection.create_item(
            label,
            vec![(ACCOUNT_ID_KEY, &format!("{}", account_id)), APPLICATION_ATTRS],
            secret.as_bytes(),
            false,
            "text/plain",
        )?;

        Ok(())
    }

    fn secret(account_id: u32) -> Result<Option<String>> {
        let ss = SecretService::new(EncryptionType::Dh)?;

        let search_items: Vec<Item> = ss.search_items(vec![(ACCOUNT_ID_KEY, &format!("{}", account_id)), APPLICATION_ATTRS])?;

        search_items
            .get(0)
            .map(|i| i.get_secret())
            .map(|r: Result<Vec<u8>>| r.and_then(|s: Vec<u8>| String::from_utf8(s).map_err(|e| SsError::NoResult)))
            .map(|r| r.map(|v| Some(v)))
            .unwrap_or(Ok(None))
    }
}
