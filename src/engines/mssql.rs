use super::{defaults, Engine, EngineId, Field};
use crate::config::EngineSettings;

pub struct MsSql;

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
}
