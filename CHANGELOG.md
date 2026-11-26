# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.4] - 2026-11-26

Cards explain why a database does not start, your own SQL scripts run when
a database is created, and the update check repeats once a day.

### Added

- When a database does not start, its card shows why: the output of the
  failing healthcheck (once it has failed for a while or is unhealthy) or
  the error line of the log of a container that stopped with an error. If
  the data belongs to another version (PostgreSQL major, Firebird on-disk
  structure, MySQL or MariaDB downgrade), the card says so and offers
  **Reset data**.
- Your own scripts: every database except InterBase has a **Scripts**
  button that opens its folder (`scripts/<engine>` in the data directory).
  `.sql` files there run in name order, after the sample data, when the
  data is created. SQLite builds the file under a temporary name, so a
  failing script leaves no half-made database behind.

### Changed

- The update check also runs once a day while DBDock stays open, not only
  on start. A failed later check keeps a notice already shown.
- PostgreSQL, MySQL, MariaDB and Firebird mount a generated `init/<engine>`
  folder (sample data and copies of your scripts) at
  `/docker-entrypoint-initdb.d` instead of the single seed file.

## [0.1.3] - 2026-11-26

English sample data, a Firebird alias for the database name, EOL marks for
old versions, and clearer InterBase license handling.

### Added

- Firebird registers the database name as an alias: clients can connect to
  `testdb` (`localhost/3050:testdb`) instead of the full file path
  `/var/lib/firebird/data/testdb.fdb`, which still works.
- Version suggestions that no longer get updates from the vendor are marked
  **EOL** (PostgreSQL 13 and 14, MySQL 8.0, MariaDB 10.6); hovering the mark
  explains what end of life means. They stay available for testing older
  applications.
- FAQ entry for a database that does not start after a version change.

### Changed

- The sample data is in English: tables `customers` and `orders` and the
  view `v_customer_summary` replace `klienci`, `zamowienia` and
  `v_podsumowanie`. Names and cities still contain accented letters (Zoë,
  Zürich, José, Łukasz, Chloé) to catch encoding bugs. Existing databases
  keep their tables; use **Reset data** to get the new ones.
- InterBase without a registered license no longer looks like a running
  server that refuses every connection: the container stops at once and the
  card shows "License required" with **Set up license…** again.
- InterBase has a healthcheck and shows "Healthy" once the server accepts
  connections, instead of "Running".
- The license wizard says to choose [2] Direct register, then [4] Save and
  [5] Quit; without Save the license was silently lost.
- Changing the version in **Settings** warns that existing data may not
  start with another version (always so for another PostgreSQL major or a
  downgrade) and offers **Apply and reset data**.
- InterBase is marked as experimental in the README and the license wizard:
  running it with a license has not been tested yet, and the license has to
  be for InterBase 2020, the only version the Docker image exists for.

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

[Unreleased]: https://github.com/Pascold/DBDock/compare/v0.1.4...HEAD
[0.1.4]: https://github.com/Pascold/DBDock/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/Pascold/DBDock/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/Pascold/DBDock/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Pascold/DBDock/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Pascold/DBDock/releases/tag/v0.1.0
