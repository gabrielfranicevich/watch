//! Core library for watch.

pub mod application;
pub mod cli;
pub mod domain;
pub mod infrastructure;
pub mod media;
pub mod providers;

/// Current package version, exposed for diagnostics and tests.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn version() -> &'static str {
  VERSION
}

#[cfg(test)]
#[path = "../tests/unit/library_tests.rs"]
mod tests;
