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
    assert!(
      !valid_branch_name(branch),
      "accepted invalid branch {branch}"
    );
  }
}

#[test]
fn cargo_install_uses_persistent_checkout_and_staging_root() {
  let args = install_arguments(
    Path::new("/home/user/.cache/watch/updater/repository"),
    Path::new("/home/user/.cargo/.watch-update-123"),
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
      "/home/user/.cargo/.watch-update-123",
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
  assert!(needs_install(None, ""));
  assert!(needs_install(Some(""), ""));
  assert!(needs_install(Some("same-commit"), "  "));
}

#[test]
fn changed_remote_revision_requires_installation() {
  assert!(needs_install(Some("old-commit"), "new-commit"));
}

#[test]
fn rejects_checkouts_with_an_unexpected_origin() {
  let root = temporary_repository_path();
  create_git_checkout(&root);
  let output = Command::new("git")
    .args([
      "remote",
      "set-url",
      "origin",
      "https://example.invalid/watch",
    ])
    .current_dir(&root)
    .status()
    .unwrap();
  assert!(output.success());
  assert!(!is_git_checkout(&root));
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn staged_install_replaces_the_existing_executable() {
  let root = temporary_repository_path();
  let install_root = root.join("install");
  let staging_root = root.join("staging");
  let installed_dir = install_root.join("bin");
  let staged_dir = staging_root.join("bin");
  std::fs::create_dir_all(&installed_dir).unwrap();
  std::fs::create_dir_all(&staged_dir).unwrap();
  let binary_name = installed_binary_name();
  std::fs::write(installed_dir.join(binary_name), b"old executable").unwrap();
  std::fs::write(staged_dir.join(binary_name), b"new executable").unwrap();

  install_staged_binary(&staging_root, &install_root).unwrap();

  assert_eq!(
    std::fs::read(installed_dir.join(binary_name)).unwrap(),
    b"new executable"
  );
  assert!(!staged_dir.join(binary_name).exists());
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_staged_install_preserves_the_existing_executable() {
  let root = temporary_repository_path();
  let install_root = root.join("install");
  let staging_root = root.join("staging");
  let installed_dir = install_root.join("bin");
  std::fs::create_dir_all(&installed_dir).unwrap();
  std::fs::create_dir_all(staging_root.join("bin")).unwrap();
  let installed_binary = installed_dir.join(installed_binary_name());
  std::fs::write(&installed_binary, b"old executable").unwrap();

  assert!(install_staged_binary(&staging_root, &install_root).is_err());
  assert_eq!(std::fs::read(&installed_binary).unwrap(), b"old executable");

  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_binary_replacement_keeps_or_restores_the_previous_executable() {
  let root = temporary_repository_path();
  std::fs::create_dir_all(&root).unwrap();
  let installed_binary = root.join(installed_binary_name());
  let staged_binary = root.join("missing-staged-binary");
  let backup_binary = root.join("previous-binary");
  std::fs::write(&installed_binary, b"old executable").unwrap();

  assert!(replace_binary(&staged_binary, &installed_binary, &backup_binary).is_err());
  assert_eq!(std::fs::read(&installed_binary).unwrap(), b"old executable");
  assert!(!backup_binary.exists());

  std::fs::remove_dir_all(root).unwrap();
}

fn temporary_repository_path() -> PathBuf {
  std::env::temp_dir().join(format!(
    "watch-updater-test-{}-{}",
    std::process::id(),
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or_default()
      .as_nanos()
  ))
}

fn create_git_checkout(root: &Path) {
  std::fs::create_dir_all(root).unwrap();
  let init = Command::new("git")
    .args(["init", "--quiet"])
    .current_dir(root)
    .status()
    .unwrap();
  assert!(init.success());

  for (key, value) in [
    ("user.name", "Watch Tests"),
    ("user.email", "watch-tests@example.invalid"),
  ] {
    let config = Command::new("git")
      .args(["config", key, value])
      .current_dir(root)
      .status()
      .unwrap();
    assert!(config.success());
  }

  std::fs::write(root.join("README.md"), "fixture\n").unwrap();
  for arguments in [
    vec!["add", "README.md"],
    vec!["commit", "--quiet", "-m", "initial"],
    vec![
      "remote",
      "add",
      "origin",
      "https://github.com/gabrielfranicevich/watch",
    ],
  ] {
    let output = Command::new("git")
      .args(arguments)
      .current_dir(root)
      .status()
      .unwrap();
    assert!(output.success());
  }
}

#[test]
fn recognizes_persistent_git_checkouts() {
  let root = temporary_repository_path();
  create_git_checkout(&root);
  assert!(is_git_checkout(&root));
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_cache_directories_without_git_metadata() {
  let root = temporary_repository_path();
  std::fs::create_dir_all(&root).unwrap();
  assert!(!is_git_checkout(&root));
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_git_directories_with_an_invalid_head() {
  let root = temporary_repository_path();
  create_git_checkout(&root);
  std::fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/missing\n").unwrap();
  assert!(!is_git_checkout(&root));
  std::fs::remove_dir_all(root).unwrap();
}
