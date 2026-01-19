use log::{debug, error, info, warn};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::helpers::{Database, Keyring, RepositoryError, SecretType};

pub struct Paths;

impl Paths {
    pub fn db_path() -> PathBuf {
        let mut path = Self::path();
        path.push("authenticator.db");
        path
    }

    pub fn icons_path(filename: &str) -> std::path::PathBuf {
        let mut path = Self::path();
        path.push("icons");

        if !filename.is_empty() {
            path.push(filename);
        }

        path
    }

    pub fn path() -> PathBuf {
        match directories::ProjectDirs::from("uk.co", "grumlimited", "authenticator-rs") {
            Some(project_dirs) => project_dirs.data_dir().into(),
            None => match std::env::current_dir() {
                Ok(dir) => {
                    warn!("Could not determine platform config dir; falling back to current directory {}", dir.display());
                    dir
                }
                Err(e) => {
                    error!("Could not determine project dir or current directory: {:?}", e);
                    // fall back to a relative path to avoid panic
                    PathBuf::from(".")
                }
            },
        }
    }

    pub fn check_configuration_dir() -> Result<(), RepositoryError> {
        let base = Self::path();

        if !base.exists() {
            debug!("Creating directory {}", base.display());
        }

        std::fs::create_dir_all(&base)?;

        let icons_dir = Self::icons_path("");
        if !icons_dir.exists() {
            debug!("Creating icons directory {}", icons_dir.display());
        }
        std::fs::create_dir_all(&icons_dir)?;

        Ok(())
    }

    pub fn update_keyring_secrets(connection: Arc<Mutex<Connection>>) -> Result<(), RepositoryError> {
        let connection = connection.lock().unwrap_or_else(|poisoned| {
            warn!("Database connection mutex was poisoned. Recovering.");
            poisoned.into_inner()
        });

        let accounts = Database::load_account_groups(&connection, None)?;

        accounts
            .iter()
            .flat_map(|group| group.entries.iter().cloned())
            .filter(|account| account.secret_type == SecretType::LOCAL)
            .for_each(|ref mut account| {
                info!("Adding {} to keyring", account.label);
                Keyring::upsert(account.label.as_str(), account.id, account.secret.as_str()).unwrap();
                "".clone_into(&mut account.secret);

                account.secret_type = SecretType::KEYRING;
                Database::update_account(&connection, account).unwrap();
            });

        Ok(())
    }
}
