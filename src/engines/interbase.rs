use super::{defaults, Engine, EngineId, Field};
use crate::config::EngineSettings;

pub struct InterBase;

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
        &["2020.4"]
    }

    fn admin_user(&self) -> Option<&'static str> {
        Some("SYSDBA")
    }
}
