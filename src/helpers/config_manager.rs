use serde::{Deserialize, Serialize};

use rusqlite::{named_params, params, Connection, OpenFlags, Result, NO_PARAMS};

use crate::ui::{Account, AccountGroup};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigManager {
    pub groups: Vec<AccountGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoadError {
    #[allow(dead_code)]
    FileError,
    #[allow(dead_code)]
    FormatError,
    #[allow(dead_code)]
    SaveError,
    DbError(String),
}

impl ConfigManager {
    fn path() -> std::path::PathBuf {
        let mut path = if let Some(project_dirs) =
            directories::ProjectDirs::from("uk.co", "grumlimited", "authenticator-rs")
        {
            project_dirs.data_dir().into()
        } else {
            std::env::current_dir().unwrap_or_default()
        };

        path.push("authenticator.db");

        path
    }

    pub async fn async_load_account_groups(
        conn: Arc<Mutex<Box<Connection>>>,
    ) -> Result<Vec<AccountGroup>, LoadError> {
        let conn = conn.lock().unwrap();
        Self::load_account_groups(&conn)
    }

    fn load_account_groups(conn: &Connection) -> Result<Vec<AccountGroup>, LoadError> {
        Self::init_tables(&conn).unwrap();

        let mut stmt = conn.prepare("SELECT id, name FROM groups").unwrap();

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

    pub fn create_connection() -> Result<Connection, LoadError> {
        Connection::open_with_flags(Self::path(), OpenFlags::default())
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    fn create_group(conn: &Connection, group_name: &str) -> Result<AccountGroup, LoadError> {
        conn.execute("INSERT INTO groups (name) VALUES (?1)", params![group_name])
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
            .unwrap();

        let mut stmt = conn.prepare("SELECT last_insert_rowid()").unwrap();

        stmt.query_row(NO_PARAMS, |row| row.get::<usize, u32>(0))
            .map(|id| AccountGroup {
                id,
                name: group_name.to_owned(),
                entries: vec![],
            })
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn get_or_create_group(
        conn: &Connection,
        group_name: &str,
    ) -> Result<AccountGroup, LoadError> {
        let mut stmt = conn
            .prepare("SELECT id FROM groups WHERE name = :name")
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))?;

        let group = stmt
            .query_row_named(
                named_params! {
                ":name": group_name
                },
                |row| {
                    let group_id: u32 = row.get_unwrap(0);

                    let mut stmt = conn
                        .prepare(
                            "SELECT id, label, group_id, secret FROM accounts WHERE group_id = ?1",
                        )
                        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
                        .unwrap();

                    let accounts = stmt
                        .query_map(params![group_id], |row| {
                            let label = row.get::<usize, String>(1).unwrap();
                            let secret = row.get::<usize, String>(3).unwrap();
                            let mut account =
                                Account::new(group_id, label.as_str(), secret.as_str());
                            account.id = row.get(0)?;

                            Ok(account)
                        })
                        .unwrap()
                        .map(|e| e.unwrap())
                        .collect();

                    row.get::<usize, u32>(0)
                        .map(|id| AccountGroup::new(id, group_name, accounts))
                },
            )
            .map_err(|e| LoadError::DbError(format!("{:?}", e)));

        group.or_else(|_| Self::create_group(conn, group_name))
    }

    pub async fn _async_save_account<'a>(
        conn: Arc<Mutex<Box<Connection>>>,
        account: &'a mut Account,
        group_name: &str,
    ) -> Result<&'a mut Account, LoadError> {
        let conn = conn.lock().unwrap();

        Self::save_account(&conn, account, group_name)
    }

    pub fn save_account<'a>(
        conn: &Connection,
        account: &'a mut Account,
        group_name: &str,
    ) -> Result<&'a mut Account, LoadError> {
        let group = Self::get_or_create_group(conn, group_name).unwrap();

        conn.execute(
            "INSERT INTO accounts (label, group_id, secret) VALUES (?1, ?2, ?3)",
            params![account.label, group.id, account.secret],
        )
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))?;

        let mut stmt = conn
            .prepare("SELECT last_insert_rowid()")
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))?;

        stmt.query_row(NO_PARAMS, |row| row.get::<usize, u32>(0))
            .map(|id| {
                account.id = id;
                account.group_id = group.id;
                account
            })
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    fn init_tables(conn: &Connection) -> Result<usize, rusqlite::Error> {
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

    fn get_accounts(conn: &Connection, group_id: u32) -> Result<Vec<Account>, rusqlite::Error> {
        let mut _stmt =
            conn.prepare("SELECT id, label, secret FROM accounts WHERE group_id = ?1")?;

        let accounts_iter = _stmt.query_map(params![group_id], |row| {
            let id: u32 = row.get(0)?;
            let group_id: u32 = group_id;
            let label: String = row.get(1)?;
            let secret: String = row.get(2)?;

            let mut account = Account::new(group_id, label.as_str(), secret.as_str());
            account.id = id;
            Ok(account)
        })?;

        Ok(accounts_iter.map(|x| x.unwrap()).collect())
    }

    pub async fn _deserialise(path: &Path) -> Result<ConfigManager, LoadError> {
        let accounts = async_std::fs::read_to_string(path)
            .await
            .map_err(|_| LoadError::FileError)?;

        serde_json::from_str(&accounts)
            .map(|mut cm: ConfigManager| {
                // sort groups
                cm.groups
                    .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                cm
            })
            .map(|mut cm: ConfigManager| {
                // sort entries in each group
                cm.groups.iter_mut().for_each(|account_group| {
                    account_group
                        .entries
                        .sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
                });

                cm
            })
            .map_err(|_| LoadError::FormatError)
    }

    pub async fn _write<C: ToString>(path: &Path, contents: C) -> Result<(), LoadError> {
        async_std::fs::write(path, contents.to_string())
            .await
            .map_err(|_| LoadError::SaveError)
    }

    pub async fn _write_config(config_manager: ConfigManager) -> Result<(), LoadError> {
        let value = serde_json::to_value(config_manager).unwrap();

        Self::_write(&Self::path(), value).await
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::{Account, AccountGroup};

    use super::ConfigManager;
    use rusqlite::Connection;
    use std::path::Path;

    use async_std::task;

    #[test]
    fn create_new_account_and_new_group() {
        let conn = Connection::open_in_memory().unwrap();

        let _ = ConfigManager::init_tables(&conn);

        let mut account = Account::new(0, "label", "secret");

        let result = ConfigManager::save_account(&conn, &mut account, "group name")
            .unwrap()
            .clone();

        assert!(result.id > 0);
        assert!(result.group_id > 0);
        assert_eq!("label", result.label);
    }

    #[test]
    fn create_new_account_with_existing_group() {
        let conn = Connection::open_in_memory().unwrap();

        let _ = ConfigManager::init_tables(&conn);

        let group = ConfigManager::create_group(&conn, "existing_group2").unwrap();

        let mut account = Account::new(group.id, "label", "secret");

        let result = ConfigManager::save_account(&conn, &mut account, "existing_group2")
            .unwrap()
            .clone();

        assert!(result.id > 0);
        assert_eq!(group.id, result.group_id);

        let reloaded_group = ConfigManager::get_or_create_group(&conn, "existing_group2").unwrap();
        assert_eq!(group.id, reloaded_group.id);
        assert_eq!("existing_group2", reloaded_group.name);
        assert_eq!(vec![account], reloaded_group.entries);
    }

    #[test]
    fn get_or_create_group_with_new_group() {
        let conn = Connection::open_in_memory().unwrap();

        let _ = ConfigManager::init_tables(&conn);

        let group = ConfigManager::get_or_create_group(&conn, "existing_group2").unwrap();

        assert!(group.id > 0);
        assert_eq!("existing_group2", group.name);
    }

    #[test]
    fn serializing_then_deserialising_accounts() {
        let account = Account::new(1, "label", "secret");

        let mut groups = AccountGroup::new(1, "name", vec![]);
        groups._add(account);

        let config_manager = ConfigManager {
            groups: vec![groups.clone()], //cloning to helping with assert_eq! on itself further down
        };

        let value = serde_json::to_value(config_manager).unwrap();
        assert_eq!("{\"groups\":[{\"entries\":[{\"group_id\":1,\"id\":0,\"label\":\"label\",\"secret\":\"secret\"}],\"id\":1,\"name\":\"name\"}]}", value.to_string());

        let destination = Path::new("test-serialisation.json");

        let write_result = task::block_on(ConfigManager::_write(&destination, value));
        assert_eq!(Ok(()), write_result);

        let mut config_manager: ConfigManager =
            task::block_on(ConfigManager::_deserialise(&destination)).unwrap();

        config_manager.groups.iter_mut().for_each(|x| x.update());

        assert_eq!(vec![groups], config_manager.groups);
    }
}
