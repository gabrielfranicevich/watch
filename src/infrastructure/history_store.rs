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
            .partition(|entry| !entry.anime_id.contains('-'));

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
                writeln!(
                    file,
                    "{}\t{}\t{}",
                    clean_field(&entry.episode_number),
                    clean_field(&entry.anime_id),
                    clean_field(&entry.anime_title)
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
    let mut fields = line.splitn(3, '\t');
    let episode_number = fields.next()?;
    let anime_id = fields.next()?;
    let anime_title = fields.next()?;
    if episode_number.is_empty() || anime_id.is_empty() {
        return None;
    }
    Some(HistoryEntry::new(episode_number, anime_id, anime_title))
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
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEST_PATH: AtomicU64 = AtomicU64::new(0);

    fn test_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "ani-cli-history-{}-{nonce}-{sequence}",
            std::process::id()
        ))
    }

    fn remove_history_artifacts(path: &Path) {
        for suffix in ["", ".v4", ".lock"] {
            let candidate = suffix_path(path, suffix);
            let _ = fs::remove_file(candidate);
        }
    }

    #[test]
    fn reads_legacy_tab_delimited_history() {
        let path = test_path();
        fs::write(&path, "12\tfrieren-123\tFrieren: Beyond Journey's End\n").unwrap();

        let entries = HistoryStore::new(&path).read().unwrap();

        assert_eq!(
            entries,
            vec![HistoryEntry::new(
                "12",
                "frieren-123",
                "Frieren: Beyond Journey's End"
            )]
        );
        remove_history_artifacts(&path);
    }

    #[test]
    fn writes_and_reads_history_entries() {
        let path = test_path();
        let store = HistoryStore::new(&path);
        let entries = vec![HistoryEntry::new("5", "frieren-123", "Frieren")];

        store.write(&entries).unwrap();

        assert_eq!(store.read().unwrap(), entries);
        remove_history_artifacts(&path);
    }

    #[test]
    fn ignores_malformed_history_lines() {
        let path = test_path();
        fs::write(&path, "broken-line\n5\tfrieren-123\tFrieren\n").unwrap();

        assert_eq!(HistoryStore::new(&path).read().unwrap().len(), 1);
        remove_history_artifacts(&path);
    }

    #[test]
    fn backs_up_legacy_ids_and_keeps_slug_entries_active() {
        let path = test_path();
        let store = HistoryStore::new(&path);
        store
            .write(&[
                HistoryEntry::new("4", "12345", "Old title"),
                HistoryEntry::new("7", "frieren-123", "Frieren"),
            ])
            .unwrap();

        store.backup_legacy_entries().unwrap();

        assert_eq!(
            store.read().unwrap(),
            vec![HistoryEntry::new("7", "frieren-123", "Frieren")]
        );
        assert_eq!(
            fs::read_to_string(suffix_path(&path, ".v4")).unwrap(),
            "4\t12345\tOld title\n"
        );
        remove_history_artifacts(&path);
    }

    #[test]
    fn appends_legacy_backups_and_removes_old_entries_from_active_history() {
        let path = test_path();
        let store = HistoryStore::new(&path);
        store
            .write(&[HistoryEntry::new("4", "12345", "First")])
            .unwrap();
        store.backup_legacy_entries().unwrap();
        store
            .write(&[HistoryEntry::new("9", "99999", "Second")])
            .unwrap();
        store.backup_legacy_entries().unwrap();

        assert_eq!(
            fs::read_to_string(suffix_path(&path, ".v4")).unwrap(),
            "4\t12345\tFirst\n9\t99999\tSecond\n"
        );
        assert!(store.read().unwrap().is_empty());
        assert!(fs::metadata(suffix_path(&path, ".lock")).is_err());
        remove_history_artifacts(&path);
    }

    #[test]
    fn slug_only_history_does_not_create_a_backup_file() {
        let path = test_path();
        let store = HistoryStore::new(&path);
        let entries = vec![HistoryEntry::new("5", "frieren-123", "Frieren")];
        store.write(&entries).unwrap();

        store.backup_legacy_entries().unwrap();

        assert_eq!(store.read().unwrap(), entries);
        assert!(fs::metadata(suffix_path(&path, ".v4")).is_err());
        remove_history_artifacts(&path);
    }

    #[test]
    fn sanitizes_delimiters_and_line_breaks() {
        assert_eq!(clean_field("a\tb\nc\rd"), "a b c d");
    }

    #[test]
    fn refuses_to_overwrite_when_lock_exists() {
        let path = test_path();
        let lock_path = suffix_path(&path, ".lock");
        fs::write(&lock_path, "").unwrap();

        let result = HistoryStore::new(&path).write(&[]);

        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        remove_history_artifacts(&path);
    }
    #[test]
    fn retrying_legacy_backup_does_not_duplicate_existing_rows() {
        let path = test_path();
        let store = HistoryStore::new(&path);
        let legacy = HistoryEntry::new("4", "12345", "Old title");

        store.write(std::slice::from_ref(&legacy)).unwrap();
        store.backup_legacy_entries().unwrap();

        // Simulate recovery from an interrupted migration that made the
        // legacy entry active again after its backup was already persisted.
        store.write(std::slice::from_ref(&legacy)).unwrap();
        store.backup_legacy_entries().unwrap();

        assert_eq!(
            fs::read_to_string(suffix_path(&path, ".v4")).unwrap(),
            "4\t12345\tOld title\n"
        );
        assert!(store.read().unwrap().is_empty());
        remove_history_artifacts(&path);
    }
}
