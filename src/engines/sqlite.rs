use super::{Engine, EngineId, Field};
use crate::config::EngineSettings;

pub struct Sqlite;

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
        }
    }

    fn fields(&self) -> &'static [Field] {
        &[Field::Version, Field::Database, Field::Seed]
    }

    fn version_suggestions(&self) -> &'static [&'static str] {
        &["3", "edge"]
    }
}
