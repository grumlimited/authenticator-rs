use rusqlite::{named_params, params, Connection, OpenFlags, Result, NO_PARAMS};

use crate::model::{Account, AccountGroup};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
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
    pub fn _log4rs() -> std::path::PathBuf {
        let mut path = ConfigManager::path();
        path.push("log4rs.yaml");

        path
    }

    fn db_path() -> std::path::PathBuf {
        let mut path = ConfigManager::path();
        path.push("authenticator.db");

        path
    }

    fn path() -> std::path::PathBuf {
        if let Some(project_dirs) =
            directories::ProjectDirs::from("uk.co", "grumlimited", "authenticator-rs")
        {
            project_dirs.data_dir().into()
        } else {
            std::env::current_dir().unwrap_or_default()
        }
    }

    pub fn load_account_groups(conn: &Connection) -> Result<Vec<AccountGroup>, LoadError> {
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
        Connection::open_with_flags(Self::db_path(), OpenFlags::default())
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    fn _create_group(conn: &Connection, group_name: &str) -> Result<AccountGroup, LoadError> {
        conn.execute("INSERT INTO groups (name) VALUES (?1)", params![group_name])
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
            .unwrap();

        let mut stmt = conn.prepare("SELECT last_insert_rowid()").unwrap();

        stmt.query_row(NO_PARAMS, |row| row.get::<usize, u32>(0))
            .map(|id| AccountGroup {
                id,
                name: group_name.to_owned(),
                ..Default::default()
            })
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub async fn _async_get_group(
        conn: Arc<Mutex<Box<Connection>>>,
        group_id: u32,
    ) -> Result<AccountGroup, LoadError> {
        let conn = conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name FROM groups WHERE id = ?1")
            .unwrap();

        stmt.query_row(params![group_id], |group_row| {
            let group_name: String = group_row.get_unwrap(1);

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

            Ok(AccountGroup::new(group_id, group_name.as_str(), accounts))
        })
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn _get_or_create_group(
        conn: &Connection,
        group_name: &str,
    ) -> Result<AccountGroup, LoadError> {
        let mut stmt = conn
            .prepare("SELECT id FROM groups WHERE name = :name")
            .unwrap();

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
                        .unwrap();

                    let accounts = stmt
                        .query_map(params![group_id], |row| {
                            let label = row.get_unwrap::<usize, String>(1);
                            let secret = row.get_unwrap::<usize, String>(3);
                            let id = row.get(0)?;
                            let account =
                                Account::new(id, group_id, label.as_str(), secret.as_str());

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

        group.or_else(|_| Self::_create_group(conn, group_name))
    }

    pub fn _save_account<'a>(
        conn: &Connection,
        account: &'a mut Account,
        group_name: &str,
    ) -> Result<&'a mut Account, LoadError> {
        let group = Self::_get_or_create_group(conn, group_name).unwrap();

        conn.execute(
            "INSERT INTO accounts (label, group_id, secret) VALUES (?1, ?2, ?3)",
            params![account.label, group.id, account.secret],
        )
        .unwrap();

        let mut stmt = conn.prepare("SELECT last_insert_rowid()").unwrap();

        stmt.query_row(NO_PARAMS, |row| row.get::<usize, u32>(0))
            .map(|id| {
                account.id = id;
                account.group_id = group.id;
                account
            })
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))
    }

    pub fn update_account(
        connection: Arc<Mutex<Connection>>,
        account: &mut Account,
    ) -> Result<&mut Account, LoadError> {
        let conn = connection.lock().unwrap();

        conn.execute(
            "UPDATE accounts SET label = ?2, secret = ?3, group_id = ?4 WHERE id = ?1",
            params![account.id, account.label, account.secret, account.group_id],
        )
        .map(|_| account)
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
        let mut stmt =
            conn.prepare("SELECT id, label, secret FROM accounts WHERE group_id = ?1")?;

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
}

#[cfg(test)]
mod tests {
    use crate::ui::{Account, AccountGroup};

    use super::ConfigManager;
    use rusqlite::Connection;
    use std::path::Path;

    use async_std::task;
    use std::sync::{Arc, Mutex};

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

        let account_reloaded = ConfigManager::get_account(&conn, account.id).unwrap();
        assert_eq!(account, account_reloaded);

        let mut account_reloaded = account_reloaded.clone();
        account_reloaded.label = "new label".to_owned();
        account_reloaded.secret = "new secret".to_owned();
        let account_reloaded = ConfigManager::update_account(&conn, &mut account_reloaded).unwrap();

        assert_eq!("new label", account_reloaded.label);
        assert_eq!("new secret", account_reloaded.secret);
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

        let conn = Arc::new(Mutex::new(Box::new(conn)));
        let group: AccountGroup =
            task::block_on(ConfigManager::_async_get_group(conn, group.id)).unwrap();

        assert!(group.id > 0);
        assert_eq!("existing_group2", group.name);
    }

    #[test]
    fn delete_account() {
        let conn = Connection::open_in_memory().unwrap();

        let _ = ConfigManager::init_tables(&conn);

        let mut account = Account::new(0, "label", "secret");

        let result = ConfigManager::save_account(&conn, &mut account, "group name")
            .unwrap()
            .clone();

        assert_eq!(1, result.id);

        let result = ConfigManager::delete_account(&conn, account.id).unwrap();
        assert!(result > 0);
    }
}
