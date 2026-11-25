//! Running the Docker CLI.
//!
//! Everything here blocks; the UI only talks to [`worker::Docker`], which
//! runs these functions on background threads.

pub mod status;
pub mod worker;

use std::io::{BufRead as _, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::compose::{MANUAL_PROFILE, PROJECT};

/// Outcome of the Docker availability check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockerCheck {
    Ready {
        server_version: String,
        compose_version: String,
    },
    /// The `docker` executable was not found.
    NotInstalled,
    /// The CLI exists but the daemon does not answer (Docker Desktop not
    /// started, missing permissions, ...).
    DaemonDown(String),
    /// Docker works but the Compose v2 plugin is missing.
    ComposeMissing,
}

impl DockerCheck {
    pub fn is_ready(&self) -> bool {
        matches!(self, DockerCheck::Ready { .. })
    }
}

/// Result of a finished command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    /// Short error description: stderr, or stdout if stderr is empty.
    pub fn error_text(&self) -> String {
        let text = if self.stderr.trim().is_empty() {
            &self.stdout
        } else {
            &self.stderr
        };
        let text = text.trim();
        if text.is_empty() {
            "command failed without output".to_owned()
        } else {
            text.to_owned()
        }
    }
}

/// Path of the `docker` executable.
///
/// GUI applications on macOS start with a minimal `PATH`, so well-known
/// install locations are tried when `PATH` does not contain Docker.
pub fn docker_program() -> &'static Path {
    static PROGRAM: OnceLock<PathBuf> = OnceLock::new();
    PROGRAM.get_or_init(|| find_docker().unwrap_or_else(|| PathBuf::from("docker")))
}

fn find_docker() -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        "docker.exe"
    } else {
        "docker"
    };
    let from_path = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();

    let mut candidates: Vec<PathBuf> = from_path.into_iter().map(|dir| dir.join(exe)).collect();
    if cfg!(windows) {
        for var in ["ProgramFiles", "ProgramW6432"] {
            if let Some(dir) = std::env::var_os(var) {
                candidates.push(
                    PathBuf::from(dir)
                        .join("Docker")
                        .join("Docker")
                        .join("resources")
                        .join("bin")
                        .join(exe),
                );
            }
        }
    } else {
        candidates.extend(
            [
                "/usr/local/bin/docker",
                "/opt/homebrew/bin/docker",
                "/usr/bin/docker",
                "/Applications/Docker.app/Contents/Resources/bin/docker",
            ]
            .map(PathBuf::from),
        );
        if let Some(home) = std::env::var_os("HOME") {
            candidates.push(PathBuf::from(home).join(".docker").join("bin").join(exe));
        }
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// A `docker` command that never opens a console window on Windows and does
/// not inherit stdin.
pub fn docker_command() -> Command {
    let mut command = Command::new(docker_program());
    command.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Arguments for `docker compose` on the DBDock project. The manual profile
/// is always active so InterBase can be targeted and listed.
pub fn compose_args(file: &Path, args: &[&str]) -> Vec<String> {
    let mut all = vec![
        "compose".to_owned(),
        "-p".to_owned(),
        PROJECT.to_owned(),
        "-f".to_owned(),
        file.to_string_lossy().into_owned(),
        "--profile".to_owned(),
        MANUAL_PROFILE.to_owned(),
        "--ansi".to_owned(),
        "never".to_owned(),
    ];
    all.extend(args.iter().map(|a| (*a).to_owned()));
    all
}

/// Printable command line (for logs and the "copy command" buttons).
pub fn display_command(args: &[String]) -> String {
    std::iter::once("docker".to_owned())
        .chain(args.iter().map(|a| {
            if a.is_empty() || a.contains([' ', '"', '\'']) {
                format!("\"{}\"", a.replace('"', "\\\""))
            } else {
                a.clone()
            }
        }))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Runs `docker <args>` and waits at most `timeout`.
pub fn run(args: &[String], timeout: Duration) -> std::io::Result<CommandOutput> {
    let mut child = docker_command()
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Drain both pipes on threads so a chatty command cannot block on a full
    // pipe while we wait for it.
    let stdout = child.stdout.take().map(read_to_string_thread);
    let stderr = child.stderr.take().map(read_to_string_thread);

    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if Instant::now() >= deadline {
            // Best effort: the process may have exited in the meantime.
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(25));
    };

    let join = |handle: Option<std::thread::JoinHandle<String>>| {
        handle.and_then(|h| h.join().ok()).unwrap_or_default()
    };
    let stdout = join(stdout);
    let mut stderr = join(stderr);
    if status.is_none() {
        stderr.push_str(&format!("\ntimed out after {} s", timeout.as_secs()));
    }
    Ok(CommandOutput {
        success: status.is_some_and(|s| s.success()),
        stdout,
        stderr,
    })
}

/// Runs `docker <args>`, calling `on_line` for every line of stdout and
/// stderr as it arrives. Returns whether the command succeeded.
pub fn run_streaming(
    args: &[String],
    on_line: impl Fn(String) + Send + Sync + 'static,
) -> std::io::Result<bool> {
    let mut child = docker_command()
        .args(args)
        .env("COMPOSE_PROGRESS", "plain")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let on_line = std::sync::Arc::new(on_line);
    let readers: Vec<_> = [
        child
            .stdout
            .take()
            .map(|r| Box::new(r) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|r| Box::new(r) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    .map(|reader| {
        let on_line = std::sync::Arc::clone(&on_line);
        std::thread::spawn(move || {
            for line in BufReader::new(reader).lines().map_while(Result::ok) {
                // Progress output may use carriage returns to redraw a line.
                if let Some(last) = line.rsplit('\r').find(|part| !part.trim().is_empty()) {
                    on_line(last.trim_end().to_owned());
                }
            }
        })
    })
    .collect();

    let status = child.wait()?;
    for reader in readers {
        let _ = reader.join();
    }
    Ok(status.success())
}

fn read_to_string_thread(
    mut reader: impl Read + Send + 'static,
) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

/// Checks that the Docker CLI, the daemon and the Compose plugin work.
pub fn check() -> DockerCheck {
    const TIMEOUT: Duration = Duration::from_secs(20);
    let args = |a: &[&str]| a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();

    let server = match run(
        &args(&["version", "--format", "{{.Server.Version}}"]),
        TIMEOUT,
    ) {
        Err(_) => return DockerCheck::NotInstalled,
        Ok(out) if !out.success || out.stdout.trim().is_empty() => {
            return DockerCheck::DaemonDown(out.error_text())
        }
        Ok(out) => out.stdout.trim().to_owned(),
    };
    match run(&args(&["compose", "version", "--short"]), TIMEOUT) {
        Ok(out) if out.success => DockerCheck::Ready {
            server_version: server,
            compose_version: out.stdout.trim().to_owned(),
        },
        _ => DockerCheck::ComposeMissing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_args_select_project_file_and_profile() {
        let args = compose_args(
            Path::new("/x/docker-compose.yml"),
            &["up", "-d", "postgres"],
        );
        assert_eq!(
            display_command(&args),
            "docker compose -p dbdock -f /x/docker-compose.yml --profile interbase \
             --ansi never up -d postgres"
        );
    }

    #[test]
    fn display_command_quotes_spaces() {
        let args = vec!["compose".to_owned(), "C:\\My Files\\a.yml".to_owned()];
        assert_eq!(
            display_command(&args),
            "docker compose \"C:\\My Files\\a.yml\""
        );
    }

    #[test]
    fn error_text_prefers_stderr() {
        let out = CommandOutput {
            success: false,
            stdout: "out".into(),
            stderr: " err \n".into(),
        };
        assert_eq!(out.error_text(), "err");
        let out = CommandOutput {
            stderr: String::new(),
            ..out
        };
        assert_eq!(out.error_text(), "out");
    }
}
