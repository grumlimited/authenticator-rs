use log::{info, warn};
use rusqlite::types::ToSqlOutput;
use rusqlite::{named_params, params, Connection, OpenFlags, OptionalExtension, Params, Row, Statement, ToSql};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::str::FromStr;
use std::string::ToString;
use strum_macros::Display;
use strum_macros::EnumString;

use crate::helpers::repository_error::RepositoryError;
use crate::helpers::Paths;
use crate::helpers::SecretType::{KEYRING, LOCAL};
use crate::model::{Account, AccountGroup};

#[derive(Debug, Clone)]
pub struct Database;

#[derive(Debug, Eq, PartialEq, EnumString, Serialize, Deserialize, Clone, Display, Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum SecretType {
    LOCAL,
    #[default]
    KEYRING,
}

type Result<T> = rusqlite::Result<T, RepositoryError>;

impl Database {
    pub fn has_groups(connection: &Connection) -> Result<bool> {
        let mut stmt = connection.prepare("SELECT COUNT(*) FROM groups")?;

        stmt.query_row(params![], |row| {
            let count: u32 = row.get_unwrap(0);
            Ok(count)
        })
        .map(|count| count > 0)
        .map_err(RepositoryError::SqlError)
    }

    pub fn load_account_groups(connection: &Connection, filter: Option<&str>) -> Result<Vec<AccountGroup>> {
        let mut stmt = connection.prepare("SELECT id, name, icon, url, collapsed FROM groups ORDER BY LOWER(name)")?;

        let row_iter = stmt.query_map(params![], |row| {
            let id = row.get_unwrap(0);
            let name: String = row.get_unwrap(1);
            let icon: Option<String> = row.get(2).optional().unwrap_or(None);
            let url: Option<String> = row.get(3).optional().unwrap_or(None);
            let collapsed: bool = row.get_unwrap(4);

            let entries = Self::get_accounts(connection, id, filter).map_err(|_| rusqlite::Error::InvalidQuery)?;

            Ok(AccountGroup::new(id, name.as_str(), icon.as_deref(), url.as_deref(), collapsed, entries))
        })?;

        let account_groups = row_iter
            .flatten()
            .filter(|account_group| !account_group.entries.is_empty() || filter.is_none())
            .collect::<Vec<AccountGroup>>();

        Ok(account_groups)
    }

    pub fn create_connection() -> Result<Connection> {
        Connection::open_with_flags(Paths::db_path(), OpenFlags::default()).map_err(RepositoryError::SqlError)
    }

    pub fn update_group(connection: &Connection, group: &AccountGroup) -> Result<()> {
        info!("Updating group {}", group.name);
        connection
            .execute(
                "UPDATE groups SET name = ?2, icon = ?3, url = ?4, collapsed = ?5 WHERE id = ?1",
                params![group.id, group.name, group.icon, group.url, group.collapsed],
            )
            .map(|_| ())
            .map_err(RepositoryError::SqlError)
    }

    pub fn save_group(connection: &Connection, group: &mut AccountGroup) -> Result<()> {
        info!("Adding group {}", group.name);

        connection.execute(
            "INSERT INTO groups (name, icon, url, collapsed) VALUES (?1, ?2, ?3, ?4)",
            params![group.name, group.icon, group.url, group.collapsed],
        )?;

        let mut stmt = connection.prepare("SELECT last_insert_rowid()")?;

        stmt.query_row([], |row| row.get(0))
            .map(|id| {
                group.id = id;
            })
            .map_err(RepositoryError::SqlError)
    }

    fn group_by_name(connection: &Connection, name: &str) -> Result<Option<AccountGroup>> {
        let mut stmt = connection.prepare("SELECT id, name, icon, url, collapsed FROM groups WHERE name = :name")?;

        stmt.query_row(named_params! {":name": name}, |row| {
            let group_id = row.get_unwrap(0);
            let group_name: String = row.get_unwrap(1);
            let group_icon: Option<String> = row.get(2).optional().unwrap_or(None);
            let group_url: Option<String> = row.get(3).optional().unwrap_or(None);
            let collapsed: bool = row.get_unwrap(4);

            Ok(AccountGroup::new(
                group_id,
                group_name.as_str(),
                group_icon.as_deref(),
                group_url.as_deref(),
                collapsed,
                vec![],
            ))
        })
        .optional()
        .map_err(RepositoryError::SqlError)
    }

    pub fn save_group_and_accounts(connection: &Connection, group: &mut AccountGroup) -> Result<()> {
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
                    Self::upsert_account(connection, account)
                })
                .collect::<Result<Vec<u32>>>()
                .map(|_| ()),
            Err(group_saved_error) => Err(group_saved_error),
        }
    }

    pub fn group_exists(connection: &Connection, name: &str) -> Result<Option<u32>> {
        let mut stmt = connection.prepare("SELECT id FROM groups WHERE name = :name")?;

        stmt.query_row(
            named_params! {
            ":name": name
            },
            |row| {
                let group_id: u32 = row.get_unwrap(0);
                Ok(group_id)
            },
        )
        .optional()
        .map_err(RepositoryError::SqlError)
    }

    pub fn account_exists(connection: &Connection, name: &str, group_id: u32) -> Result<Option<u32>> {
        let mut stmt = connection.prepare("SELECT id FROM accounts WHERE label = :label AND group_id = :group_id")?;

        stmt.query_row(
            named_params! {
            ":label": name,
            ":group_id": group_id,
            },
            |row| {
                let account_id: u32 = row.get_unwrap(0);
                Ok(account_id)
            },
        )
        .optional()
        .map_err(RepositoryError::SqlError)
    }

    pub fn get_group(connection: &Connection, group_id: u32) -> Result<AccountGroup> {
        let mut stmt = connection.prepare("SELECT id, name, icon, url, collapsed FROM groups WHERE id = :group_id")?;

        stmt.query_row(
            named_params! {
            ":group_id": group_id
            },
            |row| {
                let group_id: u32 = row.get_unwrap(0);
                let group_name: String = row.get_unwrap(1);
                let group_icon: Option<String> = row.get(2).optional().unwrap_or(None);
                let group_url: Option<String> = row.get(3).optional().unwrap_or(None);
                let collapsed: bool = row.get_unwrap(4);

                let accounts = match Self::get_accounts(connection, group_id, None) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("Error getting accounts for group {}: {:?}", group_id, e);
                        vec![]
                    }
                };

                row.get(0)
                    .map(|id| AccountGroup::new(id, group_name.as_str(), group_icon.as_deref(), group_url.as_deref(), collapsed, accounts))
            },
        )
        .map_err(RepositoryError::SqlError)
    }

    pub fn upsert_account(connection: &Connection, account: &mut Account) -> Result<u32> {
        match Self::get_account_by_name(connection, account.label.as_str()).unwrap() {
            Some(a) => {
                account.id = a.id;
                account.secret_type = LOCAL; // so that keyring get updated too
                Self::update_account(connection, account)
            }
            None => Self::save_account(connection, account),
        }
    }

    pub fn save_account(connection: &Connection, account: &mut Account) -> Result<u32> {
        info!("Adding account {}", account.label);
        let secret = if account.secret_type == KEYRING { "" } else { account.secret.as_str() };

        connection
            .execute(
                "INSERT INTO accounts (label, group_id, secret, secret_type) VALUES (?1, ?2, ?3, ?4)",
                params![account.label, account.group_id, secret, account.secret_type],
            )
            .map_err(RepositoryError::SqlError)?;

        let mut stmt = connection.prepare("SELECT last_insert_rowid()")?;

        let result = stmt.query_row([], |row| row.get(0)).map_err(RepositoryError::SqlError);
        result.iter().for_each(|id| account.id = *id);

        result
    }

    pub fn update_account(connection: &Connection, account: &mut Account) -> Result<u32> {
        info!("Updating account [{}:{}]", account.label, account.id);
        let secret = if account.secret_type == KEYRING { "" } else { account.secret.as_str() };

        connection
            .execute(
                "UPDATE accounts SET label = ?2, secret = ?3, group_id = ?4, secret_type = ?5 WHERE id = ?1",
                params![account.id, account.label, secret, account.group_id, account.secret_type],
            )
            .map(|_| account.id)
            .map_err(RepositoryError::SqlError)
    }

    pub fn get_account(connection: &Connection, account_id: u32) -> Result<Option<Account>> {
        let stmt = connection.prepare("SELECT id, group_id, label, secret, secret_type FROM accounts WHERE id = ?1")?;
        Self::_get_account(stmt, params![account_id])
    }

    pub fn get_account_by_name(connection: &Connection, name: &str) -> Result<Option<Account>> {
        let stmt = connection.prepare("SELECT id, group_id, label, secret, secret_type FROM accounts WHERE label = ?1")?;
        Self::_get_account(stmt, params![name])
    }

    fn _get_account<T: Params>(mut statement: Statement, params: T) -> Result<Option<Account>> {
        statement
            .query_row(params, |row| {
                let group_id: u32 = row.get_unwrap(1);
                let label: String = row.get_unwrap(2);
                let secret: String = row.get_unwrap(3);
                let id = row.get_unwrap(0);

                let secret_type = Self::extract_secret_type(row, 4);

                let account = Account::new(id, group_id, label.as_str(), secret.as_str(), secret_type?);

                Ok(account)
            })
            .optional()
            .map_err(RepositoryError::SqlError)
    }

    fn extract_secret_type(row: &Row, idx: usize) -> rusqlite::Result<SecretType, rusqlite::Error> {
        match row.get::<usize, String>(idx) {
            Ok(v) => match SecretType::from_str(v.as_str()) {
                Ok(secret_type) => Ok(secret_type),
                Err(_) => {
                    warn!("Invalid secret type [{}]", v);
                    Ok(LOCAL)
                }
            },
            Err(e) => Err(e),
        }
    }

    pub fn delete_group(connection: &Connection, group_id: u32) -> Result<usize> {
        let mut stmt = connection.prepare("DELETE FROM groups WHERE id = ?1").unwrap();

        stmt.execute(params![group_id]).map_err(RepositoryError::SqlError)
    }

    pub fn delete_account(connection: &Connection, account_id: u32) -> Result<usize> {
        let mut stmt = connection.prepare("DELETE FROM accounts WHERE id = ?1").unwrap();

        stmt.execute(params![account_id]).map_err(RepositoryError::SqlError)
    }

    fn get_accounts(connection: &Connection, group_id: u32, filter: Option<&str>) -> Result<Vec<Account>> {
        let mut stmt = connection.prepare("SELECT id, label, secret, secret_type FROM accounts WHERE group_id = ?1 AND label LIKE ?2 ORDER BY LOWER(label)")?;

        let label_filter = filter.map(|f| format!("%{}%", f)).unwrap_or_else(|| "%".to_owned());

        stmt.query_map(params![group_id, label_filter], |row| {
            let id: u32 = row.get_unwrap(0);
            let label: String = row.get_unwrap(1);

            let secret_type = Self::extract_secret_type(row, 3);

            let secret: String = row.get_unwrap(2);

            let account = Account::new(id, group_id, label.as_str(), secret.as_str(), secret_type?);
            Ok(account)
        })
        .map(|rows| rows.map(|row| row.unwrap()).collect())
        .map_err(RepositoryError::SqlError)
    }
}

impl ToSql for SecretType {
    #[inline]
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::helpers::runner;
    use crate::helpers::SecretType::LOCAL;
    use crate::model::{Account, AccountGroup};

    use super::Database;

    #[test]
    fn create_new_account_and_new_group() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "new group", None, None, false, vec![]);
        let mut account = Account::new(0, 0, "label", "secret", LOCAL);

        Database::save_group(&connection, &mut group).unwrap();

        account.group_id = group.id;

        Database::save_account(&connection, &mut account).unwrap();

        assert!(account.id > 0);
        assert!(account.group_id > 0);
        assert_eq!("label", account.label);

        let mut account_reloaded = Database::get_account(&connection, account.id).unwrap().unwrap();

        assert_eq!(account, account_reloaded);

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

        let mut group = AccountGroup::new(0, "new group", None, None, false, vec![]);

        Database::save_group(&connection, &mut group).unwrap();

        assert_eq!("new group", group.name);

        group.name = "other name".to_owned();
        group.url = Some("url".to_owned());
        group.icon = Some("icon".to_owned());

        Database::update_group(&connection, &group).unwrap();

        let group = Database::get_group(&connection, group.id).unwrap();

        assert_eq!("other name", group.name);
        assert_eq!("url", group.url.unwrap());
        assert_eq!("icon", group.icon.unwrap());
    }

    #[test]
    fn create_new_account_with_existing_group() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "existing_group2", None, None, false, vec![]);

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

        let mut group = AccountGroup::new(0, "bbb", Some("icon"), Some("url"), false, vec![]);
        Database::save_group(&connection, &mut group).unwrap();

        let mut account1 = Account::new(0, group.id, "hhh", "secret3", LOCAL);
        Database::save_account(&connection, &mut account1).expect("boom!");

        let expected = AccountGroup::new(
            1,
            "bbb",
            Some("icon"),
            Some("url"),
            false,
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

        let mut group = AccountGroup::new(0, "bbb", None, None, false, vec![]);
        Database::save_group(&connection, &mut group).unwrap();

        let mut account = Account::new(0, group.id, "hhh", "secret3", LOCAL);
        Database::save_account(&connection, &mut account).expect("boom!");
        let mut account = Account::new(0, group.id, "ccc", "secret3", LOCAL);
        Database::save_account(&connection, &mut account).expect("boom!");

        let mut group = AccountGroup::new(0, "AAA", None, None, false, vec![]);
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

        let mut group = AccountGroup::new(0, "bbb", None, None, false, vec![]);
        Database::save_group(&connection, &mut group).unwrap();

        let mut account = Account::new(0, group.id, "hhh", "secret3", LOCAL);
        Database::save_account(&connection, &mut account).expect("boom!");

        let result = Database::has_groups(&connection).unwrap();
        assert!(result);
    }

    #[test]
    fn group_exists() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut group = AccountGroup::new(0, "bbb", None, None, false, vec![]);
        Database::save_group(&connection, &mut group).unwrap();

        let result = Database::group_exists(&connection, "bbb").unwrap();
        assert!(result.is_some());

        let result = Database::group_exists(&connection, "non_existent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn account_exists() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let mut account = Account::new(0, 1, "label", "secret", LOCAL);
        let _ = Database::save_account(&connection, &mut account);

        let result = Database::account_exists(&connection, "label", 1).unwrap();
        assert!(result.is_some());

        let result = Database::account_exists(&connection, "non_existent", 1).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn save_group_and_accounts() {
        let mut connection = Connection::open_in_memory().unwrap();

        runner::run(&mut connection).unwrap();

        let account1 = Account::new(0, 0, "label", "secret", LOCAL);
        let account2 = Account::new(0, 0, "label2", "secret2", LOCAL);
        let mut account_group = AccountGroup::new(0, "group", None, None, false, vec![account1, account2]);

        Database::save_group_and_accounts(&connection, &mut account_group).expect("could not save");

        assert!(account_group.id > 0);
        assert_eq!(2, account_group.entries.len());
        assert!(account_group.entries.first().unwrap().id > 0);

        // saving sames accounts a second time should not produce duplicates
        Database::save_group_and_accounts(&connection, &mut account_group).expect("could not save");

        let accounts = Database::get_accounts(&connection, account_group.id, None).unwrap();
        assert_eq!(2, account_group.entries.len());
        assert_eq!(2, accounts.len());
    }
}
