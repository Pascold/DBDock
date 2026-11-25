//! DBDock: one-click test database servers in Docker.
//!
//! The binary in `main.rs` is a thin wrapper around this library; the library
//! target exists so integration tests can use the compose generator.

pub mod app;
pub mod compose;
pub mod config;
pub mod docker;
pub mod engines;
pub mod i18n;
pub mod paths;
pub mod terminal;
pub mod ui;
