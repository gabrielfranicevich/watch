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
  assert!(!is_git_checkout_from(&root, REPOSITORY));
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupted_update_staging_directories_are_cleaned_without_touching_other_paths() {
  let root = temporary_repository_path();
  let stale = root.join(".watch-update-interrupted");
  let unrelated = root.join("other-directory");
  std::fs::create_dir_all(&stale).unwrap();
  std::fs::create_dir_all(&unrelated).unwrap();
  std::fs::write(stale.join("partial"), b"partial install").unwrap();

  cleanup_stale_staging_roots(&root, SystemTime::now() + Duration::from_secs(1)).unwrap();

  assert!(!stale.exists());
  assert!(unrelated.is_dir());
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn recent_update_staging_directories_are_not_cleaned() {
  let root = temporary_repository_path();
  let recent = root.join(".watch-update-active");
  std::fs::create_dir_all(&recent).unwrap();

  cleanup_stale_staging_roots(&root, UNIX_EPOCH).unwrap();

  assert!(recent.is_dir());
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn metadata_sync_failure_restores_the_previous_executable() {
  let root = temporary_repository_path();
  let install_root = root.join("install");
  let staging_root = root.join("staging");
  let installed_dir = install_root.join("bin");
  let staged_dir = staging_root.join("bin");
  std::fs::create_dir_all(&installed_dir).unwrap();
  std::fs::create_dir_all(&staged_dir).unwrap();
  std::fs::write(
    installed_dir.join(installed_binary_name()),
    b"old executable",
  )
  .unwrap();
  std::fs::write(staged_dir.join(installed_binary_name()), b"new executable").unwrap();
  std::fs::write(staging_root.join(".crates.toml"), "watch = []\n").unwrap();
  let blocked_metadata = install_root.join(format!(
    ".watch-update-{}-.crates.toml.tmp",
    std::process::id()
  ));
  std::fs::create_dir_all(&blocked_metadata).unwrap();

  assert!(install_staged_binary(&staging_root, &install_root).is_err());
  assert_eq!(
    std::fs::read(installed_dir.join(installed_binary_name())).unwrap(),
    b"old executable"
  );

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

#[test]
fn seeds_existing_cargo_tracking_metadata_into_the_staging_root() {
  let root = temporary_repository_path();
  let install_root = root.join("install");
  let staging_root = root.join("staging");
  std::fs::create_dir_all(&install_root).unwrap();
  std::fs::create_dir_all(&staging_root).unwrap();
  std::fs::write(install_root.join(".crates.toml"), "v1\n").unwrap();
  std::fs::write(install_root.join(".crates2.json"), "{\"v1\" : []}").unwrap();

  seed_cargo_tracking_metadata(&install_root, &staging_root).unwrap();

  assert_eq!(
    std::fs::read_to_string(staging_root.join(".crates.toml")).unwrap(),
    "v1\n"
  );
  assert_eq!(
    std::fs::read_to_string(staging_root.join(".crates2.json")).unwrap(),
    "{\"v1\" : []}"
  );
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn syncs_staged_cargo_metadata_without_dropping_other_installed_packages() {
  let root = temporary_repository_path();
  let install_root = root.join("install");
  let staging_root = root.join("staging");
  std::fs::create_dir_all(&install_root).unwrap();
  std::fs::create_dir_all(&staging_root).unwrap();
  std::fs::write(install_root.join(".crates.toml"), "existing-package = []\n").unwrap();
  std::fs::write(
    install_root.join(".crates2.json"),
    "{\"existing-package\":{}}",
  )
  .unwrap();
  seed_cargo_tracking_metadata(&install_root, &staging_root).unwrap();
  std::fs::write(
    staging_root.join(".crates.toml"),
    "existing-package = []\nwatch = []\n",
  )
  .unwrap();
  std::fs::write(
    staging_root.join(".crates2.json"),
    "{\"existing-package\":{},\"watch\":{}}",
  )
  .unwrap();

  sync_cargo_tracking_metadata(&staging_root, &install_root).unwrap();

  assert_eq!(
    std::fs::read_to_string(install_root.join(".crates.toml")).unwrap(),
    "existing-package = []\nwatch = []\n"
  );
  assert_eq!(
    std::fs::read_to_string(install_root.join(".crates2.json")).unwrap(),
    "{\"existing-package\":{},\"watch\":{}}"
  );
  assert!(
    !install_root
      .join(".watch-update-meta-.crates.toml.tmp")
      .exists()
  );
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn leaves_existing_cargo_metadata_untouched_when_staging_has_no_tracking_files() {
  let root = temporary_repository_path();
  let install_root = root.join("install");
  let staging_root = root.join("staging");
  std::fs::create_dir_all(&install_root).unwrap();
  std::fs::create_dir_all(&staging_root).unwrap();
  std::fs::write(install_root.join(".crates.toml"), "existing = []\n").unwrap();

  sync_cargo_tracking_metadata(&staging_root, &install_root).unwrap();

  assert_eq!(
    std::fs::read_to_string(install_root.join(".crates.toml")).unwrap(),
    "existing = []\n"
  );
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cargo_tracking_metadata_stays_coherent_after_a_staged_install() {
  let root = temporary_repository_path();
  let install_root = root.join("cargo");
  let staging_root = root.join("staging");
  let target_dir = root.join("target");
  let existing_package = root.join("existing-package");
  let watch_package = root.join("watch-package");
  create_fixture_package(&existing_package, "watch-existing-fixture");
  create_fixture_package(&watch_package, "watch-staged-fixture");

  install_fixture_package(&existing_package, &install_root, &target_dir);
  std::fs::create_dir_all(&staging_root).unwrap();
  seed_cargo_tracking_metadata(&install_root, &staging_root).unwrap();
  install_fixture_package(&watch_package, &staging_root, &target_dir);
  sync_cargo_tracking_metadata(&staging_root, &install_root).unwrap();

  let listing = Command::new("cargo")
    .args(["install", "--list", "--root"])
    .arg(&install_root)
    .output()
    .unwrap();
  assert!(listing.status.success());
  let listing = String::from_utf8_lossy(&listing.stdout);
  assert!(listing.contains("watch-existing-fixture"), "{listing}");
  assert!(listing.contains("watch-staged-fixture"), "{listing}");
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cargo_uninstall_removes_staged_package_without_dropping_existing_packages() {
  let root = temporary_repository_path();
  let install_root = root.join("cargo");
  let staging_root = root.join("staging");
  let target_dir = root.join("target");
  let existing_package = root.join("existing-package");
  let staged_package = root.join("staged-package");
  create_fixture_package(&existing_package, "watch-existing-fixture");
  create_fixture_package(&staged_package, "watch-staged-fixture");

  install_fixture_package(&existing_package, &install_root, &target_dir);
  std::fs::create_dir_all(&staging_root).unwrap();
  seed_cargo_tracking_metadata(&install_root, &staging_root).unwrap();
  install_fixture_package(&staged_package, &staging_root, &target_dir);
  sync_cargo_tracking_metadata(&staging_root, &install_root).unwrap();

  let uninstall = Command::new("cargo")
    .args(["uninstall", "--root"])
    .arg(&install_root)
    .arg("watch-staged-fixture")
    .output()
    .unwrap();
  assert!(
    uninstall.status.success(),
    "{}",
    String::from_utf8_lossy(&uninstall.stderr)
  );

  let listing = Command::new("cargo")
    .args(["install", "--list", "--root"])
    .arg(&install_root)
    .output()
    .unwrap();
  assert!(listing.status.success());
  let listing = String::from_utf8_lossy(&listing.stdout);
  assert!(listing.contains("watch-existing-fixture"), "{listing}");
  assert!(!listing.contains("watch-staged-fixture"), "{listing}");

  let uninstall = Command::new("cargo")
    .args(["uninstall", "--root"])
    .arg(&install_root)
    .arg("watch-existing-fixture")
    .output()
    .unwrap();
  assert!(
    uninstall.status.success(),
    "{}",
    String::from_utf8_lossy(&uninstall.stderr)
  );
  let listing = Command::new("cargo")
    .args(["install", "--list", "--root"])
    .arg(&install_root)
    .output()
    .unwrap();
  assert!(listing.status.success());
  assert!(
    String::from_utf8_lossy(&listing.stdout).trim().is_empty(),
    "{}",
    String::from_utf8_lossy(&listing.stdout)
  );

  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cargo_build_failure_preserves_installed_binary_and_tracking_metadata() {
  let root = temporary_repository_path();
  let existing_package = root.join("existing-package");
  let failing_package = root.join("failing-package");
  let install_root = root.join("cargo");
  let staging_root = root.join("staging");
  let target_dir = root.join("target");
  create_fixture_package(&existing_package, "watch-existing-fixture");
  create_fixture_package(&failing_package, "watch-failing-fixture");
  std::fs::write(
    failing_package.join("src/main.rs"),
    "fn main() { let value: = ; }\n",
  )
  .unwrap();

  install_fixture_package(&existing_package, &install_root, &target_dir);
  let installed_binary = install_root.join("bin").join(installed_binary_name());
  let previous_binary = std::fs::read(&installed_binary).unwrap();
  let previous_crates = std::fs::read(install_root.join(".crates.toml")).unwrap();
  let previous_crates2 = std::fs::read(install_root.join(".crates2.json")).unwrap();
  std::fs::create_dir_all(&staging_root).unwrap();
  seed_cargo_tracking_metadata(&install_root, &staging_root).unwrap();

  let output = Command::new("cargo")
    .args(["install", "--path"])
    .arg(&failing_package)
    .arg("--root")
    .arg(&staging_root)
    .args(["--force", "--locked", "--verbose"])
    .env("CARGO_TARGET_DIR", &target_dir)
    .env("CARGO_INCREMENTAL", "1")
    .output()
    .unwrap();
  assert!(!output.status.success());

  assert_eq!(std::fs::read(&installed_binary).unwrap(), previous_binary);
  assert_eq!(
    std::fs::read(install_root.join(".crates.toml")).unwrap(),
    previous_crates
  );
  assert_eq!(
    std::fs::read(install_root.join(".crates2.json")).unwrap(),
    previous_crates2
  );
  assert!(!staging_root.join("bin").join(installed_binary_name()).exists());

  let listing = Command::new("cargo")
    .args(["install", "--list", "--root"])
    .arg(&install_root)
    .output()
    .unwrap();
  assert!(listing.status.success());
  let listing = String::from_utf8_lossy(&listing.stdout);
  assert!(listing.contains("watch-existing-fixture"), "{listing}");
  assert!(!listing.contains("watch-failing-fixture"), "{listing}");

  std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn install_permission_failure_keeps_the_previous_executable() {
  use std::os::unix::fs::PermissionsExt;

  if unsafe { libc::geteuid() } == 0 {
    return;
  }

  let root = temporary_repository_path();
  let install_root = root.join("install");
  let staging_root = root.join("staging");
  let installed_dir = install_root.join("bin");
  let staged_dir = staging_root.join("bin");
  std::fs::create_dir_all(&installed_dir).unwrap();
  std::fs::create_dir_all(&staged_dir).unwrap();
  let installed_binary = installed_dir.join(installed_binary_name());
  std::fs::write(&installed_binary, b"previous executable").unwrap();
  std::fs::write(staged_dir.join(installed_binary_name()), b"staged executable").unwrap();
  std::fs::set_permissions(&install_root, std::fs::Permissions::from_mode(0o555)).unwrap();

  let result = install_staged_binary(&staging_root, &install_root);
  std::fs::set_permissions(&install_root, std::fs::Permissions::from_mode(0o755)).unwrap();

  assert!(result.is_err());
  assert_eq!(
    std::fs::read(&installed_binary).unwrap(),
    b"previous executable"
  );
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cargo_installs_reuse_the_persistent_incremental_target_cache() {
  let root = temporary_repository_path();
  let package = root.join("package");
  let install_root = root.join("install");
  let target_dir = root.join("target");
  create_fixture_package(&package, "watch-cache-fixture");

  let first = install_fixture_package(&package, &install_root, &target_dir);
  let incremental_dir = target_dir.join("release").join("incremental");
  assert!(incremental_dir.is_dir());
  let first_status = format!(
    "{}{}",
    String::from_utf8_lossy(&first.stdout),
    String::from_utf8_lossy(&first.stderr)
  );
  assert!(first_status.contains("Compiling"), "{first_status}");

  let second = install_fixture_package(&package, &install_root, &target_dir);
  let second_status = format!(
    "{}{}",
    String::from_utf8_lossy(&second.stdout),
    String::from_utf8_lossy(&second.stderr)
  );
  assert!(second_status.contains("Fresh"), "{second_status}");
  assert!(incremental_dir.is_dir());

  std::fs::remove_dir_all(root).unwrap();
}

fn create_fixture_package(path: &Path, name: &str) {
  std::fs::create_dir_all(path.join("src")).unwrap();
  std::fs::write(
    path.join("Cargo.toml"),
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
  )
  .unwrap();
  std::fs::write(path.join("src/main.rs"), "fn main() {}\n").unwrap();
  let output = Command::new("cargo")
    .args(["generate-lockfile", "--manifest-path"])
    .arg(path.join("Cargo.toml"))
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );
}

fn install_fixture_package(package: &Path, install_root: &Path, target_dir: &Path) -> Output {
  let output = Command::new("cargo")
    .args(["install", "--path"])
    .arg(package)
    .arg("--root")
    .arg(install_root)
    .args(["--force", "--locked", "--verbose"])
    .env("CARGO_TARGET_DIR", target_dir)
    .env("CARGO_INCREMENTAL", "1")
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );
  output
}

#[cfg(windows)]
#[test]
fn refuses_to_replace_a_running_executable_that_is_locked() {
  use std::os::windows::fs::OpenOptionsExt;

  let root = temporary_repository_path();
  std::fs::create_dir_all(&root).unwrap();
  let installed_binary = root.join("watch.exe");
  let staged_binary = root.join("staged.exe");
  let backup_binary = root.join("previous.exe");
  std::fs::write(&installed_binary, b"old executable").unwrap();
  std::fs::write(&staged_binary, b"new executable").unwrap();
  let _lock = std::fs::OpenOptions::new()
    .read(true)
    .share_mode(0)
    .open(&installed_binary)
    .unwrap();

  assert!(replace_binary(&staged_binary, &installed_binary, &backup_binary).is_err());
  assert!(!backup_binary.exists());
  drop(_lock);
  assert_eq!(std::fs::read(&installed_binary).unwrap(), b"old executable");
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
fn failed_fetch_preserves_a_valid_checkout_for_retry() {
  let root = temporary_repository_path();
  let remote = root.join("remote.git");
  let source = root.join("source");
  let cache_root = root.join("cache");
  let repository_dir = cache_root.join("repository");
  std::fs::create_dir_all(&source).unwrap();
  std::fs::create_dir_all(&cache_root).unwrap();

  let initialized = Command::new("git")
    .args(["init", "--bare", "--quiet"])
    .arg(&remote)
    .status()
    .unwrap();
  assert!(initialized.success());
  run_git_in(&source, &["init", "--quiet"]);
  run_git_in(&source, &["checkout", "-b", "main"]);
  for (key, value) in [
    ("user.name", "Watch Tests"),
    ("user.email", "watch-tests@example.invalid"),
  ] {
    run_git_in(&source, &["config", key, value]);
  }
  std::fs::write(source.join("README.md"), "preserve me\n").unwrap();
  run_git_in(&source, &["add", "README.md"]);
  run_git_in(&source, &["commit", "--quiet", "-m", "initial"]);
  let remote_url = remote.to_str().unwrap();
  run_git_in(&source, &["remote", "add", "origin", remote_url]);
  run_git_in(&source, &["push", "--quiet", "-u", "origin", "main"]);

  ensure_repository_checkout(
    &cache_root,
    &repository_dir,
    "main",
    remote_url,
  )
  .unwrap();
  let original_revision = git_output(&repository_dir, &["rev-parse", "HEAD"]).unwrap();

  assert!(ensure_repository_checkout(
    &cache_root,
    &repository_dir,
    "missing-branch",
    remote_url,
  )
  .is_err());

  assert!(repository_dir.is_dir());
  assert!(is_git_checkout_from(&repository_dir, remote_url));
  assert_eq!(
    git_output(&repository_dir, &["rev-parse", "HEAD"])
      .unwrap()
      .trim(),
    original_revision.trim()
  );
  assert_eq!(
    std::fs::read_to_string(repository_dir.join("README.md")).unwrap(),
    "preserve me\n"
  );

  ensure_repository_checkout(
    &cache_root,
    &repository_dir,
    "main",
    remote_url,
  )
  .unwrap();
  assert!(is_git_checkout_from(&repository_dir, remote_url));
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn clones_fetches_and_recovers_a_corrupted_checkout_end_to_end() {
  let root = temporary_repository_path();
  let remote = root.join("remote.git");
  let source = root.join("source");
  let cache_root = root.join("cache");
  let repository_dir = cache_root.join("repository");
  std::fs::create_dir_all(&cache_root).unwrap();
  std::fs::create_dir_all(&source).unwrap();

  let initialized = Command::new("git")
    .args(["init", "--bare", "--quiet"])
    .arg(&remote)
    .status()
    .unwrap();
  assert!(initialized.success());
  run_git_in(&source, &["init", "--quiet"]);
  run_git_in(&source, &["checkout", "-b", "main"]);
  for (key, value) in [
    ("user.name", "Watch Tests"),
    ("user.email", "watch-tests@example.invalid"),
  ] {
    run_git_in(&source, &["config", key, value]);
  }
  std::fs::write(source.join("README.md"), "initial\n").unwrap();
  run_git_in(&source, &["add", "README.md"]);
  run_git_in(&source, &["commit", "--quiet", "-m", "initial"]);
  let remote_url = remote.to_str().unwrap();
  run_git_in(&source, &["remote", "add", "origin", remote_url]);
  run_git_in(&source, &["push", "--quiet", "-u", "origin", "main"]);

  std::fs::create_dir_all(&repository_dir).unwrap();
  std::fs::write(repository_dir.join("interrupted"), b"partial clone").unwrap();
  ensure_repository_checkout(
    &cache_root,
    &repository_dir,
    "main",
    remote.to_str().unwrap(),
  )
  .unwrap();
  assert!(is_git_checkout_from(
    &repository_dir,
    remote.to_str().unwrap()
  ));
  assert!(!repository_dir.join("interrupted").exists());
  let initial_revision = git_output(&repository_dir, &["rev-parse", "HEAD"]).unwrap();

  std::fs::write(source.join("README.md"), "updated\n").unwrap();
  run_git_in(&source, &["add", "README.md"]);
  run_git_in(&source, &["commit", "--quiet", "-m", "updated"]);
  run_git_in(&source, &["push", "--quiet", "origin", "main"]);
  let fetched_revision = fetch_remote_branch(&repository_dir, "main").unwrap();
  assert_ne!(initial_revision.trim(), fetched_revision.trim());
  assert_eq!(
    git_output(&repository_dir, &["rev-parse", "FETCH_HEAD"])
      .unwrap()
      .trim(),
    fetched_revision.trim()
  );

  run_git_in(
    &repository_dir,
    &[
      "remote",
      "set-url",
      "origin",
      "https://example.invalid/corrupted",
    ],
  );
  ensure_repository_checkout(
    &cache_root,
    &repository_dir,
    "main",
    remote.to_str().unwrap(),
  )
  .unwrap();
  assert!(is_git_checkout_from(
    &repository_dir,
    remote.to_str().unwrap()
  ));
  assert_eq!(
    git_output(&repository_dir, &["rev-parse", "HEAD"])
      .unwrap()
      .trim(),
    fetched_revision.trim()
  );
  let readme = std::fs::read_to_string(repository_dir.join("README.md"))
    .unwrap()
    .replace("\r\n", "\n");
  assert_eq!(readme, "updated\n");

  std::fs::remove_dir_all(root).unwrap();
}

fn run_git_in(directory: &Path, arguments: &[&str]) {
  let output = Command::new("git")
    .args(arguments)
    .current_dir(directory)
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "git {} failed: {}",
    arguments.join(" "),
    String::from_utf8_lossy(&output.stderr)
  );
}

#[test]
fn recognizes_persistent_git_checkouts() {
  let root = temporary_repository_path();
  create_git_checkout(&root);
  assert!(is_git_checkout_from(&root, REPOSITORY));
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_cache_directories_without_git_metadata() {
  let root = temporary_repository_path();
  std::fs::create_dir_all(&root).unwrap();
  assert!(!is_git_checkout_from(&root, REPOSITORY));
  std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_git_directories_with_an_invalid_head() {
  let root = temporary_repository_path();
  create_git_checkout(&root);
  std::fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/missing\n").unwrap();
  assert!(!is_git_checkout_from(&root, REPOSITORY));
  std::fs::remove_dir_all(root).unwrap();
}
