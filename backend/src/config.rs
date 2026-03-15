use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub auth: AuthConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StorageConfig {
    pub lancedb_path: PathBuf,
    pub sqlite_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthConfig {
    pub runtime: TokenConfig,
    pub admin: TokenConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TokenConfig {
    pub token: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let cfg: Self = toml::from_str(&raw)
            .with_context(|| format!("failed to parse TOML config: {}", path.display()))?;
        cfg.validate()
            .with_context(|| format!("invalid backend config loaded from {}", path.display()))?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        if self.server.bind.trim().is_empty() {
            anyhow::bail!("server.bind cannot be empty");
        }
        if self.auth.runtime.token.trim().is_empty() {
            anyhow::bail!("auth.runtime.token cannot be empty");
        }
        if self.auth.admin.token.trim().is_empty() {
            anyhow::bail!("auth.admin.token cannot be empty");
        }
        if self.storage.sqlite_path.as_os_str().is_empty() {
            anyhow::bail!("storage.sqlite_path cannot be empty");
        }
        if self.storage.lancedb_path.as_os_str().is_empty() {
            anyhow::bail!("storage.lancedb_path cannot be empty");
        }
        Ok(())
    }
}
