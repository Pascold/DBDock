# Contributing to DBDock

Thanks for taking the time to contribute! Bug reports, ideas and pull
requests are all welcome. By participating you agree to follow the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Reporting bugs and requesting features

- Search the [existing issues](https://github.com/Pascold/DBDock/issues) first.
- Use the issue templates. For bugs, include your OS, the DBDock version,
  `docker version` / `docker compose version` and, if relevant, the output of
  the failing operation (the toast's **Show output** button) or the database
  logs.
- Security problems go through a private report instead, see
  [SECURITY.md](SECURITY.md).

## Development setup

You need a stable Rust toolchain (see `rust-version` in `Cargo.toml`) and,
to try the app for real, Docker with the Compose plugin.

On Linux, install the libraries eframe needs to build:

```sh
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
```

Then:

```sh
cargo run
```

Set `DBDOCK_HOME` to a scratch folder to keep your real configuration out of
the way while developing:

```sh
DBDOCK_HOME=/tmp/dbdock-dev cargo run
```

## Before you open a pull request

Every commit must pass the same checks as CI:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

If you changed the compose generator, also run the Docker-backed test and
review the snapshot changes:

```sh
cargo test -- --ignored          # runs `docker compose config -q`
cargo insta review               # requires `cargo install cargo-insta`
```

## Guidelines

- **Commits** follow [Conventional Commits](https://www.conventionalcommits.org/)
  in English: `type(scope): subject`, imperative mood, lower case, no final
  period, at most 72 characters. Types: `feat`, `fix`, `refactor`, `test`,
  `docs`, `ci`, `build`, `chore`, `style`, `perf`. Explain the *why* in the
  body when it is not obvious. One logical change per commit.
- **No `unwrap()`** in production code except where failure is impossible;
  show errors to the user instead.
- **The UI never blocks on Docker.** Run commands through
  `docker::worker::Docker`, which reports back over a channel.
- **Texts are translated.** Add every visible string to both
  `assets/i18n/en.toml` and `assets/i18n/pl.toml`; a test checks that the
  keys match.
- **New engines** implement the `Engine` trait in `src/engines/`, get a seed
  script in `assets/seed/` and compose snapshot coverage.
- Update `CHANGELOG.md` under *Unreleased* for user-visible changes.

## Updating the screenshot

`docs/screenshot.png` is shown at the top of both READMEs. To replace it, run
DBDock in English with the dark theme and a few databases started, capture
the window at about 1000 × 740 pixels and save it under the same name
(PNG, ideally below 200 KB).

## Releasing (maintainers)

1. Move the *Unreleased* entries in `CHANGELOG.md` to a new version section
   and bump `version` in `Cargo.toml`.
2. Commit, then tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. The `release` workflow builds all platforms, attaches SHA-256 checksums
   and publishes the GitHub release with the changelog section as notes.

## License

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in DBDock by you, as defined in the Apache-2.0
license, shall be dual licensed as in [README.md](README.md#license),
without any additional terms or conditions.
