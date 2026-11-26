use super::{defaults, url, ConnectionInfo, Engine, EngineId, Field, HOST};
use crate::compose::{self, ComposeContext, Yaml};
use crate::config::EngineSettings;
use crate::paths::AppPaths;

pub struct Postgres;

impl Engine for Postgres {
    fn id(&self) -> EngineId {
        EngineId::Postgres
    }

    fn default_settings(&self) -> EngineSettings {
        EngineSettings {
            enabled: true,
            version: "17".into(),
            port: 5432,
            database: defaults::DATABASE.into(),
            user: defaults::USER.into(),
            password: defaults::PASSWORD.into(),
            admin_password: String::new(),
            seed: true,
            lan_access: false,
        }
    }

    fn fields(&self) -> &'static [Field] {
        &[
            Field::Version,
            Field::Port,
            Field::Database,
            Field::User,
            Field::Password,
            Field::Seed,
            Field::LanAccess,
        ]
    }

    fn version_suggestions(&self) -> &'static [&'static str] {
        &["18", "17", "16", "15", "14", "13"]
    }

    // 13 ended in November 2025, 14 in November 2026.
    fn eol_versions(&self) -> &'static [&'static str] {
        &["14", "13"]
    }

    fn image(&self, s: &EngineSettings) -> String {
        format!("postgres:{}", s.version)
    }

    fn services(&self, s: &EngineSettings, ctx: &ComposeContext<'_>) -> Vec<(String, Yaml)> {
        // Seed and the user's scripts, see `compose::write_files`.
        let volumes = vec![
            compose::volume("postgres-data", data_dir(&s.version)),
            compose::bind(
                &ctx.paths.init_dir(EngineId::Postgres),
                "/docker-entrypoint-initdb.d",
                true,
            ),
        ];
        let service = compose::service(self.image(s))
            .with(
                "environment",
                compose::environment([
                    ("POSTGRES_DB", s.database.as_str()),
                    ("POSTGRES_USER", s.user.as_str()),
                    ("POSTGRES_PASSWORD", s.password.as_str()),
                ]),
            )
            .with("ports", compose::ports(s, 5432))
            .with("volumes", Yaml::Seq(volumes))
            .with(
                "healthcheck",
                // -h forces TCP: during initialisation the server listens on
                // the Unix socket only, so this waits for the final server.
                compose::healthcheck(
                    "pg_isready -h 127.0.0.1 -U \"$$POSTGRES_USER\" -d \"$$POSTGRES_DB\"",
                    3,
                    40,
                    5,
                ),
            );
        vec![("postgres".into(), service)]
    }

    fn volumes(&self) -> &'static [&'static str] {
        &["postgres-data"]
    }

    fn uses_initdb_dir(&self) -> bool {
        true
    }

    fn run_scripts_args(&self, _s: &EngineSettings) -> Option<Vec<String>> {
        let run = "psql -v ON_ERROR_STOP=1 -U \"$POSTGRES_USER\" -d \"$POSTGRES_DB\" -f \"$f\"";
        Some(super::exec_args(
            "postgres",
            super::scripts_loop(super::INIT_SCRIPTS, run),
        ))
    }

    fn start_services(&self) -> &'static [&'static str] {
        &["postgres"]
    }

    fn seed_sql(&self) -> Option<&'static str> {
        Some(include_str!("../../assets/seed/postgres.sql"))
    }

    fn connection(&self, s: &EngineSettings, _paths: &AppPaths) -> ConnectionInfo {
        ConnectionInfo::server(s, s.database.clone(), &s.user, &s.password)
            .with("URL", url("postgres", s))
            .with(
                "JDBC",
                format!("jdbc:postgresql://{HOST}:{}/{}", s.port, s.database),
            )
            .with(
                "ADO.NET (Npgsql)",
                format!(
                    "Host={HOST};Port={};Database={};Username={};Password={}",
                    s.port, s.database, s.user, s.password
                ),
            )
    }
}

/// Data directory inside the container. PostgreSQL 18 images moved `PGDATA`
/// to a versioned folder under `/var/lib/postgresql`, which must be mounted
/// as a whole. Non-numeric tags (`latest`, `alpine`) are treated as newest.
pub fn data_dir(version: &str) -> &'static str {
    let major = version
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|digits| digits.parse::<u32>().ok());
    match major {
        Some(major) if major < 18 => "/var/lib/postgresql/data",
        _ => "/var/lib/postgresql",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_path_depends_on_major_version() {
        assert_eq!(data_dir("17"), "/var/lib/postgresql/data");
        assert_eq!(data_dir("13.4-alpine"), "/var/lib/postgresql/data");
        assert_eq!(data_dir("9.6"), "/var/lib/postgresql/data");
        assert_eq!(data_dir("18"), "/var/lib/postgresql");
        assert_eq!(data_dir("18.1-bookworm"), "/var/lib/postgresql");
        assert_eq!(data_dir("19beta1"), "/var/lib/postgresql");
        assert_eq!(data_dir("latest"), "/var/lib/postgresql");
        assert_eq!(data_dir("alpine"), "/var/lib/postgresql");
    }

    #[test]
    fn end_of_life_versions() {
        for eol in ["13", "13.4", "13-alpine", "14", " 14.2 "] {
            assert!(Postgres.is_eol(eol), "{eol}");
        }
        for supported in ["15", "17", "18", "130", "1", "latest", ""] {
            assert!(!Postgres.is_eol(supported), "{supported}");
        }
    }

    #[test]
    fn connection_strings() {
        let mut s = Postgres.default_settings();
        s.password = "p@ss:w/rd".into();
        let info = Postgres.connection(&s, &AppPaths::new("/x"));
        assert_eq!(info.port, Some(5432));
        assert_eq!(
            info.strings[0].1,
            "postgres://tester:p%40ss%3Aw%2Frd@localhost:5432/testdb"
        );
        assert_eq!(info.strings[1].1, "jdbc:postgresql://localhost:5432/testdb");
        assert_eq!(
            info.strings[2].1,
            "Host=localhost;Port=5432;Database=testdb;Username=tester;Password=p@ss:w/rd"
        );
    }
}
