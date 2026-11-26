//! Validation of engine settings.

use std::net::{Ipv4Addr, TcpListener};

use super::{AppConfig, EngineSettings};
use crate::engines::{EngineId, Field};

/// Characters that may not appear in passwords. They would have to be
/// escaped differently in SQL literals, shells and connection strings.
pub const FORBIDDEN_PASSWORD_CHARS: &[char] = &['\'', '"', '\\', '`', ';'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Starting the engine would fail.
    Error,
    /// Probably a problem, but starting is allowed.
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    Empty,
    /// Port 0 or no port.
    InvalidPort,
    /// Another enabled engine uses the same host port.
    PortConflict(EngineId),
    /// Some other program listens on the port.
    PortInUse,
    /// SQL Server rejects the SA password (min. 8 characters, 3 of 4
    /// character categories).
    WeakMssqlPassword,
    /// Password contains whitespace, non-ASCII or a forbidden character.
    UnsupportedCharacters,
    /// Database or user name is not a plain identifier.
    InvalidIdentifier,
    /// SQLite file name contains path separators or other odd characters.
    InvalidFileName,
    /// Image tag contains characters Docker does not accept.
    InvalidTag,
    /// MySQL/MariaDB create `root` themselves; it cannot be the app user.
    ReservedUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Issue {
    pub field: Field,
    pub problem: Problem,
}

impl Issue {
    pub fn severity(&self) -> Severity {
        match self.problem {
            Problem::PortInUse => Severity::Warning,
            _ => Severity::Error,
        }
    }
}

/// Validates `settings` (possibly an unsaved draft) of engine `id` against
/// the rest of `config`. `port_free` reports whether a host port is free; the
/// caller decides when to probe (e.g. not while the engine itself runs).
pub fn validate_engine(
    config: &AppConfig,
    id: EngineId,
    settings: &EngineSettings,
    port_free: impl Fn(u16) -> bool,
) -> Vec<Issue> {
    let engine = id.engine();
    let mut issues = Vec::new();
    let mut push = |field, problem| issues.push(Issue { field, problem });

    for &field in engine.fields() {
        match field {
            Field::Version => {
                if settings.version.trim().is_empty() {
                    push(field, Problem::Empty);
                } else if !is_valid_tag(&settings.version) {
                    push(field, Problem::InvalidTag);
                }
            }
            Field::Port => {
                if settings.port == 0 {
                    push(field, Problem::InvalidPort);
                } else if let Some(other) = port_conflict(config, id, settings) {
                    push(field, Problem::PortConflict(other));
                } else if !port_free(settings.port) {
                    push(field, Problem::PortInUse);
                }
            }
            Field::Database => {
                let valid = if id == EngineId::Sqlite {
                    is_valid_file_name(&settings.database)
                } else {
                    is_identifier(&settings.database)
                };
                if settings.database.is_empty() {
                    push(field, Problem::Empty);
                } else if !valid {
                    let problem = if id == EngineId::Sqlite {
                        Problem::InvalidFileName
                    } else {
                        Problem::InvalidIdentifier
                    };
                    push(field, problem);
                }
            }
            Field::User => {
                if settings.user.is_empty() {
                    push(field, Problem::Empty);
                } else if !is_identifier(&settings.user) {
                    push(field, Problem::InvalidIdentifier);
                } else if matches!(id, EngineId::MySql | EngineId::MariaDb)
                    && settings.user.eq_ignore_ascii_case("root")
                {
                    push(field, Problem::ReservedUser);
                }
            }
            Field::Password => {
                if let Some(problem) = password_problem(&settings.password) {
                    push(field, problem);
                }
            }
            Field::AdminPassword => {
                if let Some(problem) = password_problem(&settings.admin_password) {
                    push(field, problem);
                } else if id == EngineId::MsSql && !mssql_password_ok(&settings.admin_password) {
                    push(field, Problem::WeakMssqlPassword);
                }
            }
            Field::Seed | Field::LanAccess => {}
        }
    }
    issues
}

/// Validates every enabled engine without probing ports. Used before
/// "Start all".
pub fn validate_enabled(config: &AppConfig) -> Vec<(EngineId, Issue)> {
    config
        .engines
        .iter()
        .filter(|(_, s)| s.enabled)
        .flat_map(|(&id, s)| {
            validate_engine(config, id, s, |_| true)
                .into_iter()
                .map(move |issue| (id, issue))
        })
        .collect()
}

/// First other enabled engine that uses the same host port.
pub fn port_conflict(
    config: &AppConfig,
    id: EngineId,
    settings: &EngineSettings,
) -> Option<EngineId> {
    if !settings.enabled {
        return None;
    }
    config
        .engines
        .iter()
        .filter(|(&other, s)| {
            other != id && s.enabled && other.engine().uses(Field::Port) && s.port == settings.port
        })
        .map(|(&other, _)| other)
        .next()
}

/// SQL Server password policy: at least 8 characters from at least three of
/// the four categories upper case, lower case, digits and symbols.
pub fn mssql_password_ok(password: &str) -> bool {
    let categories = [
        password.chars().any(|c| c.is_ascii_uppercase()),
        password.chars().any(|c| c.is_ascii_lowercase()),
        password.chars().any(|c| c.is_ascii_digit()),
        password.chars().any(|c| !c.is_alphanumeric()),
    ];
    password.chars().count() >= 8 && categories.iter().filter(|&&c| c).count() >= 3
}

fn password_problem(password: &str) -> Option<Problem> {
    if password.is_empty() {
        Some(Problem::Empty)
    } else if password
        .chars()
        .any(|c| !c.is_ascii_graphic() || FORBIDDEN_PASSWORD_CHARS.contains(&c))
    {
        Some(Problem::UnsupportedCharacters)
    } else {
        None
    }
}

/// Plain SQL identifier: ASCII letter or `_`, then letters, digits or `_`.
pub fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && s.len() <= 63
}

fn is_valid_file_name(s: &str) -> bool {
    !s.starts_with('.')
        && s.len() <= 100
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// Docker tag grammar: `[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}`.
fn is_valid_tag(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        && s.len() <= 128
}

/// True if nothing listens on `port`, neither on all interfaces nor on
/// loopback (Windows allows binding the more specific address even when the
/// wildcard one is taken, so both are checked).
pub fn is_port_free(port: u16) -> bool {
    [Ipv4Addr::UNSPECIFIED, Ipv4Addr::LOCALHOST]
        .into_iter()
        .all(|ip| TcpListener::bind((ip, port)).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(config: &AppConfig, id: EngineId) -> Vec<Issue> {
        validate_engine(config, id, config.engine(id), |_| true)
    }

    #[test]
    fn defaults_are_valid() {
        let config = AppConfig::default();
        for id in EngineId::ALL {
            assert_eq!(check(&config, id), vec![], "{id:?}");
        }
        assert!(validate_enabled(&config).is_empty());
    }

    #[test]
    fn mssql_password_policy() {
        assert!(mssql_password_ok("Test_Passw0rd!"));
        assert!(mssql_password_ok("Abcdefg1"));
        assert!(mssql_password_ok("abcdef1!"));
        assert!(!mssql_password_ok("Abc1!"), "too short");
        assert!(!mssql_password_ok("abcdefgh1"), "two categories");
        assert!(!mssql_password_ok("ABCDEFGH"), "one category");
        assert!(!mssql_password_ok(""));
    }

    #[test]
    fn weak_sa_password_is_reported() {
        let mut config = AppConfig::default();
        config.engine_mut(EngineId::MsSql).admin_password = "tester123".into();
        assert_eq!(
            check(&config, EngineId::MsSql),
            vec![Issue {
                field: Field::AdminPassword,
                problem: Problem::WeakMssqlPassword
            }]
        );
    }

    #[test]
    fn port_conflicts_between_enabled_engines() {
        let mut config = AppConfig::default();
        config.engine_mut(EngineId::MySql).port = 5432;
        let issues = check(&config, EngineId::MySql);
        assert_eq!(issues[0].problem, Problem::PortConflict(EngineId::Postgres));
        assert_eq!(
            check(&config, EngineId::Postgres)[0].problem,
            Problem::PortConflict(EngineId::MySql)
        );

        config.engine_mut(EngineId::Postgres).enabled = false;
        assert!(check(&config, EngineId::MySql).is_empty());
        assert!(check(&config, EngineId::Postgres).is_empty());
    }

    #[test]
    fn disabled_interbase_does_not_conflict() {
        let mut config = AppConfig::default();
        config.engine_mut(EngineId::Firebird).port = 3051;
        assert!(check(&config, EngineId::Firebird).is_empty());
        config.engine_mut(EngineId::InterBase).enabled = true;
        assert_eq!(
            check(&config, EngineId::Firebird)[0].problem,
            Problem::PortConflict(EngineId::InterBase)
        );
    }

    #[test]
    fn busy_port_is_a_warning() {
        let config = AppConfig::default();
        let id = EngineId::Postgres;
        let issues = validate_engine(&config, id, config.engine(id), |_| false);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].problem, Problem::PortInUse);
        assert_eq!(issues[0].severity(), Severity::Warning);
    }

    #[test]
    fn port_probe_detects_listener() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(!is_port_free(port));
    }

    #[test]
    fn empty_and_invalid_fields() {
        let mut config = AppConfig::default();
        let pg = config.engine_mut(EngineId::Postgres);
        pg.version = " ".into();
        pg.port = 0;
        pg.database = "test db".into();
        pg.user = String::new();
        pg.password = "a'b".into();
        let problems: Vec<_> = check(&config, EngineId::Postgres)
            .into_iter()
            .map(|i| (i.field, i.problem))
            .collect();
        assert_eq!(
            problems,
            vec![
                (Field::Version, Problem::Empty),
                (Field::Port, Problem::InvalidPort),
                (Field::Database, Problem::InvalidIdentifier),
                (Field::User, Problem::Empty),
                (Field::Password, Problem::UnsupportedCharacters),
            ]
        );
    }

    #[test]
    fn mysql_user_cannot_be_root() {
        let mut config = AppConfig::default();
        config.engine_mut(EngineId::MySql).user = "Root".into();
        assert_eq!(
            check(&config, EngineId::MySql)[0].problem,
            Problem::ReservedUser
        );
        config.engine_mut(EngineId::Postgres).user = "root".into();
        assert!(check(&config, EngineId::Postgres).is_empty());
    }

    #[test]
    fn sqlite_file_name() {
        let mut config = AppConfig::default();
        for bad in ["../x.db", ".hidden", "a/b.db", "a b.db"] {
            config.engine_mut(EngineId::Sqlite).database = bad.into();
            assert_eq!(
                check(&config, EngineId::Sqlite)[0].problem,
                Problem::InvalidFileName,
                "{bad}"
            );
        }
        config.engine_mut(EngineId::Sqlite).database = "my-test_1.sqlite".into();
        assert!(check(&config, EngineId::Sqlite).is_empty());
    }

    #[test]
    fn identifiers_and_tags() {
        assert!(is_identifier("testdb"));
        assert!(is_identifier("_x1"));
        assert!(!is_identifier("1abc"));
        assert!(!is_identifier("zażółć"));
        assert!(!is_identifier(&"a".repeat(64)));
        assert!(is_valid_tag("2022-latest"));
        assert!(is_valid_tag("17.2-alpine"));
        assert!(!is_valid_tag("-x"));
        assert!(!is_valid_tag("a:b"));
    }

    #[test]
    fn passwords_with_unsupported_characters() {
        for bad in [
            "with space",
            "quo\"te",
            "back\\slash",
            "semi;colon",
            "zażółć1A!",
        ] {
            assert_eq!(
                password_problem(bad),
                Some(Problem::UnsupportedCharacters),
                "{bad}"
            );
        }
        assert_eq!(password_problem("p@ss:w/rd$#%"), None);
    }
}
