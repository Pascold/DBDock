## What and why

<!-- What does this change and why is it needed? Link related issues: "Fixes #123". -->

## How was it tested?

<!-- Platforms and databases you tried, new or updated tests. -->

## Checklist

- [ ] `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass
- [ ] New UI texts are in both `assets/i18n/en.toml` and `assets/i18n/pl.toml`
- [ ] Compose snapshot changes are reviewed (`cargo insta review`), if any
- [ ] `CHANGELOG.md` is updated under *Unreleased* for user-visible changes
- [ ] Commits follow Conventional Commits
