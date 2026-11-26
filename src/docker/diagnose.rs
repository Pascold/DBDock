//! Why a container does not work: the failing healthcheck output or the
//! error lines of its log, shown on the engine card.
//!
//! Docker keeps healthcheck results out of `docker compose ps` and the logs,
//! so without this a database stuck in "Starting" gave no hint at all.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::status::ContainerState;
use super::{run, CommandOutput};

/// A problem found for one container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnosis {
    /// The most telling line(s), already trimmed.
    pub text: String,
    /// The data volume holds data of another version; only a reset helps.
    pub incompatible_data: bool,
}

/// Failed healthchecks in a row before a starting container is reported.
/// Servers fail a few checks while they boot; 20 checks are a minute or more.
const MIN_FAILING_STREAK: u32 = 20;
/// How long a diagnosis is reused before Docker is asked again.
const REFRESH: Duration = Duration::from_secs(10);
const TIMEOUT: Duration = Duration::from_secs(10);
const LOG_LINES: &str = "40";
/// Exit codes of a server that was stopped normally.
const NORMAL_EXIT: [i64; 3] = [0, 137, 143];
const MAX_LEN: usize = 600;

/// Messages of a server that refuses data written by another version.
const INCOMPATIBLE_DATA: [&str; 7] = [
    // PostgreSQL, another major version.
    "database files are incompatible with server",
    // PostgreSQL 18+ images finding data of the old layout.
    "pg_ctlcluster",
    // Firebird, another on-disk structure (ODS).
    "unsupported on-disk structure",
    // MySQL downgrade.
    "downgrade is not supported",
    "cannot boot server version",
    // MariaDB downgrade.
    "unsupported redo log format",
    "upgrade after a crash is not supported",
];

/// Words that mark an error line in a log.
const ERROR_WORDS: [&str; 7] = [
    "fatal",
    "error",
    "failed",
    "unsupported",
    "incompatible",
    "cannot",
    "not supported",
];

/// What to ask Docker about a container, if anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Check {
    Health,
    Logs,
}

fn check_for(c: &ContainerState) -> Option<Check> {
    match c.state.as_str() {
        "running" if matches!(c.health.as_str(), "starting" | "unhealthy") => Some(Check::Health),
        "restarting" => Some(Check::Logs),
        "exited" | "dead" if !NORMAL_EXIT.contains(&c.exit_code) => {
            // InterBase without a license exits on purpose; its card says so.
            let license_exit = c.service == "interbase"
                && c.exit_code == crate::engines::interbase::LICENSE_MISSING_EXIT;
            (!license_exit).then_some(Check::Logs)
        }
        _ => None,
    }
}

/// True if `text` says the data was written by another version.
pub fn is_incompatible_data(text: &str) -> bool {
    let lower = text.to_lowercase();
    INCOMPATIBLE_DATA.iter().any(|m| lower.contains(m))
}

#[derive(Deserialize)]
struct Health {
    #[serde(rename = "Status", default)]
    status: String,
    #[serde(rename = "FailingStreak", default)]
    failing_streak: u32,
    #[serde(rename = "Log", default)]
    log: Vec<HealthLog>,
}

#[derive(Deserialize)]
struct HealthLog {
    #[serde(rename = "ExitCode", default)]
    exit_code: i64,
    #[serde(rename = "Output", default)]
    output: String,
}

/// Diagnosis from `docker inspect --format '{{json .State.Health}}'`:
/// the output of the last failed check, once the container is unhealthy
/// or has failed for a while.
pub fn from_health(json: &str) -> Option<Diagnosis> {
    let health: Health = serde_json::from_str(json.trim()).ok()?;
    if health.status != "unhealthy" && health.failing_streak < MIN_FAILING_STREAK {
        return None;
    }
    let output = &health.log.iter().rev().find(|l| l.exit_code != 0)?.output;
    let text = output
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())?;
    Some(Diagnosis {
        text: shorten(text),
        incompatible_data: is_incompatible_data(output),
    })
}

/// Diagnosis from the last lines of `docker logs`: the last error line,
/// with a following PostgreSQL `DETAIL:` line.
pub fn from_logs(logs: &str) -> Option<Diagnosis> {
    let lines: Vec<&str> = logs.lines().map(str::trim).collect();
    let index = lines.iter().rposition(|line| {
        let lower = line.to_lowercase();
        ERROR_WORDS.iter().any(|w| lower.contains(w))
    })?;
    let mut text = strip_prefix(lines[index]).to_owned();
    if let Some(detail) = lines.get(index + 1).filter(|l| l.contains("DETAIL:")) {
        text.push('\n');
        text.push_str(strip_prefix(detail));
    }
    Some(Diagnosis {
        text: shorten(&text),
        incompatible_data: is_incompatible_data(logs),
    })
}

/// Drops a timestamp or similar prefix before the level of a log line.
fn strip_prefix(line: &str) -> &str {
    ["FATAL:", "ERROR:", "DETAIL:", "[ERROR]", "Error:"]
        .iter()
        .filter_map(|level| line.find(level))
        .min()
        .map_or(line, |start| &line[start..])
}

fn shorten(text: &str) -> String {
    if text.chars().count() <= MAX_LEN {
        return text.to_owned();
    }
    let mut short: String = text.chars().take(MAX_LEN).collect();
    short.push('…');
    short
}

/// Asks Docker about containers with a problem and remembers the answers
/// for a few seconds, so polling every few seconds stays cheap.
#[derive(Default)]
pub struct Diagnoser {
    cache: HashMap<String, (String, Instant, Option<Diagnosis>)>,
}

impl Diagnoser {
    /// Sets [`ContainerState::diagnosis`] on every container with a problem.
    pub fn fill(&mut self, containers: &mut [ContainerState]) {
        let now = Instant::now();
        self.cache
            .retain(|name, _| containers.iter().any(|c| &c.name == name));
        for c in containers.iter_mut() {
            let Some(check) = check_for(c).filter(|_| !c.name.is_empty()) else {
                continue;
            };
            // A change of state asks again at once.
            let key = format!("{}|{}|{}", c.state, c.health, c.exit_code);
            if let Some((cached_key, at, diagnosis)) = self.cache.get(&c.name) {
                if *cached_key == key && now.duration_since(*at) < REFRESH {
                    c.diagnosis = diagnosis.clone();
                    continue;
                }
            }
            let diagnosis = match check {
                Check::Health => {
                    docker(&["inspect", "--format", "{{json .State.Health}}", &c.name])
                        .and_then(|out| from_health(&out.stdout))
                }
                Check::Logs => docker(&["logs", "--tail", LOG_LINES, &c.name])
                    .and_then(|out| from_logs(&format!("{}\n{}", out.stdout, out.stderr))),
            };
            self.cache
                .insert(c.name.clone(), (key, now, diagnosis.clone()));
            c.diagnosis = diagnosis;
        }
    }
}

fn docker(args: &[&str]) -> Option<CommandOutput> {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    run(&args, TIMEOUT).ok().filter(|out| out.success)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn health(status: &str, streak: u32, output: &str) -> String {
        serde_json::json!({
            "Status": status,
            "FailingStreak": streak,
            "Log": [
                {"ExitCode": 0, "Output": "ok"},
                {"ExitCode": 1, "Output": output}
            ]
        })
        .to_string()
    }

    #[test]
    fn firebird_with_data_of_another_version() {
        let json = health(
            "starting",
            32,
            "Statement failed, SQLSTATE = HY000\nunsupported on-disk structure for file \
             /var/lib/firebird/data/testdb.fdb; found 12.0, support 13.1\n",
        );
        let d = from_health(&json).unwrap();
        assert!(d.text.starts_with("unsupported on-disk structure"));
        assert!(d.incompatible_data);
    }

    #[test]
    fn a_booting_server_is_not_reported() {
        assert_eq!(
            from_health(&health("starting", 3, "connection refused")),
            None
        );
        assert_eq!(from_health(&health("healthy", 0, "")), None);
        let d = from_health(&health("unhealthy", 3, "Login failed for user 'sa'.")).unwrap();
        assert_eq!(d.text, "Login failed for user 'sa'.");
        assert!(!d.incompatible_data);
        assert_eq!(from_health("garbage"), None);
    }

    #[test]
    fn postgres_major_version_change() {
        let logs = "PostgreSQL Database directory appears to contain a database; Skipping initialization\n\
            \n\
            2026-11-26 21:21:48.351 UTC [1] FATAL:  database files are incompatible with server\n\
            2026-11-26 21:21:48.351 UTC [1] DETAIL:  The data directory was initialized by PostgreSQL version 17, which is not compatible with this version 16.11.\n";
        let d = from_logs(logs).unwrap();
        assert_eq!(
            d.text,
            "FATAL:  database files are incompatible with server\n\
             DETAIL:  The data directory was initialized by PostgreSQL version 17, which is not compatible with this version 16.11."
        );
        assert!(d.incompatible_data);
    }

    #[test]
    fn postgres_18_finds_the_old_layout() {
        let logs =
            "Error: in 18+, these Docker images are configured to store database data in a\n\
            format which is compatible with \"pg_ctlcluster\" (specifically, using\n\
            major-version-specific directory names).\n\
            See also https://github.com/docker-library/postgres/pull/1259\n";
        let d = from_logs(logs).unwrap();
        assert!(d.text.starts_with("Error: in 18+"));
        assert!(d.incompatible_data);
    }

    #[test]
    fn other_errors_are_shown_without_a_reset_hint() {
        let d = from_logs("starting\nSqlcmd: Error: Login failed for user 'sa'.\n").unwrap();
        assert_eq!(d.text, "Error: Login failed for user 'sa'.");
        assert!(!d.incompatible_data);
        assert_eq!(from_logs("all good\nready\n"), None);
    }

    #[test]
    fn what_is_checked() {
        let c = |state: &str, health: &str, exit_code: i64, service: &str| ContainerState {
            service: service.into(),
            name: format!("dbdock-{service}-1"),
            state: state.into(),
            health: health.into(),
            exit_code,
            diagnosis: None,
        };
        assert_eq!(
            check_for(&c("running", "starting", 0, "pg")),
            Some(Check::Health)
        );
        assert_eq!(
            check_for(&c("running", "unhealthy", 0, "pg")),
            Some(Check::Health)
        );
        assert_eq!(check_for(&c("running", "healthy", 0, "pg")), None);
        assert_eq!(check_for(&c("running", "", 0, "pg")), None);
        assert_eq!(check_for(&c("exited", "", 1, "pg")), Some(Check::Logs));
        assert_eq!(check_for(&c("restarting", "", 1, "pg")), Some(Check::Logs));
        for normal in NORMAL_EXIT {
            assert_eq!(check_for(&c("exited", "", normal, "pg")), None);
        }
        assert_eq!(check_for(&c("exited", "", 78, "interbase")), None);
        assert_eq!(
            check_for(&c("exited", "", 78, "postgres")),
            Some(Check::Logs)
        );
    }

    #[test]
    fn long_text_is_shortened() {
        let d = from_logs(&format!("FATAL: {}", "x".repeat(2000))).unwrap();
        assert_eq!(d.text.chars().count(), MAX_LEN + 1);
    }
}
