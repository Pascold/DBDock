//! Background execution of Docker commands.
//!
//! The UI owns a [`Docker`] handle, starts work through it and drains
//! [`Event`]s every frame. No method blocks.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use super::status::{parse_ps, ContainerState};
use super::{check, compose_args, run, run_streaming, DockerCheck};
use crate::compose::PROJECT;
use crate::engines::EngineId;

/// Identifies a running operation.
pub type OpId = u64;

/// Operations on engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    /// `up -d`: create if needed and start.
    Start,
    /// `stop`; with no engines, stops the whole project.
    Stop,
    /// `restart` of existing containers.
    Restart,
    /// `up -d --force-recreate`: apply changed settings.
    Recreate,
    /// Remove containers and data volumes.
    Reset,
    /// Remove containers but keep data (used when an engine is disabled).
    Remove,
}

/// Messages from background threads to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Checked(DockerCheck),
    /// Fresh container list, or the error text of a failed `ps`.
    Status(Result<Vec<ContainerState>, String>),
    /// One line of output of a running operation.
    Output {
        op: OpId,
        line: String,
    },
    Finished {
        op: OpId,
        result: Result<(), String>,
    },
    Logs {
        engine: EngineId,
        result: Result<String, String>,
    },
}

/// Number of log lines fetched for the logs window.
pub const LOG_LINES: usize = 500;

const STATUS_INTERVAL: Duration = Duration::from_secs(3);
const STATUS_TIMEOUT: Duration = Duration::from_secs(15);
const LOGS_TIMEOUT: Duration = Duration::from_secs(30);

type Notify = Arc<dyn Fn() + Send + Sync>;

/// Handle to the background Docker machinery.
pub struct Docker {
    tx: Sender<Event>,
    rx: Receiver<Event>,
    notify: Notify,
    poke: Option<Sender<()>>,
    next_op: OpId,
}

impl Docker {
    /// `notify` is called after every event, e.g. to request a repaint.
    pub fn new(notify: impl Fn() + Send + Sync + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            notify: Arc::new(notify),
            poke: None,
            next_op: 0,
        }
    }

    /// Returns all events received so far.
    pub fn drain(&self) -> Vec<Event> {
        self.rx.try_iter().collect()
    }

    /// Checks Docker availability in the background.
    pub fn check(&self) {
        let sender = self.sender();
        std::thread::spawn(move || sender.send(Event::Checked(check())));
    }

    /// Starts polling the project status every few seconds. Calling it again
    /// replaces the previous poller.
    pub fn start_polling(&mut self, compose_file: PathBuf) {
        let (poke_tx, poke_rx) = mpsc::channel();
        self.poke = Some(poke_tx);
        let sender = self.sender();
        std::thread::spawn(move || loop {
            let result = fetch_status(&compose_file);
            if !sender.send(Event::Status(result)) {
                return;
            }
            match poke_rx.recv_timeout(STATUS_INTERVAL) {
                Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                // The handle was dropped or replaced.
                Err(RecvTimeoutError::Disconnected) => return,
            }
        });
    }

    /// Asks the poller to refresh now.
    pub fn refresh_status(&self) {
        if let Some(poke) = &self.poke {
            let _ = poke.send(());
        }
    }

    /// Runs an operation on `engines` in the background. Output lines and
    /// the result arrive as events tagged with the returned id.
    pub fn run_op(&mut self, kind: OpKind, compose_file: &Path, engines: &[EngineId]) -> OpId {
        self.next_op += 1;
        let op = self.next_op;
        let commands = op_commands(kind, compose_file, engines);
        let sender = self.sender();
        let poke = self.poke.clone();
        std::thread::spawn(move || {
            let result = run_all(op, &commands, &sender);
            sender.send(Event::Finished { op, result });
            if let Some(poke) = poke {
                let _ = poke.send(());
            }
        });
        op
    }

    /// Fetches the last [`LOG_LINES`] log lines of an engine.
    pub fn logs(&self, compose_file: &Path, engine: EngineId) {
        let mut args = vec!["logs", "--no-color", "--tail"];
        let tail = LOG_LINES.to_string();
        args.push(&tail);
        args.extend(engine.engine().start_services());
        let args = compose_args(compose_file, &args);
        let sender = self.sender();
        std::thread::spawn(move || {
            let result = match run(&args, LOGS_TIMEOUT) {
                Ok(out) if out.success => Ok(format!("{}{}", out.stdout, out.stderr)),
                Ok(out) => Err(out.error_text()),
                Err(e) => Err(e.to_string()),
            };
            sender.send(Event::Logs { engine, result });
        });
    }

    fn sender(&self) -> EventSender {
        EventSender {
            tx: self.tx.clone(),
            notify: Arc::clone(&self.notify),
        }
    }
}

/// Sends an event and wakes the UI.
#[derive(Clone)]
struct EventSender {
    tx: Sender<Event>,
    notify: Notify,
}

impl EventSender {
    /// Returns false once the UI side is gone.
    fn send(&self, event: Event) -> bool {
        let ok = self.tx.send(event).is_ok();
        (self.notify)();
        ok
    }
}

fn fetch_status(compose_file: &Path) -> Result<Vec<ContainerState>, String> {
    let args = compose_args(compose_file, &["ps", "-a", "--format", "json"]);
    let out = run(&args, STATUS_TIMEOUT).map_err(|e| e.to_string())?;
    if !out.success {
        return Err(out.error_text());
    }
    parse_ps(&out.stdout).map_err(|e| format!("cannot parse docker compose ps output: {e}"))
}

fn run_all(op: OpId, commands: &[Vec<String>], sender: &EventSender) -> Result<(), String> {
    for args in commands {
        let line_sender = sender.clone();
        line_sender.send(Event::Output {
            op,
            line: format!("$ {}", super::display_command(args)),
        });
        let tail = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let tail_writer = Arc::clone(&tail);
        let ok = run_streaming(args, move |line| {
            if let Ok(mut tail) = tail_writer.lock() {
                tail.push(line.clone());
                if tail.len() > 5 {
                    tail.remove(0);
                }
            }
            line_sender.send(Event::Output { op, line });
        })
        .map_err(|e| format!("cannot run docker: {e}"))?;
        if !ok {
            let tail = tail.lock().map(|t| t.join("\n")).unwrap_or_default();
            return Err(if tail.is_empty() {
                "docker command failed".to_owned()
            } else {
                tail
            });
        }
    }
    Ok(())
}

/// Docker invocations (argument lists) for an operation.
pub fn op_commands(kind: OpKind, compose_file: &Path, engines: &[EngineId]) -> Vec<Vec<String>> {
    let services: Vec<&str> = engines
        .iter()
        .flat_map(|id| id.engine().start_services().iter().copied())
        .collect();
    let compose = |verb: &[&str]| {
        let mut args = verb.to_vec();
        args.extend(&services);
        compose_args(compose_file, &args)
    };
    match kind {
        OpKind::Start => vec![compose(&["up", "-d"])],
        OpKind::Stop => vec![compose(&["stop"])],
        OpKind::Restart => vec![compose(&["restart"])],
        OpKind::Recreate => vec![compose(&["up", "-d", "--force-recreate"])],
        OpKind::Remove => vec![compose(&["rm", "--stop", "--force"])],
        OpKind::Reset => {
            let mut commands = vec![compose(&["rm", "--stop", "--force", "--volumes"])];
            let volumes: Vec<String> = engines
                .iter()
                .flat_map(|id| id.engine().volumes().iter())
                .map(|v| format!("{PROJECT}_{v}"))
                .collect();
            if !volumes.is_empty() {
                let mut args = vec!["volume".to_owned(), "rm".to_owned(), "--force".to_owned()];
                args.extend(volumes);
                commands.push(args);
            }
            commands
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commands(kind: OpKind, engines: &[EngineId]) -> Vec<String> {
        op_commands(kind, Path::new("f.yml"), engines)
            .iter()
            .map(|args| super::super::display_command(args))
            .collect()
    }

    const PREFIX: &str = "docker compose -p dbdock -f f.yml --profile interbase --ansi never";

    #[test]
    fn start_includes_helper_services() {
        assert_eq!(
            commands(OpKind::Start, &[EngineId::Postgres, EngineId::MsSql]),
            vec![format!("{PREFIX} up -d postgres mssql mssql-init")]
        );
    }

    #[test]
    fn stop_without_engines_stops_everything() {
        assert_eq!(commands(OpKind::Stop, &[]), vec![format!("{PREFIX} stop")]);
    }

    #[test]
    fn reset_removes_data_volumes_only() {
        assert_eq!(
            commands(OpKind::Reset, &[EngineId::InterBase]),
            vec![
                format!("{PREFIX} rm --stop --force --volumes interbase"),
                "docker volume rm --force dbdock_interbase-data".to_owned(),
            ]
        );
        assert_eq!(
            commands(OpKind::Reset, &[EngineId::Sqlite]),
            vec![format!("{PREFIX} rm --stop --force --volumes sqlite")]
        );
    }

    #[test]
    fn recreate_forces_new_containers() {
        assert_eq!(
            commands(OpKind::Recreate, &[EngineId::MySql]),
            vec![format!("{PREFIX} up -d --force-recreate mysql")]
        );
    }

    #[test]
    fn events_reach_the_receiver() {
        let docker = Docker::new(|| {});
        let sender = docker.sender();
        assert!(sender.send(Event::Checked(DockerCheck::NotInstalled)));
        assert_eq!(
            docker.drain(),
            vec![Event::Checked(DockerCheck::NotInstalled)]
        );
    }
}
