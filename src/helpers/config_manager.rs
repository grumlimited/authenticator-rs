use serde::{Deserialize, Serialize};

use rusqlite::{params, Connection, MappedRows, OpenFlags, Result, Row, NO_PARAMS};

use crate::ui::{Account, AccountGroup};
use std::error::Error;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigManager {
    pub groups: Vec<AccountGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoadError {
    FileError,
    FormatError,
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

        path.push("authenticator.json");

        path
    }

    fn path2() -> std::path::PathBuf {
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

    pub async fn load() -> Result<ConfigManager, LoadError> {
        Self::load_from_path(&Self::path()).await
    }

    pub async fn load_from_path(path: &Path) -> Result<ConfigManager, LoadError> {
        let conn = Connection::open_with_flags(&Self::path2(), OpenFlags::default())
            .map_err(|e| LoadError::DbError(format!("{:?}", e)))?;

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
        .map_err(|e| LoadError::DbError(format!("{:?}", e)))?;

            let mut _stmt = conn.prepare("SELECT id, name FROM groups").unwrap();

            let groups_iter = _stmt
                .query_map(params![], |row| {
                    let id: u32 = row.get(0)?;
                    let name: String = row.get(1)?;

                    let mut _stmt = conn
                        .prepare("SELECT id, label, group_id, secret FROM accounts")
                        .unwrap();

                    let accounts_iter = _stmt
                        .query_map(params![], |row| {
                            let id: u32 = row.get(0)?;
                            let group_id: u32 = row.get(2)?;
                            let label: String = row.get(1)?;
                            let secret: String = row.get(3)?;

                            Ok(Account::new(id, group_id, label.as_str(), secret.as_str()))
                        })
                        .unwrap();

                    Ok(AccountGroup::new(
                        id,
                        name.as_str(),
                        accounts_iter.map(|x| x.unwrap()).collect(),
                    ))
                })
                .unwrap();

            let groups: Vec<AccountGroup> = groups_iter.map(|x| x.unwrap()).collect();

            Ok(ConfigManager {
                groups
            })
    }

    pub async fn write<C: ToString>(path: &Path, contents: C) -> Result<(), LoadError> {
        async_std::fs::write(path, contents.to_string())
            .await
            .map_err(|_| LoadError::SaveError)
    }

    pub async fn write_config(config_manager: ConfigManager) -> Result<(), LoadError> {
        let value = serde_json::to_value(config_manager).unwrap();

        Self::write(&Self::path(), value).await
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use async_std::task;

    use crate::ui::Account;
    use crate::ui::AccountGroup;

    use super::ConfigManager;

    #[test]
    fn serializing_then_deserialazing_accounts() {
        let account = Account::new("label", "secret");

        let mut groups = AccountGroup::new("name");
        groups.add(account);

        let config_manager = ConfigManager {
            groups: vec![groups.clone()], //cloning to helping with assert_eq! on itself further down
        };

        let value = serde_json::to_value(config_manager).unwrap();
        assert_eq!("{\"groups\":[{\"entries\":[{\"label\":\"label\",\"secret\":\"secret\"}],\"name\":\"name\"}]}", value.to_string());

        let destination = Path::new("test-serialisation.json");

        let write_result = task::block_on(ConfigManager::write(&destination, value));
        assert_eq!(Ok(()), write_result);

        let mut config_manager: ConfigManager =
            task::block_on(ConfigManager::load_from_path(&destination)).unwrap();

        config_manager.groups.iter_mut().for_each(|x| x.update());

        assert_eq!(vec![groups], config_manager.groups);
    }
}
