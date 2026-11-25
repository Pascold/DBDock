use super::{defaults, Engine, EngineId, Field};
use crate::config::EngineSettings;

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
        ]
    }

    fn version_suggestions(&self) -> &'static [&'static str] {
        &["18", "17", "16", "15", "14", "13"]
    }
}
