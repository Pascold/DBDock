//! Locations of everything DBDock writes to disk.

use std::path::{Path, PathBuf};

use crate::engines::EngineId;

/// Environment variable that overrides the data directory (useful for tests
/// and portable setups).
pub const HOME_ENV: &str = "DBDOCK_HOME";

/// All files live in one directory: `config.toml`, the generated
/// `docker-compose.yml`, seed scripts and the SQLite folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    base: PathBuf,
}

impl AppPaths {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    /// `DBDOCK_HOME` if set, otherwise the platform config directory
    /// (`%APPDATA%\DBDock\config`, `~/.config/dbdock`,
    /// `~/Library/Application Support/DBDock`).
    pub fn detect() -> Option<Self> {
        if let Some(home) = std::env::var_os(HOME_ENV).filter(|v| !v.is_empty()) {
            return Some(Self::new(home));
        }
        directories::ProjectDirs::from("", "", "DBDock").map(|d| Self::new(d.config_dir()))
    }

    pub fn base(&self) -> &Path {
        &self.base
    }

    pub fn config_file(&self) -> PathBuf {
        self.base.join("config.toml")
    }

    pub fn compose_file(&self) -> PathBuf {
        self.base.join("docker-compose.yml")
    }

    pub fn seed_dir(&self) -> PathBuf {
        self.base.join("seed")
    }

    pub fn seed_file(&self, id: EngineId) -> PathBuf {
        self.seed_dir().join(format!("{}.sql", id.key()))
    }

    pub fn sqlite_dir(&self) -> PathBuf {
        self.base.join("sqlite")
    }

    /// The user's own `.sql` files for an engine, run when its data is
    /// created (after the sample data).
    pub fn scripts_dir(&self, id: EngineId) -> PathBuf {
        self.base.join("scripts").join(id.key())
    }

    /// Generated `/docker-entrypoint-initdb.d` contents of an engine: the
    /// seed and copies of the user's scripts, rebuilt before every start.
    pub fn init_dir(&self, id: EngineId) -> PathBuf {
        self.base.join("init").join(id.key())
    }

    /// Saved copies of an engine's data, see [`crate::snapshots`].
    pub fn snapshots_dir(&self, id: EngineId) -> PathBuf {
        self.base.join("snapshots").join(id.key())
    }
}
