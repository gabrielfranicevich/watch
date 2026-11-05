use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const REPOSITORY: &str = "https://github.com/gabrielfranicevich/watch";
const INSTALLABLE_BINARY: &str = "watch";

pub fn update_rust_installation(branch: &str) -> Result<(), Box<dyn std::error::Error>> {
  if !valid_branch_name(branch) {
    return Err(format!("invalid Git branch name '{branch}'").into());
  }

  let install_root = cargo_install_root()?;
  let cache_root = updater_cache_dir()
    .ok_or("cannot determine updater cache directory; set HOME or XDG_CACHE_HOME")?;
  let repository_dir = cache_root.join("repository");
  let target_dir = cache_root.join("target");
  let installed_revision_path = cache_root.join("installed-revision");
  std::fs::create_dir_all(&cache_root)?;
  let stale_before = SystemTime::now()
    .checked_sub(Duration::from_secs(24 * 60 * 60))
    .unwrap_or(UNIX_EPOCH);
  cleanup_stale_staging_roots(&install_root, stale_before)?;

  if repository_dir.exists() && !is_git_checkout(&repository_dir) {
    if repository_dir.is_dir() {
      std::fs::remove_dir_all(&repository_dir)?;
    } else {
      std::fs::remove_file(&repository_dir)?;
    }
  }

  if repository_dir.exists() {
    run_git(
      &repository_dir,
      &["fetch", "--quiet", "--depth=1", "origin", branch],
    )?;
  } else {
    clone_repository(&cache_root, &repository_dir, branch)?;
  }

  let current = git_output(&repository_dir, &["rev-parse", "HEAD"])?;
  let available = git_output(&repository_dir, &["rev-parse", "FETCH_HEAD"])?;
  if available.trim().is_empty() {
    return Err("git returned an empty fetched revision".into());
  }
  let installed_revision = std::fs::read_to_string(&installed_revision_path).ok();
  if !needs_install(
    installed_revision.as_deref().map(str::trim),
    available.trim(),
  ) {
    eprintln!("watch is already up to date ({})", &current.trim()[..12]);
    return Ok(());
  }

  run_git(
    &repository_dir,
    &[
      "checkout",
      "--quiet",
      "--force",
      "-B",
      "watch-updater",
      "FETCH_HEAD",
    ],
  )?;

  eprintln!(
    "Updating watch from branch '{branch}' ({} -> {})...",
    &current.trim()[..12],
    &available.trim()[..12]
  );
  let staging_root = staging_install_root(&install_root);
  std::fs::create_dir_all(&staging_root)?;
  let install_result = (|| {
    let mut cargo = Command::new("cargo");
    cargo
      .args(install_arguments(&repository_dir, &staging_root))
      .env("CARGO_TARGET_DIR", &target_dir)
      .env("CARGO_INCREMENTAL", "1");
    let output = cargo.output().map_err(|error| {
      if error.kind() == std::io::ErrorKind::NotFound {
        std::io::Error::new(
          std::io::ErrorKind::NotFound,
          "cargo was not found; install the Rust toolchain to use --update",
        )
      } else {
        error
      }
    })?;
    ensure_command_success(output, "cargo install")?;
    install_staged_binary(&staging_root, &install_root)
  })();
  let _ = std::fs::remove_dir_all(&staging_root);
  install_result?;
  std::fs::write(&installed_revision_path, format!("{}\n", available.trim()))?;
  eprintln!("watch update completed.");
  Ok(())
}

fn is_git_checkout(repository_dir: &Path) -> bool {
  repository_dir.join(".git").exists()
    && git_output(repository_dir, &["rev-parse", "--is-inside-work-tree"])
      .is_ok_and(|output| output.trim() == "true")
    && git_output(repository_dir, &["rev-parse", "--verify", "HEAD^{commit}"]).is_ok()
    && git_output(repository_dir, &["remote", "get-url", "origin"]).is_ok_and(|remote| {
      remote.trim().trim_end_matches('/').trim_end_matches(".git") == REPOSITORY
    })
}

fn needs_install(installed: Option<&str>, available: &str) -> bool {
  let available = available.trim();
  available.is_empty() || installed.map(str::trim) != Some(available)
}

fn cargo_install_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
  let executable = std::env::current_exe()?;
  let bin_dir = executable
    .parent()
    .ok_or("cannot determine the current executable directory")?;
  if bin_dir.file_name() != Some(OsStr::new("bin")) {
    return Err(format!(
      "cannot infer Cargo installation root from {}; install with cargo install or update manually",
      executable.display()
    )
    .into());
  }
  bin_dir
    .parent()
    .map(Path::to_path_buf)
    .ok_or_else(|| "cannot determine Cargo installation root".into())
}

fn updater_cache_dir() -> Option<PathBuf> {
  if cfg!(target_os = "windows") {
    return std::env::var_os("LOCALAPPDATA")
      .or_else(|| std::env::var_os("APPDATA"))
      .map(PathBuf::from)
      .map(|path| path.join("watch").join("updater"));
  }
  if cfg!(target_os = "macos") {
    return std::env::var_os("HOME").map(PathBuf::from).map(|path| {
      path
        .join("Library")
        .join("Caches")
        .join("watch")
        .join("updater")
    });
  }
  std::env::var_os("XDG_CACHE_HOME")
    .map(PathBuf::from)
    .or_else(|| {
      std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|path| path.join(".cache"))
    })
    .map(|path| path.join("watch").join("updater"))
}

fn clone_repository(
  cache_root: &Path,
  repository_dir: &Path,
  branch: &str,
) -> Result<(), Box<dyn std::error::Error>> {
  let nonce = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_nanos();
  let staging_dir = cache_root.join(format!("repository-clone-{}-{nonce}", std::process::id()));
  let mut command = Command::new("git");
  command
    .args([
      "clone",
      "--quiet",
      "--depth=1",
      "--single-branch",
      "--branch",
    ])
    .arg(branch)
    .arg(REPOSITORY)
    .arg(&staging_dir);
  let output = command.output()?;
  if let Err(error) = ensure_command_success(output, "git clone") {
    let _ = std::fs::remove_dir_all(&staging_dir);
    return Err(error);
  }
  std::fs::rename(&staging_dir, repository_dir)?;
  Ok(())
}

fn git_output(repository: &Path, arguments: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
  let mut command = Command::new("git");
  command.arg("-C").arg(repository).args(arguments);
  let output = command.output()?;
  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if stderr.trim().is_empty() {
      stdout.trim()
    } else {
      stderr.trim()
    };
    return Err(format!("git failed: {detail}").into());
  }
  Ok(String::from_utf8(output.stdout)?)
}

fn run_git(repository: &Path, arguments: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
  let mut command = Command::new("git");
  command.arg("-C").arg(repository).args(arguments);
  ensure_command_success(command.output()?, "git")
}

fn ensure_command_success(output: Output, program: &str) -> Result<(), Box<dyn std::error::Error>> {
  if output.status.success() {
    return Ok(());
  }
  let stderr = String::from_utf8_lossy(&output.stderr);
  let stdout = String::from_utf8_lossy(&output.stdout);
  let detail = if stderr.trim().is_empty() {
    stdout.trim()
  } else {
    stderr.trim()
  };
  Err(format!("{program} failed: {detail}").into())
}

fn cleanup_stale_staging_roots(
  install_root: &Path,
  stale_before: SystemTime,
) -> Result<(), Box<dyn std::error::Error>> {
  for entry in std::fs::read_dir(install_root)? {
    let entry = entry?;
    let name = entry.file_name();
    if !name.to_string_lossy().starts_with(".watch-update-") || !entry.file_type()?.is_dir() {
      continue;
    }
    if entry
      .metadata()?
      .modified()
      .is_ok_and(|modified| modified <= stale_before)
    {
      std::fs::remove_dir_all(entry.path())?;
    }
  }
  Ok(())
}

fn staging_install_root(install_root: &Path) -> PathBuf {
  let nonce = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_nanos();
  install_root.join(format!(".watch-update-{}-{nonce}", std::process::id()))
}

fn installed_binary_name() -> &'static str {
  if cfg!(windows) {
    "watch.exe"
  } else {
    INSTALLABLE_BINARY
  }
}

fn install_staged_binary(
  staging_root: &Path,
  install_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
  let staged_binary = staging_root.join("bin").join(installed_binary_name());
  let installed_binary = install_root.join("bin").join(installed_binary_name());
  let backup_binary = staging_root.join(format!("{}.previous", installed_binary_name()));
  if !staged_binary.is_file() {
    return Err(
      format!(
        "cargo install did not produce the expected executable at {}",
        staged_binary.display()
      )
      .into(),
    );
  }
  replace_binary(&staged_binary, &installed_binary, &backup_binary)?;
  Ok(())
}

fn replace_binary(
  staged_binary: &Path,
  installed_binary: &Path,
  backup_binary: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
  #[cfg(not(windows))]
  {
    let _ = backup_binary;
    std::fs::rename(staged_binary, installed_binary)?;
  }

  #[cfg(windows)]
  {
    let had_installed_binary = installed_binary.exists();
    if had_installed_binary {
      std::fs::rename(installed_binary, backup_binary)?;
    }
    if let Err(error) = std::fs::rename(staged_binary, installed_binary) {
      if had_installed_binary {
        if let Err(restore_error) = std::fs::rename(backup_binary, installed_binary) {
          return Err(std::io::Error::other(format!(
            "failed to install the staged executable ({error}) and failed to restore the previous executable ({restore_error})"
          ))
          .into());
        }
      }
      return Err(error.into());
    }
    if had_installed_binary {
      std::fs::remove_file(backup_binary)?;
    }
  }
  Ok(())
}

fn install_arguments(repository_dir: &Path, install_root: &Path) -> Vec<OsString> {
  vec![
    OsString::from("install"),
    OsString::from("--path"),
    repository_dir.as_os_str().to_owned(),
    OsString::from("--root"),
    install_root.as_os_str().to_owned(),
    OsString::from("--force"),
    OsString::from("--locked"),
    OsString::from("--bin"),
    OsString::from(INSTALLABLE_BINARY),
  ]
}

fn valid_branch_name(branch: &str) -> bool {
  !branch.is_empty()
    && !branch.starts_with('-')
    && !branch.ends_with('/')
    && !branch.ends_with('.')
    && !branch.contains("..")
    && !branch.contains("//")
    && !branch.contains("@{")
    && branch
      .split('/')
      .all(|part| !part.is_empty() && !part.starts_with('.') && !part.ends_with(".lock"))
    && branch
      .chars()
      .all(|character| character.is_ascii_alphanumeric() || "-_./".contains(character))
}

#[cfg(test)]
#[path = "../../tests/unit/infrastructure/updater.rs"]
mod tests;
