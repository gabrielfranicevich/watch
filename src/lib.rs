//! Core library for the ani-cli Rust implementation.

pub mod cli;
pub mod domain;
pub mod media;
pub mod infrastructure;
pub mod providers;

/// Current package version, exposed for diagnostics and tests.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_package_version() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
