use super::{defaults, Engine, EngineId, Field};
use crate::config::EngineSettings;

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

    fn admin_user(&self) -> Option<&'static str> {
        Some("root")
    }
}
