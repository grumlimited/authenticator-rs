use std::fs::File;
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
        let file = File::create(out).map_err(RepositoryError::IoError);

        let yaml = serde_yaml::to_string(&account_groups).map_err(RepositoryError::SerialisationError);

        let combined = file.and_then(|file| yaml.map(|yaml| (yaml, file)));

        combined.and_then(|(yaml, file)| {
            let mut file = &file;
            let yaml = yaml.as_bytes();

            file.write_all(yaml).map_err(RepositoryError::IoError)
        })
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
        let mut accounts = Self::deserialise_accounts(path.as_path())?;

        let connection = connection.lock().unwrap();

        accounts.iter_mut().for_each(|group| {
            group.entries.iter_mut().for_each(|account| account.secret_type = SecretType::LOCAL);
        });

        accounts
            .iter_mut()
            .try_for_each(|account_groups| Database::save_group_and_accounts(&connection, account_groups))?;

        Ok(())
    }

    async fn restore_gauth_accounts(path: PathBuf, connection: Arc<Mutex<Connection>>) -> Result<(), RepositoryError> {
        use google_authenticator_converter::process_data;

        let result = QrCode::process_qr_code(path.to_str().unwrap().to_owned()).await;

        match result {
            QrCodeResult::Valid(qr_code) => {
                let accounts = process_data(qr_code.qr_code_payload.as_str());

                let entries = accounts
                    .unwrap()
                    .iter()
                    .map(|account| {
                        let secret = account.secret.clone();
                        let secret_type = SecretType::LOCAL;
                        Account::new(0, 0, &account.name, &secret, secret_type)
                    })
                    .collect();

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
        let file = File::open(out).map_err(RepositoryError::IoError);

        file.and_then(|file| serde_yaml::from_reader(file).map_err(RepositoryError::SerialisationError))
    }
}

#[cfg(test)]
mod tests {
    use async_std::task;
    use rusqlite::Connection;

    use crate::helpers::SecretType::{KEYRING, LOCAL};
    use crate::helpers::{runner, Backup, Database};
    use crate::model::{Account, AccountGroup};

    use super::Arc;
    use super::Mutex;
    use super::PathBuf;

    #[test]
    fn serialise_accounts() {
        let account = Account::new(1, 0, "label", "secret", KEYRING);
        let account_group = AccountGroup::new(2, "group", Some("icon"), Some("url"), false, vec![account]);

        let path = PathBuf::from("test.yaml");
        let path = path.as_path();
        Backup::serialise_accounts(vec![account_group], path).unwrap();

        let account_from_yaml = Account::new(0, 0, "label", "secret", KEYRING);
        let account_group_from_yaml = AccountGroup::new(0, "group", None, Some("url"), false, vec![account_from_yaml]);

        let result = Backup::deserialise_accounts(path).unwrap();
        assert_eq!(vec![account_group_from_yaml], result);
    }

    #[test]
    fn restore_accounts() {
        let connection = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));

        {
            let mut connection = connection.lock().unwrap();
            runner::run(&mut connection).unwrap();

            let account = Account::new(1, 0, "label", "secret", LOCAL);
            let account_group = AccountGroup::new(2, "group", None, None, false, vec![account]);

            let path = PathBuf::from("test.yaml");
            let path = path.as_path();
            Backup::serialise_accounts(vec![account_group], path).unwrap();
        }

        task::block_on(Backup::restore_accounts(PathBuf::from("test.yaml"), connection.clone())).unwrap();

        {
            let connection = connection.lock().unwrap();
            let account_groups = Database::load_account_groups(&connection, None).unwrap();

            assert_eq!(1, account_groups.len());
            assert!(account_groups.first().unwrap().id > 0);
            assert_eq!(1, account_groups.first().unwrap().entries.len());
            assert!(account_groups.first().unwrap().entries.first().unwrap().id > 0);
        }
    }
}
