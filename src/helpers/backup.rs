use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use log::warn;
use rusqlite::Connection;

use crate::exporting::{AccountsImportExportResult, ImportType};
use crate::helpers::RepositoryError::GAuthQrCodeError;
use crate::helpers::{Database, Keyring, Paths, QrCode, QrCodeResult, RepositoryError, SecretType};
use crate::model::{Account, AccountGroup};

pub struct Backup;

impl Backup {
    pub async fn save_accounts(
        path: PathBuf,
        connection: Arc<Mutex<Connection>>,
        all_secrets: Vec<(String, String)>,
        tx: async_channel::Sender<AccountsImportExportResult>,
    ) {
        let group_accounts = {
            let connection = connection.lock().unwrap();

            let mut group_accounts = Database::load_account_groups(&connection, None).unwrap();
            Keyring::associate_secrets(&mut group_accounts, &all_secrets, &connection).unwrap();

            group_accounts
        };

        let path = path.as_path();
        match Self::serialise_accounts(group_accounts, path) {
            Ok(()) => tx.send(Ok(())).await.expect("Could not send message"),
            Err(e) => tx.send(Err(e)).await.expect("Could not send message"),
        }
    }

    pub fn serialise_accounts(account_groups: Vec<AccountGroup>, out: &Path) -> Result<(), RepositoryError> {
        let mut file = File::create(out).map_err(RepositoryError::IoError)?;
        let yaml = serde_yaml::to_string(&account_groups).map_err(RepositoryError::SerialisationError)?;
        file.write_all(yaml.as_bytes()).map_err(RepositoryError::IoError)
    }

    pub async fn restore_account_and_signal_back(
        import_type: ImportType,
        path: PathBuf,
        connection: Arc<Mutex<Connection>>,
        tx: async_channel::Sender<AccountsImportExportResult>,
    ) {
        let db = match import_type {
            ImportType::Internal => Self::restore_accounts(path, connection.clone()).await,
            ImportType::GoogleAuthenticator => Self::restore_gauth_accounts(path, connection.clone()).await,
        };

        match db.and_then(|_| Paths::update_keyring_secrets(connection)) {
            Ok(_) => tx.send(Ok(())).await.expect("Could not send message"),
            Err(e) => tx.send(Err(e)).await.expect("Could not send message"),
        }
    }

    async fn restore_accounts(path: PathBuf, connection: Arc<Mutex<Connection>>) -> Result<(), RepositoryError> {
        let mut account_groups = Self::deserialise_accounts(path.as_path())?;

        let connection = connection.lock().unwrap();

        // Mark incoming secrets as LOCAL so they will be migrated to keyring later.
        account_groups
            .iter_mut()
            .for_each(|group| group.entries.iter_mut().for_each(|account| account.secret_type = SecretType::LOCAL));

        for group in account_groups.iter_mut() {
            Database::save_group_and_accounts(&connection, group)?;
        }

        Ok(())
    }

    async fn restore_gauth_accounts(path: PathBuf, connection: Arc<Mutex<Connection>>) -> Result<(), RepositoryError> {
        use google_authenticator_converter::process_data;

        let path_str = path
            .to_str()
            .ok_or_else(|| RepositoryError::IoError(io::Error::new(io::ErrorKind::InvalidInput, "invalid unicode in path")))?;

        let qr_result = QrCode::process_qr_code(path_str.to_owned()).await;

        match qr_result {
            QrCodeResult::Valid(qr_code) => {
                let accounts = process_data(qr_code.qr_code_payload.as_str()).map_err(|e| {
                    warn!("Failed to parse GAuth payload: {:?}", e);
                    GAuthQrCodeError(format!("Invalid GAuth data: {}", e))
                })?;

                let entries = accounts
                    .into_iter()
                    .map(|account| {
                        let secret = account.secret.clone();
                        let secret_type = SecretType::LOCAL;
                        Account::new(0, 0, &account.name, &secret, secret_type)
                    })
                    .collect::<Vec<Account>>();

                let mut account_groups = AccountGroup::new(0, "GAuth", None, None, false, entries);

                let connection = connection.lock().unwrap();
                Database::save_group_and_accounts(&connection, &mut account_groups)?;
                Ok(())
            }
            QrCodeResult::Invalid(e) => {
                warn!("Invalid GAuth QR code: {}", e);
                Err(GAuthQrCodeError(format!("Invalid GAuth code: {}", e)))
            }
        }
    }

    fn deserialise_accounts(out: &Path) -> Result<Vec<AccountGroup>, RepositoryError> {
        let file = File::open(out).map_err(RepositoryError::IoError)?;
        serde_yaml::from_reader(file).map_err(RepositoryError::SerialisationError)
    }
}
