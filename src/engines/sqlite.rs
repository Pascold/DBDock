use std::path::PathBuf;

use super::{ConnectionInfo, Engine, EngineId, Field};
use crate::compose::{self, ComposeContext, Yaml};
use crate::config::EngineSettings;
use crate::paths::AppPaths;

pub struct Sqlite;

/// Location of the database file on the host.
pub fn database_file(s: &EngineSettings, paths: &AppPaths) -> PathBuf {
    paths.sqlite_dir().join(&s.database)
}

impl Engine for Sqlite {
    fn id(&self) -> EngineId {
        EngineId::Sqlite
    }

    fn default_settings(&self) -> EngineSettings {
        EngineSettings {
            enabled: true,
            // Tag of the `alpine` image used to run the sqlite3 CLI.
            version: "3".into(),
            port: 0,
            database: "test.db".into(),
            user: String::new(),
            password: String::new(),
            admin_password: String::new(),
            seed: true,
            lan_access: false,
        }
    }

    fn fields(&self) -> &'static [Field] {
        &[Field::Version, Field::Database, Field::Seed]
    }

    fn version_suggestions(&self) -> &'static [&'static str] {
        &["3", "edge"]
    }

    fn image(&self, s: &EngineSettings) -> String {
        format!("alpine:{}", s.version)
    }

    fn services(&self, s: &EngineSettings, ctx: &ComposeContext<'_>) -> Vec<(String, Yaml)> {
        // No server: a one-shot container creates the file in the SQLite
        // folder of the data directory, where the user opens it directly.
        let mut volumes = vec![
            compose::bind(&ctx.paths.sqlite_dir(), "/data", false),
            compose::bind(
                &ctx.paths.scripts_dir(EngineId::Sqlite),
                "/dbdock-scripts",
                true,
            ),
        ];
        let create = if s.seed {
            volumes.push(compose::bind(
                &ctx.paths.seed_file(EngineId::Sqlite),
                "/seed/sqlite.sql",
                true,
            ));
            "sqlite3 -bail \"$$tmp\" < /seed/sqlite.sql"
        } else {
            "sqlite3 \"$$tmp\" 'VACUUM;'"
        };
        let chown = ctx
            .owner
            .map(|(uid, gid)| format!("  chown {uid}:{gid} \"$$db\"\n"))
            .unwrap_or_default();
        // The file is built under a temporary name, so a failing script does
        // not leave a half-initialised database behind.
        let script = format!(
            "set -e\n\
             apk add --no-cache sqlite > /dev/null\n\
             db=\"/data/$$DBDOCK_DB\"\n\
             if [ -f \"$$db\" ]; then\n  \
               echo \"DBDock: $$db already exists\"\n\
             else\n  \
               tmp=\"$$db.part\"\n  \
               rm -f \"$$tmp\"\n  \
               {create}\n  \
               for f in /dbdock-scripts/*.sql; do\n    \
                 [ -e \"$$f\" ] || continue\n    \
                 echo \"DBDock: running $$f\"\n    \
                 sqlite3 -bail \"$$tmp\" < \"$$f\"\n  \
               done\n  \
               mv \"$$tmp\" \"$$db\"\n\
             {chown}  \
               echo \"DBDock: created $$db\"\n\
             fi\n"
        );
        let service = compose::service(self.image(s))
            .with(
                "environment",
                compose::environment([("DBDOCK_DB", s.database.as_str())]),
            )
            .with("volumes", Yaml::Seq(volumes))
            .with("entrypoint", Yaml::strings(["/bin/sh", "-c", &script]))
            .with("restart", "no");
        vec![("sqlite".into(), service)]
    }

    fn run_scripts_args(&self, _s: &EngineSettings) -> Option<Vec<String>> {
        let script = format!(
            "set -e\n\
             apk add --no-cache sqlite > /dev/null\n\
             db=\"/data/$DBDOCK_DB\"\n\
             if [ ! -f \"$db\" ]; then echo \"DBDock: $db does not exist yet\" >&2; exit 1; fi\n\
             {}",
            super::scripts_loop("/dbdock-scripts/*.sql", "sqlite3 -bail \"$db\" < \"$f\"")
        );
        Some(super::run_args("sqlite", "/bin/sh", script))
    }

    fn start_services(&self) -> &'static [&'static str] {
        &["sqlite"]
    }

    fn seed_sql(&self) -> Option<&'static str> {
        Some(include_str!("../../assets/seed/sqlite.sql"))
    }

    fn connection(&self, s: &EngineSettings, paths: &AppPaths) -> ConnectionInfo {
        let file = database_file(s, paths).to_string_lossy().into_owned();
        ConnectionInfo {
            host: String::new(),
            port: None,
            database: file.clone(),
            user: String::new(),
            password: String::new(),
            strings: vec![
                ("Path", file.clone()),
                ("ADO.NET", format!("Data Source={file}")),
                ("JDBC", format!("jdbc:sqlite:{file}")),
            ],
        }
    }
}
