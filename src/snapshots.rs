//! Snapshots of a database's data: "Save state" and "Restore state".
//!
//! A snapshot is a folder `snapshots/<engine>/<name>/` in the data directory
//! with `snapshot.toml` and one `<volume>.tar.gz` per data volume (for
//! SQLite `sqlite.tar.gz` with the database file). The archives are made and
//! unpacked by a throwaway Alpine container, so the same code works for every
//! engine and on every system. Files are copied, not dumped as SQL, so a
//! snapshot only fits the version that wrote it.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::compose::PROJECT;
use crate::config::EngineSettings;
use crate::docker::compose_args;
use crate::engines::EngineId;
use crate::paths::AppPaths;

/// Image used to pack and unpack the archives.
pub const HELPER_IMAGE: &str = "alpine:3";
const META: &str = "snapshot.toml";
const SQLITE_ARCHIVE: &str = "sqlite.tar.gz";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Meta {
    engine: String,
    /// Image tag of the engine when the snapshot was taken.
    version: String,
    /// Seconds since the Unix epoch.
    created: u64,
}

/// One saved snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub name: String,
    pub dir: PathBuf,
    pub version: String,
    pub created: u64,
    /// All archives exist; false while saving or after a failed save.
    pub complete: bool,
}

impl Snapshot {
    /// True if the snapshot can be restored with `settings`: data files only
    /// fit the version that wrote them (any version reads a SQLite file).
    pub fn fits(&self, id: EngineId, settings: &EngineSettings) -> bool {
        id == EngineId::Sqlite || self.version.trim() == settings.version.trim()
    }
}

/// Archive files a complete snapshot of `id` contains.
pub fn archives(id: EngineId) -> Vec<String> {
    if id == EngineId::Sqlite {
        vec![SQLITE_ARCHIVE.to_owned()]
    } else {
        id.engine()
            .volumes()
            .iter()
            .map(|v| format!("{v}.tar.gz"))
            .collect()
    }
}

/// True if DBDock can snapshot `id` (it has data to copy).
pub fn supported(id: EngineId) -> bool {
    !archives(id).is_empty()
}

/// Snapshots of `id`, newest first.
pub fn list(paths: &AppPaths, id: EngineId) -> Vec<Snapshot> {
    let Ok(entries) = std::fs::read_dir(paths.snapshots_dir(id)) else {
        return Vec::new();
    };
    let mut snapshots: Vec<Snapshot> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| read(id, &entry.path()))
        .collect();
    snapshots.sort_by(|a, b| b.created.cmp(&a.created).then(b.name.cmp(&a.name)));
    snapshots
}

fn read(id: EngineId, dir: &Path) -> Option<Snapshot> {
    let text = std::fs::read_to_string(dir.join(META)).ok()?;
    let meta: Meta = toml::from_str(&text).ok()?;
    if meta.engine != id.key() {
        return None;
    }
    let complete = archives(id).iter().all(|a| dir.join(a).is_file());
    Some(Snapshot {
        name: dir.file_name()?.to_string_lossy().into_owned(),
        dir: dir.to_owned(),
        version: meta.version,
        created: meta.created,
        complete,
    })
}

/// Creates the folder and metadata of a new snapshot named after the time.
pub fn prepare(
    paths: &AppPaths,
    id: EngineId,
    settings: &EngineSettings,
) -> std::io::Result<Snapshot> {
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let parent = paths.snapshots_dir(id);
    std::fs::create_dir_all(&parent)?;
    let base = file_stamp(created);
    let mut name = base.clone();
    let mut n = 2;
    while parent.join(&name).exists() {
        name = format!("{base}-{n}");
        n += 1;
    }
    let dir = parent.join(&name);
    std::fs::create_dir(&dir)?;
    let meta = Meta {
        engine: id.key().to_owned(),
        version: settings.version.clone(),
        created,
    };
    let text = toml::to_string(&meta).map_err(std::io::Error::other)?;
    std::fs::write(dir.join(META), text)?;
    Ok(Snapshot {
        name,
        dir,
        version: meta.version,
        created,
        complete: false,
    })
}

/// Deletes a snapshot folder.
pub fn delete(snapshot: &Snapshot) -> std::io::Result<()> {
    std::fs::remove_dir_all(&snapshot.dir)
}

/// Docker invocations that save the data of `id` into `snapshot`. A running
/// server is stopped for a consistent copy and started again afterwards.
pub fn save_commands(
    paths: &AppPaths,
    id: EngineId,
    settings: &EngineSettings,
    snapshot: &Snapshot,
    was_up: bool,
    owner: Option<(u32, u32)>,
) -> Vec<Vec<String>> {
    // SQLite has no server to stop.
    let was_up = was_up && id != EngineId::Sqlite;
    let compose_file = paths.compose_file();
    let services = id.engine().start_services();
    let chown = owner
        .map(|(uid, gid)| format!(" && chown -R {uid}:{gid} /backup"))
        .unwrap_or_default();
    let backup = bind(&snapshot.dir, "/backup", false);
    let mut commands = Vec::new();
    if was_up {
        commands.push(compose(&compose_file, "stop", services));
    }
    if id == EngineId::Sqlite {
        // Written under a temporary name, so a broken copy never looks
        // complete.
        commands.push(helper(
            &[bind(&paths.sqlite_dir(), "/data", true), backup],
            &[("DB", &settings.database)],
            &format!(
                "tar czf /backup/{SQLITE_ARCHIVE}.part -C /data \"$DB\" \
                 && mv /backup/{SQLITE_ARCHIVE}.part /backup/{SQLITE_ARCHIVE}{chown}"
            ),
        ));
    } else {
        for volume in id.engine().volumes() {
            commands.push(helper(
                &[volume_mount(volume, true), backup.clone()],
                &[],
                &format!(
                    "tar czf /backup/{volume}.tar.gz.part -C /volume . \
                     && mv /backup/{volume}.tar.gz.part /backup/{volume}.tar.gz{chown}"
                ),
            ));
        }
    }
    if was_up {
        commands.push(compose(&compose_file, "up", services));
    }
    commands
}

/// Docker invocations that replace the data of `id` with `snapshot`. The
/// engine runs again afterwards if it ran before.
pub fn restore_commands(
    paths: &AppPaths,
    id: EngineId,
    settings: &EngineSettings,
    snapshot: &Snapshot,
    was_up: bool,
    owner: Option<(u32, u32)>,
) -> Vec<Vec<String>> {
    let compose_file = paths.compose_file();
    let services = id.engine().start_services();
    let backup = bind(&snapshot.dir, "/backup", true);
    let mut commands = Vec::new();
    if id == EngineId::Sqlite {
        let chown = owner
            .map(|(uid, gid)| format!(" && chown {uid}:{gid} \"/data/$DB\""))
            .unwrap_or_default();
        // The archive may hold the file under an older name.
        commands.push(helper(
            &[bind(&paths.sqlite_dir(), "/data", false), backup],
            &[("DB", &settings.database)],
            &format!(
                "mkdir /r && tar xzf /backup/{SQLITE_ARCHIVE} -C /r \
                 && rm -f \"/data/$DB\" && mv /r/* \"/data/$DB\"{chown}"
            ),
        ));
        return commands;
    }
    commands.push(compose(&compose_file, "stop", services));
    // Creates the containers and labelled volumes after a reset.
    commands.push(compose(&compose_file, "create", services));
    for volume in id.engine().volumes() {
        commands.push(helper(
            &[volume_mount(volume, false), backup.clone()],
            &[],
            &format!(
                "find /volume -mindepth 1 -delete \
                 && tar xzf /backup/{volume}.tar.gz -C /volume"
            ),
        ));
    }
    if was_up {
        commands.push(compose(&compose_file, "up", services));
    }
    commands
}

fn compose(compose_file: &Path, verb: &str, services: &[&str]) -> Vec<String> {
    let mut args = vec![verb];
    if verb == "up" {
        args.push("-d");
    }
    args.extend(services);
    compose_args(compose_file, &args)
}

/// `docker run --rm` of the helper image with mounts, environment and a
/// shell command.
fn helper(mounts: &[String], env: &[(&str, &str)], script: &str) -> Vec<String> {
    let mut args = vec!["run".to_owned(), "--rm".to_owned()];
    for mount in mounts {
        args.push("--mount".to_owned());
        args.push(mount.clone());
    }
    for (key, value) in env {
        args.push("-e".to_owned());
        args.push(format!("{key}={value}"));
    }
    args.extend([
        HELPER_IMAGE.to_owned(),
        "sh".to_owned(),
        "-c".to_owned(),
        format!("set -e; {script}"),
    ]);
    args
}

fn bind(source: &Path, target: &str, read_only: bool) -> String {
    let mut mount = format!(
        "type=bind,source={},target={target}",
        source.to_string_lossy()
    );
    if read_only {
        mount.push_str(",readonly");
    }
    mount
}

fn volume_mount(volume: &str, read_only: bool) -> String {
    let mut mount = format!("type=volume,source={PROJECT}_{volume},target=/volume");
    if read_only {
        mount.push_str(",readonly");
    }
    mount
}

/// `2026-11-26_23-40-05` (UTC), used as the folder name.
fn file_stamp(secs: u64) -> String {
    let (y, mo, d, h, mi, s) = civil(secs);
    format!("{y:04}-{mo:02}-{d:02}_{h:02}-{mi:02}-{s:02}")
}

/// `2026-11-26 23:40 UTC`, shown in the list.
pub fn display_time(secs: u64) -> String {
    let (y, mo, d, h, mi, _) = civil(secs);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02} UTC")
}

/// Calendar date and time (UTC) of a Unix timestamp.
fn civil(secs: u64) -> (i64, u32, u32, u64, u64, u64) {
    let days = (secs / 86_400) as i64;
    let rest = secs % 86_400;
    // Howard Hinnant's days-to-civil algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d, rest / 3_600, rest % 3_600 / 60, rest % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docker::display_command;

    #[test]
    fn timestamps() {
        assert_eq!(file_stamp(0), "1970-01-01_00-00-00");
        // 2026-11-26 23:40:05 UTC
        assert_eq!(file_stamp(1_795_736_405), "2026-11-26_23-40-05");
        assert_eq!(display_time(1_795_736_405), "2026-11-26 23:40 UTC");
        // Leap day.
        assert_eq!(file_stamp(1_709_164_800), "2024-02-29_00-00-00");
    }

    #[test]
    fn prepare_list_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path());
        let id = EngineId::Postgres;
        let settings = id.engine().default_settings();
        let first = prepare(&paths, id, &settings).unwrap();
        let second = prepare(&paths, id, &settings).unwrap();
        assert_ne!(first.name, second.name);
        assert!(!first.complete);

        std::fs::write(first.dir.join("postgres-data.tar.gz"), "x").unwrap();
        let listed = list(&paths, id);
        assert_eq!(listed.len(), 2);
        let first_listed = listed.iter().find(|s| s.name == first.name).unwrap();
        assert!(first_listed.complete);
        assert!(first_listed.fits(id, &settings));
        let mut other = settings.clone();
        other.version = "16".into();
        assert!(!first_listed.fits(id, &other));

        // Other engines and stray folders are ignored.
        assert!(list(&paths, EngineId::MySql).is_empty());
        std::fs::create_dir(paths.snapshots_dir(id).join("junk")).unwrap();
        assert_eq!(list(&paths, id).len(), 2);

        delete(first_listed).unwrap();
        assert_eq!(list(&paths, id).len(), 1);
    }

    #[test]
    fn save_stops_copies_and_starts_again() {
        let paths = AppPaths::new("/home/tester/.config/dbdock");
        let id = EngineId::MsSql;
        let settings = id.engine().default_settings();
        let snapshot = Snapshot {
            name: "s".into(),
            dir: paths.snapshots_dir(id).join("s"),
            version: settings.version.clone(),
            created: 0,
            complete: false,
        };
        let commands: Vec<String> =
            save_commands(&paths, id, &settings, &snapshot, true, Some((1000, 1000)))
                .iter()
                .map(|c| display_command(c))
                .collect();
        assert_eq!(commands.len(), 3);
        assert!(commands[0].ends_with(" stop mssql mssql-init"));
        assert!(commands[1].starts_with(
            "docker run --rm --mount type=volume,source=dbdock_mssql-data,target=/volume,readonly"
        ));
        assert!(commands[1].contains("s,target=/backup"));
        assert!(commands[1].contains("chown -R 1000:1000 /backup"));
        assert!(commands[2].ends_with(" up -d mssql mssql-init"));

        let stopped = save_commands(&paths, id, &settings, &snapshot, false, None);
        assert_eq!(stopped.len(), 1);
    }

    #[test]
    fn restore_recreates_volumes_and_keeps_a_stopped_engine_stopped() {
        let paths = AppPaths::new("/x");
        let id = EngineId::Postgres;
        let settings = id.engine().default_settings();
        let snapshot = Snapshot {
            name: "s".into(),
            dir: PathBuf::from("/x/snapshots/postgres/s"),
            version: settings.version.clone(),
            created: 0,
            complete: true,
        };
        let commands: Vec<String> = restore_commands(&paths, id, &settings, &snapshot, false, None)
            .iter()
            .map(|c| display_command(c))
            .collect();
        assert_eq!(commands.len(), 3);
        assert!(commands[0].ends_with(" stop postgres"));
        assert!(commands[1].ends_with(" create postgres"));
        assert!(commands[2].contains("find /volume -mindepth 1 -delete"));
        assert!(commands[2].contains("tar xzf /backup/postgres-data.tar.gz"));
    }

    #[test]
    fn sqlite_copies_the_file() {
        let paths = AppPaths::new("/x");
        let id = EngineId::Sqlite;
        let settings = id.engine().default_settings();
        assert_eq!(archives(id), ["sqlite.tar.gz"]);
        let snapshot = Snapshot {
            name: "s".into(),
            dir: PathBuf::from("/x/snapshots/sqlite/s"),
            version: "old".into(),
            created: 0,
            complete: true,
        };
        // Any version reads a SQLite file.
        assert!(snapshot.fits(id, &settings));
        let save = save_commands(&paths, id, &settings, &snapshot, true, None);
        assert_eq!(save.len(), 1, "SQLite has no server to stop");
        let restore = restore_commands(&paths, id, &settings, &snapshot, true, None);
        assert_eq!(restore.len(), 1);
        assert!(restore[0].contains(&"DB=test.db".to_owned()));
    }

    #[test]
    fn every_engine_has_something_to_save() {
        for id in EngineId::ALL {
            assert!(supported(id), "{id:?}");
        }
    }
}
