use std::{sync::mpsc, thread};

use super::hianime::HiAnimeProvider;
use super::site::{Language, ProviderId, ProviderStreamResolver, SiteProvider};
use crate::domain::{anime::Anime, episode::Episode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAnime {
  pub provider: ProviderId,
  pub anime: Anime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimeOption {
  pub label: String,
  pub sources: Vec<ProviderAnime>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProviderCatalog;

impl ProviderCatalog {
  pub fn new() -> Self {
    Self
  }

  pub fn search(&self, query: &str) -> Result<Vec<AnimeOption>, Box<dyn std::error::Error>> {
    let (sender, receiver) = mpsc::channel();
    for provider in ProviderId::ACTIVE {
      let query = query.to_owned();
      let sender = sender.clone();
      thread::spawn(move || {
        let result = if provider == ProviderId::HiAnime {
          HiAnimeProvider::new()
            .search(&query)
            .map_err(|error| error.to_string())
        } else {
          SiteProvider::new(provider)
            .search(&query)
            .map_err(|error| error.to_string())
        };
        let _ = sender.send((provider, result));
      });
    }
    drop(sender);

    let mut sources = Vec::new();
    let mut succeeded = 0usize;
    let mut failures = Vec::new();

    for _ in 0..ProviderId::ACTIVE.len() {
      match receiver.recv() {
        Ok((provider, Ok(results))) => {
          succeeded += 1;
          sources.extend(
            results
              .into_iter()
              .filter(|anime| !is_episode_search_result(&anime.title, &anime.url))
              .map(|anime| ProviderAnime { provider, anime }),
          );
        }
        Ok((provider, Err(error))) => {
          failures.push(format!("{}: {error}", provider.name()));
        }
        Err(_) => {
          failures.push("provider search worker terminated without returning a result".to_owned());
          break;
        }
      }
    }

    if succeeded == 0 {
      return Err(format!("all anime providers failed: {}", failures.join("; ")).into());
    }
    sort_sources_by_provider_order(&mut sources);
    let options = group_results(sources);
    if options.is_empty() {
      return Err(format!("no results found for '{query}' across available providers").into());
    }
    Ok(options)
  }

  pub fn episodes(
    &self,
    source: &ProviderAnime,
  ) -> Result<Vec<Episode>, Box<dyn std::error::Error>> {
    if source.provider == ProviderId::HiAnime {
      return Ok(HiAnimeProvider::new().episodes(&source.anime)?);
    }
    Ok(SiteProvider::new(source.provider).episodes(&source.anime)?)
  }

  pub fn resolver(&self, provider: ProviderId) -> ProviderStreamResolver {
    ProviderStreamResolver::new(provider)
  }
}

#[derive(Debug)]
struct TitleBucket {
  key: String,
  display_title: String,
  english: Vec<ProviderAnime>,
  spanish: Vec<ProviderAnime>,
}

fn group_results(sources: Vec<ProviderAnime>) -> Vec<AnimeOption> {
  let mut buckets: Vec<TitleBucket> = Vec::new();

  for source in sources {
    let key = normalize_provider_title(&source.anime.title);
    if key.is_empty() {
      continue;
    }
    let index = match buckets.iter().position(|bucket| bucket.key == key) {
      Some(index) => index,
      None => {
        buckets.push(TitleBucket {
          key: key.clone(),
          display_title: clean_display_title(&source.anime.title),
          english: Vec::new(),
          spanish: Vec::new(),
        });
        buckets.len() - 1
      }
    };

    let bucket = &mut buckets[index];
    let group = match source.provider.language() {
      Language::English => &mut bucket.english,
      Language::Spanish => &mut bucket.spanish,
    };
    if !group
      .iter()
      .any(|existing| existing.provider == source.provider)
    {
      group.push(source);
    }
  }

  let mut choices = Vec::new();
  for bucket in buckets {
    if !bucket.english.is_empty() {
      choices.push(AnimeOption {
        label: language_label(&bucket.display_title, Language::English),
        sources: bucket.english,
      });
    }
    if !bucket.spanish.is_empty() {
      choices.push(AnimeOption {
        label: language_label(&bucket.display_title, Language::Spanish),
        sources: bucket.spanish,
      });
    }
  }
  choices
}

fn sort_sources_by_provider_order(sources: &mut [ProviderAnime]) {
  sources.sort_by_key(|source| {
    ProviderId::ACTIVE
      .iter()
      .position(|provider| *provider == source.provider)
      .unwrap_or(usize::MAX)
  });
}

fn language_label(title: &str, language: Language) -> String {
  format!("({}) {title}", language.short_code())
}

fn is_episode_search_result(title: &str, url: &str) -> bool {
  let title_words: Vec<_> = title
    .split_whitespace()
    .map(|word| word.trim_matches(|character: char| !character.is_alphanumeric()))
    .collect();
  let title_is_episode = title_words.windows(2).any(|words| {
    matches!(
      words[0].to_ascii_lowercase().as_str(),
      "episode" | "episodio" | "capitulo"
    ) && words[1]
      .chars()
      .next()
      .is_some_and(|character| character.is_ascii_digit())
  });
  let path_is_episode = url.split(['?', '#']).next().is_some_and(|url| {
    let path = url.to_ascii_lowercase();
    [
      "/ver/",
      "/ver-",
      "/episode-",
      "-episode-",
      "/episodio-",
      "-episodio-",
      "/ep-",
    ]
    .iter()
    .any(|marker| path.contains(marker))
  });
  title_is_episode || path_is_episode
}

fn clean_display_title(title: &str) -> String {
  let mut title = title.trim().to_owned();
  for prefix in ["Serie En emision ", "Serie En emisión "] {
    if title.to_lowercase().starts_with(&prefix.to_lowercase()) {
      title = title[prefix.len()..].trim().to_owned();
      break;
    }
  }
  for suffix in [
    " (English Subbed)",
    " (English Dubbed)",
    " (Spanish Subbed)",
    " [English Subbed]",
    " [English Dubbed]",
    " [Spanish Subbed]",
    " (Sub Español)",
    " (Sub Espanol)",
    " [Sub Español]",
    " [Sub Espanol]",
    " (Español Latino)",
    " (Espanol Latino)",
    " [Español Latino]",
    " English Subbed",
    " English Dubbed",
    " English Sub",
    " English Dub",
    " Sub Español",
    " Sub Espanol",
    " Español Latino",
    " Espanol Latino",
    " Spanish Subbed",
    " Acción",
    " Accion",
  ] {
    if title.to_lowercase().ends_with(&suffix.to_lowercase()) {
      title.truncate(title.len() - suffix.len());
      title = title
        .trim()
        .trim_end_matches([':', '-', '–', '—', '|'])
        .trim()
        .to_owned();
    }
  }
  title
}

fn normalize_provider_title(title: &str) -> String {
  let title = clean_display_title(title)
    .to_lowercase()
    .replace("&amp;", "&")
    .replace("&#039;", "'")
    .replace("&quot;", "\"");
  title
    .chars()
    .filter(|character| character.is_alphanumeric())
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn source(provider: ProviderId, title: &str, slug: &str) -> ProviderAnime {
    ProviderAnime {
      provider,
      anime: Anime::new(slug, title, format!("{}/{}", provider.base_url(), slug)),
    }
  }

  #[test]
  fn two_english_providers_collapse_to_one_english_label() {
    let choices = group_results(vec![
      source(ProviderId::GoGoAnime, "Dragon Ball Super: Beerus", "beerus"),
      source(ProviderId::NineAnime, "Dragon Ball Super Beerus", "beerus"),
    ]);
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].label, "(en) Dragon Ball Super: Beerus");
    assert_eq!(choices[0].sources.len(), 2);
  }

  #[test]
  fn two_spanish_providers_collapse_to_one_spanish_label() {
    let choices = group_results(vec![
      source(ProviderId::TioAnime, "Dragon Ball Super: Beerus", "beerus"),
      source(ProviderId::JkAnime, "Dragon Ball Super Beerus", "beerus"),
    ]);
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].label, "(es) Dragon Ball Super: Beerus");
    assert_eq!(choices[0].sources.len(), 2);
  }

  #[test]
  fn bilingual_title_is_listed_in_english_then_spanish_order() {
    let choices = group_results(vec![
      source(
        ProviderId::NineAnime,
        "Frieren: Beyond Journey's End",
        "frieren-en",
      ),
      source(
        ProviderId::MonosChinos,
        "Frieren: Beyond Journey's End",
        "frieren-es",
      ),
    ]);
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].label, "(en) Frieren: Beyond Journey's End");
    assert_eq!(choices[1].label, "(es) Frieren: Beyond Journey's End");
    assert_eq!(choices[0].sources.len(), 1);
    assert_eq!(choices[1].sources.len(), 1);
  }

  #[test]
  fn unambiguous_title_includes_its_language_label() {
    let choices = group_results(vec![source(ProviderId::NineAnime, "Frieren", "frieren")]);
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].label, "(en) Frieren");
  }

  #[test]
  fn episode_pages_and_episode_titled_results_are_not_listed_as_anime() {
    assert!(is_episode_search_result(
      "Episodio 1 Dragon Ball Super: Beerus 2026-10-11",
      "https://tioanime.com/ver/dragon-ball-super-beerus-1"
    ));
    assert!(is_episode_search_result(
      "Dragon Ball Super: Beerus",
      "https://jkanime.surf/dragon-ball-super-beerus-episodio-3/"
    ));
    assert!(is_episode_search_result(
      "Miss Kobayashi's Dragon Maid Episode 14: Valentine's Day",
      "https://hianime.at/watch/miss-kobayashi-episode-14"
    ));
    assert!(!is_episode_search_result(
      "Dragon Ball Z Special 1: Bardock, The Father of Goku",
      "https://hianime.at/watch/dragon-ball-z-special-1"
    ));
  }

  #[test]
  fn strips_search_genre_and_status_prefixes_before_adding_language_labels() {
    let choices = group_results(vec![
      source(
        ProviderId::HiAnime,
        "Dragon Ball Super: Beerus",
        "beerus-en",
      ),
      source(
        ProviderId::TioAnime,
        "Dragon Ball Super: Beerus Acción",
        "beerus-es",
      ),
      source(
        ProviderId::JkAnime,
        "Serie En emision Dragon Ball Super: Beerus",
        "beerus-jk",
      ),
    ]);
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].label, "(en) Dragon Ball Super: Beerus");
    assert_eq!(choices[1].label, "(es) Dragon Ball Super: Beerus");
    assert_eq!(choices[1].sources.len(), 2);
  }

  #[test]
  fn grouping_discards_duplicate_cards_from_the_same_provider() {
    let choices = group_results(vec![
      source(ProviderId::NineAnime, "Frieren", "frieren"),
      source(ProviderId::NineAnime, "Frieren", "frieren-copy"),
      source(ProviderId::GoGoAnime, "Frieren", "frieren-two"),
    ]);
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].label, "(en) Frieren");
    assert_eq!(choices[0].sources.len(), 2);
  }

  #[test]
  fn search_choice_order_does_not_depend_on_provider_completion_order() {
    let mut sources = vec![
      source(ProviderId::JkAnime, "Beerus", "jkanime"),
      source(ProviderId::HiAnime, "Beerus", "hianime"),
      source(ProviderId::MonosChinos, "Beerus", "monoschinos"),
      source(ProviderId::TioAnime, "Beerus", "tioanime"),
    ];
    sort_sources_by_provider_order(&mut sources);
    assert_eq!(
      sources
        .iter()
        .map(|source| source.provider)
        .collect::<Vec<_>>(),
      vec![
        ProviderId::HiAnime,
        ProviderId::TioAnime,
        ProviderId::JkAnime,
        ProviderId::MonosChinos,
      ]
    );
  }

  #[test]
  fn normalizes_punctuation_and_common_language_suffixes() {
    assert_eq!(
      normalize_provider_title("My Hero Academia: English Subbed"),
      normalize_provider_title("My Hero Academia")
    );
    assert_eq!(
      normalize_provider_title("Frieren: Beyond Journey's End"),
      normalize_provider_title("Frieren Beyond Journey s End")
    );
  }
}
