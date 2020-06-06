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

    pub async fn write<C: std::string::ToString>(path: &Path, contents: C) -> Result<(), LoadError> {
        async_std::fs::write(path, contents.to_string()).await.map_err(|_| LoadError::FileError)
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
            groups: vec![groups],
        };

        let value = serde_json::to_value(config_manager).unwrap();
        assert_eq!("{\"groups\":[{\"entries\":[{\"label\":\"label\",\"secret\":\"secret\"}],\"name\":\"name\"}]}", value.to_string());

        let destination = directories::UserDirs::new().unwrap().home_dir().join(Path::new("test-serialisation.json"));

        let write_result = task::block_on(ConfigManager::write(&destination, value));
        assert_eq!(Ok(()), write_result);
    }
}
