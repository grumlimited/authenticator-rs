pub struct Paths;

use anyhow::Result;
use log::debug;

impl Paths {
    pub fn db_path() -> std::path::PathBuf {
        let mut path = Self::path();
        path.push("authenticator.db");

        path
    }

    pub fn icons_path(filename: &str) -> std::path::PathBuf {
        let mut path = Self::path();
        path.push("icons");
        path.push(filename);

        path
    }

    pub fn path() -> std::path::PathBuf {
        if let Some(project_dirs) = directories::ProjectDirs::from("uk.co", "grumlimited", "authenticator-rs") {
            project_dirs.data_dir().into()
        } else {
            std::env::current_dir().unwrap_or_default()
        }
    }

    pub fn check_configuration_dir() -> Result<()> {
        let path = Paths::path();

        if !path.exists() {
            debug!("Creating directory {}", path.display());
        }

        std::fs::create_dir_all(path).map(|_| ())?;

        let path = Paths::icons_path("");

        if !path.exists() {
            debug!("Creating directory {}", path.display());
        }

        std::fs::create_dir_all(path).map(|_| ())?;

        Ok(())
    }
}
