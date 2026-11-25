# Security Policy

## Supported versions

Only the latest release receives fixes.

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |
| < 0.1   | No        |

## Scope

DBDock manages **local test databases**. Some behaviour is intentional and
not considered a vulnerability:

- Passwords are stored in plain text in `config.toml` and in the generated
  `docker-compose.yml`. Use test credentials only.
- Default credentials are well known (`tester` / `tester123`, ...).

To limit exposure, every port is published on `127.0.0.1` only, so the
databases are not reachable from other machines.

Problems that are in scope include, for example: DBDock exposing a database
to the network, running commands that were not requested, writing outside
its data directory, or anything that lets a third party influence what
DBDock executes.

## Reporting a vulnerability

Please **do not open a public issue**. Report it privately through
[GitHub private vulnerability reporting](https://github.com/Pascold/DBDock/security/advisories/new).

Include the DBDock version, your operating system, steps to reproduce and the
impact you expect. You will get an answer within a week; once a fix is
available it is released and credited in the changelog unless you prefer to
stay anonymous.
