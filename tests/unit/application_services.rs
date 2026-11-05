use std::time::{SystemTime, UNIX_EPOCH};

use super::filter_anime_options;
use crate::application_services::{ApplicationError, SearchRequest, WatchApplication};
use crate::domain::{anime::Anime, episode::Episode, history::HistoryEntry};
use crate::infrastructure::history_store::HistoryStore;
use crate::providers::catalog::{AnimeOption, ProviderAnime};
use crate::providers::site::{Language, ProviderId};

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
      audio: None,
      subtitles: None,
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
  app
    .record_history(&anime, &first, ProviderId::HiAnime)
    .unwrap();
  app
    .record_history(&anime, &second, ProviderId::HiAnime)
    .unwrap();

  let entries = app.history().unwrap();
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].episode_number, "2");
  assert_eq!(entries[0].anime_id, anime.id);
  assert_eq!(entries[0].provider, None);

  std::fs::remove_file(path).unwrap();
}

#[test]
fn recording_hianime_history_replaces_legacy_entries_without_provider_metadata() {
  let path = temporary_history_path();
  let store = HistoryStore::new(path.clone());
  store
    .write(&[HistoryEntry::new("1", "dragon-ball", "Dragon Ball")])
    .unwrap();

  let app = WatchApplication::new(Some(path.clone()));
  let anime = Anime::new("dragon-ball", "Dragon Ball", "https://example.test/anime");
  let episode = Episode::new("2", None, "https://example.test/episode-2");
  app
    .record_history(&anime, &episode, ProviderId::HiAnime)
    .unwrap();

  let entries = app.history().unwrap();
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].episode_number, "2");
  assert_eq!(entries[0].provider, None);

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

#[test]
fn audio_and_subtitle_filters_are_applied_independently() {
  let english = ProviderAnime {
    provider: ProviderId::GoGoAnime,
    anime: Anime::new("english", "Title", "https://example.test/english"),
  };
  let spanish = ProviderAnime {
    provider: ProviderId::TioAnime,
    anime: Anime::new("spanish", "Title", "https://example.test/spanish"),
  };
  let options = vec![
    AnimeOption {
      label: "ja/en Title".to_owned(),
      sources: vec![english],
    },
    AnimeOption {
      label: "ja/es Title".to_owned(),
      sources: vec![spanish],
    },
  ];

  let audio_filtered = filter_anime_options(options.clone(), Some("ja"), None);
  assert_eq!(audio_filtered.len(), 2);

  let subtitle_filtered = filter_anime_options(options.clone(), None, Some("es"));
  assert_eq!(subtitle_filtered.len(), 1);
  assert_eq!(
    subtitle_filtered[0].sources[0].provider,
    ProviderId::TioAnime
  );

  let combined = filter_anime_options(options, Some("ja"), Some("en"));
  assert_eq!(combined.len(), 1);
  assert_eq!(combined[0].sources[0].provider, ProviderId::GoGoAnime);
}
