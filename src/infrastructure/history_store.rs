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

        Ok(content
            .lines()
            .filter_map(parse_history_line)
            .collect())
    }

    pub fn write(&self, entries: &[HistoryEntry]) -> io::Result<()> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;

        let lock_path = suffix_path(&self.path, ".lock");
        let _lock = LockFile::acquire(lock_path)?;
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
        OpenOptions::new().write(true).create_new(true).open(&path)?;
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ani-cli-history-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn reads_legacy_tab_delimited_history() {
        let path = test_path();
        fs::write(&path, "12\tfrieren\tFrieren: Beyond Journey's End\n").unwrap();

        let entries = HistoryStore::new(&path).read().unwrap();

        assert_eq!(entries, vec![HistoryEntry::new(
            "12",
            "frieren",
            "Frieren: Beyond Journey's End"
        )]);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn writes_and_reads_history_entries() {
        let path = test_path();
        let store = HistoryStore::new(&path);
        let entries = vec![HistoryEntry::new("5", "frieren", "Frieren")];

        store.write(&entries).unwrap();

        assert_eq!(store.read().unwrap(), entries);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn ignores_malformed_history_lines() {
        let path = test_path();
        fs::write(&path, "broken-line\n5\tfrieren\tFrieren\n").unwrap();

        assert_eq!(HistoryStore::new(&path).read().unwrap().len(), 1);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_to_overwrite_when_lock_exists() {
        let path = test_path();
        let lock_path = suffix_path(&path, ".lock");
        fs::write(&lock_path, "").unwrap();

        let result = HistoryStore::new(&path).write(&[]);

        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        fs::remove_file(lock_path).unwrap();
    }
}
