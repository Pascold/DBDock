use super::{defaults, ConnectionInfo, Engine, EngineId, Field, HOST};
use crate::compose::{self, ComposeContext, Yaml};
use crate::config::EngineSettings;
use crate::paths::AppPaths;

pub struct MsSql;

/// Creates the database, the login and the user. Runs on every start, so
/// password changes are applied to an existing login too.
const INIT_SQL: &str = include_str!("../../assets/seed/mssql-init.sql");

const SQLCMD: &str = "/opt/mssql-tools18/bin/sqlcmd";

impl Engine for MsSql {
    fn id(&self) -> EngineId {
        EngineId::MsSql
    }

    fn default_settings(&self) -> EngineSettings {
        EngineSettings {
            enabled: true,
            version: "2022-latest".into(),
            port: 1433,
            database: defaults::DATABASE.into(),
            user: defaults::USER.into(),
            password: defaults::PASSWORD.into(),
            admin_password: defaults::SA_PASSWORD.into(),
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
        &["2025-latest", "2022-latest", "2019-latest"]
    }

    fn admin_user(&self) -> Option<&'static str> {
        Some("sa")
    }

    fn image(&self, s: &EngineSettings) -> String {
        format!("mcr.microsoft.com/mssql/server:{}", s.version)
    }

    fn services(&self, s: &EngineSettings, ctx: &ComposeContext<'_>) -> Vec<(String, Yaml)> {
        let server = compose::service(self.image(s))
            .with("platform", "linux/amd64")
            .with(
                "environment",
                compose::environment([
                    ("ACCEPT_EULA", "Y"),
                    ("MSSQL_PID", "Developer"),
                    ("MSSQL_SA_PASSWORD", s.admin_password.as_str()),
                ]),
            )
            .with("ports", compose::ports(s.port, 1433))
            .with(
                "volumes",
                Yaml::Seq(vec![compose::volume("mssql-data", "/var/opt/mssql")]),
            )
            .with(
                "healthcheck",
                compose::healthcheck(
                    &format!(
                        "{SQLCMD} -S localhost -U sa -P \"$$MSSQL_SA_PASSWORD\" -C -b \
                         -Q \"SELECT 1\" -o /dev/null"
                    ),
                    5,
                    60,
                    15,
                ),
            );

        // SQL Server images have no init-script folder, so a one-shot
        // container creates the database and the user once the server is up.
        let sqlcmd = format!("{SQLCMD} -S mssql -U sa -P \"$$MSSQL_SA_PASSWORD\" -C -b -f 65001");
        let mut script = format!("set -e\n{sqlcmd} -i /dbdock/mssql-init.sql\n");
        if s.seed {
            script.push_str(&format!(
                "{sqlcmd} -d \"$$DBDOCK_DB\" -i /dbdock/mssql.sql\n"
            ));
        }
        script.push_str("echo \"DBDock: database $$DBDOCK_DB is ready\"\n");

        let init = compose::service(self.image(s))
            .with("platform", "linux/amd64")
            .with(
                "environment",
                compose::environment([
                    ("MSSQL_SA_PASSWORD", s.admin_password.as_str()),
                    ("DBDOCK_DB", s.database.as_str()),
                    ("DBDOCK_USER", s.user.as_str()),
                    ("DBDOCK_PASSWORD", s.password.as_str()),
                ]),
            )
            .with(
                "volumes",
                Yaml::Seq(vec![compose::bind(&ctx.paths.seed_dir(), "/dbdock", true)]),
            )
            .with("entrypoint", Yaml::strings(["/bin/bash", "-c", &script]))
            .with(
                "depends_on",
                Yaml::map().with("mssql", Yaml::map().with("condition", "service_healthy")),
            )
            .with("restart", "no");

        vec![("mssql".into(), server), ("mssql-init".into(), init)]
    }

    fn volumes(&self) -> &'static [&'static str] {
        &["mssql-data"]
    }

    fn start_services(&self) -> &'static [&'static str] {
        &["mssql", "mssql-init"]
    }

    fn seed_sql(&self) -> Option<&'static str> {
        Some(include_str!("../../assets/seed/mssql.sql"))
    }

    fn helper_files(&self) -> &'static [(&'static str, &'static str)] {
        &[("mssql-init.sql", INIT_SQL)]
    }

    fn amd64_only(&self) -> bool {
        true
    }

    fn connection(&self, s: &EngineSettings, _paths: &AppPaths) -> ConnectionInfo {
        ConnectionInfo::server(s, s.database.clone(), &s.user, &s.password)
            .with(
                "ADO.NET",
                format!(
                    "Server={HOST},{};Database={};User Id={};Password={};TrustServerCertificate=True",
                    s.port, s.database, s.user, s.password
                ),
            )
            .with(
                "JDBC",
                format!(
                    "jdbc:sqlserver://{HOST}:{};databaseName={};encrypt=true;trustServerCertificate=true",
                    s.port, s.database
                ),
            )
            .with(
                "sqlcmd",
                format!(
                    "sqlcmd -S {HOST},{} -d {} -U {} -P \"{}\" -C",
                    s.port, s.database, s.user, s.password
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ado_net_string_trusts_the_certificate() {
        let info = MsSql.connection(&MsSql.default_settings(), &AppPaths::new("/x"));
        assert_eq!(
            info.strings[0].1,
            "Server=localhost,1433;Database=testdb;User Id=tester;Password=tester123;\
             TrustServerCertificate=True"
        );
    }
}
