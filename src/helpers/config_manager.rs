use serde::{Deserialize, Serialize};

use crate::ui::AccountGroup;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigManager {
    pub groups: Vec<AccountGroup>,
}

#[derive(Debug, Clone)]
pub enum LoadError {
    FileError,
    FormatError,
}

impl ConfigManager {
    fn path() -> std::path::PathBuf {
        let mut path = if let Some(project_dirs) = directories::ProjectDirs::from("uk.co", "grumlimited", "authenticator-rs") {
            project_dirs.data_dir().into()
        } else {
            std::env::current_dir().unwrap_or(std::path::PathBuf::new())
        };

        path.push("authenticator.json");

        path
    }

    pub async fn load() -> Result<ConfigManager, LoadError> {
        let accounts = async_std::fs::read_to_string(Self::path()).await.map_err(|_| LoadError::FileError)?;

        serde_json::from_str(&accounts).map_err(|_| LoadError::FormatError)
    }
}
