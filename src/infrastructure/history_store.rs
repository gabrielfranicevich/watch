use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::history::HistoryEntry;

#[derive(Debug)]
pub struct HistoryStore {
  path: PathBuf,
}

impl HistoryStore {
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self { path: path.into() }
  }

  pub fn read(&self) -> io::Result<Vec<HistoryEntry>> {
    let content = match fs::read_to_string(&self.path) {
      Ok(content) => content,
      Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
      Err(error) => return Err(error),
    };

    Ok(content.lines().filter_map(parse_history_line).collect())
  }

  pub fn write(&self, entries: &[HistoryEntry]) -> io::Result<()> {
    self.create_parent_dir()?;
    let lock_path = suffix_path(&self.path, ".lock");
    let _lock = LockFile::acquire(lock_path)?;
    self.write_without_lock(entries)
  }

  /// Back up legacy history entries whose IDs do not use the current slug
  /// format, then remove them from the active history. This mirrors the
  /// shell implementation's .v4 backup file.
  pub fn backup_legacy_entries(&self) -> io::Result<()> {
    self.create_parent_dir()?;
    let lock_path = suffix_path(&self.path, ".lock");
    let _lock = LockFile::acquire(lock_path)?;

    let entries = self.read()?;
    let (legacy, current): (Vec<_>, Vec<_>) = entries
      .into_iter()
      .partition(|entry| entry.provider.is_none() && !entry.anime_id.contains('-'));

    if legacy.is_empty() {
      return Ok(());
    }

    let backup_path = suffix_path(&self.path, ".v4");
    let existing_backup = match fs::read_to_string(&backup_path) {
      Ok(content) => content.lines().map(str::to_owned).collect::<HashSet<_>>(),
      Err(error) if error.kind() == io::ErrorKind::NotFound => HashSet::new(),
      Err(error) => return Err(error),
    };
    let mut backed_up_rows = existing_backup;
    let mut backup = OpenOptions::new()
      .create(true)
      .append(true)
      .open(backup_path)?;
    for entry in &legacy {
      let row = format!(
        "{}\t{}\t{}",
        clean_field(&entry.episode_number),
        clean_field(&entry.anime_id),
        clean_field(&entry.anime_title)
      );
      if backed_up_rows.insert(row.clone()) {
        writeln!(backup, "{row}")?;
      }
    }
    backup.sync_all()?;

    self.write_without_lock(&current)
  }

  fn create_parent_dir(&self) -> io::Result<()> {
    let parent = self
      .path
      .parent()
      .filter(|path| !path.as_os_str().is_empty())
      .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
  }

  fn write_without_lock(&self, entries: &[HistoryEntry]) -> io::Result<()> {
    let temp_path = unique_temp_path(&self.path);
    let result = (|| {
      let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)?;
      for entry in entries {
        let provider = entry
          .provider
          .as_deref()
          .map(|provider| format!("\tprovider:{}", clean_field(provider)))
          .unwrap_or_default();
        writeln!(
          file,
          "{}\t{}\t{}{}",
          clean_field(&entry.episode_number),
          clean_field(&entry.anime_id),
          clean_field(&entry.anime_title),
          provider
        )?;
      }
      file.sync_all()?;
      replace_file(&temp_path, &self.path)
    })();

    if result.is_err() {
      let _ = fs::remove_file(&temp_path);
    }
    result
  }
}

fn parse_history_line(line: &str) -> Option<HistoryEntry> {
  let mut fields = line.splitn(4, '\t');
  let episode_number = fields.next()?;
  let anime_id = fields.next()?;
  let title = fields.next()?;
  if episode_number.is_empty() || anime_id.is_empty() {
    return None;
  }

  let (anime_title, provider) = match fields.next() {
    Some(value) if value.starts_with("provider:") && value.len() > "provider:".len() => (
      title.to_owned(),
      Some(value["provider:".len()..].to_owned()),
    ),
    Some(value) => (format!("{title}\t{value}"), None),
    None => (title.to_owned(), None),
  };

  Some(HistoryEntry {
    episode_number: episode_number.to_owned(),
    anime_id: anime_id.to_owned(),
    anime_title,
    provider,
  })
}

fn clean_field(value: &str) -> String {
  value.replace(['\t', '\n', '\r'], " ")
}

fn suffix_path(path: &Path, suffix: &str) -> PathBuf {
  let mut name = path.as_os_str().to_os_string();
  name.push(suffix);
  PathBuf::from(name)
}

fn unique_temp_path(path: &Path) -> PathBuf {
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_nanos();
  let mut name = path.as_os_str().to_os_string();
  name.push(format!(".{}.{}.tmp", std::process::id(), nanos));
  PathBuf::from(name)
}

fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
  #[cfg(windows)]
  if destination.exists() {
    fs::remove_file(destination)?;
  }
  fs::rename(source, destination)
}

struct LockFile {
  path: PathBuf,
}

impl LockFile {
  fn acquire(path: PathBuf) -> io::Result<Self> {
    OpenOptions::new()
      .write(true)
      .create_new(true)
      .open(&path)?;
    Ok(Self { path })
  }
}

impl Drop for LockFile {
  fn drop(&mut self) {
    let _ = fs::remove_file(&self.path);
  }
}

#[cfg(test)]
#[path = "../../tests/unit/infrastructure/history_store.rs"]
mod tests;
