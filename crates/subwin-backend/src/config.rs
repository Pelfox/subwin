use std::path::PathBuf;

use directories::ProjectDirs;
use subwin_bridge::config::Config;
use tokio::{
    fs::{OpenOptions, create_dir_all, read_to_string},
    io::AsyncWriteExt,
};

// TODO: add migrations for config files.

/// Errors that can occur while loading or resolving application configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Failed to determine the user's configuration or data directories. This
    /// usually occurs when required environment variables are missing (e.g.,
    /// `$HOME` on Unix or `%APPDATA%` on Windows).
    #[error("failed to obtain user's directories")]
    DirectoriesNotFound,
    /// An I/O error occurred while reading or writing the configuration file.
    #[error("failed to read config: {0}")]
    IoError(#[from] std::io::Error),
    /// The configuration file contains invalid TOML or does not match the expected structure.
    #[error("failed to deserialize config: {0}")]
    DeserializeError(#[from] toml::de::Error),
    /// Failed to serialize the configuration to TOML (e.g., when saving changes).
    #[error("failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),
}

fn build_project_dirs() -> Result<(PathBuf, PathBuf), ConfigError> {
    match ProjectDirs::from("dev", "pelfox", "subwin") {
        Some(path) => Ok((
            path.config_dir().to_path_buf(),
            path.cache_dir().to_path_buf(),
        )),
        None => Err(ConfigError::DirectoriesNotFound),
    }
}

/// Loads the application configuration from disk. Returns the loaded config,
/// as well as path to the cache directory.
pub async fn load_config() -> Result<(Config, PathBuf), ConfigError> {
    let (config_dir, cache_dir) = build_project_dirs()?;

    let config_path = config_dir.join("config.toml");
    log::info!("Loading configuration from {config_path:?}");
    if config_path.exists() {
        let contents = read_to_string(config_path).await?;
        let config: Config = toml::from_str(&contents)?;
        return Ok((config, cache_dir));
    }

    let config = Config::default();
    if let Some(parent) = config_path.parent() {
        create_dir_all(parent).await?;
    }

    let contents = toml::to_string_pretty(&config)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(config_path)
        .await?;
    file.write_all(contents.as_bytes()).await?;
    file.sync_all().await?;

    Ok((config, cache_dir))
}

/// Saves the current configuration to disk. This function serializes the
/// provided `Config` to pretty-printed TOML and writes it to `config.toml` in
/// the user's configuration directory, overwriting any existing file.
pub async fn save_config(config: &Config) -> Result<(), ConfigError> {
    let (config_dir, _) = build_project_dirs()?;

    let config_path = config_dir.join("config.toml");
    if let Some(parent) = config_path.parent() {
        create_dir_all(parent).await?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(config_path)
        .await?;

    let contents = toml::to_string_pretty(&config)?;
    file.write_all(contents.as_bytes()).await?;
    file.sync_all().await?;

    Ok(())
}
