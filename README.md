<div align="center">

<img src="assets/icon/dbdock-512.png" alt="DBDock logo" width="128">

# DBDock

**One click to start and stop test database servers in Docker.**

English | [Polski](README.pl.md)

[![CI](https://github.com/Pascold/DBDock/actions/workflows/ci.yml/badge.svg)](https://github.com/Pascold/DBDock/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/Pascold/DBDock?sort=semver)](https://github.com/Pascold/DBDock/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Pascold/DBDock/total)](https://github.com/Pascold/DBDock/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![Rust 1.95+](https://img.shields.io/badge/rust-1.95%2B-orange?logo=rust)](https://www.rust-lang.org)
[![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](#installation)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)](CONTRIBUTING.md)

<img src="docs/screenshot.png" alt="DBDock main window" width="820">

</div>

DBDock is a small, single-file desktop app for developers who test their
software — SQL clients, ORMs, migrations, drivers — against many database
engines. It generates a `docker-compose.yml` and drives `docker compose` for
you, so you don't need to know Docker. You only need to have it installed.

## Features

- **Seven engines**: PostgreSQL, MariaDB, MySQL, MS SQL Server, Firebird,
  InterBase and SQLite, each in its own container.
- **One click** to start, stop, restart or reset a database, or to start all
  enabled ones at once. Live status (Healthy, Starting, Stopped, ...) from
  the containers' healthchecks.
- **Ready-to-copy connection details**: host, port, user, password and
  connection strings (URL, JDBC, ADO.NET, Firebird, sqlcmd).
- **Sample data** with Polish characters (ąćęłńóśźż) to catch encoding bugs:
  tables `klienci` and `zamowienia` and the view `v_podsumowanie`.
- **Settings per database**: image version, port, database name, user and
  passwords, with live validation (port conflicts, busy ports, SQL Server
  password policy).
- **Logs** of each database, the full output of every Docker command and
  clear error messages.
- **Safe defaults**: ports are published on `127.0.0.1` only and everything
  runs in a separate compose project called `dbdock`.
- **Export** the generated `docker-compose.yml` to use it without DBDock.
- **Update notice**: a red "Outdated" badge in the status bar links to the
  newest release. DBDock makes one anonymous request to the public GitHub
  API on start for this (switch it off with "Check for updates" in the status
  bar); nothing else leaves your computer.
- English and Polish UI, light and dark theme, no installer.

## Supported databases

| Engine | Default image | Host port → container | Database | User / password | Admin |
|---|---|---|---|---|---|
| PostgreSQL | `postgres:17` | 5432 → 5432 | `testdb` | `tester` / `tester123` | — (the user is a superuser) |
| MariaDB | `mariadb:11.4` | 3306 → 3306 | `testdb` | `tester` / `tester123` | `root` / `root123` |
| MySQL | `mysql:8.4` | 3307 → 3306 | `testdb` | `tester` / `tester123` | `root` / `root123` |
| MS SQL Server | `mcr.microsoft.com/mssql/server:2022-latest` | 1433 → 1433 | `testdb` | `tester` / `tester123` | `sa` / `Test_Passw0rd!` |
| Firebird | `firebirdsql/firebird:5` | 3050 → 3050 | `/var/lib/firebird/data/testdb.fdb` | `tester` / `tester123` | `SYSDBA` / `masterkey` |
| InterBase | `radstudio/interbase:2020.4` | 3051 → 3050 | created by you (see below) | — | `SYSDBA` / `masterkey` |
| SQLite | `alpine:3` + `sqlite` | — | `test.db` in the data folder | — | — |

Every value can be changed in the database's **Settings**. PostgreSQL 18 and
newer store their data in a different folder; DBDock picks the right volume
path from the version automatically.

## Requirements

- [Docker Desktop](https://www.docker.com/products/docker-desktop/) on Windows
  and macOS, or [Docker Engine](https://docs.docker.com/engine/install/) with
  the Compose plugin (`docker compose`) on Linux.
- Disk space for the images (SQL Server alone is about 1.5 GB).

DBDock checks for Docker on start and explains what is missing.

## Installation

Download the file for your system from the
[latest release](https://github.com/Pascold/DBDock/releases/latest):

| System | File |
|---|---|
| Windows x86_64 | `dbdock-vX.Y.Z-windows-x86_64.exe` |
| Linux x86_64 | `dbdock-vX.Y.Z-linux-x86_64.tar.gz` |
| macOS Apple Silicon | `dbdock-vX.Y.Z-macos-aarch64.tar.gz` |
| macOS Intel | `dbdock-vX.Y.Z-macos-x86_64.tar.gz` |

There is no installer: put the executable anywhere and run it. Checksums are
in `SHA256SUMS.txt`.

- **Windows**: the binary is not code-signed yet, so SmartScreen may warn on
  the first start (*More info → Run anyway*).
- **macOS**: the binary is not notarized. After unpacking, remove the
  quarantine flag once: `xattr -d com.apple.quarantine ./dbdock`.
- **Linux**: `tar xzf dbdock-*.tar.gz && ./dbdock-*/dbdock`.

## Usage

1. Start Docker, then DBDock. The green **Docker** light shows it is ready.
2. Click **Start** on a card (or **Start all enabled**). The first start
   downloads the image; click the spinner to watch the progress.
3. When the card says **Healthy**, open **Connection** or click **Copy** to
   copy the connection string.
4. **Stop** keeps the data, **Reset data** deletes it (after a confirmation),
   **Logs** shows the database output.
5. Use the checkbox on a card to enable or disable a database for
   **Start all enabled** and for the exported compose file.

Changing settings of an existing database: **Apply and restart** recreates
the container. Database name, user and passwords are used only when the data
is created, so after changing them choose **Apply and reset data**.

### Where are my files?

| System | Data folder |
|---|---|
| Windows | `%APPDATA%\DBDock\config` |
| macOS | `~/Library/Application Support/DBDock` |
| Linux | `~/.config/dbdock` |

It contains `config.toml`, `window.ron` (window size and position), the
generated `docker-compose.yml`, the seed
scripts and the `sqlite` folder. **Open data folder** in the top bar opens it.
Set the environment variable `DBDOCK_HOME` to use another folder.

> **Passwords are stored in plain text** in `config.toml` and
> `docker-compose.yml`. DBDock is meant for local test databases; never use
> production credentials.

### Using the compose file without DBDock

**Export compose** saves `dbdock-docker-compose.yml` to your Downloads
folder. It refers to the seed scripts in the data folder, so keep DBDock's
data folder in place, then run:

```sh
docker compose -f dbdock-docker-compose.yml up -d
```

InterBase is in a separate profile and only starts with
`--profile interbase`.

## Connecting from your app

With the default settings:

| Engine | Connection string |
|---|---|
| PostgreSQL | `postgres://tester:tester123@localhost:5432/testdb` |
| PostgreSQL (JDBC) | `jdbc:postgresql://localhost:5432/testdb` |
| MariaDB | `mariadb://tester:tester123@localhost:3306/testdb` |
| MySQL | `mysql://tester:tester123@localhost:3307/testdb` |
| MySQL (ADO.NET) | `Server=localhost;Port=3307;Database=testdb;Uid=tester;Pwd=tester123` |
| MS SQL Server (ADO.NET) | `Server=localhost,1433;Database=testdb;User Id=tester;Password=tester123;TrustServerCertificate=True` |
| MS SQL Server (JDBC) | `jdbc:sqlserver://localhost:1433;databaseName=testdb;encrypt=true;trustServerCertificate=true` |
| Firebird | `localhost/3050:/var/lib/firebird/data/testdb.fdb` |
| Firebird (JDBC) | `jdbc:firebird://localhost:3050//var/lib/firebird/data/testdb.fdb` |
| InterBase | `localhost/3051:/opt/interbase/testdb.ib` |
| SQLite | the file `sqlite/test.db` in the data folder |

The **Connection** window shows these strings for your current settings,
with special characters in passwords already URL-encoded where needed.

## InterBase licensing

InterBase is commercial software from Embarcadero. The official image
`radstudio/interbase` requires a license; the free **Developer Edition** is
enough for testing. InterBase is therefore disabled by default, and its card
offers **Set up license…**, a short wizard that:

1. links to the [Developer Edition registration](https://www.embarcadero.com/products/interbase/start-for-free)
   (the serial number arrives by e-mail);
2. shows the command that runs the license manager interactively (copy it, or
   let DBDock open a terminal with it). The license is stored in the volume
   `dbdock_interbase-license` and survives **Reset data**;
3. starts InterBase normally once you confirm the registration;
4. shows how to create a database with `isql`, for example
   `CREATE DATABASE '/opt/interbase/testdb.ib';`.

The image exists for x86-64 only.

## FAQ / Troubleshooting

**"Another program listens on this port"** — something else (often a locally
installed database) uses the port. Stop it or pick another host port in
**Settings**; your connection strings follow automatically.

**SQL Server on Apple Silicon (ARM)** — Microsoft publishes x86-64 images
only. DBDock runs them with `platform: linux/amd64`; enable *Use Rosetta for
x86_64/amd64 emulation* in Docker Desktop's settings. Starts are slower, and
some SQL Server versions may not run under emulation at all. The same applies
to InterBase.

**"The certificate chain was issued by an authority that is not trusted"**
— SQL Server uses a self-signed certificate. Add
`TrustServerCertificate=True` (ADO.NET), `trustServerCertificate=true`
(JDBC) or `-C` (sqlcmd).

**SQL Server stays in "Starting"** — the server needs a strong SA password
(8+ characters, three of: upper case, lower case, digits, symbols); DBDock
validates it, but check **Logs** if you edited the compose file by hand. The
first start can take a minute.

**I changed the password but the old one still works** — the official images
create users only when the data volume is empty. Use **Apply and reset data**
(or **Reset data**) to start from scratch.

**How do I reset a database?** — **Reset data** on its card removes the
container and its volume; the next start initialises it again, including the
sample data. For SQLite the file is deleted.

**Docker is not detected** — make sure Docker Desktop is running (or the
`docker` service on Linux, and your user is in the `docker` group), then
click **Check again**. On macOS, DBDock also looks in `/usr/local/bin`,
`/opt/homebrew/bin` and `~/.docker/bin`.

**Can other computers connect?** — No, ports are bound to `127.0.0.1` on
purpose.

## Building from source

```sh
git clone https://github.com/Pascold/DBDock.git
cd DBDock
cargo build --release
```

The binary is `target/release/dbdock` (`dbdock.exe` on Windows). On Linux,
install `libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
libxkbcommon-dev` first. Rust 1.95 or newer is required.

Tests: `cargo test`; with Docker available also `cargo test -- --ignored`,
which validates generated files with `docker compose config`.

## Roadmap

- More engines: Oracle Database Free, IBM Db2, ClickHouse, MongoDB, Redis.
- Several instances or versions of the same engine side by side.
- Custom init scripts per database.
- Optional LAN access (binding to `0.0.0.0`) per database.
- Signed and notarized releases, a macOS app bundle.

Ideas and pull requests are welcome.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the
[Code of Conduct](CODE_OF_CONDUCT.md). Security issues: [SECURITY.md](SECURITY.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. DBDock only orchestrates the official database images; each
database is subject to its own license (SQL Server Developer Edition and
InterBase Developer Edition are free for development and testing only).
