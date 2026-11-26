# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- InterBase without a registered license no longer looks like a running
  server that refuses every connection: the container stops at once and the
  card shows "License required" with **Set up license…** again.
- InterBase has a healthcheck and shows "Healthy" once the server accepts
  connections, instead of "Running".
- The license wizard says to choose [2] Direct register, then [4] Save and
  [5] Quit; without Save the license was silently lost.

## [0.1.2] - 2026-11-26

**Hotfix** for 0.1.1 — please update.

### Fixed

- Links did nothing when clicked: the footer (GitHub, Releases, Report a
  bug, Discussions), the "Outdated" badge, the InterBase Developer Edition
  link and the Docker download links now open in the browser.
- The Copy buttons in the logs and command output windows now copy to the
  clipboard.

## [0.1.1] - 2026-11-26

### Added

- Footer with the version and links to the repository, releases, bug
  reports and discussions.
- Update notice: a red "Outdated" badge links to the newest release when
  one is available. DBDock asks the public GitHub API once on start; the
  check can be turned off in the footer (`check_updates` in `config.toml`).
- The window size and position are remembered between starts
  (`window.ron` in the data folder).

### Changed

- Updated the GitHub Actions used by CI and the release workflow.

## [0.1.0] - 2026-11-26

### Added

- One-click start, stop, restart and reset of test database servers in Docker:
  PostgreSQL, MariaDB, MySQL, MS SQL Server, Firebird, InterBase and SQLite.
- Generated `docker-compose.yml` (project `dbdock`) with healthchecks, named
  volumes and ports published on `127.0.0.1` only; export for use without the
  app.
- Optional sample data (`klienci`, `zamowienia`, `v_podsumowanie`) with Polish
  characters for encoding tests.
- Per-database settings with live validation: port conflicts, ports taken by
  other programs, the SQL Server password policy and malformed values.
- Connection window with ready-to-copy connection strings (URL, JDBC, ADO.NET,
  Firebird, sqlcmd) and a logs window with the last 500 lines.
- InterBase first-run wizard for the free Developer Edition license.
- English and Polish user interface, light and dark theme.
- Clear guidance when Docker or the Compose plugin is missing.
- Release builds for Windows x86_64, Linux x86_64 and macOS (x86_64, aarch64)
  with SHA-256 checksums.

[Unreleased]: https://github.com/Pascold/DBDock/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/Pascold/DBDock/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Pascold/DBDock/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Pascold/DBDock/releases/tag/v0.1.0
