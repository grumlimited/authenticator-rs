pub struct Paths;

use log::debug;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub enum PathError {
    WriteError(#[from] io::Error),
}

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

    pub fn check_configuration_dir() -> Result<(), PathError> {
        let path = Paths::path();

        if !path.exists() {
            debug!("Creating directory {}", path.display());
        }

        std::fs::create_dir_all(path).map(|_| ()).map_err(PathError::WriteError)?;

        let path = Paths::icons_path("");

        if !path.exists() {
            debug!("Creating directory {}", path.display());
        }

        std::fs::create_dir_all(path).map(|_| ()).map_err(PathError::WriteError)
    }
}
