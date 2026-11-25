//! Opening a terminal window that runs a command (used for the interactive
//! InterBase license registration).

use std::process::Command;

/// Opens a new terminal window running `command` (a shell command line).
///
/// Windows and macOS have a reliable way to do this. On Linux common
/// terminal emulators are tried in turn; if none is found an error is
/// returned and the user copies the command instead.
pub fn open_with_command(command: &str) -> Result<(), String> {
    spawn(command).map_err(|e| e.to_string())
}

#[cfg(windows)]
fn spawn(command: &str) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt as _;
    // `start` opens a new console; `cmd /K` keeps it open afterwards. The
    // command line is passed raw because cmd.exe does not follow the quoting
    // rules Rust uses for regular arguments.
    Command::new("cmd")
        .raw_arg(format!("/C start \"DBDock\" cmd /K {command}"))
        .spawn()
        .map(drop)
}

#[cfg(target_os = "macos")]
fn spawn(command: &str) -> std::io::Result<()> {
    Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "tell application \"Terminal\" to do script \"{}\"",
            applescript_escape(command)
        ))
        .arg("-e")
        .arg("tell application \"Terminal\" to activate")
        .spawn()
        .map(drop)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn(command: &str) -> std::io::Result<()> {
    // Keep the window open after the command ends so errors stay readable.
    let script = format!("{command}; echo; echo 'Press Enter to close'; read _");
    let candidates: [(&str, &[&str]); 6] = [
        ("x-terminal-emulator", &["-e", "sh", "-c"]),
        ("gnome-terminal", &["--", "sh", "-c"]),
        ("konsole", &["-e", "sh", "-c"]),
        ("xfce4-terminal", &["-x", "sh", "-c"]),
        ("kitty", &["sh", "-c"]),
        ("xterm", &["-e", "sh", "-c"]),
    ];
    for (program, args) in candidates {
        match Command::new(program).args(args).arg(&script).spawn() {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no supported terminal emulator found",
    ))
}

/// Escapes a string for use inside an AppleScript string literal.
#[cfg(any(target_os = "macos", test))]
fn applescript_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applescript_strings_are_escaped() {
        assert_eq!(
            applescript_escape(r#"docker -f "C:\x" run"#),
            r#"docker -f \"C:\\x\" run"#
        );
    }
}
