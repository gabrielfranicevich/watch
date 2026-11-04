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
fn first_update_installs_even_when_the_new_checkout_matches_the_remote() {
  assert!(needs_install(true, "same-commit", "same-commit"));
}

#[test]
fn cached_checkout_skips_install_when_the_remote_is_unchanged() {
  assert!(!needs_install(false, "same-commit", "same-commit"));
}

#[test]
fn cached_checkout_installs_when_the_remote_commit_changes() {
  assert!(needs_install(false, "old-commit", "new-commit"));
}
