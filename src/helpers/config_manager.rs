use serde::{Deserialize, Serialize};

use crate::ui::AccountGroup;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigManager {
    pub groups: Vec<AccountGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoadError {
    FileError,
    FormatError,
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

    pub async fn load() -> Result<ConfigManager, LoadError> {
        Self::load_from_path(&Self::path()).await
    }

    pub async fn load_from_path(path: &Path) -> Result<ConfigManager, LoadError> {
        let accounts = async_std::fs::read_to_string(path)
            .await
            .map_err(|_| LoadError::FileError)?;

        serde_json::from_str(&accounts).map_err(|_| LoadError::FormatError)
    }

    pub async fn _write<C: ToString>(path: &Path, contents: C) -> Result<(), LoadError> {
        async_std::fs::write(path, contents.to_string())
            .await
            .map_err(|_| LoadError::FileError)
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
        let account = Account::_new("label", "secret");

        let mut groups = AccountGroup::_new("name");
        groups._add(account);

        let config_manager = ConfigManager {
            groups: vec![groups.clone()], //cloning to helping with assert_eq! on itself further down
        };

        let value = serde_json::to_value(config_manager).unwrap();
        assert_eq!("{\"groups\":[{\"entries\":[{\"label\":\"label\",\"secret\":\"secret\"}],\"name\":\"name\"}]}", value.to_string());

        let destination = Path::new("test-serialisation.json");

        let write_result = task::block_on(ConfigManager::_write(&destination, value));
        assert_eq!(Ok(()), write_result);

        let config_manager = task::block_on(ConfigManager::load_from_path(&destination)).unwrap();

        assert_eq!(vec![groups], config_manager.groups);
    }
}
