use super::{defaults, Engine, EngineId, Field};
use crate::config::EngineSettings;

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

    fn admin_user(&self) -> Option<&'static str> {
        Some("root")
    }
}
