use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use glib::Sender;
use log::error;
use rusqlite::{named_params, params, Connection, OpenFlags, OptionalExtension, Result, NO_PARAMS};
use thiserror::Error;

use crate::helpers::LoadError::{FileError, SaveError};
use crate::helpers::Paths;
use crate::model::{Account, AccountGroup};
use std::{thread, time};

#[derive(Debug, Clone)]
pub struct ConfigManager;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum LoadError {
    #[error("file error `{0}`")]
    FileError(String),

    #[error("file saving error `{0}`")]
    SaveError(String),

    #[error("database error `{0}`")]
    DbError(String),
}

impl ConfigManager {
    pub fn has_groups(connection: &Connection) -> Result<bool, LoadError> {
        let mut stmt = connection.prepare("SELECT COUNT(*) FROM groups").unwrap();

        stmt.query_row(params![], |row| {
            let count: u32 = row.get_unwrap(0);
            Ok(count)
        })
        .map(|count| count > 0)
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn load_account_groups(connection: &Connection, filter: Option<&str>) -> Result<Vec<AccountGroup>, LoadError> {
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
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn create_connection() -> Result<Connection, LoadError> {
        Connection::open_with_flags(Paths::db_path(), OpenFlags::default()).map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn update_group(connection: &Connection, group: &AccountGroup) -> Result<(), LoadError> {
        connection
            .execute(
                "UPDATE groups SET name = ?2, icon = ?3, url = ?4 WHERE id = ?1",
                params![group.id, group.name, group.icon, group.url],
            )
            .map(|_| ())
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn save_group(connection: &Connection, group: &mut AccountGroup) -> Result<(), LoadError> {
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
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    fn group_by_name(connection: &Connection, name: &str) -> Result<Option<AccountGroup>, LoadError> {
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
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn save_group_and_accounts(connection: &Connection, group: &mut AccountGroup) -> Result<(), LoadError> {
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
                .collect::<Result<Vec<()>, LoadError>>()
                .map(|_| ()),
            Err(group_saved_error) => Err(group_saved_error),
        }
    }

    pub fn get_group(connection: &Connection, group_id: u32) -> Result<AccountGroup, LoadError> {
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

                let mut stmt = connection
                    .prepare("SELECT id, label, group_id, secret FROM accounts WHERE group_id = ?1")
                    .unwrap();

                let accounts = stmt
                    .query_map(params![group_id], |row| {
                        let label: String = row.get_unwrap(1);
                        let secret: String = row.get_unwrap(3);
                        let id: u32 = row.get(0)?;
                        let account = Account::new(id, group_id, label.as_str(), secret.as_str());

                        Ok(account)
                    })
                    .unwrap()
                    .map(|e| e.unwrap())
                    .collect();

                row.get(0)
                    .map(|id| AccountGroup::new(id, group_name.as_str(), group_icon.as_deref(), group_url.as_deref(), accounts))
            },
        )
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn save_account(connection: &Connection, account: &mut Account) -> Result<(), LoadError> {
        connection
            .execute(
                "INSERT INTO accounts (label, group_id, secret) VALUES (?1, ?2, ?3)",
                params![account.label, account.group_id, account.secret],
            )
            .unwrap();

        let mut stmt = connection.prepare("SELECT last_insert_rowid()").unwrap();

        stmt.query_row(NO_PARAMS, |row| row.get(0))
            .map(|id| {
                account.id = id;
                id
            })
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
            .map(|_| ())
    }

    pub fn update_account(connection: &Connection, account: &mut Account) -> Result<(), LoadError> {
        connection
            .execute(
                "UPDATE accounts SET label = ?2, secret = ?3, group_id = ?4 WHERE id = ?1",
                params![account.id, account.label, account.secret, account.group_id],
            )
            .map(|_| account.id)
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
            .map(|_| ())
    }

    pub fn get_account(connection: &Connection, account_id: u32) -> Result<Account, LoadError> {
        let mut stmt = connection.prepare("SELECT id, group_id, label, secret FROM accounts WHERE id = ?1").unwrap();

        stmt.query_row(params![account_id], |row| {
            let group_id: u32 = row.get_unwrap(1);
            let label: String = row.get_unwrap(2);
            let secret: String = row.get_unwrap(3);
            let id = row.get_unwrap(0);

            let account = Account::new(id, group_id, label.as_str(), secret.as_str());

            Ok(account)
        })
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn delete_group(connection: &Connection, group_id: u32) -> Result<usize, LoadError> {
        let mut stmt = connection.prepare("DELETE FROM groups WHERE id = ?1").unwrap();

        stmt.execute(params![group_id]).map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn delete_account(connection: &Connection, account_id: u32) -> Result<usize, LoadError> {
        let mut stmt = connection.prepare("DELETE FROM accounts WHERE id = ?1").unwrap();

        stmt.execute(params![account_id]).map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    fn get_accounts(connection: &Connection, group_id: u32, filter: Option<&str>) -> Result<Vec<Account>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT id, label, secret FROM accounts WHERE group_id = ?1 AND label LIKE ?2 ORDER BY LOWER(label)")?;

        let label_filter = filter.map(|f| format!("%{}%", f)).unwrap_or_else(|| "%".to_owned());

        stmt.query_map(params![group_id, label_filter], |row| {
            let id: u32 = row.get_unwrap(0);
            let label: String = row.get_unwrap(1);
            let secret: String = row.get_unwrap(2);

            let account = Account::new(id, group_id, label.as_str(), secret.as_str());
            Ok(account)
        })
        .map(|rows| rows.map(|row| row.unwrap()).collect())
    }

    pub async fn save_accounts(path: PathBuf, connection: Arc<Mutex<Connection>>, tx: Sender<bool>) {
        thread::sleep(time::Duration::from_millis(10 * 1000));
        let group_accounts = {
            let connection = connection.lock().unwrap();
            Self::load_account_groups(&connection, None).unwrap()
        };

        let path = path.as_path();
        match Self::serialise_accounts(group_accounts, path) {
            Ok(()) => tx.send(true).expect("Could not send message"),
            Err(_) => tx.send(false).expect("Could not send message"),
        }
    }

    pub fn serialise_accounts(account_groups: Vec<AccountGroup>, out: &Path) -> Result<(), LoadError> {
        let file = std::fs::File::create(out).map_err(|_| SaveError(format!("Could not open file {} for writing.", out.display())));

        let yaml = serde_yaml::to_string(&account_groups).map_err(|_| SaveError("Could not serialise accounts".to_owned()));

        let combined = file.and_then(|file| yaml.map(|yaml| (yaml, file)));

        combined.and_then(|(yaml, file)| {
            let mut file = &file;
            let yaml = yaml.as_bytes();

            file.write_all(yaml)
                .map_err(|_| SaveError(format!("Could not write serialised accounts to {}", out.display())))
        })
    }

    pub async fn restore_account_and_signal_back(path: PathBuf, connection: Arc<Mutex<Connection>>, tx: Sender<bool>) {
        thread::sleep(time::Duration::from_millis(10 * 1000));
        let results = Self::restore_accounts(path, connection).await;

        match results {
            Ok(_) => tx.send(true).expect("Could not send message"),
            Err(e) => {
                tx.send(false).expect("Could not send message");
                error!("{:?}", e);
            }
        }
    }

    async fn restore_accounts(path: PathBuf, connection: Arc<Mutex<Connection>>) -> Result<(), LoadError> {
        let deserialised_accounts: Result<Vec<AccountGroup>, LoadError> = Self::deserialise_accounts(path.as_path());

        let connection = connection.lock().unwrap();

        deserialised_accounts.and_then(|ref mut account_groups| {
            let results: Vec<Result<(), LoadError>> = account_groups
                .iter_mut()
                .map(|group| Self::save_group_and_accounts(&connection, group))
                .collect();

            results.iter().cloned().collect::<Result<(), LoadError>>()
        })
    }

    fn deserialise_accounts(out: &Path) -> Result<Vec<AccountGroup>, LoadError> {
        let file = std::fs::File::open(out).map_err(|_| FileError(format!("Could not open file {} for reading.", out.display())));

        file.and_then(|file| serde_yaml::from_reader(file).map_err(|e| SaveError(format!("Could not serialise accounts: {}", e))))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use async_std::task;
    use rusqlite::Connection;

    use crate::helpers::runner;
    use crate::model::{Account, AccountGroup};

    use super::ConfigManager;

    #[test]
    fn create_new_account_and_new_group() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "new group", None, None, vec![]);
        let mut account = Account::new(0, 0, "label", "secret");

        ConfigManager::save_group(&connection, &mut group).unwrap();

        account.group_id = group.id;

        ConfigManager::save_account(&connection, &mut account).unwrap();

        assert!(account.id > 0);
        assert!(account.group_id > 0);
        assert_eq!("label", account.label);

        let account_reloaded = ConfigManager::get_account(&connection, account.id).unwrap();

        assert_eq!(account, account_reloaded);

        let mut account_reloaded = account_reloaded.clone();
        account_reloaded.label = "new label".to_owned();
        account_reloaded.secret = "new secret".to_owned();
        ConfigManager::update_account(&connection, &mut account_reloaded).unwrap();

        assert_eq!("new label", account_reloaded.label);
        assert_eq!("new secret", account_reloaded.secret);
    }

    #[test]
    fn test_update_group() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "new group", None, None, vec![]);

        ConfigManager::save_group(&connection, &mut group).unwrap();

        assert_eq!("new group", group.name);

        group.name = "other name".to_owned();
        group.url = Some("url".to_owned());
        group.icon = Some("icon".to_owned());

        ConfigManager::update_group(&connection, &mut group).unwrap();

        let group = ConfigManager::get_group(&connection, group.id).unwrap();

        assert_eq!("other name", group.name);
        assert_eq!("url", group.url.unwrap());
        assert_eq!("icon", group.icon.unwrap());
    }

    #[test]
    fn create_new_account_with_existing_group() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "existing_group2", None, None, vec![]);

        ConfigManager::save_group(&connection, &mut group).unwrap();

        let mut account = Account::new(0, group.id, "label", "secret");

        ConfigManager::save_account(&connection, &mut account).unwrap();

        assert!(account.id > 0);
        assert_eq!(group.id, account.group_id);

        let reloaded_group = ConfigManager::get_group(&connection, group.id).unwrap();
        assert_eq!(group.id, reloaded_group.id);
        assert_eq!("existing_group2", reloaded_group.name);
        assert_eq!(vec![account], reloaded_group.entries);
    }

    #[test]
    fn load_account_groups() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "bbb", Some("icon"), Some("url"), vec![]);
        ConfigManager::save_group(&connection, &mut group).unwrap();

        let mut account1 = Account::new(0, group.id, "hhh", "secret3");
        ConfigManager::save_account(&connection, &mut account1).expect("boom!");

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
            }],
        );
        let groups = ConfigManager::load_account_groups(&connection, None).unwrap();

        assert_eq!(vec![expected], groups);
    }

    #[test]
    fn save_group_ordering() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "bbb", None, None, vec![]);
        ConfigManager::save_group(&connection, &mut group).unwrap();

        let mut account = Account::new(0, group.id, "hhh", "secret3");
        ConfigManager::save_account(&connection, &mut account).expect("boom!");
        let mut account = Account::new(0, group.id, "ccc", "secret3");
        ConfigManager::save_account(&connection, &mut account).expect("boom!");

        let mut group = AccountGroup::new(0, "AAA", None, None, vec![]);
        ConfigManager::save_group(&connection, &mut group).expect("boom!");
        let mut account = Account::new(0, group.id, "ppp", "secret3");
        ConfigManager::save_account(&connection, &mut account).expect("boom!");

        let results = ConfigManager::load_account_groups(&connection, None).unwrap();

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

        let mut account = Account::new(0, 0, "label", "secret");

        ConfigManager::save_account(&connection, &mut account).unwrap();

        assert_eq!(1, account.id);

        let result = ConfigManager::delete_account(&connection, account.id).unwrap();
        assert!(result > 0);
    }

    #[test]
    fn has_groups() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "bbb", None, None, vec![]);
        ConfigManager::save_group(&connection, &mut group).unwrap();

        let mut account = Account::new(0, group.id, "hhh", "secret3");
        ConfigManager::save_account(&connection, &mut account).expect("boom!");

        let result = ConfigManager::has_groups(&connection).unwrap();
        assert!(result);
    }

    #[test]
    fn serialise_accounts() {
        let account = Account::new(1, 0, "label", "secret");
        let account_group = AccountGroup::new(2, "group", Some("icon"), Some("url"), vec![account]);

        let path = PathBuf::from("test.yaml");
        let path = path.as_path();
        let result = ConfigManager::serialise_accounts(vec![account_group], path).unwrap();

        assert_eq!((), result);

        let account_from_yaml = Account::new(0, 0, "label", "secret");
        let account_group_from_yaml = AccountGroup::new(0, "group", None, Some("url"), vec![account_from_yaml]);

        let result = ConfigManager::deserialise_accounts(path).unwrap();
        assert_eq!(vec![account_group_from_yaml], result);
    }

    #[test]
    fn save_group_and_accounts() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let account = Account::new(0, 0, "label", "secret");
        let mut account_group = AccountGroup::new(0, "group", None, None, vec![account]);

        ConfigManager::save_group_and_accounts(&connection, &mut account_group).expect("could not save");

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

            let account = Account::new(1, 0, "label", "secret");
            let account_group = AccountGroup::new(2, "group", None, None, vec![account]);

            let path = PathBuf::from("test.yaml");
            let path = path.as_path();
            let result = ConfigManager::serialise_accounts(vec![account_group], path).unwrap();

            assert_eq!((), result);
        }

        let result = { task::block_on(ConfigManager::restore_accounts(PathBuf::from("test.yaml"), connection.clone())) };

        assert_eq!(Ok(()), result);

        {
            let connection = connection.lock().unwrap();
            let account_groups = ConfigManager::load_account_groups(&connection, None).unwrap();

            assert_eq!(1, account_groups.len());
            assert!(account_groups.first().unwrap().id > 0);
            assert_eq!(1, account_groups.first().unwrap().entries.len());
            assert!(account_groups.first().unwrap().entries.first().unwrap().id > 0);
        }
    }
}
