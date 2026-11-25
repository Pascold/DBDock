//! Database engines supported by DBDock.
//!
//! Each engine lives in its own module and implements [`Engine`].

mod firebird;
mod interbase;
mod mariadb;
mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use serde::{Deserialize, Serialize};

use crate::config::EngineSettings;

/// Identifier of a supported engine. Also used as the compose service and
/// profile name and as the key in `config.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineId {
    Postgres,
    MariaDb,
    MySql,
    MsSql,
    Firebird,
    InterBase,
    Sqlite,
}

impl EngineId {
    /// All engines in display order.
    pub const ALL: [EngineId; 7] = [
        EngineId::Postgres,
        EngineId::MariaDb,
        EngineId::MySql,
        EngineId::MsSql,
        EngineId::Firebird,
        EngineId::InterBase,
        EngineId::Sqlite,
    ];

    /// Stable lowercase key (config key, compose service and profile name).
    pub fn key(self) -> &'static str {
        match self {
            EngineId::Postgres => "postgres",
            EngineId::MariaDb => "mariadb",
            EngineId::MySql => "mysql",
            EngineId::MsSql => "mssql",
            EngineId::Firebird => "firebird",
            EngineId::InterBase => "interbase",
            EngineId::Sqlite => "sqlite",
        }
    }

    /// Human readable product name.
    pub fn display_name(self) -> &'static str {
        match self {
            EngineId::Postgres => "PostgreSQL",
            EngineId::MariaDb => "MariaDB",
            EngineId::MySql => "MySQL",
            EngineId::MsSql => "MS SQL Server",
            EngineId::Firebird => "Firebird",
            EngineId::InterBase => "InterBase",
            EngineId::Sqlite => "SQLite",
        }
    }

    /// Parses a [`Self::key`] back into an id.
    pub fn from_key(key: &str) -> Option<EngineId> {
        EngineId::ALL.into_iter().find(|id| id.key() == key)
    }

    /// The engine implementation.
    pub fn engine(self) -> &'static dyn Engine {
        match self {
            EngineId::Postgres => &postgres::Postgres,
            EngineId::MariaDb => &mariadb::MariaDb,
            EngineId::MySql => &mysql::MySql,
            EngineId::MsSql => &mssql::MsSql,
            EngineId::Firebird => &firebird::Firebird,
            EngineId::InterBase => &interbase::InterBase,
            EngineId::Sqlite => &sqlite::Sqlite,
        }
    }
}

/// A user-editable setting. Engines declare which of them they use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    Version,
    Port,
    Database,
    User,
    Password,
    AdminPassword,
    Seed,
}

/// Behaviour shared by all engines.
pub trait Engine: Sync {
    fn id(&self) -> EngineId;

    /// Factory defaults for this engine.
    fn default_settings(&self) -> EngineSettings;

    /// Settings shown in the UI and validated for this engine.
    fn fields(&self) -> &'static [Field];

    /// Suggested image tags, newest first.
    fn version_suggestions(&self) -> &'static [&'static str];

    /// Name of the administrator account (`root`, `sa`, `SYSDBA`), if any.
    fn admin_user(&self) -> Option<&'static str> {
        None
    }

    fn uses(&self, field: Field) -> bool {
        self.fields().contains(&field)
    }
}

/// Default credentials shared by most engines.
pub(crate) mod defaults {
    pub const DATABASE: &str = "testdb";
    pub const USER: &str = "tester";
    pub const PASSWORD: &str = "tester123";
    pub const ROOT_PASSWORD: &str = "root123";
    pub const SA_PASSWORD: &str = "Test_Passw0rd!";
    pub const SYSDBA_PASSWORD: &str = "masterkey";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        for id in EngineId::ALL {
            assert_eq!(EngineId::from_key(id.key()), Some(id));
            assert_eq!(id.engine().id(), id);
        }
    }

    #[test]
    fn only_interbase_is_disabled_by_default() {
        for id in EngineId::ALL {
            let enabled = id.engine().default_settings().enabled;
            assert_eq!(enabled, id != EngineId::InterBase, "{id:?}");
        }
    }

    #[test]
    fn default_version_is_suggested() {
        for id in EngineId::ALL {
            let engine = id.engine();
            let version = engine.default_settings().version;
            assert!(
                engine.version_suggestions().contains(&version.as_str()),
                "{id:?}"
            );
        }
    }
}
