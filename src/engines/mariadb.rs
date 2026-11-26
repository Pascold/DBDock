use super::{defaults, mysql_family_connection, ConnectionInfo, Engine, EngineId, Field};
use crate::compose::{self, ComposeContext, Yaml};
use crate::config::EngineSettings;
use crate::paths::AppPaths;

pub struct MariaDb;

impl Engine for MariaDb {
    fn id(&self) -> EngineId {
        EngineId::MariaDb
    }

    fn default_settings(&self) -> EngineSettings {
        EngineSettings {
            enabled: true,
            version: "11.4".into(),
            port: 3306,
            database: defaults::DATABASE.into(),
            user: defaults::USER.into(),
            password: defaults::PASSWORD.into(),
            admin_password: defaults::ROOT_PASSWORD.into(),
            seed: true,
        }
    }

    fn fields(&self) -> &'static [Field] {
        &[
            Field::Version,
            Field::Port,
            Field::Database,
            Field::User,
            Field::Password,
            Field::AdminPassword,
            Field::Seed,
        ]
    }

    fn version_suggestions(&self) -> &'static [&'static str] {
        &["11.8", "11.4", "10.11", "10.6"]
    }

    // 10.6 ended in July 2026.
    fn eol_versions(&self) -> &'static [&'static str] {
        &["10.6"]
    }

    fn admin_user(&self) -> Option<&'static str> {
        Some("root")
    }

    fn image(&self, s: &EngineSettings) -> String {
        format!("mariadb:{}", s.version)
    }

    fn services(&self, s: &EngineSettings, ctx: &ComposeContext<'_>) -> Vec<(String, Yaml)> {
        let mut volumes = vec![compose::volume("mariadb-data", "/var/lib/mysql")];
        if s.seed {
            volumes.push(compose::bind(
                &ctx.paths.seed_file(EngineId::MariaDb),
                "/docker-entrypoint-initdb.d/10-dbdock-seed.sql",
                true,
            ));
        }
        let service = compose::service(self.image(s))
            .with(
                "environment",
                compose::environment([
                    ("MARIADB_DATABASE", s.database.as_str()),
                    ("MARIADB_USER", s.user.as_str()),
                    ("MARIADB_PASSWORD", s.password.as_str()),
                    ("MARIADB_ROOT_PASSWORD", s.admin_password.as_str()),
                ]),
            )
            .with("ports", compose::ports(s.port, 3306))
            .with("volumes", Yaml::Seq(volumes))
            .with(
                "healthcheck",
                compose::healthcheck("healthcheck.sh --connect --innodb_initialized", 3, 40, 5),
            );
        vec![("mariadb".into(), service)]
    }

    fn volumes(&self) -> &'static [&'static str] {
        &["mariadb-data"]
    }

    fn start_services(&self) -> &'static [&'static str] {
        &["mariadb"]
    }

    fn seed_sql(&self) -> Option<&'static str> {
        Some(include_str!("../../assets/seed/mariadb.sql"))
    }

    fn connection(&self, s: &EngineSettings, _paths: &AppPaths) -> ConnectionInfo {
        mysql_family_connection("mariadb", s)
    }
}
