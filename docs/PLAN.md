# DBDock – implementation plan

This document records the architecture, the chosen dependencies and every
deliberate deviation from the original specification.

## Crate layout

DBDock is a single binary crate with a library target, so integration tests in
`tests/` can reach the compose generator.

```
build.rs             embeds the icon and version info into the Windows .exe
src/
  main.rs            entry point, window setup, windows_subsystem
  lib.rs             module tree
  app.rs             application state, Docker events, user actions
  paths.rs           application directories (config, compose, seeds, SQLite)
  terminal.rs        opening a terminal with a command (InterBase license)
  config/
    mod.rs           AppConfig, EngineSettings, load/save, migrations
    validate.rs      field validation, MSSQL password policy, port checks
  engines/
    mod.rs           EngineId, the Engine trait, connection strings
    postgres.rs mariadb.rs mysql.rs mssql.rs firebird.rs interbase.rs sqlite.rs
  compose/
    mod.rs           compose generation, helper builders, file writer
    yaml.rs          small, ordered, always-quoting YAML emitter
  docker/
    mod.rs           docker discovery, Command builder (CREATE_NO_WINDOW), check
    status.rs        `docker compose ps --format json` parsing, engine status
    worker.rs        background threads, events over std::sync::mpsc
  i18n/mod.rs        tiny translation system backed by assets/i18n/*.toml
  ui/
    mod.rs           top bar, operation output window
    cards.rs         database cards
    settings.rs      settings window and validation messages
    windows.rs       connection, logs and reset confirmation windows
    interbase.rs     InterBase license wizard
    docker_missing.rs, toasts.rs, theme.rs
assets/
  i18n/en.toml pl.toml
  seed/<engine>.sql, mssql-init.sql
  icon/dbdock.svg (source), dbdock.png, dbdock-512.png, dbdock.ico
tests/compose_config.rs   `docker compose config -q` (ignored without Docker)
```

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| eframe / egui | 0.37 | UI (MSRV 1.95 comes from here) |
| serde | 1 | (de)serialization |
| toml | 1 | `config.toml` and translation files |
| serde_json | 1 | `docker compose ps --format json` |
| directories | 6 | platform config and Downloads directories |
| thiserror | 2 | typed configuration errors |
| arboard | 3 (no default features) | clipboard (text only) |
| open | 5 | open folders and links |
| sys-locale | 0.3 | default UI language |
| winresource (build) | 0.1 | embed the icon into the `.exe` |
| insta, tempfile (dev) | 1, 3 | compose snapshots, temporary directories |

`anyhow` from the specification is not needed: errors are either typed
(`ConfigError`) or shown to the user as text coming from Docker.

## Deviations from the specification (with reasons)

1. **No `serde_yaml`.** It is archived and unmaintained. Compose files are
   produced by a small in-house emitter (`compose/yaml.rs`) that keeps key
   order and always double-quotes scalar strings, so values such as
   `"5432:5432"`, `"on"` or passwords with `:` and `#` can never be misread.
   Every `$` in user values is escaped as `$$` so Compose does not
   interpolate it. The result is validated with `docker compose config -q`.
2. **Own i18n instead of `rust-i18n`.** Two TOML files embedded with
   `include_str!`, `t("key")` / `tf("key", args)` with English fallback, and
   tests that both languages define the same keys and placeholders and that
   every key used in the sources exists. No proc macros, faster builds.
3. **Only enabled engines are in the compose file; only InterBase has a
   profile.** A plain `docker compose up -d` on the exported file starts
   exactly what is enabled, except InterBase, which needs a registered
   license and therefore sits behind the `interbase` profile. The app always
   passes `--profile interbase` and names the services it targets. Disabling
   an engine whose container exists first removes the container (the data
   volume stays), so nothing is left outside the file.
4. **Ports bound to `127.0.0.1`.** Test databases with well-known passwords
   should not be reachable from the LAN.
5. **Password character set.** Passwords may not contain `'`, `"`, `\`,
   `` ` ``, `;`, whitespace or non-ASCII characters. They end up in SQL
   literals, shell commands and connection strings; forbidding these keeps
   every one of them correct.
6. **One settings struct for all engines.** `EngineSettings` has the union of
   fields; each engine declares which fields it uses. This keeps the TOML flat
   (`[engines.postgres]`) and the settings UI generic.
7. **Firebird database file** is `<database>.fdb` (default `testdb.fdb`),
   InterBase databases live in `/opt/interbase/<database>.ib`.
8. **MSSQL and InterBase use `platform: linux/amd64`**, so Apple Silicon runs
   them under emulation instead of failing; the UI warns about it.
9. **Reset data** removes the container and its named volume
   (`docker compose rm -s -f -v` + `docker volume rm dbdock_<engine>-data`);
   for SQLite it deletes the database file. The InterBase license volume is
   kept.
10. **Export compose** copies the file to the Downloads folder instead of
    showing a save dialog, which would need another native dependency.
11. **No emoji glyphs in the UI.** egui 0.37 ships emoji fonts only behind an
    extra feature; status lights are drawn instead, keeping the binary small.
12. **Edition 2021** as specified, with `rust-version = "1.95"` (required by
    egui 0.37).

## Docker integration

- `docker` is looked up on `PATH` and in well-known locations
  (`/usr/local/bin`, `/opt/homebrew/bin`, `~/.docker/bin`, Docker Desktop on
  Windows), because GUI apps on macOS start with a minimal `PATH`.
- All commands use `-p dbdock -f <config dir>/docker-compose.yml`.
- A poller thread runs `ps -a --format json` every 3 s (and right after each
  operation). Output in both the NDJSON and the JSON-array form is accepted.
- Long operations stream their output to the UI line by line.

## Testing

- Unit tests: validation, MSSQL password policy, port conflicts, connection
  strings, PostgreSQL volume path by version, config round-trip and migration,
  i18n key parity, `ps` JSON parsing, engine status mapping, operation
  command lines.
- `insta` snapshots of the generated compose file (default and modified
  configurations) with a fixed fake base directory.
- `tests/compose_config.rs` (`#[ignore]`): writes the file and runs
  `docker compose config -q` when Docker is available.
