use rusqlite::{named_params, params, Connection, OpenFlags, Result, NO_PARAMS};

use crate::helpers::LoadError::{FileError, SaveError};
use crate::model::{Account, AccountGroup};
use glib::Sender;
use log::debug;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use log::error;

#[derive(Debug, Clone)]
pub struct ConfigManager {
    pub groups: Vec<AccountGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoadError {
    #[allow(dead_code)]
    FileError(String),
    #[allow(dead_code)]
    FormatError,
    #[allow(dead_code)]
    SaveError(String),
    DbError(String),
}

impl ConfigManager {
    fn db_path() -> std::path::PathBuf {
        let mut path = ConfigManager::path();
        path.push("authenticator.db");

        path
    }

    pub fn path() -> std::path::PathBuf {
        if let Some(project_dirs) =
            directories::ProjectDirs::from("uk.co", "grumlimited", "authenticator-rs")
        {
            project_dirs.data_dir().into()
        } else {
            std::env::current_dir().unwrap_or_default()
        }
    }

    pub fn load_account_groups(
        conn: Arc<Mutex<Connection>>,
    ) -> Result<Vec<AccountGroup>, LoadError> {
        {
            let conn = conn.clone();
            Self::init_tables(conn).unwrap();
        }

        let conn = conn.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT id, name FROM groups ORDER BY LOWER(name)")
            .unwrap();

        stmt.query_map(params![], |row| {
            let id: u32 = row.get(0)?;
            let name: String = row.get(1)?;

            Ok(AccountGroup::new(
                id,
                name.as_str(),
                Self::get_accounts(&conn, id)?,
            ))
        })
        .map(|rows| rows.map(|each| each.unwrap()).collect())
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn check_configuration_dir() -> Result<(), LoadError> {
        let path = Self::path();

        if !path.exists() {
            debug!("Creating directory {}", path.display());
        }

        std::fs::create_dir_all(path)
            .map(|_| ())
            .map_err(|e| LoadError::FileError(format!("Could not create directory {:?}", e)))
    }

    pub fn create_connection() -> Result<Connection, LoadError> {
        Connection::open_with_flags(Self::db_path(), OpenFlags::default())
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn update_group(
        conn: Arc<Mutex<Connection>>,
        group: &AccountGroup,
    ) -> Result<(), LoadError> {
        let conn = conn.lock().unwrap();

        conn.execute(
            "UPDATE groups SET name = ?2 WHERE id = ?1",
            params![group.id, group.name],
        )
        .map(|_| ())
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn save_group(
        conn: Arc<Mutex<Connection>>,
        group: &mut AccountGroup,
    ) -> Result<(), LoadError> {
        let conn = conn.lock().unwrap();

        conn.execute("INSERT INTO groups (name) VALUES (?1)", params![group.name])
            .unwrap();

        let mut stmt = conn.prepare("SELECT last_insert_rowid()").unwrap();

        stmt.query_row(NO_PARAMS, |row| row.get::<usize, u32>(0))
            .map(|id| {
                group.id = id;
            })
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    fn group_by_name(
        connection: Arc<Mutex<Connection>>,
        name: &str,
    ) -> Result<AccountGroup, LoadError> {
        let connection = connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT id, name FROM groups WHERE name = :name")
            .unwrap();

        stmt.query_row_named(
            named_params! {
            ":name": name
            },
            |row| {
                let group_id: u32 = row.get_unwrap(0);
                let group_name: String = row.get_unwrap(1);

                Ok(AccountGroup::new(group_id, group_name.as_str(), vec![]))
            },
        )
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn save_group_and_accounts(
        connection: Arc<Mutex<Connection>>,
        group: &mut AccountGroup,
    ) -> Result<(), LoadError> {
        let existing_group = {
            let connection = connection.clone();
            Self::group_by_name(connection, group.name.as_str())
        };

        let group_saved_result = {
            let connection = connection.clone();
            match existing_group {
                Ok(group) => Ok(group.id),
                Err(_) => Self::save_group(connection, group).map(|_| group.id),
            }
        };

        let accounts_saved_results: Vec<Result<(), LoadError>> = match group_saved_result {
            Ok(group_id) => group
                .entries
                .iter_mut()
                .map(|account| {
                    let connection = connection.clone();
                    account.group_id = group_id;
                    Self::save_account(connection, account)
                })
                .collect::<Vec<Result<(), LoadError>>>(),

            Err(group_saved_error) => vec![Err(group_saved_error)],
        };

        accounts_saved_results.iter().cloned().collect()
    }

    pub fn get_group(
        conn: Arc<Mutex<Connection>>,
        group_id: u32,
    ) -> Result<AccountGroup, LoadError> {
        let conn = conn.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT id, name FROM groups WHERE id = :group_id")
            .unwrap();

        stmt.query_row_named(
            named_params! {
            ":group_id": group_id
            },
            |row| {
                let group_id: u32 = row.get_unwrap(0);
                let group_name: String = row.get_unwrap(1);

                let mut stmt = conn
                    .prepare("SELECT id, label, group_id, secret FROM accounts WHERE group_id = ?1")
                    .unwrap();

                let accounts = stmt
                    .query_map(params![group_id], |row| {
                        let label = row.get_unwrap::<usize, String>(1);
                        let secret = row.get_unwrap::<usize, String>(3);
                        let id = row.get(0)?;
                        let account = Account::new(id, group_id, label.as_str(), secret.as_str());

                        Ok(account)
                    })
                    .unwrap()
                    .map(|e| e.unwrap())
                    .collect();

                row.get::<usize, u32>(0)
                    .map(|id| AccountGroup::new(id, group_name.as_str(), accounts))
            },
        )
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn save_account(
        conn: Arc<Mutex<Connection>>,
        account: &mut Account,
    ) -> Result<(), LoadError> {
        let conn = conn.lock().unwrap();

        conn.execute(
            "INSERT INTO accounts (label, group_id, secret) VALUES (?1, ?2, ?3)",
            params![account.label, account.group_id, account.secret],
        )
        .unwrap();

        let mut stmt = conn.prepare("SELECT last_insert_rowid()").unwrap();

        stmt.query_row(NO_PARAMS, |row| row.get::<usize, u32>(0))
            .map(|id| {
                account.id = id;
                account.group_id = account.group_id;
            })
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn update_account(
        connection: Arc<Mutex<Connection>>,
        account: &mut Account,
    ) -> Result<(), LoadError> {
        let conn = connection.lock().unwrap();

        conn.execute(
            "UPDATE accounts SET label = ?2, secret = ?3, group_id = ?4 WHERE id = ?1",
            params![account.id, account.label, account.secret, account.group_id],
        )
        .map(|_| ())
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    fn init_tables(conn: Arc<Mutex<Connection>>) -> Result<usize, rusqlite::Error> {
        let conn = conn.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS accounts (
                  id              INTEGER PRIMARY KEY,
                  label           TEXT NOT NULL,
                  group_id        INTEGER NOT NULL,
                  secret          TEXT NOT NULL
                  )",
            params![],
        )
        .and(conn.execute(
            "CREATE TABLE IF NOT EXISTS groups (
                  id             INTEGER PRIMARY KEY,
                  name           TEXT NOT NULL)",
            params![],
        ))
    }

    pub fn get_account(
        connection: Arc<Mutex<Connection>>,
        account_id: u32,
    ) -> Result<Account, LoadError> {
        let conn = connection.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT id, group_id, label, secret FROM accounts WHERE id = ?1")
            .unwrap();

        stmt.query_row(params![account_id], |row| {
            let group_id: u32 = row.get(1)?;
            let label: String = row.get(2)?;
            let secret: String = row.get(3)?;
            let id = row.get(0)?;

            let account = Account::new(id, group_id, label.as_str(), secret.as_str());

            Ok(account)
        })
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn delete_group(
        connection: Arc<Mutex<Connection>>,
        group_id: u32,
    ) -> Result<usize, LoadError> {
        let conn = connection.lock().unwrap();
        let mut stmt = conn.prepare("DELETE FROM groups WHERE id = ?1").unwrap();

        stmt.execute(params![group_id])
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn delete_account(
        connection: Arc<Mutex<Connection>>,
        account_id: u32,
    ) -> Result<usize, LoadError> {
        let conn = connection.lock().unwrap();
        let mut stmt = conn.prepare("DELETE FROM accounts WHERE id = ?1").unwrap();

        stmt.execute(params![account_id])
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    fn get_accounts(conn: &Connection, group_id: u32) -> Result<Vec<Account>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, label, secret FROM accounts WHERE group_id = ?1 ORDER BY LOWER(label)",
        )?;

        stmt.query_map(params![group_id], |row| {
            let id: u32 = row.get(0)?;
            let group_id: u32 = group_id;
            let label: String = row.get(1)?;
            let secret: String = row.get(2)?;

            let account = Account::new(id, group_id, label.as_str(), secret.as_str());
            Ok(account)
        })
        .map(|rows| rows.map(|row| row.unwrap()).collect())
    }

    pub async fn save_accounts(
        path: PathBuf,
        connection: Arc<Mutex<Connection>>,
        tx: Sender<bool>,
    ) {
        let group_accounts = ConfigManager::load_account_groups(connection).unwrap();

        async {
            let path = path.as_path();
            match ConfigManager::serialise_accounts(group_accounts, path) {
                Ok(()) => tx.send(true).expect("Could not send message"),
                Err(_) => tx.send(false).expect("Could not send message"),
            }
        }
        .await;
    }

    pub fn serialise_accounts(
        account_groups: Vec<AccountGroup>,
        out: &Path,
    ) -> Result<(), LoadError> {
        let file = std::fs::File::create(out).map_err(|_| {
            SaveError(format!(
                "Could not open file {} for writing.",
                out.display()
            ))
        });

        let yaml = serde_yaml::to_string(&account_groups)
            .map_err(|_| SaveError("Could not serialise accounts".to_owned()));

        let combined = file.and_then(|file| yaml.map(|yaml| (yaml, file)));

        combined.and_then(|(yaml, file)| {
            let mut file = &file;
            let yaml = yaml.as_bytes();

            file.write_all(yaml).map_err(|_| {
                SaveError(format!(
                    "Could not write serialised accounts to {}",
                    out.display()
                ))
            })
        })
    }

    pub async fn restore_account_and_signal_back(
        path: PathBuf,
        connection: Arc<Mutex<Connection>>,
        tx: Sender<bool>,
    ) {
        let results = Self::restore_accounts(path, connection).await;

        match results {
            Ok(_) => tx.send(true).expect("Could not send message"),
            Err(e) => {
                tx.send(false).expect("Could not send message");
                error!("{:?}", e);
            }
        }
    }

    async fn restore_accounts(
        path: PathBuf,
        connection: Arc<Mutex<Connection>>,
    ) -> Result<(), LoadError> {
        let deserialised_accounts: Result<Vec<AccountGroup>, LoadError> =
            ConfigManager::deserialise_accounts(path.as_path());

        deserialised_accounts.and_then(|ref mut account_groups| {
            let results: Vec<Result<(), LoadError>> = account_groups
                .iter_mut()
                .map(|group| {
                    let connection = connection.clone();
                    Self::save_group_and_accounts(connection, group)
                })
                .collect();

            results.iter().cloned().collect::<Result<(), LoadError>>()
        })
    }

    fn deserialise_accounts(out: &Path) -> Result<Vec<AccountGroup>, LoadError> {
        let file = std::fs::File::open(out).map_err(|_| {
            FileError(format!(
                "Could not open file {} for reading.",
                out.display()
            ))
        });

        file.and_then(|file| {
            serde_yaml::from_reader(file)
                .map_err(|e| SaveError(format!("Could not serialise accounts: {}", e)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ConfigManager;
    use rusqlite::Connection;

    use crate::model::{Account, AccountGroup};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use async_std::task;

    #[test]
    fn create_new_account_and_new_group() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));

        {
            let conn = conn.clone();
            ConfigManager::init_tables(conn).expect("boom!");
        }

        let mut group = AccountGroup::new(0, "new group", vec![]);
        let mut account = Account::new(0, 0, "label", "secret");

        {
            let conn = conn.clone();
            ConfigManager::save_group(conn, &mut group).unwrap().clone()
        }

        account.group_id = group.id;

        {
            let conn = conn.clone();
            ConfigManager::save_account(conn, &mut account)
                .unwrap()
                .clone()
        }

        assert!(account.id > 0);
        assert!(account.group_id > 0);
        assert_eq!("label", account.label);

        let account_reloaded = {
            let conn = conn.clone();
            ConfigManager::get_account(conn, account.id).unwrap()
        };

        assert_eq!(account, account_reloaded);

        let mut account_reloaded = account_reloaded.clone();
        account_reloaded.label = "new label".to_owned();
        account_reloaded.secret = "new secret".to_owned();
        ConfigManager::update_account(conn, &mut account_reloaded).unwrap();

        assert_eq!("new label", account_reloaded.label);
        assert_eq!("new secret", account_reloaded.secret);
    }

    #[test]
    fn test_update_group() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));

        {
            let conn = conn.clone();
            ConfigManager::init_tables(conn).expect("boom!");
        }

        let mut group = AccountGroup::new(0, "new group", vec![]);

        {
            let conn = conn.clone();
            ConfigManager::save_group(conn, &mut group).unwrap().clone()
        }

        assert_eq!("new group", group.name);

        group.name = "other name".to_owned();

        {
            let conn = conn.clone();
            ConfigManager::update_group(conn, &mut group)
                .unwrap()
                .clone()
        }

        assert_eq!("other name", group.name);
    }

    #[test]
    fn create_new_account_with_existing_group() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));

        let _ = {
            let conn = conn.clone();
            ConfigManager::init_tables(conn).expect("boom!");
        };

        let mut group = AccountGroup::new(0, "existing_group2", vec![]);

        {
            let conn = conn.clone();
            ConfigManager::save_group(conn, &mut group).unwrap()
        };

        let mut account = Account::new(0, group.id, "label", "secret");

        {
            let conn = conn.clone();
            ConfigManager::save_account(conn, &mut account)
                .unwrap()
                .clone()
        };

        assert!(account.id > 0);
        assert_eq!(group.id, account.group_id);

        let reloaded_group = ConfigManager::get_group(conn, group.id).unwrap();
        assert_eq!(group.id, reloaded_group.id);
        assert_eq!("existing_group2", reloaded_group.name);
        assert_eq!(vec![account], reloaded_group.entries);
    }

    #[test]
    fn save_group_ordering() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));

        {
            let conn = conn.clone();
            ConfigManager::init_tables(conn).expect("boom!");
        };

        {
            let conn = conn.clone();
            let conn2 = conn.clone();
            let conn3 = conn.clone();
            let mut group = AccountGroup::new(0, "bbb", vec![]);
            ConfigManager::save_group(conn, &mut group).unwrap();

            let mut account1 = Account::new(0, group.id, "hhh", "secret3");
            ConfigManager::save_account(conn2, &mut account1).expect("boom!");
            let mut account2 = Account::new(0, group.id, "ccc", "secret3");
            ConfigManager::save_account(conn3, &mut account2).expect("boom!");
        };

        {
            let conn = conn.clone();
            let mut group = AccountGroup::new(0, "AAA", vec![]);
            ConfigManager::save_group(conn, &mut group).expect("boom!");
        };

        let results = ConfigManager::load_account_groups(conn).unwrap();

        //groups in order
        assert_eq!("AAA", results.get(0).unwrap().name);
        assert_eq!("bbb", results.get(1).unwrap().name);

        //accounts in order
        assert_eq!("ccc", results.get(1).unwrap().entries.get(0).unwrap().label);
        assert_eq!("hhh", results.get(1).unwrap().entries.get(1).unwrap().label);
    }

    #[test]
    fn delete_account() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));

        let _ = {
            let conn = conn.clone();
            ConfigManager::init_tables(conn).expect("boom!");
        };

        let mut account = Account::new(0, 0, "label", "secret");

        {
            let conn = conn.clone();
            ConfigManager::save_account(conn, &mut account)
                .unwrap()
                .clone()
        }

        assert_eq!(1, account.id);

        let result = ConfigManager::delete_account(conn, account.id).unwrap();
        assert!(result > 0);
    }

    #[test]
    fn serialise_accounts() {
        let account = Account::new(1, 0, "label", "secret");
        let account_group = AccountGroup::new(2, "group", vec![account]);

        let path = PathBuf::from("test.yaml");
        let path = path.as_path();
        let result = ConfigManager::serialise_accounts(vec![account_group], path).unwrap();

        assert_eq!((), result);

        let account_from_yaml = Account::new(0, 0, "label", "secret");
        let account_group_from_yaml = AccountGroup::new(0, "group", vec![account_from_yaml]);

        let result = ConfigManager::deserialise_accounts(path).unwrap();
        assert_eq!(vec![account_group_from_yaml], result);
    }

    #[test]
    fn save_group_and_accounts() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));

        {
            let conn = conn.clone();
            ConfigManager::init_tables(conn).expect("boom!");
        }

        let account = Account::new(0, 0, "label", "secret");
        let mut account_group = AccountGroup::new(0, "group", vec![account]);

        ConfigManager::save_group_and_accounts(conn, &mut account_group).expect("could not save");

        assert!(account_group.id > 0);
        assert_eq!(1, account_group.entries.len());
        assert!(account_group.entries.first().unwrap().id > 0);
    }

    #[test]
    fn restore_accounts() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));

        {
            let conn = conn.clone();
            ConfigManager::init_tables(conn).expect("boom!");
        }

        let account = Account::new(1, 0, "label", "secret");
        let account_group = AccountGroup::new(2, "group", vec![account]);

        let path = PathBuf::from("test.yaml");
        let path = path.as_path();
        let result = ConfigManager::serialise_accounts(vec![account_group], path).unwrap();

        assert_eq!((), result);

        let result = {
            let conn = conn.clone();
            task::block_on(ConfigManager::restore_accounts(
                PathBuf::from("test.yaml"),
                conn,
            ))
        };

        assert_eq!(Ok(()), result);

        let account_groups = {
            let conn = conn.clone();
            ConfigManager::load_account_groups(conn)
        }
        .unwrap();

        assert_eq!(1, account_groups.len());
        assert!(account_groups.first().unwrap().id > 0);
        assert_eq!(1, account_groups.first().unwrap().entries.len());
        assert!(account_groups.first().unwrap().entries.first().unwrap().id > 0);
    }
}
