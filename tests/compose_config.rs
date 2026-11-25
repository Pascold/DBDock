//! Validates generated compose files with the real Docker CLI.
//!
//! Ignored by default because it needs Docker with the Compose plugin:
//! `cargo test -- --ignored`

use std::process::Command;

use dbdock::compose::{self, ComposeContext, MANUAL_PROFILE};
use dbdock::config::AppConfig;
use dbdock::engines::EngineId;
use dbdock::paths::AppPaths;

fn docker_compose_available() -> bool {
    Command::new("docker")
        .args(["compose", "version"])
        .output()
        .is_ok_and(|o| o.status.success())
}

fn assert_valid(config: &AppConfig) {
    let dir = tempfile::tempdir().expect("temp dir");
    let paths = AppPaths::new(dir.path());
    let file = compose::write_files(config, &ComposeContext::for_host(&paths)).expect("write");
    let output = Command::new("docker")
        .args(["compose", "-p", compose::PROJECT, "-f"])
        .arg(&file)
        .args(["--profile", MANUAL_PROFILE, "config", "-q"])
        .output()
        .expect("run docker compose");
    assert!(
        output.status.success(),
        "docker compose config failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        std::fs::read_to_string(&file).unwrap_or_default()
    );
}

#[test]
#[ignore = "requires Docker"]
fn generated_compose_files_are_valid() {
    if !docker_compose_available() {
        eprintln!("docker compose not available, skipping");
        return;
    }

    let mut config = AppConfig::default();
    assert_valid(&config);

    for id in EngineId::ALL {
        config.engine_mut(id).enabled = true;
    }
    let pg = config.engine_mut(EngineId::Postgres);
    pg.version = "18".into();
    pg.password = "pa$word:#x".into();
    config.engine_mut(EngineId::MsSql).seed = false;
    config.engine_mut(EngineId::Sqlite).seed = false;
    assert_valid(&config);
}
