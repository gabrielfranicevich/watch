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
fn reads_and_writes_provider_aware_history_without_changing_legacy_rows() {
  let path = test_path();
  let store = HistoryStore::new(&path);
  let entries = vec![HistoryEntry::with_provider(
    "3",
    "anime/frieren",
    "Frieren",
    "monoschinos",
  )];

  store.write(&entries).unwrap();
  store.backup_legacy_entries().unwrap();

  assert_eq!(
    fs::read_to_string(&path).unwrap(),
    "3\tanime/frieren\tFrieren\tprovider:monoschinos\n"
  );
  assert_eq!(store.read().unwrap(), entries);
  assert!(!suffix_path(&path, ".v4").exists());
  remove_history_artifacts(&path);
}

#[test]
fn keeps_legacy_titles_with_extra_tabs_readable() {
  let path = test_path();
  fs::write(&path, "1\tfrieren-1\tA title\twith tabs\n").unwrap();

  let entries = HistoryStore::new(&path).read().unwrap();

  assert_eq!(entries[0].anime_title, "A title\twith tabs");
  assert_eq!(entries[0].provider, None);
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
