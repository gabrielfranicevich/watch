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

#[test]
fn grouped_search_results_keep_language_variants_separate_and_filter_sources() {
  use crate::providers::catalog::group_results;

  let sources = [
    (ProviderId::GoGoAnime, "frieren-en"),
    (ProviderId::AniWatch, "frieren-aniwatch"),
    (ProviderId::TioAnime, "frieren-es"),
    (ProviderId::JkAnime, "frieren-jk"),
  ]
  .into_iter()
  .map(|(provider, id)| ProviderAnime {
    provider,
    anime: Anime::new(
      id,
      "Frieren: Beyond Journey's End",
      format!("https://example.test/{id}"),
    ),
  })
  .collect();

  let grouped = group_results(sources);
  assert_eq!(grouped.len(), 2);
  assert_eq!(grouped[0].label, "(ja/en) Frieren: Beyond Journey's End");
  assert_eq!(grouped[1].label, "(ja/es) Frieren: Beyond Journey's End");
  assert_eq!(grouped[0].sources.len(), 2);
  assert_eq!(grouped[1].sources.len(), 2);

  let english = filter_anime_options(grouped.clone(), Some("ja"), Some("en"));
  assert_eq!(english.len(), 1);
  assert_eq!(english[0].sources.len(), 2);
  assert!(english[0].sources.iter().all(|source| {
    matches!(
        source.provider,
        ProviderId::GoGoAnime | ProviderId::AniWatch
      )
  }));

  let spanish = filter_anime_options(grouped, Some("ja"), Some("es"));
  assert_eq!(spanish.len(), 1);
  assert_eq!(spanish[0].sources.len(), 2);
  assert!(
    spanish[0]
      .sources
      .iter()
      .all(|source| { matches!(source.provider, ProviderId::TioAnime | ProviderId::JkAnime) })
  );
}

#[test]
fn combined_language_filters_remove_results_with_unknown_metadata() {
  use crate::domain::language::LanguagePair;
  use crate::providers::catalog::group_results;

  let mut unknown = ProviderAnime {
    provider: ProviderId::Gnula,
    anime: Anime::new("unknown", "Unknown title", "https://example.test/unknown"),
  };
  unknown.anime.language_pair = Some(LanguagePair::new(None, None));

  let grouped = group_results(vec![unknown]);
  assert_eq!(grouped.len(), 1);
  assert_eq!(grouped[0].label, "(?/?) Unknown title");
  assert!(filter_anime_options(grouped, None, Some("en")).is_empty());
}
