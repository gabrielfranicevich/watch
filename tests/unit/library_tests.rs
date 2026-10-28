use super::*;

#[test]
fn version_matches_cargo_package_version() {
  assert_eq!(version(), env!("CARGO_PKG_VERSION"));
}
