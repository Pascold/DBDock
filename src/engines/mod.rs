//! Database engines supported by DBDock.
//!
//! Each engine lives in its own module and implements [`Engine`].

mod firebird;
pub mod interbase;
mod mariadb;
mod mssql;
mod mysql;
mod postgres;
pub mod sqlite;

use serde::{Deserialize, Serialize};

use crate::compose::{ComposeContext, Yaml};
use crate::config::EngineSettings;
use crate::paths::AppPaths;

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

    /// Suggested tags whose version no longer gets updates from the vendor
    /// (end of life). They stay available for testing older applications.
    fn eol_versions(&self) -> &'static [&'static str] {
        &[]
    }

    /// True if `version` is an end-of-life tag or a more specific tag of one
    /// (`13.4`, `13-alpine`).
    fn is_eol(&self, version: &str) -> bool {
        let version = version.trim();
        self.eol_versions().iter().any(|tag| {
            version
                .strip_prefix(tag)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '-']))
        })
    }

    /// Name of the administrator account (`root`, `sa`, `SYSDBA`), if any.
    fn admin_user(&self) -> Option<&'static str> {
        None
    }

    fn uses(&self, field: Field) -> bool {
        self.fields().contains(&field)
    }

    /// Full image reference for the configured version.
    fn image(&self, settings: &EngineSettings) -> String;

    /// Compose services (name, definition). The first one is the main
    /// service named after [`EngineId::key`].
    fn services(&self, settings: &EngineSettings, ctx: &ComposeContext<'_>) -> Vec<(String, Yaml)>;

    /// Named volumes holding the engine's data (removed by "Reset data").
    fn volumes(&self) -> &'static [&'static str] {
        &[]
    }

    /// Named volumes that survive "Reset data" (e.g. the InterBase license).
    fn kept_volumes(&self) -> &'static [&'static str] {
        &[]
    }

    /// Services to pass to `docker compose up` when starting the engine.
    fn start_services(&self) -> &'static [&'static str];

    /// Sample data script (`assets/seed/<engine>.sql`), if supported.
    fn seed_sql(&self) -> Option<&'static str> {
        None
    }

    /// Extra files written next to the seeds (`(file name, contents)`).
    fn helper_files(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }

    /// True if the image only exists for linux/amd64 (emulated on ARM).
    fn amd64_only(&self) -> bool {
        false
    }

    /// Connection details shown in the connection window.
    fn connection(&self, settings: &EngineSettings, paths: &AppPaths) -> ConnectionInfo;
}

/// Ready-to-copy connection details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub host: String,
    /// `None` for SQLite.
    pub port: Option<u16>,
    /// Database name, server-side path or local file.
    pub database: String,
    pub user: String,
    pub password: String,
    /// Labelled connection strings, most common first.
    pub strings: Vec<(&'static str, String)>,
}

impl ConnectionInfo {
    fn server(settings: &EngineSettings, database: String, user: &str, password: &str) -> Self {
        Self {
            host: HOST.into(),
            port: Some(settings.port),
            database,
            user: user.into(),
            password: password.into(),
            strings: Vec::new(),
        }
    }

    fn with(mut self, label: &'static str, value: String) -> Self {
        self.strings.push((label, value));
        self
    }
}

/// Host name used in connection strings.
pub const HOST: &str = "localhost";

/// Percent-encodes a URL user-info or path component.
pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Connection details of MySQL and MariaDB (same wire protocol).
fn mysql_family_connection(scheme: &str, s: &EngineSettings) -> ConnectionInfo {
    ConnectionInfo::server(s, s.database.clone(), &s.user, &s.password)
        .with("URL", url(scheme, s))
        .with(
            "JDBC",
            format!("jdbc:{scheme}://{HOST}:{}/{}", s.port, s.database),
        )
        .with(
            "ADO.NET",
            format!(
                "Server={HOST};Port={};Database={};Uid={};Pwd={}",
                s.port, s.database, s.user, s.password
            ),
        )
}

/// `scheme://user:password@localhost:port/database`
fn url(scheme: &str, s: &EngineSettings) -> String {
    format!(
        "{scheme}://{}:{}@{HOST}:{}/{}",
        url_encode(&s.user),
        url_encode(&s.password),
        s.port,
        url_encode(&s.database)
    )
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
