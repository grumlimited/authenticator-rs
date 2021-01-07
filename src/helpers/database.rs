use std::fmt::Debug;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::string::ToString;
use std::sync::{Arc, Mutex};

use glib::Sender;
use log::error;
use log::warn;
use rusqlite::{Connection, named_params, NO_PARAMS, OpenFlags, OptionalExtension, params, Result, Row, ToSql};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use secret_service::SsError;
use serde::{Deserialize, Serialize};
use strum_macros::Display;
use strum_macros::EnumString;
use thiserror::Error;

use crate::helpers::{Keyring, Paths};
use crate::helpers::SecretType::LOCAL;
use crate::model::{Account, AccountGroup};

#[derive(Debug, Clone)]
pub struct Database;

#[derive(Debug, Error)]
#[error("{0}")]
pub enum RepositoryError {
    SqlError(#[from] rusqlite::Error),
    IoError(#[from] io::Error),
    SerialisationError(#[from] serde_yaml::Error),
    KeyringError(#[from] SsError),
    KeyringDecodingError(#[from] std::string::FromUtf8Error),
}

#[derive(Debug, PartialEq, EnumString, Serialize, Deserialize, Clone, Display)]
pub enum SecretType {
    LOCAL,
    KEYRING,
}

impl Database {
    pub fn has_groups(connection: &Connection) -> Result<bool, RepositoryError> {
        let mut stmt = connection.prepare("SELECT COUNT(*) FROM groups").unwrap();

        stmt.query_row(params![], |row| {
            let count: u32 = row.get_unwrap(0);
            Ok(count)
        })
        .map(|count| count > 0)
        .map_err(RepositoryError::SqlError)
    }

    pub fn load_account_groups(connection: &Connection, filter: Option<&str>) -> Result<Vec<AccountGroup>, RepositoryError> {
        let mut stmt = connection.prepare("SELECT id, name, icon, url FROM groups ORDER BY LOWER(name)").unwrap();

        stmt.query_map(params![], |row| {
            let id = row.get_unwrap(0);
            let name: String = row.get_unwrap(1);
            let icon: Option<String> = row.get(2).optional().unwrap_or(None);
            let url: Option<String> = row.get(3).optional().unwrap_or(None);

            Ok(AccountGroup::new(
                id,
                name.as_str(),
                icon.as_deref(),
                url.as_deref(),
                Self::get_accounts(&connection, id, filter)?,
            ))
        })
        .map(|rows| {
            rows.map(|each| each.unwrap())
                .collect::<Vec<AccountGroup>>()
                .into_iter()
                //filter out empty groups - unless no filter is applied then display everything
                .filter(|account_group| !account_group.entries.is_empty() || filter.is_none())
                .collect()
        })
        .map_err(RepositoryError::SqlError)
    }

    pub fn create_connection() -> Result<Connection, RepositoryError> {
        Connection::open_with_flags(Paths::db_path(), OpenFlags::default()).map_err(RepositoryError::SqlError)
    }

    pub fn update_group(connection: &Connection, group: &AccountGroup) -> Result<(), RepositoryError> {
        connection
            .execute(
                "UPDATE groups SET name = ?2, icon = ?3, url = ?4 WHERE id = ?1",
                params![group.id, group.name, group.icon, group.url],
            )
            .map(|_| ())
            .map_err(RepositoryError::SqlError)
    }

    pub fn save_group(connection: &Connection, group: &mut AccountGroup) -> Result<(), RepositoryError> {
        connection
            .execute(
                "INSERT INTO groups (name, icon, url) VALUES (?1, ?2, ?3)",
                params![group.name, group.icon, group.url],
            )
            .unwrap();

        let mut stmt = connection.prepare("SELECT last_insert_rowid()").unwrap();

        stmt.query_row(NO_PARAMS, |row| row.get(0))
            .map(|id| {
                group.id = id;
            })
            .map_err(RepositoryError::SqlError)
    }

    fn group_by_name(connection: &Connection, name: &str) -> Result<Option<AccountGroup>, RepositoryError> {
        let mut stmt = connection.prepare("SELECT id, name, icon, url FROM groups WHERE name = :name").unwrap();

        stmt.query_row_named(named_params! {":name": name}, |row| {
            let group_id = row.get_unwrap(0);
            let group_name: String = row.get_unwrap(1);
            let group_icon: Option<String> = row.get(2).optional().unwrap_or(None);
            let group_url: Option<String> = row.get(3).optional().unwrap_or(None);

            Ok(AccountGroup::new(
                group_id,
                group_name.as_str(),
                group_icon.as_deref(),
                group_url.as_deref(),
                vec![],
            ))
        })
        .optional()
        .map_err(RepositoryError::SqlError)
    }

    pub fn save_group_and_accounts(connection: &Connection, group: &mut AccountGroup) -> Result<(), RepositoryError> {
        let existing_group = Self::group_by_name(connection, group.name.as_str())?;

        let group_saved_result = match existing_group {
            Some(group) => Ok(group.id),
            None => Self::save_group(connection, group).map(|_| group.id),
        };

        match group_saved_result {
            Ok(group_id) => group
                .entries
                .iter_mut()
                .map(|account| {
                    account.group_id = group_id;
                    Self::save_account(&connection, account)
                })
                .into_iter()
                .collect::<Result<Vec<u32>, RepositoryError>>()
                .map(|_| ()),
            Err(group_saved_error) => Err(group_saved_error),
        }
    }

    pub fn get_group(connection: &Connection, group_id: u32) -> Result<AccountGroup, RepositoryError> {
        let mut stmt = connection.prepare("SELECT id, name, icon, url FROM groups WHERE id = :group_id").unwrap();

        stmt.query_row_named(
            named_params! {
            ":group_id": group_id
            },
            |row| {
                let group_id: u32 = row.get_unwrap(0);
                let group_name: String = row.get_unwrap(1);
                let group_icon: Option<String> = row.get(2).optional().unwrap_or(None);
                let group_url: Option<String> = row.get(3).optional().unwrap_or(None);

                let accounts = match Self::get_accounts(connection, group_id, None) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("Error getting accounts for group {}: {:?}", group_id, e);
                        vec![]
                    }
                };

                row.get(0)
                    .map(|id| AccountGroup::new(id, group_name.as_str(), group_icon.as_deref(), group_url.as_deref(), accounts))
            },
        )
        .map_err(RepositoryError::SqlError)
    }

    pub fn save_account(connection: &Connection, account: &mut Account) -> Result<u32, RepositoryError> {
        connection
            .execute(
                "INSERT INTO accounts (label, group_id, secret, secret_type) VALUES (?1, ?2, ?3, ?4)",
                params![account.label, account.group_id, account.secret, account.secret_type],
            )
            .map_err(RepositoryError::SqlError)?;

        let mut stmt = connection.prepare("SELECT last_insert_rowid()").unwrap();

        stmt.query_row(NO_PARAMS, |row| row.get(0))
            .map(|id| {
                account.id = id;
                id
            })
            .map_err(RepositoryError::SqlError)
    }

    pub fn update_account(connection: &Connection, account: &mut Account) -> Result<u32, RepositoryError> {
        connection
            .execute(
                "UPDATE accounts SET label = ?2, secret = ?3, group_id = ?4, secret_type = ?5 WHERE id = ?1",
                params![account.id, account.label, account.secret, account.group_id, account.secret_type],
            )
            .map(|_| account.id)
            .map_err(RepositoryError::SqlError)
    }

    pub fn get_account(connection: &Connection, account_id: u32) -> Result<Account, RepositoryError> {
        let mut stmt = connection
            .prepare("SELECT id, group_id, label, secret, secret_type FROM accounts WHERE id = ?1")
            .unwrap();

        stmt.query_row(params![account_id], |row| {
            let group_id: u32 = row.get_unwrap(1);
            let label: String = row.get_unwrap(2);
            let secret: String = row.get_unwrap(3);
            let id = row.get_unwrap(0);

            let secret_type = Database::extract_secret_type(row, 4);

            let account = Account::new(id, group_id, label.as_str(), secret.as_str(), secret_type);

            Ok(account)
        })
        .map_err(RepositoryError::SqlError)
    }

    fn extract_secret_type(row: &Row, idx: usize) -> SecretType {
        match row.get::<_, String>(idx) {
            Ok(v) => match SecretType::from_str(v.as_str()) {
                Ok(secret_type) => secret_type,
                Err(_) => {
                    warn!("Invalid secret type [{}]", v);
                    LOCAL
                }
            },
            Err(e) => {
                warn!("Invalid secret type [{:?}]", e);
                LOCAL
            }
        }
    }

    pub fn delete_group(connection: &Connection, group_id: u32) -> Result<usize, RepositoryError> {
        let mut stmt = connection.prepare("DELETE FROM groups WHERE id = ?1").unwrap();

        stmt.execute(params![group_id]).map_err(RepositoryError::SqlError)
    }

    pub fn delete_account(connection: &Connection, account_id: u32) -> Result<usize, RepositoryError> {
        let mut stmt = connection.prepare("DELETE FROM accounts WHERE id = ?1").unwrap();

        stmt.execute(params![account_id]).map_err(RepositoryError::SqlError)
    }

    fn get_accounts(connection: &Connection, group_id: u32, filter: Option<&str>) -> Result<Vec<Account>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT id, label, secret, secret_type FROM accounts WHERE group_id = ?1 AND label LIKE ?2 ORDER BY LOWER(label)")?;

        let label_filter = filter.map(|f| format!("%{}%", f)).unwrap_or_else(|| "%".to_owned());

        stmt.query_map(params![group_id, label_filter], |row| {
            let id: u32 = row.get_unwrap(0);
            let label: String = row.get_unwrap(1);

            let secret_type = Self::extract_secret_type(&row, 3);

            let secret: String = row.get_unwrap(2);

            let account = Account::new(id, group_id, label.as_str(), secret.as_str(), secret_type);
            Ok(account)
        })
        .map(|rows| rows.map(|row| row.unwrap()).collect())
    }

    pub async fn save_accounts(path: PathBuf, connection: Arc<Mutex<Connection>>, all_secrets: Vec<(String, String)>, tx: Sender<bool>) {
        let mut group_accounts = {
            let connection = connection.lock().unwrap();
            Self::load_account_groups(&connection, None).unwrap()
        };

        let _ = Keyring::associate_secrets(&mut group_accounts, &all_secrets).unwrap();

        let path = path.as_path();
        match Self::serialise_accounts(group_accounts, path) {
            Ok(()) => tx.send(true).expect("Could not send message"),
            Err(_) => tx.send(false).expect("Could not send message"),
        }
    }

    pub fn serialise_accounts(account_groups: Vec<AccountGroup>, out: &Path) -> Result<(), RepositoryError> {
        let file = std::fs::File::create(out).map_err(RepositoryError::IoError);

        let yaml = serde_yaml::to_string(&account_groups).map_err(RepositoryError::SerialisationError);

        let combined = file.and_then(|file| yaml.map(|yaml| (yaml, file)));

        combined.and_then(|(yaml, file)| {
            let mut file = &file;
            let yaml = yaml.as_bytes();

            file.write_all(yaml).map_err(RepositoryError::IoError)
        })
    }

    pub async fn restore_account_and_signal_back(path: PathBuf, connection: Arc<Mutex<Connection>>, tx: Sender<bool>) {
        let results = Self::restore_accounts(path, connection).await;

        let _ = Paths::update_keyring_secrets().unwrap();

        match results {
            Ok(_) => tx.send(true).expect("Could not send message"),
            Err(e) => {
                tx.send(false).expect("Could not send message");
                error!("{:?}", e);
            }
        }
    }

    async fn restore_accounts(path: PathBuf, connection: Arc<Mutex<Connection>>) -> Result<(), RepositoryError> {
        let mut deserialised_accounts = Self::deserialise_accounts(path.as_path())?;

        let connection = connection.lock().unwrap();

        deserialised_accounts
            .iter_mut()
            .map(|group| {
                group.entries.iter_mut().for_each(|account| account.secret_type = SecretType::LOCAL);

                group
            })
            .try_for_each(|account_groups| Self::save_group_and_accounts(&connection, account_groups))?;

        Ok(())
    }

    fn deserialise_accounts(out: &Path) -> Result<Vec<AccountGroup>, RepositoryError> {
        let file = std::fs::File::open(out).map_err(RepositoryError::IoError);

        file.and_then(|file| serde_yaml::from_reader(file).map_err(RepositoryError::SerialisationError))
    }
}

/**
* avoids
* *const std::ffi::c_void` cannot be shared between threads safely
* when using ...?; with anyhow.
*/
unsafe impl Sync for RepositoryError {}

impl Default for SecretType {
    fn default() -> Self { SecretType::KEYRING }
}

impl ToSql for SecretType {
    #[inline]
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

impl FromSql for SecretType {
    #[inline]
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(s) => SecretType::from_str(std::str::from_utf8(s).unwrap()),
            _ => return Err(FromSqlError::InvalidType),
        }
        .map_err(|err| FromSqlError::Other(Box::new(err)))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use async_std::task;
    use rusqlite::Connection;

    use crate::helpers::runner;
    use crate::helpers::SecretType::LOCAL;
    use crate::model::{Account, AccountGroup};

    use super::Database;

    #[test]
    fn create_new_account_and_new_group() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "new group", None, None, vec![]);
        let mut account = Account::new(0, 0, "label", "secret", LOCAL);

        Database::save_group(&connection, &mut group).unwrap();

        account.group_id = group.id;

        Database::save_account(&connection, &mut account).unwrap();

        assert!(account.id > 0);
        assert!(account.group_id > 0);
        assert_eq!("label", account.label);

        let account_reloaded = Database::get_account(&connection, account.id).unwrap();

        assert_eq!(account, account_reloaded);

        let mut account_reloaded = account_reloaded.clone();
        account_reloaded.label = "new label".to_owned();
        account_reloaded.secret = "new secret".to_owned();
        Database::update_account(&connection, &mut account_reloaded).unwrap();

        assert_eq!("new label", account_reloaded.label);
        assert_eq!("new secret", account_reloaded.secret);
    }

    #[test]
    fn test_update_group() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "new group", None, None, vec![]);

        Database::save_group(&connection, &mut group).unwrap();

        assert_eq!("new group", group.name);

        group.name = "other name".to_owned();
        group.url = Some("url".to_owned());
        group.icon = Some("icon".to_owned());

        Database::update_group(&connection, &mut group).unwrap();

        let group = Database::get_group(&connection, group.id).unwrap();

        assert_eq!("other name", group.name);
        assert_eq!("url", group.url.unwrap());
        assert_eq!("icon", group.icon.unwrap());
    }

    #[test]
    fn create_new_account_with_existing_group() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "existing_group2", None, None, vec![]);

        Database::save_group(&connection, &mut group).unwrap();

        let mut account = Account::new(0, group.id, "label", "secret", LOCAL);

        Database::save_account(&connection, &mut account).unwrap();

        assert!(account.id > 0);
        assert_eq!(group.id, account.group_id);

        let reloaded_group = Database::get_group(&connection, group.id).unwrap();
        assert_eq!(group.id, reloaded_group.id);
        assert_eq!("existing_group2", reloaded_group.name);
        assert_eq!(vec![account], reloaded_group.entries);
    }

    #[test]
    fn load_account_groups() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "bbb", Some("icon"), Some("url"), vec![]);
        Database::save_group(&connection, &mut group).unwrap();

        let mut account1 = Account::new(0, group.id, "hhh", "secret3", LOCAL);
        Database::save_account(&connection, &mut account1).expect("boom!");

        let expected = AccountGroup::new(
            1,
            "bbb",
            Some("icon"),
            Some("url"),
            vec![Account {
                id: 1,
                group_id: 1,
                label: "hhh".to_owned(),
                secret: "secret3".to_owned(),
                secret_type: LOCAL,
            }],
        );
        let groups = Database::load_account_groups(&connection, None).unwrap();

        assert_eq!(vec![expected], groups);
    }

    #[test]
    fn save_group_ordering() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "bbb", None, None, vec![]);
        Database::save_group(&connection, &mut group).unwrap();

        let mut account = Account::new(0, group.id, "hhh", "secret3", LOCAL);
        Database::save_account(&connection, &mut account).expect("boom!");
        let mut account = Account::new(0, group.id, "ccc", "secret3", LOCAL);
        Database::save_account(&connection, &mut account).expect("boom!");

        let mut group = AccountGroup::new(0, "AAA", None, None, vec![]);
        Database::save_group(&connection, &mut group).expect("boom!");
        let mut account = Account::new(0, group.id, "ppp", "secret3", LOCAL);
        Database::save_account(&connection, &mut account).expect("boom!");

        let results = Database::load_account_groups(&connection, None).unwrap();

        //groups in order
        assert_eq!("AAA", results.get(0).unwrap().name);
        assert_eq!("bbb", results.get(1).unwrap().name);

        //accounts in order
        assert_eq!("ccc", results.get(1).unwrap().entries.get(0).unwrap().label);
        assert_eq!("hhh", results.get(1).unwrap().entries.get(1).unwrap().label);
        assert_eq!("ppp", results.get(0).unwrap().entries.get(0).unwrap().label);
    }

    #[test]
    fn delete_account() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut account = Account::new(0, 0, "label", "secret", LOCAL);

        Database::save_account(&connection, &mut account).unwrap();

        assert_eq!(1, account.id);

        let result = Database::delete_account(&connection, account.id).unwrap();
        assert!(result > 0);
    }

    #[test]
    fn has_groups() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "bbb", None, None, vec![]);
        Database::save_group(&connection, &mut group).unwrap();

        let mut account = Account::new(0, group.id, "hhh", "secret3", LOCAL);
        Database::save_account(&connection, &mut account).expect("boom!");

        let result = Database::has_groups(&connection).unwrap();
        assert!(result);
    }

    #[test]
    fn serialise_accounts() {
        let account = Account::new(1, 0, "label", "secret", LOCAL);
        let account_group = AccountGroup::new(2, "group", Some("icon"), Some("url"), vec![account]);

        let path = PathBuf::from("test.yaml");
        let path = path.as_path();
        let result = Database::serialise_accounts(vec![account_group], path).unwrap();

        assert_eq!((), result);

        let account_from_yaml = Account::new(0, 0, "label", "secret", LOCAL);
        let account_group_from_yaml = AccountGroup::new(0, "group", None, Some("url"), vec![account_from_yaml]);

        let result = Database::deserialise_accounts(path).unwrap();
        assert_eq!(vec![account_group_from_yaml], result);
    }

    #[test]
    fn save_group_and_accounts() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let account = Account::new(0, 0, "label", "secret", LOCAL);
        let mut account_group = AccountGroup::new(0, "group", None, None, vec![account]);

        Database::save_group_and_accounts(&connection, &mut account_group).expect("could not save");

        assert!(account_group.id > 0);
        assert_eq!(1, account_group.entries.len());
        assert!(account_group.entries.first().unwrap().id > 0);
    }

    #[test]
    fn restore_accounts() {
        let connection = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));

        {
            let mut connection = connection.lock().unwrap();
            runner::run(&mut connection).unwrap();

            let account = Account::new(1, 0, "label", "secret", LOCAL);
            let account_group = AccountGroup::new(2, "group", None, None, vec![account]);

            let path = PathBuf::from("test.yaml");
            let path = path.as_path();
            let result = Database::serialise_accounts(vec![account_group], path).unwrap();

            assert_eq!((), result);
        }

        let result = { task::block_on(Database::restore_accounts(PathBuf::from("test.yaml"), connection.clone())) }.unwrap();

        assert_eq!((), result);

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
