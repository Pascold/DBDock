//! Parsing `docker compose ps --format json` and deriving engine states.

use serde::Deserialize;

use crate::engines::EngineId;

/// One container as reported by `docker compose ps -a --format json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ContainerState {
    #[serde(rename = "Service")]
    pub service: String,
    /// `running`, `exited`, `created`, `restarting`, `paused`, `dead`, ...
    #[serde(rename = "State", default)]
    pub state: String,
    /// `healthy`, `starting`, `unhealthy` or empty without a healthcheck.
    #[serde(rename = "Health", default)]
    pub health: String,
    #[serde(rename = "ExitCode", default)]
    pub exit_code: i64,
}

/// Parses `ps` output. Compose prints one JSON object per line (newer
/// versions) or a single JSON array (older ones); both are accepted.
pub fn parse_ps(output: &str) -> Result<Vec<ContainerState>, serde_json::Error> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str(trimmed);
    }
    trimmed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect()
}

/// State of an engine as shown on its card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStatus {
    /// No container exists (never started or reset).
    NotCreated,
    Stopped,
    /// Container runs but is not healthy yet (or initialisation runs).
    Starting,
    /// Healthcheck passes (for SQLite: the database file exists).
    Healthy,
    /// Running, without a healthcheck to confirm readiness.
    Running,
    /// Unhealthy, crash-looping or initialisation failed.
    Error,
}

impl EngineStatus {
    /// True if a container exists, i.e. data may have been initialised.
    pub fn is_created(self) -> bool {
        self != EngineStatus::NotCreated
    }

    /// True if the server process is (or is becoming) available.
    pub fn is_up(self) -> bool {
        matches!(
            self,
            EngineStatus::Starting | EngineStatus::Healthy | EngineStatus::Running
        )
    }
}

/// Status of `id`, derived from all containers of the project.
/// `sqlite_file_exists` tells whether the SQLite database file is present.
pub fn engine_status(
    id: EngineId,
    containers: &[ContainerState],
    sqlite_file_exists: bool,
) -> EngineStatus {
    let find = |service: &str| containers.iter().find(|c| c.service == service);

    if id == EngineId::Sqlite {
        return match find(id.key()) {
            Some(c) if c.state == "running" => EngineStatus::Starting,
            _ if sqlite_file_exists => EngineStatus::Healthy,
            Some(c) if c.exit_code != 0 => EngineStatus::Error,
            _ => EngineStatus::NotCreated,
        };
    }

    let Some(main) = find(id.key()) else {
        return EngineStatus::NotCreated;
    };
    let status = container_status(main);

    // SQL Server is usable only after the init container created the
    // database and the user.
    if id == EngineId::MsSql && status == EngineStatus::Healthy {
        return match find("mssql-init") {
            Some(init) if init.state == "running" || init.state == "created" => {
                EngineStatus::Starting
            }
            Some(init) if init.exit_code != 0 => EngineStatus::Error,
            None => EngineStatus::Starting,
            Some(_) => EngineStatus::Healthy,
        };
    }
    status
}

fn container_status(c: &ContainerState) -> EngineStatus {
    match c.state.as_str() {
        "running" => match c.health.as_str() {
            "healthy" => EngineStatus::Healthy,
            "starting" => EngineStatus::Starting,
            "unhealthy" => EngineStatus::Error,
            _ => EngineStatus::Running,
        },
        "restarting" | "dead" => EngineStatus::Error,
        "removing" => EngineStatus::NotCreated,
        // exited, created, paused: the container exists but does not serve.
        // Exit codes of stopped servers vary (0, 137, 143...), so they are
        // not treated as errors.
        _ => EngineStatus::Stopped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(service: &str, state: &str, health: &str, exit_code: i64) -> ContainerState {
        ContainerState {
            service: service.into(),
            state: state.into(),
            health: health.into(),
            exit_code,
        }
    }

    #[test]
    fn parses_ndjson_and_arrays() {
        let line = r#"{"Command":"x","ExitCode":0,"Health":"healthy","ID":"6e","Service":"firebird","State":"running","Labels":"a=b"}"#;
        let ndjson = format!("{line}\n{}\n", line.replace("firebird", "mysql"));
        let parsed = parse_ps(&ndjson).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], c("firebird", "running", "healthy", 0));
        assert_eq!(parsed[1].service, "mysql");

        let array = format!("[{line}]");
        assert_eq!(
            parse_ps(&array).unwrap(),
            vec![c("firebird", "running", "healthy", 0)]
        );
        assert_eq!(parse_ps("  \n").unwrap(), vec![]);
        assert!(parse_ps("garbage").is_err());
    }

    #[test]
    fn missing_optional_fields() {
        let parsed = parse_ps(r#"{"Service":"sqlite","State":"exited"}"#).unwrap();
        assert_eq!(parsed[0], c("sqlite", "exited", "", 0));
    }

    #[test]
    fn server_states() {
        let id = EngineId::Postgres;
        let status = |containers: &[ContainerState]| engine_status(id, containers, false);
        assert_eq!(status(&[]), EngineStatus::NotCreated);
        assert_eq!(
            status(&[c("mysql", "running", "healthy", 0)]),
            EngineStatus::NotCreated
        );
        assert_eq!(
            status(&[c("postgres", "running", "starting", 0)]),
            EngineStatus::Starting
        );
        assert_eq!(
            status(&[c("postgres", "running", "healthy", 0)]),
            EngineStatus::Healthy
        );
        assert_eq!(
            status(&[c("postgres", "running", "unhealthy", 0)]),
            EngineStatus::Error
        );
        assert_eq!(
            status(&[c("postgres", "running", "", 0)]),
            EngineStatus::Running
        );
        assert_eq!(
            status(&[c("postgres", "exited", "", 143)]),
            EngineStatus::Stopped
        );
        assert_eq!(
            status(&[c("postgres", "created", "", 0)]),
            EngineStatus::Stopped
        );
        assert_eq!(
            status(&[c("postgres", "restarting", "", 1)]),
            EngineStatus::Error
        );
    }

    #[test]
    fn mssql_waits_for_init_container() {
        let server = c("mssql", "running", "healthy", 0);
        let status = |init: Option<ContainerState>| {
            let mut all = vec![server.clone()];
            all.extend(init);
            engine_status(EngineId::MsSql, &all, false)
        };
        assert_eq!(status(None), EngineStatus::Starting);
        assert_eq!(
            status(Some(c("mssql-init", "running", "", 0))),
            EngineStatus::Starting
        );
        assert_eq!(
            status(Some(c("mssql-init", "exited", "", 1))),
            EngineStatus::Error
        );
        assert_eq!(
            status(Some(c("mssql-init", "exited", "", 0))),
            EngineStatus::Healthy
        );
        assert_eq!(
            engine_status(
                EngineId::MsSql,
                &[c("mssql", "running", "starting", 0)],
                false
            ),
            EngineStatus::Starting
        );
    }

    #[test]
    fn sqlite_depends_on_the_file() {
        let id = EngineId::Sqlite;
        assert_eq!(engine_status(id, &[], false), EngineStatus::NotCreated);
        assert_eq!(engine_status(id, &[], true), EngineStatus::Healthy);
        assert_eq!(
            engine_status(id, &[c("sqlite", "running", "", 0)], false),
            EngineStatus::Starting
        );
        assert_eq!(
            engine_status(id, &[c("sqlite", "exited", "", 1)], false),
            EngineStatus::Error
        );
        assert_eq!(
            engine_status(id, &[c("sqlite", "exited", "", 0)], true),
            EngineStatus::Healthy
        );
    }
}
