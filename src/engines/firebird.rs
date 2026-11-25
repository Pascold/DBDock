use super::{defaults, Engine, EngineId, Field};
use crate::config::EngineSettings;

pub struct Firebird;

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
}
