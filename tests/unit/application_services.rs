use std::time::{SystemTime, UNIX_EPOCH};

use ani_cli::application_services::{ApplicationError, SearchRequest, WatchApplication};
use ani_cli::domain::{anime::Anime, episode::Episode};
use ani_cli::providers::site::{Language, ProviderId};

fn temporary_history_path() -> std::path::PathBuf {
  let nonce = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  std::env::temp_dir().join(format!("watch-app-history-{}-{nonce}", std::process::id()))
}

#[test]
fn search_rejects_empty_queries_without_network_access() {
  let app = WatchApplication::default();
  let error = app
    .search(&SearchRequest {
      query: "  ".to_owned(),
      language: Some(Language::English),
    })
    .unwrap_err();
  assert!(matches!(error, ApplicationError::InvalidInput(_)));
}

#[test]
fn history_can_be_read_recorded_and_updated_through_application_api() {
  let path = temporary_history_path();
  let app = WatchApplication::new(Some(path.clone()));
  let anime = Anime::new("dragon-ball", "Dragon Ball", "https://example.test/anime");
  let first = Episode::new("1", None, "https://example.test/episode-1");
  let second = Episode::new("2", None, "https://example.test/episode-2");

  assert!(app.history().unwrap().is_empty());
  app.record_history(&anime, &first, ProviderId::HiAnime).unwrap();
  app.record_history(&anime, &second, ProviderId::HiAnime).unwrap();

  let entries = app.history().unwrap();
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].episode_number, "2");
  assert_eq!(entries[0].anime_id, anime.id);
  assert_eq!(entries[0].provider.as_deref(), Some("hianime"));

  std::fs::remove_file(path).unwrap();
}

#[test]
fn history_operations_report_missing_configuration() {
  let app = WatchApplication::default();
  let anime = Anime::new("title", "Title", "https://example.test/title");
  let episode = Episode::new("1", None, "https://example.test/episode");

  assert!(matches!(
    app.record_history(&anime, &episode, ProviderId::HiAnime),
    Err(ApplicationError::InvalidInput(_))
  ));
}
