use super::{defaults, ConnectionInfo, Engine, EngineId, Field, HOST};
use crate::compose::{self, ComposeContext, Yaml};
use crate::config::EngineSettings;
use crate::paths::AppPaths;

pub struct Firebird;

const DATA_DIR: &str = "/var/lib/firebird/data";

/// Path of the database file inside the container.
pub fn database_path(s: &EngineSettings) -> String {
    format!("{DATA_DIR}/{}.fdb", s.database)
}

const DATABASES_CONF: &str = "/opt/firebird/databases.conf";

/// Entrypoint that registers the database name as an alias, so clients can
/// connect to `testdb` instead of the full file path, then runs the image's
/// own entrypoint. `databases.conf` is not in a volume: the alias is added
/// once per container. The name is a validated identifier, safe in shell.
fn entrypoint(s: &EngineSettings) -> Yaml {
    let alias = format!("{} = {}", s.database, database_path(s));
    let script = format!(
        "grep -qx '{alias}' {DATABASES_CONF} || echo '{alias}' >> {DATABASES_CONF}; \
         exec entrypoint.sh firebird"
    );
    Yaml::strings([
        "tini".to_owned(),
        "--".into(),
        "sh".into(),
        "-c".into(),
        script,
    ])
}

impl Engine for Firebird {
    fn id(&self) -> EngineId {
        EngineId::Firebird
    }

    fn default_settings(&self) -> EngineSettings {
        EngineSettings {
            enabled: true,
            version: "5".into(),
            port: 3050,
            database: defaults::DATABASE.into(),
            user: defaults::USER.into(),
            password: defaults::PASSWORD.into(),
            admin_password: defaults::SYSDBA_PASSWORD.into(),
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
        &["5", "4", "3"]
    }

    fn admin_user(&self) -> Option<&'static str> {
        Some("SYSDBA")
    }

    fn image(&self, s: &EngineSettings) -> String {
        format!("firebirdsql/firebird:{}", s.version)
    }

    fn services(&self, s: &EngineSettings, ctx: &ComposeContext<'_>) -> Vec<(String, Yaml)> {
        let file_name = format!("{}.fdb", s.database);
        let mut volumes = vec![compose::volume("firebird-data", DATA_DIR)];
        if s.seed {
            volumes.push(compose::bind(
                &ctx.paths.seed_file(EngineId::Firebird),
                "/docker-entrypoint-initdb.d/10-dbdock-seed.sql",
                true,
            ));
        }
        let health = format!(
            "echo 'SELECT 1 FROM RDB$$DATABASE;' | /opt/firebird/bin/isql -b -q \
             -user SYSDBA -password \"$$FIREBIRD_ROOT_PASSWORD\" {HOST}:{}",
            compose::escape(&database_path(s))
        );
        let service = compose::service(self.image(s))
            .with("entrypoint", entrypoint(s))
            .with(
                "environment",
                compose::environment([
                    ("FIREBIRD_ROOT_PASSWORD", s.admin_password.as_str()),
                    ("FIREBIRD_USER", s.user.as_str()),
                    ("FIREBIRD_PASSWORD", s.password.as_str()),
                    ("FIREBIRD_DATABASE", file_name.as_str()),
                    ("FIREBIRD_DATABASE_DEFAULT_CHARSET", "UTF8"),
                    ("FIREBIRD_CONF_WireCrypt", "Enabled"),
                ]),
            )
            .with("ports", compose::ports(s.port, 3050))
            .with("volumes", Yaml::Seq(volumes))
            .with("healthcheck", compose::healthcheck(&health, 3, 40, 5));
        vec![("firebird".into(), service)]
    }

    fn volumes(&self) -> &'static [&'static str] {
        &["firebird-data"]
    }

    fn start_services(&self) -> &'static [&'static str] {
        &["firebird"]
    }

    fn seed_sql(&self) -> Option<&'static str> {
        Some(include_str!("../../assets/seed/firebird.sql"))
    }

    fn connection(&self, s: &EngineSettings, _paths: &AppPaths) -> ConnectionInfo {
        let path = database_path(s);
        ConnectionInfo::server(s, path.clone(), &s.user, &s.password)
            .with("Firebird", format!("{HOST}/{}:{path}", s.port))
            .with(
                "Firebird (alias)",
                format!("{HOST}/{}:{}", s.port, s.database),
            )
            .with(
                "JDBC (Jaybird)",
                format!("jdbc:firebird://{HOST}:{}/{path}", s.port),
            )
            .with(
                "ADO.NET",
                format!(
                    "DataSource={HOST};Port={};Database={path};User={};Password={};Charset=UTF8",
                    s.port, s.user, s.password
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_strings() {
        let info = Firebird.connection(&Firebird.default_settings(), &AppPaths::new("/x"));
        assert_eq!(info.database, "/var/lib/firebird/data/testdb.fdb");
        assert_eq!(
            info.strings[0].1,
            "localhost/3050:/var/lib/firebird/data/testdb.fdb"
        );
        assert_eq!(info.strings[1].1, "localhost/3050:testdb");
        assert_eq!(
            info.strings[2].1,
            "jdbc:firebird://localhost:3050//var/lib/firebird/data/testdb.fdb"
        );
    }

    #[test]
    fn alias_for_the_database_name() {
        let mut s = Firebird.default_settings();
        s.database = "shop_db".into();
        let Yaml::Seq(items) = entrypoint(&s) else {
            panic!("entrypoint is a list");
        };
        let script = format!("{items:?}");
        assert!(script.contains("shop_db = /var/lib/firebird/data/shop_db.fdb"));
        assert!(script.contains("exec entrypoint.sh firebird"));
    }
}
