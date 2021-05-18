use anyhow::Result;
use log::debug;

use crate::helpers::{Database, Keyring, RepositoryError, SecretType};

pub struct Paths;

impl Paths {
    pub fn db_path() -> std::path::PathBuf {
        let mut path = Self::path();
        path.push("authenticator.db");

        path
    }

    pub fn icons_path(filename: &str) -> std::path::PathBuf {
        let mut path = Self::path();
        path.push("icons");
        path.push(filename);

        path
    }

    pub fn path() -> std::path::PathBuf {
        if let Some(project_dirs) = directories::ProjectDirs::from("uk.co", "grumlimited", "authenticator-rs") {
            project_dirs.data_dir().into()
        } else {
            std::env::current_dir().unwrap_or_default()
        }
    }

    pub fn check_configuration_dir() -> Result<(), RepositoryError> {
        let path = Paths::path();

        if !path.exists() {
            debug!("Creating directory {}", path.display());
        }

        std::fs::create_dir_all(path).map(|_| ())?;

        let path = Paths::icons_path("");

        if !path.exists() {
            debug!("Creating directory {}", path.display());
        }

        std::fs::create_dir_all(path).map(|_| ())?;

        Ok(())
    }

    pub fn update_keyring_secrets() -> Result<(), RepositoryError> {
        let connection = Database::create_connection()?;

        let accounts = Database::load_account_groups(&connection, None)?;

        accounts
            .iter()
            .flat_map(|group| group.entries.iter().cloned())
            .filter(|account| account.secret_type == SecretType::LOCAL)
            .for_each(|ref mut account| {
                Keyring::upsert(account.label.as_str(), account.id, account.secret.as_str()).unwrap();
                account.secret = "".to_owned();
                account.secret_type = SecretType::KEYRING;
                Database::update_account(&connection, account).unwrap();
            });

        Ok(())
    }
}
