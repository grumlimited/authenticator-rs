pub struct Paths;

use crate::helpers::{Database, Keyring, SecretType};
use anyhow::Result;
use log::debug;

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

    pub fn check_configuration_dir() -> Result<()> {
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

    pub fn update_keyring_secrets() -> Result<()> {
        let connection = Database::create_connection()?;

        let accounts = Database::load_account_groups(&connection, None)?;

        accounts
            .iter()
            .flat_map(|group| group.entries.iter().cloned())
            .filter(|account| account.secret_type == SecretType::LOCAL)
            .for_each(|account| {
                Keyring::upsert(account.label.as_str(), account.id, account.secret.as_str()).unwrap();
                let mut c = account.clone();
                c.secret = "".to_owned();
                c.secret_type = SecretType::KEYRING;
                Database::update_account(&connection, &mut c).unwrap();
            });

        Ok(())
    }
}
