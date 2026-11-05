//! Reusable use cases shared by frontends such as the CLI and native Android UI.

use std::path::PathBuf;

use crate::domain::{
  anime::Anime, episode::Episode, history::HistoryEntry, language::LanguagePair, stream::Stream,
  subtitle::Subtitle,
};
use crate::infrastructure::history_store::HistoryStore;
use crate::providers::{
  catalog::{AnimeOption, ProviderAnime, ProviderCatalog},
  hianime_stream::{ResolvedMedia, StreamResolveError},
  site::{Language, ProviderId},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequest {
  pub query: String,
  pub language: Option<Language>,
  pub audio: Option<String>,
  pub subtitles: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackRequest {
  pub episode: Episode,
  pub provider: ProviderId,
  pub dubbed: bool,
  pub quality: String,
}

#[derive(Debug)]
pub enum ApplicationError {
  InvalidInput(String),
  Provider(String),
  History(std::io::Error),
  Stream(StreamResolveError),
}

impl std::fmt::Display for ApplicationError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::InvalidInput(message) | Self::Provider(message) => formatter.write_str(message),
      Self::History(error) => write!(formatter, "history operation failed: {error}"),
      Self::Stream(error) => write!(formatter, "stream resolution failed: {error}"),
    }
  }
}

impl std::error::Error for ApplicationError {}

impl From<std::io::Error> for ApplicationError {
  fn from(error: std::io::Error) -> Self {
    Self::History(error)
  }
}

impl From<StreamResolveError> for ApplicationError {
  fn from(error: StreamResolveError) -> Self {
    Self::Stream(error)
  }
}

#[derive(Debug, Clone)]
pub struct WatchApplication {
  catalog: ProviderCatalog,
  history_path: Option<PathBuf>,
}

impl Default for WatchApplication {
  fn default() -> Self {
    Self::new(None)
  }
}

impl WatchApplication {
  pub fn new(history_path: Option<PathBuf>) -> Self {
    Self {
      catalog: ProviderCatalog::new(),
      history_path,
    }
  }

  pub fn search(&self, request: &SearchRequest) -> Result<Vec<AnimeOption>, ApplicationError> {
    let query = request.query.trim();
    if query.is_empty() {
      return Err(ApplicationError::InvalidInput(
        "search query must not be empty".to_owned(),
      ));
    }
    let options = self
      .catalog
      .search_in_language(query, request.language)
      .map_err(|error| ApplicationError::Provider(error.to_string()))?;
    Ok(filter_anime_options(
      options,
      request.audio.as_deref(),
      request.subtitles.as_deref(),
    ))
  }

  pub fn episodes(&self, source: &ProviderAnime) -> Result<Vec<Episode>, ApplicationError> {
    self
      .catalog
      .episodes(source)
      .map_err(|error| ApplicationError::Provider(error.to_string()))
  }

  pub fn resolve_stream(
    &self,
    request: &PlaybackRequest,
  ) -> Result<ResolvedMedia, ApplicationError> {
    self
      .catalog
      .resolver(request.provider)
      .resolve(&request.episode, request.dubbed, &request.quality)
      .map_err(ApplicationError::from)
  }

  pub fn history(&self) -> Result<Vec<HistoryEntry>, ApplicationError> {
    let Some(path) = self.history_path.as_ref() else {
      return Ok(Vec::new());
    };
    HistoryStore::new(path.clone())
      .read()
      .map_err(ApplicationError::from)
  }

  pub fn record_history(
    &self,
    anime: &Anime,
    episode: &Episode,
    provider: ProviderId,
  ) -> Result<(), ApplicationError> {
    let Some(path) = self.history_path.as_ref() else {
      return Err(ApplicationError::InvalidInput(
        "history is unavailable because no history path was configured".to_owned(),
      ));
    };
    let store = HistoryStore::new(path.clone());
    let mut entries = store.read()?;
    let provider_key = provider.key();
    if let Some(existing) = entries.iter_mut().find(|entry| {
      entry.anime_id == anime.id && entry.provider.as_deref().unwrap_or("hianime") == provider_key
    }) {
      existing.episode_number = episode.number.clone();
      existing.anime_title = anime.title.clone();
    } else {
      let entry = if provider == ProviderId::HiAnime {
        HistoryEntry::new(
          episode.number.clone(),
          anime.id.clone(),
          anime.title.clone(),
        )
      } else {
        HistoryEntry::with_provider(
          episode.number.clone(),
          anime.id.clone(),
          anime.title.clone(),
          provider_key,
        )
      };
      entries.push(entry);
    }
    store.write(&entries).map_err(ApplicationError::from)
  }

  pub fn save_history(&self, entries: &[HistoryEntry]) -> Result<(), ApplicationError> {
    let Some(path) = self.history_path.as_ref() else {
      return Err(ApplicationError::InvalidInput(
        "history is unavailable because no history path was configured".to_owned(),
      ));
    };
    HistoryStore::new(path.clone())
      .write(entries)
      .map_err(ApplicationError::from)
  }
}

fn filter_anime_options(
  options: Vec<AnimeOption>,
  audio: Option<&str>,
  subtitles: Option<&str>,
) -> Vec<AnimeOption> {
  let audio = LanguagePair::new(audio, None).audio;
  let subtitles = LanguagePair::new(None, subtitles).subtitles;

  options
    .into_iter()
    .filter_map(|mut option| {
      option.sources.retain(|source| {
        let pair = source
          .anime
          .language_pair
          .clone()
          .or_else(|| source.provider.language_pair())
          .unwrap_or_else(|| LanguagePair::new(None, None));
        audio
          .as_ref()
          .is_none_or(|language| pair.audio.as_ref() == Some(language))
          && subtitles
            .as_ref()
            .is_none_or(|language| pair.subtitles.as_ref() == Some(language))
      });
      (!option.sources.is_empty()).then_some(option)
    })
    .collect()
}

pub fn playback_parts(media: ResolvedMedia) -> (Stream, Option<Subtitle>) {
  (media.stream, media.subtitle)
}

#[cfg(test)]
#[path = "../tests/unit/application_services.rs"]
mod tests;
