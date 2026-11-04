use super::*;

#[test]
fn accepts_simple_and_nested_branch_names() {
  assert!(valid_branch_name("main"));
  assert!(valid_branch_name("feature/incremental-updater"));
  assert!(valid_branch_name("release/1.2.0"));
}

#[test]
fn rejects_invalid_or_option_like_branch_names() {
  for branch in [
    "",
    "--help",
    "../main",
    "feature/../main",
    "/main",
    "main/",
    "main.",
    "feature//updater",
    "feature/.hidden",
    "feature/name.lock",
    "feature @ name",
  ] {
    assert!(!valid_branch_name(branch), "accepted invalid branch {branch}");
  }
}

#[test]
fn cargo_install_uses_persistent_checkout_and_install_root() {
  let args = install_arguments(
    Path::new("/home/user/.cache/watch/updater/repository"),
    Path::new("/home/user/.cargo"),
  );
  let args: Vec<_> = args
    .iter()
    .map(|argument| argument.to_string_lossy().into_owned())
    .collect();

  assert_eq!(
    args,
    vec![
      "install",
      "--path",
      "/home/user/.cache/watch/updater/repository",
      "--root",
      "/home/user/.cargo",
      "--force",
      "--locked",
      "--bin",
      "watch",
    ]
  );
}

#[test]
fn first_update_installs_when_no_successful_revision_is_recorded() {
  assert!(needs_install(None, "same-commit"));
}

#[test]
fn cached_checkout_skips_install_when_installed_revision_matches_remote() {
  assert!(!needs_install(Some("same-commit"), "same-commit"));
}

#[test]
fn cached_checkout_retries_after_a_failed_installation() {
  assert!(needs_install(Some("old-installed-commit"), "new-commit"));
}

#[test]
fn revision_comparison_ignores_surrounding_whitespace() {
  assert!(!needs_install(Some("same-commit\n"), " same-commit "));
}

#[test]
fn empty_remote_revision_is_never_treated_as_installed() {
  assert!(needs_install(Some(""), ""));
  assert!(needs_install(Some("same-commit"), "  "));
}

#[test]
fn changed_remote_revision_requires_installation() {
  assert!(needs_install(Some("old-commit"), "new-commit"));
}
