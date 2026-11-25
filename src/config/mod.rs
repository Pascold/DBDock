//! User settings: model, defaults, migrations and persistence.

pub mod validate;

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::engines::EngineId;

/// Current schema version of `config.toml`.
pub const CONFIG_VERSION: u32 = 1;

const FILE_HEADER: &str = "\
# DBDock configuration.
# Passwords are stored in plain text on purpose: DBDock manages local
# test databases only. Never reuse production credentials here.

";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot access {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid configuration file: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("cannot serialize configuration: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error(
        "configuration version {0} is newer than this DBDock supports ({CONFIG_VERSION}); \
         please update DBDock"
    )]
    TooNew(i64),
}

/// UI language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Pl,
}

impl Language {
    /// Language matching a BCP 47 locale such as `pl-PL`, English otherwise.
    pub fn from_locale(locale: &str) -> Language {
        if locale.to_ascii_lowercase().starts_with("pl") {
            Language::Pl
        } else {
            Language::En
        }
    }
}

/// UI colour theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

/// Settings of one engine. Engines ignore the fields they do not use
/// (see [`crate::engines::Engine::fields`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineSettings {
    pub enabled: bool,
    /// Image tag.
    pub version: String,
    /// Port on the host (0 for engines without a server).
    pub port: u16,
    /// Database name (or file name for SQLite).
    pub database: String,
    pub user: String,
    pub password: String,
    /// Password of root / sa / SYSDBA.
    pub admin_password: String,
    /// Load the sample data on first start.
    pub seed: bool,
}

impl EngineSettings {
    /// True when the settings differ in a way that only takes effect on a
    /// freshly initialised data volume.
    pub fn credentials_differ(&self, other: &EngineSettings) -> bool {
        self.database != other.database
            || self.user != other.user
            || self.password != other.password
            || self.admin_password != other.admin_password
            || self.seed != other.seed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub config_version: u32,
    /// `None` means "follow the system language".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    #[serde(default)]
    pub theme: Theme,
    /// Set once the user confirmed the InterBase license registration.
    #[serde(default)]
    pub interbase_licensed: bool,
    pub engines: BTreeMap<EngineId, EngineSettings>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_version: CONFIG_VERSION,
            language: None,
            theme: Theme::default(),
            interbase_licensed: false,
            engines: EngineId::ALL
                .into_iter()
                .map(|id| (id, id.engine().default_settings()))
                .collect(),
        }
    }
}

impl AppConfig {
    /// Settings of `id`. Every engine is always present after loading.
    /// Loading fills in every engine; should one be missing anyway (the map
    /// is public), its factory defaults are returned.
    pub fn engine(&self, id: EngineId) -> &EngineSettings {
        self.engines
            .get(&id)
            .unwrap_or_else(|| factory_defaults(id))
    }

    pub fn engine_mut(&mut self, id: EngineId) -> &mut EngineSettings {
        self.engines
            .entry(id)
            .or_insert_with(|| id.engine().default_settings())
    }

    /// Parses a configuration file, migrating old schemas and filling in
    /// defaults for anything missing.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let mut table: toml::Table = toml::from_str(text)?;
        migrate(&mut table)?;

        let mut merged = toml::Table::try_from(AppConfig::default())?;
        merge_known(&mut merged, table);
        Ok(merged.try_into()?)
    }

    pub fn to_toml(&self) -> Result<String, ConfigError> {
        Ok(format!("{FILE_HEADER}{}", toml::to_string_pretty(self)?))
    }

    /// Loads the configuration, returning defaults if the file does not exist.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_toml(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Io {
                path: path.to_owned(),
                source,
            }),
        }
    }

    /// Saves the configuration atomically (temporary file + rename).
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        write_atomic(path, self.to_toml()?.as_bytes())
    }
}

/// Factory defaults of every engine, built once.
fn factory_defaults(id: EngineId) -> &'static EngineSettings {
    static DEFAULTS: OnceLock<[EngineSettings; EngineId::ALL.len()]> = OnceLock::new();
    // `EngineId::ALL` lists the variants in declaration order, so the
    // discriminant is the index.
    &DEFAULTS.get_or_init(|| EngineId::ALL.map(|id| id.engine().default_settings()))[id as usize]
}

/// Writes `contents` to `path` via a temporary file in the same directory so
/// a crash never leaves a half-written file behind.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), ConfigError> {
    let io_err = |source| ConfigError::Io {
        path: path.to_owned(),
        source,
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io_err)?;
    }
    let tmp = path.with_extension("tmp");
    let mut file = std::fs::File::create(&tmp).map_err(io_err)?;
    file.write_all(contents).map_err(io_err)?;
    file.sync_all().map_err(io_err)?;
    drop(file);
    std::fs::rename(&tmp, path).map_err(io_err)
}

/// Migration steps: entry `n` upgrades a version `n` table to version `n + 1`.
const MIGRATIONS: [fn(&mut toml::Table); CONFIG_VERSION as usize] = [migrate_v0_to_v1];

/// Version 0 is a file written before `config_version` existed. Its layout
/// is identical to version 1.
fn migrate_v0_to_v1(_table: &mut toml::Table) {}

/// Upgrades an older configuration table to [`CONFIG_VERSION`] in place.
fn migrate(table: &mut toml::Table) -> Result<(), ConfigError> {
    let version = table
        .get("config_version")
        .and_then(toml::Value::as_integer)
        .unwrap_or(0);
    if version > i64::from(CONFIG_VERSION) {
        return Err(ConfigError::TooNew(version));
    }
    let first = usize::try_from(version).unwrap_or(0);
    for step in &MIGRATIONS[first..] {
        step(table);
    }
    table.insert(
        "config_version".into(),
        toml::Value::Integer(i64::from(CONFIG_VERSION)),
    );
    Ok(())
}

/// Recursively overlays `overlay` onto `base`, keeping only keys that exist
/// in `base` (unknown engines and fields from newer/older versions are
/// dropped). `language` is optional and therefore always accepted.
fn merge_known(base: &mut toml::Table, overlay: toml::Table) {
    for (key, value) in overlay {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(b)), toml::Value::Table(o)) => merge_known(b, o),
            (Some(slot), value) => {
                if std::mem::discriminant(slot) == std::mem::discriminant(&value) {
                    *slot = value;
                }
            }
            (None, value) if key == "language" => {
                base.insert(key, value);
            }
            (None, _) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip() {
        let config = AppConfig::default();
        let text = config.to_toml().unwrap();
        assert!(text.starts_with("# DBDock configuration."));
        assert_eq!(AppConfig::from_toml(&text).unwrap(), config);
    }

    #[test]
    fn missing_engine_falls_back_to_defaults() {
        let mut config = AppConfig::default();
        config.engines.clear();
        for id in EngineId::ALL {
            assert_eq!(config.engine(id), &id.engine().default_settings());
        }
    }
    #[test]
    fn every_engine_has_settings() {
        let config = AppConfig::default();
        for id in EngineId::ALL {
            assert_eq!(config.engine(id), &id.engine().default_settings());
        }
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let text = r#"
            config_version = 1
            theme = "light"
            [engines.postgres]
            port = 6543
        "#;
        let config = AppConfig::from_toml(text).unwrap();
        assert_eq!(config.theme, Theme::Light);
        assert_eq!(config.engine(EngineId::Postgres).port, 6543);
        assert_eq!(config.engine(EngineId::Postgres).version, "17");
        assert_eq!(
            config.engine(EngineId::MySql),
            &EngineId::MySql.engine().default_settings()
        );
    }

    #[test]
    fn version_zero_is_migrated() {
        let config = AppConfig::from_toml("language = \"pl\"").unwrap();
        assert_eq!(config.config_version, CONFIG_VERSION);
        assert_eq!(config.language, Some(Language::Pl));
    }

    #[test]
    fn newer_version_is_rejected() {
        let err = AppConfig::from_toml("config_version = 99").unwrap_err();
        assert!(matches!(err, ConfigError::TooNew(99)));
    }

    #[test]
    fn unknown_keys_and_wrong_types_are_ignored() {
        let text = r#"
            config_version = 1
            something_new = true
            [engines.oracle]
            port = 1521
            [engines.mysql]
            port = "not a number"
            extra = 1
        "#;
        let config = AppConfig::from_toml(text).unwrap();
        assert_eq!(config.engine(EngineId::MySql).port, 3307);
    }

    #[test]
    fn save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.toml");
        assert_eq!(AppConfig::load(&path).unwrap(), AppConfig::default());

        let mut config = AppConfig {
            language: Some(Language::En),
            ..AppConfig::default()
        };
        config.engine_mut(EngineId::MsSql).enabled = false;
        config.save(&path).unwrap();
        assert_eq!(AppConfig::load(&path).unwrap(), config);
    }

    #[test]
    fn locale_detection() {
        assert_eq!(Language::from_locale("pl-PL"), Language::Pl);
        assert_eq!(Language::from_locale("PL"), Language::Pl);
        assert_eq!(Language::from_locale("en-US"), Language::En);
        assert_eq!(Language::from_locale("de"), Language::En);
    }

    #[test]
    fn credential_changes_are_detected() {
        let a = EngineId::Postgres.engine().default_settings();
        let mut b = a.clone();
        b.port = 1;
        b.version = "16".into();
        assert!(!a.credentials_differ(&b));
        b.password = "other".into();
        assert!(a.credentials_differ(&b));
    }
}
