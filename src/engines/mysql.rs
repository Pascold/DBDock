use super::{defaults, mysql_family_connection, ConnectionInfo, Engine, EngineId, Field};
use crate::compose::{self, ComposeContext, Yaml};
use crate::config::EngineSettings;
use crate::paths::AppPaths;

pub struct MySql;

impl Engine for MySql {
    fn id(&self) -> EngineId {
        EngineId::MySql
    }

    fn default_settings(&self) -> EngineSettings {
        EngineSettings {
            enabled: true,
            version: "8.4".into(),
            port: 3307,
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
        &["9", "8.4", "8.0"]
    }

    // 8.0 ended in April 2026.
    fn eol_versions(&self) -> &'static [&'static str] {
        &["8.0"]
    }

    fn admin_user(&self) -> Option<&'static str> {
        Some("root")
    }

    fn image(&self, s: &EngineSettings) -> String {
        format!("mysql:{}", s.version)
    }

    fn services(&self, s: &EngineSettings, ctx: &ComposeContext<'_>) -> Vec<(String, Yaml)> {
        // Seed and the user's scripts, see `compose::write_files`.
        let volumes = vec![
            compose::volume("mysql-data", "/var/lib/mysql"),
            compose::bind(
                &ctx.paths.init_dir(EngineId::MySql),
                "/docker-entrypoint-initdb.d",
                true,
            ),
        ];
        let service = compose::service(self.image(s))
            .with(
                "environment",
                compose::environment([
                    ("MYSQL_DATABASE", s.database.as_str()),
                    ("MYSQL_USER", s.user.as_str()),
                    ("MYSQL_PASSWORD", s.password.as_str()),
                    ("MYSQL_ROOT_PASSWORD", s.admin_password.as_str()),
                ]),
            )
            .with("ports", compose::ports(s.port, 3306))
            .with("volumes", Yaml::Seq(volumes))
            .with(
                "healthcheck",
                // The temporary server used for initialisation runs with
                // networking disabled, so TCP succeeds only for the real one.
                compose::healthcheck(
                    "mysqladmin ping -h 127.0.0.1 -u root -p\"$$MYSQL_ROOT_PASSWORD\" --silent",
                    3,
                    40,
                    5,
                ),
            );
        vec![("mysql".into(), service)]
    }

    fn volumes(&self) -> &'static [&'static str] {
        &["mysql-data"]
    }

    fn uses_initdb_dir(&self) -> bool {
        true
    }

    fn start_services(&self) -> &'static [&'static str] {
        &["mysql"]
    }

    fn seed_sql(&self) -> Option<&'static str> {
        Some(include_str!("../../assets/seed/mysql.sql"))
    }

    fn connection(&self, s: &EngineSettings, _paths: &AppPaths) -> ConnectionInfo {
        mysql_family_connection("mysql", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_strings() {
        let info = MySql.connection(&MySql.default_settings(), &AppPaths::new("/x"));
        assert_eq!(
            info.strings,
            vec![
                (
                    "URL",
                    "mysql://tester:tester123@localhost:3307/testdb".to_owned()
                ),
                ("JDBC", "jdbc:mysql://localhost:3307/testdb".to_owned()),
                (
                    "ADO.NET",
                    "Server=localhost;Port=3307;Database=testdb;Uid=tester;Pwd=tester123"
                        .to_owned()
                ),
            ]
        );
    }
}
