use super::{defaults, ConnectionInfo, Engine, EngineId, Field, HOST};
use crate::compose::{self, ComposeContext, Yaml, MANUAL_PROFILE};
use crate::config::EngineSettings;
use crate::paths::AppPaths;

pub struct InterBase;

/// Installation folder; also where databases should live, because it is a
/// persistent volume.
pub const HOME: &str = "/opt/interbase";

/// Path to `isql` inside the container.
pub const ISQL: &str = "/opt/interbase/bin/isql";

/// Path of the database file inside the container.
pub fn database_path(s: &EngineSettings) -> String {
    format!("{HOME}/{}.ib", s.database)
}

impl Engine for InterBase {
    fn id(&self) -> EngineId {
        EngineId::InterBase
    }

    fn default_settings(&self) -> EngineSettings {
        EngineSettings {
            // Needs a (free) license registered interactively first.
            enabled: false,
            version: "2020.4".into(),
            port: 3051,
            database: defaults::DATABASE.into(),
            user: String::new(),
            password: String::new(),
            admin_password: defaults::SYSDBA_PASSWORD.into(),
            seed: false,
        }
    }

    fn fields(&self) -> &'static [Field] {
        &[
            Field::Version,
            Field::Port,
            Field::Database,
            Field::AdminPassword,
        ]
    }

    fn version_suggestions(&self) -> &'static [&'static str] {
        &["2020.4", "2020", "latest"]
    }

    fn admin_user(&self) -> Option<&'static str> {
        Some("SYSDBA")
    }

    fn image(&self, s: &EngineSettings) -> String {
        format!("radstudio/interbase:{}", s.version)
    }

    fn services(&self, s: &EngineSettings, _ctx: &ComposeContext<'_>) -> Vec<(String, Yaml)> {
        let service = compose::service(self.image(s))
            .with("platform", "linux/amd64")
            // Never started by a plain `docker compose up`: the license
            // manager has to run interactively first.
            .with("profiles", Yaml::strings([MANUAL_PROFILE]))
            .with("ports", compose::ports(s.port, 3050))
            .with(
                "volumes",
                Yaml::Seq(vec![
                    compose::volume("interbase-data", HOME),
                    compose::volume("interbase-license", "/opt/interbase/license"),
                ]),
            )
            .with("stdin_open", true)
            .with("tty", true);
        vec![("interbase".into(), service)]
    }

    fn volumes(&self) -> &'static [&'static str] {
        &["interbase-data"]
    }

    fn kept_volumes(&self) -> &'static [&'static str] {
        &["interbase-license"]
    }

    fn start_services(&self) -> &'static [&'static str] {
        &["interbase"]
    }

    fn amd64_only(&self) -> bool {
        true
    }

    fn connection(&self, s: &EngineSettings, _paths: &AppPaths) -> ConnectionInfo {
        let path = database_path(s);
        ConnectionInfo::server(s, path.clone(), "SYSDBA", &s.admin_password)
            .with("InterBase", format!("{HOST}/{}:{path}", s.port))
            .with(
                "JDBC (InterClient)",
                format!("jdbc:interbase://{HOST}:{}/{path}", s.port),
            )
    }
}
